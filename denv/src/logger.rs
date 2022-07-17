use log::{set_boxed_logger, set_max_level, Level, Log, Metadata, Record, SetLoggerError};

pub struct Logger {
    lvl: Level,
}

impl Logger {
    pub fn init(lvl: Level) -> Result<(), SetLoggerError> {
        set_boxed_logger(Box::new(Self::new(lvl)))?;
        set_max_level(lvl.to_level_filter());
        Ok(())
    }

    pub fn new(lvl: Level) -> Self {
        Self { lvl }
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() >= self.lvl
    }

    fn log(&self, record: &Record) {
        let color = match record.level() {
            Level::Trace => "\x1b[1;30m",
            Level::Debug => "\x1b[1;32m",
            Level::Info => "\x1b[1;34m",
            Level::Warn => "\x1b[1;33m",
            Level::Error => "\x1b[1;31m",
        };
        println!(
            "{}[{}] {}\x1b",
            color,
            env!("CARGO_PKG_NAME"),
            record.args()
        );
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
                let expected = Level::Trace;
                let logger = Logger::new(expected);
                assert_eq!(logger.lvl, expected);
            }
        }
    }
}
