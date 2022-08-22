// IMPORTS

use crate::{
    cfg::{self, Config, ConfigLoader, DefaultConfigLoader},
    cli::{Command, Options, Shell},
};
use std::{
    env,
    fmt::{self, Display, Formatter},
    io::{self, Stdout, Write},
    path::PathBuf,
    sync::Mutex,
};

// MACROS

macro_rules! write {
    ($out:expr, $($arg:tt)*) => {{
        std::write!($out, $($arg)*).map_err(|err| Error::Io(err))
    }};
}

// TYPES

pub type Result<T> = std::result::Result<T, Error>;

type ArgsFn = dyn Fn() -> Vec<String>;

// CONSTS

const DENV_CWD_VAR_NAME: &str = "DENV_CWD";

// ENUMS

#[derive(Debug)]
pub enum Error {
    Config(cfg::Error),
    Io(io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Config(err) => std::write!(f, "Unable to load configuration: {}", err),
            Self::Io(err) => std::write!(f, "{}", err),
        }
    }
}

// STRUCTS

pub struct Runner<W: Write> {
    args_fn: Box<ArgsFn>,
    cfg_loader: Box<dyn ConfigLoader>,
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
    fn load_config(&self, path: Option<PathBuf>) -> Result<Config> {
        let path = path.unwrap_or_else(|| PathBuf::from("denv.yml"));
        self.cfg_loader.load(path).map_err(Error::Config)
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
        let _ = self.load_config(opts.cfg_filepath)?;
        unimplemented!();
    }
}

impl Default for Runner<Stdout> {
    fn default() -> Self {
        Self {
            args_fn: Box::new(|| env::args().collect()),
            cfg_loader: Box::new(DefaultConfigLoader),
            out: Mutex::new(io::stdout()),
        }
    }
}

// TESTS

#[cfg(test)]
mod error_test {
    use super::*;

    mod to_string {

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
        use super::*;

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
    use crate::{cfg::StubConfigLoader, test::WriteFailer};

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
                    cfg_loader: Box::new(StubConfigLoader),
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
    }
}
