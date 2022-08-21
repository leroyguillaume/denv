// IMPORTS

use log::{
    set_boxed_logger, set_max_level, Level, LevelFilter, Log, Metadata, Record, SetLoggerError,
};
use std::{
    io::{self, Stderr, Write},
    sync::Mutex,
};

// CONSTS

const APP_NAME: &str = env!("CARGO_PKG_NAME");
const DEBUG_COLOR: &str = "\x1b[0;34m";
const ERROR_COLOR: &str = "\x1b[0;31m";
const INFO_COLOR: &str = "\x1b[0;32m";
const TRACE_COLOR: &str = "\x1b[0;30m";
const WARN_COLOR: &str = "\x1b[0;33m";

// STRUCTS

pub struct Logger<W: Write + Sync + Send> {
    level: LevelFilter,
    out: Mutex<W>,
    with_color: bool,
}

impl Logger<Stderr> {
    pub fn init(level: LevelFilter, with_color: bool) -> Result<(), SetLoggerError> {
        let logger = Self {
            level,
            out: Mutex::new(io::stderr()),
            with_color,
        };
        set_boxed_logger(Box::new(logger))?;
        set_max_level(level);
        Ok(())
    }
}

impl<W: Write + Sync + Send> Log for Logger<W> {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn flush(&self) {}

    fn log(&self, record: &Record) {
        let log = format!("[{}] {}", APP_NAME, record.args());
        let log = if self.with_color {
            let color = match record.level() {
                Level::Trace => TRACE_COLOR,
                Level::Debug => DEBUG_COLOR,
                Level::Info => INFO_COLOR,
                Level::Warn => WARN_COLOR,
                Level::Error => ERROR_COLOR,
            };
            format!("{}{}\x1b", color, log)
        } else {
            log
        };
        if let Ok(ref mut out) = self.out.lock() {
            writeln!(out, "{}", log).unwrap();
        }
    }
}

// TESTS

#[cfg(test)]
mod logger_test {
    use super::*;

    mod enabled {
        use super::*;

        struct Data {
            level: LevelFilter,
            log_level: Level,
        }

        #[test]
        fn should_return_false() {
            test(
                Data {
                    level: LevelFilter::Error,
                    log_level: Level::Warn,
                },
                |_, res| assert!(!res),
            );
        }

        #[test]
        fn should_return_true() {
            test(
                Data {
                    level: LevelFilter::Warn,
                    log_level: Level::Error,
                },
                |_, res| assert!(res),
            );
        }

        #[inline]
        fn test<F: Fn(Data, bool)>(data: Data, assert_fn: F) {
            let logger = Logger {
                level: data.level,
                out: Mutex::new(vec![]),
                with_color: true,
            };
            let metadata = Metadata::builder().level(data.log_level).build();
            assert_fn(data, logger.enabled(&metadata));
        }
    }

    mod log {
        use super::*;

        struct Data {
            logs: Vec<(Level, &'static str)>,
            with_color: bool,
        }

        impl Data {
            pub fn new(with_color: bool) -> Self {
                Self {
                    logs: vec![
                        (Level::Trace, "trace"),
                        (Level::Debug, "debug"),
                        (Level::Info, "info"),
                        (Level::Warn, "warn"),
                        (Level::Error, "error"),
                    ],
                    with_color,
                }
            }
        }

        #[test]
        fn should_return_colorized_logs() {
            test(Data::new(true), |data, res| {
                let mut logs = data
                    .logs
                    .iter()
                    .map(|(level, log)| {
                        let color = match level {
                            Level::Trace => TRACE_COLOR,
                            Level::Debug => DEBUG_COLOR,
                            Level::Info => INFO_COLOR,
                            Level::Warn => WARN_COLOR,
                            Level::Error => ERROR_COLOR,
                        };
                        format!("{}[{}] {}\x1b", color, APP_NAME, log)
                    })
                    .reduce(|logs, log| format!("{}\n{}", logs, log))
                    .unwrap();
                logs.push('\n');
                assert_eq!(res, logs);
            });
        }

        #[test]
        fn should_return_uncolorized_logs() {
            test(Data::new(false), |data, res| {
                let mut logs = data
                    .logs
                    .iter()
                    .map(|(_, log)| format!("[{}] {}", APP_NAME, log))
                    .reduce(|logs, log| format!("{}\n{}", logs, log))
                    .unwrap();
                logs.push('\n');
                assert_eq!(res, logs);
            });
        }

        #[inline]
        fn test<F: Fn(Data, String)>(data: Data, assert_fn: F) {
            let logger = Logger {
                level: LevelFilter::Trace,
                out: Mutex::new(vec![]),
                with_color: data.with_color,
            };
            for (level, log) in data.logs.iter() {
                logger.log(
                    &Record::builder()
                        .level(*level)
                        .args(format_args!("{}", log))
                        .build(),
                );
            }
            let out = logger.out.into_inner().unwrap();
            let res = String::from_utf8(out).unwrap();
            assert_fn(data, res);
        }
    }
}
