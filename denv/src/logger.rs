use log::{
    set_boxed_logger, set_max_level, Level, LevelFilter, Log, Metadata, Record, SetLoggerError,
};
use std::{
    io::{self, Stdout, Write},
    sync::Mutex,
};

const APP_NAME: &str = env!("CARGO_PKG_NAME");
const TRACE_COLOR: &str = "\x1b[0;30m";
const DEBUG_COLOR: &str = "\x1b[0;34m";
const INFO_COLOR: &str = "\x1b[0;32m";
const WARN_COLOR: &str = "\x1b[0;33m";
const ERROR_COLOR: &str = "\x1b[0;31m";

pub struct Logger<W: Write + Sync + Send> {
    lvl: LevelFilter,
    with_color: bool,
    out: Mutex<W>,
}

impl<W: Write + Sync + Send> Logger<W> {
    pub fn new(lvl: LevelFilter, with_color: bool, out: W) -> Self {
        Self {
            lvl,
            with_color,
            out: Mutex::new(out),
        }
    }
}

impl Logger<Stdout> {
    pub fn init(lvl: LevelFilter, with_color: bool) -> Result<(), SetLoggerError> {
        set_boxed_logger(Box::new(Self::new(lvl, with_color, io::stdout())))?;
        set_max_level(lvl);
        Ok(())
    }
}

impl<W: Write + Sync + Send> Log for Logger<W> {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.lvl
    }

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

    fn flush(&self) {}
}

#[cfg(test)]
mod test {
    use super::*;

    mod logger {
        use super::*;

        mod new {
            use super::*;

            #[test]
            fn should_return_logger() {
                let lvl = LevelFilter::Trace;
                let with_color = true;
                let logger = Logger::new(lvl, with_color, vec![]);
                assert_eq!(logger.lvl, lvl);
                assert_eq!(logger.with_color, with_color);
            }
        }

        mod enabled {
            use super::*;

            #[test]
            fn should_return_false() {
                let logger = Logger::new(LevelFilter::Error, true, vec![]);
                let metadata = Metadata::builder().level(Level::Warn).build();
                assert!(!logger.enabled(&metadata));
            }

            #[test]
            fn should_return_true() {
                let logger = Logger::new(LevelFilter::Warn, true, vec![]);
                let metadata = Metadata::builder().level(Level::Error).build();
                assert!(logger.enabled(&metadata));
            }
        }

        mod log {
            use super::*;

            macro_rules! test {
                ($ident:ident, $with_color:expr) => {
                    #[test]
                    fn $ident() {
                        let out = vec![];
                        let logger = Logger::new(LevelFilter::Trace, $with_color, out);
                        let trace_record = Record::builder()
                            .level(Level::Trace)
                            .args(format_args!("trace"))
                            .build();
                        let debug_record = Record::builder()
                            .level(Level::Debug)
                            .args(format_args!("debug"))
                            .build();
                        let info_record = Record::builder()
                            .level(Level::Info)
                            .args(format_args!("info"))
                            .build();
                        let warn_record = Record::builder()
                            .level(Level::Warn)
                            .args(format_args!("warn"))
                            .build();
                        let error_record = Record::builder()
                            .level(Level::Error)
                            .args(format_args!("error"))
                            .build();
                        logger.log(&trace_record);
                        logger.log(&debug_record);
                        logger.log(&info_record);
                        logger.log(&warn_record);
                        logger.log(&error_record);
                        let expected = if $with_color {
                            format!("{}[{}] trace\x1b\n{}[{}] debug\x1b\n{}[{}] info\x1b\n{}[{}] warn\x1b\n{}[{}] error\x1b\n", TRACE_COLOR, APP_NAME, DEBUG_COLOR, APP_NAME, INFO_COLOR, APP_NAME, WARN_COLOR, APP_NAME, ERROR_COLOR, APP_NAME)
                        } else {
                            format!("[{}] trace\n[{}] debug\n[{}] info\n[{}] warn\n[{}] error\n", APP_NAME, APP_NAME, APP_NAME, APP_NAME, APP_NAME)
                        };
                        let out = logger.out.lock().unwrap();
                        let logs = String::from_utf8_lossy(&out);
                        assert_eq!(logs, expected);
                    }
                };
            }

            test!(should_return_colorized_logs, true);
            test!(should_return_non_colorized_logs, false);
        }
    }
}
