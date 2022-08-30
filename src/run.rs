// IMPORTS

use crate::{
    cfg::{self, ConfigLoader, DefaultConfigLoader, SoftwareDefinition, VarDefinition},
    cli::{Command, Options, Shell},
    fs::{DefaultFileSystem, FileSystem},
    soft::{Error as SoftwareError, Software},
    var::{Error as VarError, Var},
};
use std::{
    env,
    fmt::{self, Debug, Display, Formatter},
    io::{self, Stdout, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

// MACROS

macro_rules! write {
    ($out:expr, $($arg:tt)*) => {{
        std::write!($out, $($arg)*).map_err(|err| Error::Io(err))
    }};
}

macro_rules! writeln {
    ($out:expr, $($arg:tt)*) => {{
        std::writeln!($out, $($arg)*).map_err(|err| Error::Io(err))
    }};
}

// TYPES

pub type Result<T> = std::result::Result<T, Error>;

type ArgsFn = dyn Fn() -> Vec<String>;

type CreateFsFn = dyn Fn() -> Box<dyn FileSystem>;

type ConvertSoftFn = dyn Fn(SoftwareDefinition) -> Box<dyn Software>;

type ConvertVarFn = dyn Fn(VarDefinition) -> Box<dyn Var>;

type EnvVarFn = dyn Fn(&str) -> std::result::Result<String, env::VarError>;

// CONSTS

const DENV_CFG_FILE_VAR_NAME: &str = "DENV_CONFIG_FILE";
const DENV_CWD_VAR_NAME: &str = "DENV_CWD";
const DENV_PATH_BACKUP_VAR_NAME: &str = "DENV_PATH_BACKUP";
const PATH_VAR_NAME: &str = "PATH";

// ENUMS

#[derive(Debug)]
pub enum Error {
    Compute(Vec<ComputeError>),
    Config(cfg::Error),
    EnvNotLoaded(env::VarError),
    Install(Vec<InstallError>),
    Io(io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Compute(_) => std::write!(f, "Unable to compute value of some variables"),
            Self::Config(err) => std::write!(f, "{}", err),
            Self::EnvNotLoaded(_) => std::write!(f, "No environment loaded"),
            Self::Install(_) => std::write!(f, "Unable to install some softwares"),
            Self::Io(err) => std::write!(f, "{}", err),
        }
    }
}

// STRUCTS

pub struct ComputeError {
    pub cause: VarError,
    pub var: Box<dyn Var>,
}

impl Debug for ComputeError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("ComputeError")
            .field("cause", &self.cause)
            .field("var", &self.var.name())
            .finish()
    }
}

pub struct InstallError {
    pub cause: SoftwareError,
    pub soft: Box<dyn Software>,
}

impl Debug for InstallError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let soft_str = format!("{} v{}", self.soft.name(), self.soft.version());
        f.debug_struct("InstallError")
            .field("cause", &self.cause)
            .field("soft", &soft_str)
            .finish()
    }
}

pub struct Runner<W: Write> {
    args_fn: Box<ArgsFn>,
    cfg_loader: Box<dyn ConfigLoader>,
    convert_soft_fn: Box<ConvertSoftFn>,
    convert_var_fn: Box<ConvertVarFn>,
    create_fs_fn: Box<CreateFsFn>,
    env_var_fn: Box<EnvVarFn>,
    out: Mutex<W>,
}

impl<W: Write> Runner<W> {
    pub fn run(&self, cmd: Command, opts: Options) -> Result<()> {
        match cmd {
            Command::Hook(shell) => self.run_hook(shell),
            Command::Load => self.run_load(opts),
            Command::Unload => self.run_unload(),
        }
    }

    #[inline]
    fn install_softwares(
        &self,
        cwd: &Path,
        soft_defs: Vec<SoftwareDefinition>,
        fs: &dyn FileSystem,
    ) -> Result<()> {
        let mut install_errs = vec![];
        for soft_def in soft_defs {
            let soft = (self.convert_soft_fn)(soft_def);
            let res = soft
                .install(cwd, fs)
                .map_err(|err| InstallError { cause: err, soft });
            if let Err(err) = res {
                install_errs.push(err);
            }
        }
        if install_errs.is_empty() {
            Ok(())
        } else {
            Err(Error::Install(install_errs))
        }
    }

    #[inline]
    fn print_export_statements(
        &self,
        cwd: &Path,
        env_path: &Path,
        cfg_path: &Path,
        var_defs: Vec<VarDefinition>,
    ) -> Result<()> {
        let mut out = self.out.lock().unwrap();
        writeln!(out, "export {}='{}'", DENV_CWD_VAR_NAME, cwd.display())?;
        writeln!(
            out,
            "export {}='{}'",
            DENV_CFG_FILE_VAR_NAME,
            cfg_path.display()
        )?;
        writeln!(
            out,
            "export {}='{}'",
            DENV_PATH_BACKUP_VAR_NAME,
            (self.env_var_fn)(PATH_VAR_NAME).unwrap_or_default(),
        )?;
        writeln!(
            out,
            "export {}=\"{}:${{{}}}\"",
            PATH_VAR_NAME,
            env_path.display(),
            PATH_VAR_NAME
        )?;
        let mut compute_errs = vec![];
        for var_def in var_defs {
            let var = (self.convert_var_fn)(var_def);
            let var_name: String = var.name().into();
            let compute_res = var
                .compute_value()
                .map_err(|err| ComputeError { cause: err, var });
            match compute_res {
                Err(err) => compute_errs.push(err),
                Ok(value) => writeln!(out, "export {}='{}'", var_name, value,)?,
            }
        }
        if compute_errs.is_empty() {
            Ok(())
        } else {
            Err(Error::Compute(compute_errs))
        }
    }

    #[inline]
    fn run_hook(&self, shell: Shell) -> Result<()> {
        let mut args = (self.args_fn)().into_iter();
        let program = args.next().unwrap();
        let opts = args
            .filter(|arg| arg.starts_with('-'))
            .reduce(|cli, arg| format!(" {} {}", cli, arg))
            .unwrap_or_default();
        let cli = format!("{}{}", program, opts);
        let template = match shell {
            Shell::Bash => include_str!("../resources/main/hooks/bash"),
            Shell::Zsh => include_str!("../resources/main/hooks/zsh"),
        };
        let statement = template
            .replace("<denv_cwd_var_name>", DENV_CWD_VAR_NAME)
            .replace("<load_cmd>", &cli)
            .replace("<unload_cmd>", &format!("{} unload", cli));
        let mut out = self.out.lock().unwrap();
        write!(out, "{}", statement)
    }

    #[inline]
    fn run_load(&self, opts: Options) -> Result<()> {
        let cfg_path = opts
            .cfg_filepath
            .unwrap_or_else(|| PathBuf::from("denv.yml"));
        let cfg = self.cfg_loader.load(&cfg_path).map_err(Error::Config)?;
        let fs = (self.create_fs_fn)();
        let fs = fs.as_ref();
        let cwd = fs.cwd().map_err(Error::Io)?;
        let env_dirpath = fs.ensure_env_dir(&cwd).map_err(Error::Io)?;
        self.install_softwares(&cwd, cfg.soft_defs, fs)?;
        self.print_export_statements(&cwd, &env_dirpath, &cfg_path, cfg.var_defs)
    }

    #[inline]
    fn run_unload(&self) -> Result<()> {
        let cfg_path = (self.env_var_fn)(DENV_CFG_FILE_VAR_NAME)
            .map(PathBuf::from)
            .map_err(Error::EnvNotLoaded)?;
        let cfg = self.cfg_loader.load(&cfg_path).map_err(Error::Config)?;
        let fs = (self.create_fs_fn)();
        let fs = fs.as_ref();
        let cwd = fs.cwd().map_err(Error::Io)?;
        let mut out = self.out.lock().unwrap();
        writeln!(
            out,
            "export {}='{}'",
            PATH_VAR_NAME, DENV_PATH_BACKUP_VAR_NAME
        )?;
        writeln!(out, "unset {}", DENV_CWD_VAR_NAME)?;
        writeln!(out, "unset {}", DENV_CFG_FILE_VAR_NAME)?;
        writeln!(out, "unset {}", DENV_PATH_BACKUP_VAR_NAME)?;
        for var_def in cfg.var_defs {
            writeln!(out, "unset {}", var_def.name)?;
        }
        fs.delete_env_dir(&cwd).map_err(Error::Io)
    }
}

impl Default for Runner<Stdout> {
    fn default() -> Self {
        Self {
            args_fn: Box::new(|| env::args().collect()),
            cfg_loader: Box::new(DefaultConfigLoader),
            convert_soft_fn: Box::new(SoftwareDefinition::into_software),
            convert_var_fn: Box::new(VarDefinition::into_var),
            create_fs_fn: Box::new(|| Box::new(DefaultFileSystem)),
            env_var_fn: Box::new(|var_name| env::var(var_name)),
            out: Mutex::new(io::stdout()),
        }
    }
}

// TESTS

#[cfg(test)]
mod error_test {
    use super::*;

    mod to_string {
        use super::*;

        mod compute {
            use super::*;

            #[test]
            fn should_return_str() {
                let str = "Unable to compute value of some variables";
                let err = Error::Compute(vec![]);
                assert_eq!(err.to_string(), str);
            }
        }

        mod config {
            use super::*;

            #[test]
            fn should_return_str() {
                let err = cfg::Error::Version(None);
                let str = err.to_string();
                let err = Error::Config(err);
                assert_eq!(err.to_string(), str);
            }
        }

        mod env_not_loaded {
            use super::*;

            #[test]
            fn should_return_str() {
                let str = "No environment loaded";
                let err = Error::EnvNotLoaded(env::VarError::NotPresent);
                assert_eq!(err.to_string(), str);
            }
        }

        mod install {
            use super::*;

            #[test]
            fn should_return_str() {
                let str = "Unable to install some softwares";
                let err = Error::Install(vec![]);
                assert_eq!(err.to_string(), str);
            }
        }

        mod io {
            use super::*;

            #[test]
            fn should_return_str() {
                let err = ::std::io::Error::from(std::io::ErrorKind::PermissionDenied);
                let str = err.to_string();
                let err = Error::Io(err);
                assert_eq!(err.to_string(), str);
            }
        }
    }
}

#[cfg(test)]
mod runner_test {
    use super::*;
    use crate::{
        cfg::{
            Config, SoftwareDefinition, SoftwareDefinitionKind, StubConfigLoader, VarDefinition,
            VarDefinitionKind,
        },
        fs::StubFileSystem,
        soft::StubSoftware,
        test::WriteFailer,
        var::StubVar,
    };
    use std::path::Path;

    mod run {
        use super::*;

        mod hook {
            use super::*;

            macro_rules! tests {
                ($ident:ident, $shell:expr, $template:literal) => {
                    mod $ident {
                        use super::*;

                        #[test]
                        fn should_return_io_err() {
                            test(
                                $shell,
                                WriteFailer,
                                vec!["denv".into(), "hook".into(), stringify!($ident).into()],
                                |_, res| {
                                    let err = res.unwrap_err();
                                    match err {
                                        Error::Io(_) => {}
                                        err => panic!("{}", err),
                                    }
                                },
                            );
                        }

                        #[test]
                        fn should_return_ok_with_opts() {
                            let opt1 = "-vvvv";
                            let opt2 = "--no-color";
                            let args = vec![
                                "denv".into(),
                                opt1.into(),
                                opt2.into(),
                                "hook".into(),
                                stringify!($ident).into(),
                            ];
                            test($shell, vec![], args.clone(), |out, res| {
                                let cli = format!("{} {} {}", args[0], opt1, opt2);
                                verify(out, res, cli, include_str!($template));
                            });
                        }

                        #[test]
                        fn should_return_ok_without_opts() {
                            let args =
                                vec!["denv".into(), "hook".into(), stringify!($ident).into()];
                            test($shell, vec![], args.clone(), |out, res| {
                                verify(out, res, args[0].clone(), include_str!($template));
                            });
                        }
                    }
                };
            }

            tests!(bash, Shell::Bash, "../resources/main/hooks/bash");
            tests!(zsh, Shell::Zsh, "../resources/main/hooks/zsh");

            #[inline]
            fn test<W: Write, F: Fn(W, Result<()>)>(
                shell: Shell,
                out: W,
                args: Vec<String>,
                assert_fn: F,
            ) {
                let runner = Runner {
                    args_fn: Box::new(move || args.clone()),
                    cfg_loader: Box::new(StubConfigLoader::default()),
                    convert_soft_fn: Box::new(SoftwareDefinition::into_software),
                    convert_var_fn: Box::new(VarDefinition::into_var),
                    create_fs_fn: Box::new(|| Box::new(StubFileSystem::default())),
                    env_var_fn: Box::new(|var_name| env::var(var_name)),
                    out: Mutex::new(out),
                };
                let res = runner.run(Command::Hook(shell), Options::default());
                let out = runner.out.into_inner().unwrap();
                assert_fn(out, res);
            }

            #[inline]
            fn verify(out: Vec<u8>, res: Result<()>, cli: String, template: &str) {
                res.unwrap();
                let out = String::from_utf8(out).unwrap();
                let statement = template
                    .replace("<denv_cwd_var_name>", DENV_CWD_VAR_NAME)
                    .replace("<load_cmd>", &cli)
                    .replace("<unload_cmd>", &format!("{} unload", cli));
                assert_eq!(out, statement);
            }
        }

        mod load {
            use super::*;

            struct Data {
                cfg: Config,
                cfg_path: &'static Path,
                cwd: &'static Path,
                env_dirpath: &'static Path,
                opts: Options,
                path_env_var_value: &'static str,
                soft_name: &'static str,
                var_name: &'static str,
                var_value: &'static str,
            }

            impl Default for Data {
                fn default() -> Self {
                    Self {
                        cfg: Config {
                            soft_defs: vec![SoftwareDefinition {
                                kind: SoftwareDefinitionKind::Terraform,
                                version: "1.2.3".into(),
                            }],
                            var_defs: vec![VarDefinition {
                                kind: VarDefinitionKind::Literal("value".into()),
                                name: "var".into(),
                            }],
                        },
                        cfg_path: Path::new("/config"),
                        cwd: Path::new("/cwd"),
                        env_dirpath: Path::new("/env"),
                        opts: Options {
                            cfg_filepath: Some(PathBuf::from("/config")),
                            ..Options::default()
                        },
                        path_env_var_value: "path",
                        soft_name: "soft1",
                        var_name: "var1",
                        var_value: "value1",
                    }
                }
            }

            struct Stubs {
                cfg_loader: StubConfigLoader,
                convert_soft_fn: Box<ConvertSoftFn>,
                convert_var_fn: Box<ConvertVarFn>,
                create_fs_fn: Box<CreateFsFn>,
                env_var_fn: Box<EnvVarFn>,
            }

            impl Stubs {
                fn new(data: &Data) -> Self {
                    let cfg = data.cfg.clone();
                    let cfg_path = data.cfg_path;
                    let cwd = data.cwd;
                    let env_dirpath = data.env_dirpath;
                    let expected_soft_def = cfg.soft_defs[0].clone();
                    let path_env_var_value = data.path_env_var_value;
                    let soft_name = data.soft_name;
                    let expected_var_def = cfg.var_defs[0].clone();
                    let var_name = data.var_name;
                    let var_value = data.var_value;
                    let mut stubs = Self {
                        cfg_loader: StubConfigLoader::default(),
                        convert_soft_fn: Box::new(move |soft_def| {
                            assert_eq!(soft_def, expected_soft_def);
                            Box::new(stub_software(soft_name, cwd))
                        }),
                        convert_var_fn: Box::new(move |var_def| {
                            assert_eq!(var_def, expected_var_def);
                            Box::new(stub_var(var_name, var_value))
                        }),
                        create_fs_fn: Box::new(|| Box::new(stub_fs(cwd, env_dirpath))),
                        env_var_fn: Box::new(|var_name| match var_name {
                            PATH_VAR_NAME => Ok(path_env_var_value.into()),
                            _ => panic!("unexpected {}", var_name),
                        }),
                    };
                    stubs.cfg_loader.stub_load_fn(move |path| {
                        assert_eq!(path, cfg_path);
                        Ok(cfg.clone())
                    });
                    stubs
                }
            }

            #[test]
            fn should_return_config_err() {
                let data = Data::default();
                let mut stubs = Stubs::new(&data);
                stubs
                    .cfg_loader
                    .stub_load_fn(|_| Err(cfg::Error::Version(None)));
                test(vec![], &data.opts, stubs, |_, res| match res.unwrap_err() {
                    Error::Config(_) => {}
                    err => panic!("{}", err),
                });
            }

            #[test]
            fn should_return_io_err_if_cwd_failed() {
                let data = Data::default();
                let cwd = data.cwd;
                let env_dirpath = data.env_dirpath;
                let mut stubs = Stubs::new(&data);
                stubs.create_fs_fn = Box::new(|| {
                    let mut fs = stub_fs(cwd, env_dirpath);
                    fs.stub_cwd_fn(|| Err(io::Error::from(io::ErrorKind::PermissionDenied)));
                    Box::new(fs)
                });
                test(vec![], &data.opts, stubs, |_, res| match res.unwrap_err() {
                    Error::Io(_) => {}
                    err => panic!("{}", err),
                });
            }

            #[test]
            fn should_return_io_err_if_ensure_env_dir_failed() {
                let data = Data::default();
                let cwd = data.cwd;
                let env_dirpath = data.env_dirpath;
                let mut stubs = Stubs::new(&data);
                stubs.create_fs_fn = Box::new(|| {
                    let mut fs = stub_fs(cwd, env_dirpath);
                    fs.stub_ensure_env_dir_fn(|_| {
                        Err(io::Error::from(io::ErrorKind::PermissionDenied))
                    });
                    Box::new(fs)
                });
                test(vec![], &data.opts, stubs, |_, res| match res.unwrap_err() {
                    Error::Io(_) => {}
                    err => panic!("{}", err),
                });
            }

            #[test]
            fn should_return_install_err_if_install_failed() {
                let data = Data::default();
                let cwd = data.cwd;
                let soft_name = data.soft_name;
                let mut stubs = Stubs::new(&data);
                stubs.convert_soft_fn = Box::new(move |_| {
                    let mut soft = stub_software(soft_name, cwd);
                    soft.stub_install_fn(|_, _| Err(SoftwareError::UnsupportedSystem));
                    Box::new(soft)
                });
                test(vec![], &data.opts, stubs, |_, res| match res.unwrap_err() {
                    Error::Install(errs) => {
                        assert_eq!(errs.len(), 1);
                        let err = &errs[0];
                        assert_eq!(err.soft.name(), data.soft_name);
                        match &err.cause {
                            SoftwareError::UnsupportedSystem => {}
                            err => panic!("{}", err),
                        }
                    }
                    err => panic!("{}", err),
                });
            }

            #[test]
            fn should_return_io_err_if_write_on_output_failed() {
                let data = Data::default();
                let stubs = Stubs::new(&data);
                test(WriteFailer, &data.opts, stubs, |_, res| {
                    match res.unwrap_err() {
                        Error::Io(_) => {}
                        err => panic!("{}", err),
                    }
                });
            }

            #[test]
            fn should_return_install_err_if_compute_failed() {
                let data = Data::default();
                let var_name = data.var_name;
                let var_value = data.var_value;
                let mut stubs = Stubs::new(&data);
                stubs.convert_var_fn = Box::new(|_| {
                    let mut var = stub_var(var_name, var_value);
                    var.stub_compute_value_fn(|| Err(VarError::Stub));
                    Box::new(var)
                });
                test(vec![], &data.opts, stubs, |_, res| match res.unwrap_err() {
                    Error::Compute(errs) => {
                        assert_eq!(errs.len(), 1);
                        let err = &errs[0];
                        assert_eq!(err.var.name(), data.var_name);
                        match err.cause {
                            VarError::Stub => {}
                        }
                    }
                    err => panic!("{}", err),
                });
            }

            #[test]
            fn should_return_install_ok_with_opts() {
                let data = Data::default();
                let stubs = Stubs::new(&data);
                test(vec![], &data.opts, stubs, |out, res| {
                    verify(&data, out, res);
                });
            }

            #[test]
            fn should_return_install_ok_without_opts() {
                let data = Data {
                    cfg_path: Path::new("denv.yml"),
                    opts: Options::default(),
                    ..Data::default()
                };
                let stubs = Stubs::new(&data);
                test(vec![], &data.opts, stubs, |out, res| {
                    verify(&data, out, res);
                });
            }

            #[inline]
            fn stub_fs(cwd: &'static Path, env_dirpath: &'static Path) -> StubFileSystem {
                let mut fs = StubFileSystem::default();
                fs.stub_cwd_fn(|| Ok(cwd.to_path_buf()));
                fs.stub_ensure_env_dir_fn(move |project_dirpath| {
                    assert_eq!(project_dirpath, cwd);
                    Ok(env_dirpath.to_path_buf())
                });
                fs
            }

            #[inline]
            fn stub_software(name: &'static str, cwd: &'static Path) -> StubSoftware {
                let mut soft = StubSoftware::default();
                soft.stub_install_fn(move |project_dirpath, _| {
                    assert_eq!(project_dirpath, cwd);
                    Ok(())
                });
                soft.stub_name_fn(move || name);
                soft
            }

            #[inline]
            fn stub_var(name: &'static str, value: &'static str) -> StubVar {
                let mut var = StubVar::default();
                var.stub_compute_value_fn(|| Ok(value.into()));
                var.stub_name_fn(move || name);
                var
            }

            #[inline]
            fn test<W: Write, F: Fn(W, Result<()>)>(
                out: W,
                opts: &Options,
                stubs: Stubs,
                assert_fn: F,
            ) {
                let runner = Runner {
                    args_fn: Box::new(|| env::args().collect()),
                    cfg_loader: Box::new(stubs.cfg_loader),
                    convert_soft_fn: stubs.convert_soft_fn,
                    convert_var_fn: stubs.convert_var_fn,
                    create_fs_fn: stubs.create_fs_fn,
                    env_var_fn: stubs.env_var_fn,
                    out: Mutex::new(out),
                };
                let res = runner.run(Command::Load, opts.clone());
                let out = runner.out.into_inner().unwrap();
                assert_fn(out, res);
            }

            #[inline]
            fn verify(data: &Data, out: Vec<u8>, res: Result<()>) {
                let expected_out = format!(
                    "export {}='{}'\nexport {}='{}'\nexport {}='{}'\nexport {}=\"{}:${{{}}}\"\nexport {}='{}'\n",
                    DENV_CWD_VAR_NAME,
                    data.cwd.display(),
                    DENV_CFG_FILE_VAR_NAME,
                    data.cfg_path.display(),
                    DENV_PATH_BACKUP_VAR_NAME,
                    data.path_env_var_value,
                    PATH_VAR_NAME,
                    data.env_dirpath.display(),
                    PATH_VAR_NAME,
                    data.var_name,
                    data.var_value,
                );
                res.unwrap();
                let out = String::from_utf8(out).unwrap();
                assert_eq!(out, expected_out);
            }
        }

        mod unload {
            use super::*;

            struct Data {
                cfg: Config,
                cfg_path: &'static str,
                cwd: &'static Path,
            }

            impl Default for Data {
                fn default() -> Self {
                    Self {
                        cfg: Config {
                            soft_defs: vec![],
                            var_defs: vec![VarDefinition {
                                kind: VarDefinitionKind::Literal("value".into()),
                                name: "var".into(),
                            }],
                        },
                        cfg_path: "/config",
                        cwd: Path::new("/cwd"),
                    }
                }
            }

            struct Stubs {
                cfg_loader: StubConfigLoader,
                create_fs_fn: Box<CreateFsFn>,
                env_var_fn: Box<EnvVarFn>,
            }

            impl Stubs {
                fn new(data: &Data) -> Self {
                    let cfg = data.cfg.clone();
                    let cfg_path = data.cfg_path;
                    let cwd = data.cwd;
                    let mut stubs = Self {
                        cfg_loader: StubConfigLoader::default(),
                        create_fs_fn: Box::new(|| Box::new(stub_fs(cwd))),
                        env_var_fn: Box::new(|var_name| match var_name {
                            DENV_CFG_FILE_VAR_NAME => Ok(cfg_path.into()),
                            _ => panic!("unexpected {}", var_name),
                        }),
                    };
                    stubs.cfg_loader.stub_load_fn(move |path| {
                        assert_eq!(path, Path::new(cfg_path));
                        Ok(cfg.clone())
                    });
                    stubs
                }
            }

            #[test]
            fn should_return_env_not_loaded_err() {
                let data = Data::default();
                let mut stubs = Stubs::new(&data);
                stubs.env_var_fn = Box::new(|_| Err(env::VarError::NotPresent));
                test(vec![], stubs, |_, res| match res.unwrap_err() {
                    Error::EnvNotLoaded(_) => {}
                    err => panic!("{}", err),
                });
            }

            #[test]
            fn should_return_config_err() {
                let data = Data::default();
                let mut stubs = Stubs::new(&data);
                stubs
                    .cfg_loader
                    .stub_load_fn(|_| Err(cfg::Error::Version(None)));
                test(vec![], stubs, |_, res| match res.unwrap_err() {
                    Error::Config(_) => {}
                    err => panic!("{}", err),
                });
            }

            #[test]
            fn should_return_io_err_if_cwd_failed() {
                let data = Data::default();
                let cwd = data.cwd;
                let mut stubs = Stubs::new(&data);
                stubs.create_fs_fn = Box::new(|| {
                    let mut fs = stub_fs(cwd);
                    fs.stub_cwd_fn(|| Err(io::Error::from(io::ErrorKind::PermissionDenied)));
                    Box::new(fs)
                });
                test(vec![], stubs, |_, res| match res.unwrap_err() {
                    Error::Io(_) => {}
                    err => panic!("{}", err),
                });
            }

            #[test]
            fn should_return_io_err_if_write_on_output_failed() {
                let data = Data::default();
                let stubs = Stubs::new(&data);
                test(WriteFailer, stubs, |_, res| match res.unwrap_err() {
                    Error::Io(_) => {}
                    err => panic!("{}", err),
                });
            }

            #[test]
            fn should_return_io_err_if_delete_env_dir_failed() {
                let data = Data::default();
                let cwd = data.cwd;
                let mut stubs = Stubs::new(&data);
                stubs.create_fs_fn = Box::new(|| {
                    let mut fs = stub_fs(cwd);
                    fs.stub_delete_env_dir_fn(|_| {
                        Err(io::Error::from(io::ErrorKind::PermissionDenied))
                    });
                    Box::new(fs)
                });
                test(vec![], stubs, |_, res| match res.unwrap_err() {
                    Error::Io(_) => {}
                    err => panic!("{}", err),
                });
            }

            #[test]
            fn should_return_install_ok() {
                let data = Data::default();
                let stubs = Stubs::new(&data);
                test(vec![], stubs, |out, res| {
                    verify(&data, out, res);
                });
            }

            #[inline]
            fn stub_fs(cwd: &'static Path) -> StubFileSystem {
                let mut fs = StubFileSystem::default();
                fs.stub_cwd_fn(|| Ok(cwd.to_path_buf()));
                fs.stub_delete_env_dir_fn(move |project_dirpath| {
                    assert_eq!(project_dirpath, cwd);
                    Ok(())
                });
                fs
            }

            #[inline]
            fn test<W: Write, F: Fn(W, Result<()>)>(out: W, stubs: Stubs, assert_fn: F) {
                let runner = Runner {
                    args_fn: Box::new(|| env::args().collect()),
                    cfg_loader: Box::new(stubs.cfg_loader),
                    convert_soft_fn: Box::new(SoftwareDefinition::into_software),
                    convert_var_fn: Box::new(VarDefinition::into_var),
                    create_fs_fn: stubs.create_fs_fn,
                    env_var_fn: stubs.env_var_fn,
                    out: Mutex::new(out),
                };
                let opts = Options::default();
                let res = runner.run(Command::Unload, opts);
                let out = runner.out.into_inner().unwrap();
                assert_fn(out, res);
            }

            #[inline]
            fn verify(data: &Data, out: Vec<u8>, res: Result<()>) {
                let expected_out = format!(
                    "export {}='{}'\nunset {}\nunset {}\nunset {}\nunset {}\n",
                    PATH_VAR_NAME,
                    DENV_PATH_BACKUP_VAR_NAME,
                    DENV_CWD_VAR_NAME,
                    DENV_CFG_FILE_VAR_NAME,
                    DENV_PATH_BACKUP_VAR_NAME,
                    data.cfg.var_defs[0].name,
                );
                res.unwrap();
                let out = String::from_utf8(out).unwrap();
                assert_eq!(out, expected_out);
            }
        }
    }
}
