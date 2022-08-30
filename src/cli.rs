// IMPORTS

use clap::{Args, Parser, Subcommand};
use log::LevelFilter;
use std::path::PathBuf;

// ENUMS

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
pub enum Command {
    #[clap(subcommand)]
    Hook(Shell),

    #[clap(about = "Print shell export statements")]
    Load,

    #[clap(about = "Print shell unset statements")]
    Unload,
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[clap(about = "Print shell hook statement")]
pub enum Shell {
    #[clap(about = "Print bash hook statement")]
    Bash,

    #[clap(about = "Print ZSH hook statement")]
    Zsh,
}

// DATA STRUCTS

#[derive(Clone, Debug, Eq, PartialEq, Parser)]
#[clap(name = "D-Env", author, version, about)]
pub struct Cli {
    #[clap(subcommand)]
    pub cmd: Command,

    #[clap(flatten)]
    pub opts: Options,
}

#[derive(Args, Clone, Debug, Default, Eq, PartialEq)]
pub struct Options {
    #[clap(short = 'f', long = "config", help = "Override configuration file")]
    pub cfg_filepath: Option<PathBuf>,

    #[clap(long, help = "Disable logs color")]
    pub no_color: bool,

    #[clap(flatten)]
    pub verbosity: Verbosity,
}

#[derive(Args, Clone, Debug, Eq, PartialEq)]
pub struct Verbosity {
    #[clap(
        short = 'v',
        parse(from_occurrences),
        help = "Print more logs per occurrence"
    )]
    pub count: u8,

    #[clap(
        short = 'q',
        long = "quiet",
        help = "Don't print any logs",
        conflicts_with = "count"
    )]
    pub quiet: bool,
}

impl Default for Verbosity {
    fn default() -> Self {
        Self {
            count: 1,
            quiet: false,
        }
    }
}

impl Verbosity {
    pub fn to_log_level(&self) -> LevelFilter {
        if self.quiet {
            LevelFilter::Off
        } else {
            match self.count {
                0 => LevelFilter::Error,
                1 => LevelFilter::Warn,
                2 => LevelFilter::Info,
                3 => LevelFilter::Debug,
                _ => LevelFilter::Trace,
            }
        }
    }
}

// TESTS

#[cfg(test)]
mod verbosity_test {
    use super::*;

    mod to_log_level {
        use super::*;

        #[test]
        fn should_return_off() {
            let verbosity = Verbosity {
                count: 4,
                quiet: true,
            };
            assert_eq!(verbosity.to_log_level(), LevelFilter::Off);
        }

        #[test]
        fn should_return_error() {
            let verbosity = Verbosity {
                count: 0,
                quiet: false,
            };
            assert_eq!(verbosity.to_log_level(), LevelFilter::Error);
        }

        #[test]
        fn should_return_warn() {
            let verbosity = Verbosity {
                count: 1,
                quiet: false,
            };
            assert_eq!(verbosity.to_log_level(), LevelFilter::Warn);
        }

        #[test]
        fn should_return_info() {
            let verbosity = Verbosity {
                count: 2,
                quiet: false,
            };
            assert_eq!(verbosity.to_log_level(), LevelFilter::Info);
        }

        #[test]
        fn should_return_debug() {
            let verbosity = Verbosity {
                count: 3,
                quiet: false,
            };
            assert_eq!(verbosity.to_log_level(), LevelFilter::Debug);
        }

        #[test]
        fn should_return_trace() {
            let verbosity = Verbosity {
                count: 4,
                quiet: false,
            };
            assert_eq!(verbosity.to_log_level(), LevelFilter::Trace);
        }
    }

    mod default {
        use super::*;

        #[test]
        fn should_return_verbosity() {
            let verbosity = Verbosity {
                count: 1,
                quiet: false,
            };
            assert_eq!(Verbosity::default(), verbosity);
        }
    }
}
