// IMPORTS

use clap::{Args, Parser};
use log::LevelFilter;

// DATA STRUCTS

#[derive(Clone, Debug, Eq, PartialEq, Parser)]
#[clap(name = "D-Env", author, version, about)]
pub struct Cli {
    #[clap(flatten)]
    pub opts: Options,
}

#[derive(Args, Clone, Debug, Default, Eq, PartialEq)]
pub struct Options {
    #[clap(long, help = "Disable logs color")]
    pub no_color: bool,

    #[clap(flatten)]
    pub verbosity: Verbosity,
}

#[derive(Args, Clone, Debug, Default, Eq, PartialEq)]
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
}
