// IMPORTS

use crate::{
    cfg::{self, ConfigLoader, DefaultConfigLoader, SoftwareDefinition, VarDefinition},
    cli::{Command, Options, Shell},
    fs::{DefaultFileSystem, FileSystem},
    soft::{Error as SoftwareError, Software},
    var::{Error as VarError, Var},
};
use log::debug;
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
    Install(Vec<InstallError>),
    Io(io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Compute(_) => std::write!(f, "Unable to compute value of some variables"),
            Self::Config(err) => std::write!(f, "Unable to load configuration: {}", err),
            Self::Install(_) => std::write!(f, "Unable to install some softwares"),
            Self::Io(err) => std::write!(f, "{}", err),
        }
    }
}

// STRUCTS

pub struct ComputeError {
    cause: VarError,
    var: Box<dyn Var>,
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
    cause: SoftwareError,
    soft: Box<dyn Software>,
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
    create_fs_fn: Box<CreateFsFn>,
    env_var_fn: Box<EnvVarFn>,
    out: Mutex<W>,
}

impl<W: Write> Runner<W> {
    pub fn run(&self, cmd: Command, opts: Options) -> Result<()> {
        match cmd {
            Command::Hook(shell) => self.run_hook(shell),
            Command::Load => self.run_load(opts),
        }
    }

    #[inline]
    fn install_softwares(
        &self,
        env_path: &Path,
        soft_defs: Vec<SoftwareDefinition>,
        fs: &dyn FileSystem,
    ) -> Result<()> {
        let mut install_errs = vec![];
        for soft_def in soft_defs {
            let soft = soft_def.into_software();
            let bin_paths = soft.binary_paths(fs);
            let install_res = if soft.is_installed(fs) {
                debug!("{} v{} is already installed", soft.name(), soft.version());
                Ok(())
            } else {
                soft.install(fs)
                    .map_err(|err| InstallError { cause: err, soft })
            };
            match install_res {
                Err(err) => install_errs.push(err),
                Ok(_) => {
                    for bin_path in bin_paths {
                        let filename = bin_path.file_name().unwrap();
                        let dest = env_path.join(filename);
                        fs.ensure_symlink(&bin_path, &dest).map_err(Error::Io)?;
                    }
                }
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
        env_path: &Path,
        cfg_path: &Path,
        var_defs: Vec<VarDefinition>,
        fs: &dyn FileSystem,
    ) -> Result<()> {
        let cwd = fs.cwd().map_err(Error::Io)?;
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
            let var = var_def.into_var();
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
        let env_path = fs.ensure_env_dir().map_err(Error::Io)?;
        self.install_softwares(&env_path, cfg.soft_defs, fs)?;
        self.print_export_statements(&env_path, &cfg_path, cfg.var_defs, fs)
    }
}

impl Default for Runner<Stdout> {
    fn default() -> Self {
        Self {
            args_fn: Box::new(|| env::args().collect()),
            cfg_loader: Box::new(DefaultConfigLoader),
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
                let str = format!("Unable to load configuration: {}", err);
                let err = Error::Config(err);
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
        test::WriteFailer,
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
                opts: Options,
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
                        opts: Options {
                            cfg_filepath: Some(PathBuf::from("/config")),
                            ..Options::default()
                        },
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
                    let mut stubs = Self {
                        cfg_loader: StubConfigLoader::default(),
                        create_fs_fn: Box::new(|| Box::new(StubFileSystem::default())),
                        env_var_fn: Box::new(|var_name| env::var(var_name)),
                    };
                    stubs.cfg_loader.with_load_fn(move |path| {
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
                    .with_load_fn(|_| Err(cfg::Error::Version(None)));
                test(vec![], data.opts, stubs, |_, res| match res.unwrap_err() {
                    Error::Config(_) => {}
                    err => panic!("{}", err),
                });
            }

            #[inline]
            fn test<W: Write, F: Fn(W, Result<()>)>(
                out: W,
                opts: Options,
                stubs: Stubs,
                assert_fn: F,
            ) {
                let runner = Runner {
                    args_fn: Box::new(|| env::args().collect()),
                    cfg_loader: Box::new(stubs.cfg_loader),
                    create_fs_fn: stubs.create_fs_fn,
                    env_var_fn: stubs.env_var_fn,
                    out: Mutex::new(out),
                };
                let res = runner.run(Command::Load, opts);
                let out = runner.out.into_inner().unwrap();
                assert_fn(out, res);
            }
        }
    }
}
