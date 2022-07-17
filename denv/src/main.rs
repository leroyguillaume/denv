mod logger;

use denv_lib::cfg::{Config, LoadingError};
use log::{error, Level};
use logger::Logger;
use std::{path::Path, process::exit};

fn main() {
    Logger::init(Level::Trace).unwrap();
    let cfg_filepath = Path::new("denv.yaml");
    let _cfg = match Config::load(cfg_filepath) {
        Ok(cfg) => cfg,
        Err(LoadingError::FileOpeningFailed(err)) => {
            error!("Unable to open {}: {}", cfg_filepath.display(), err);
            exit(exitcode::CONFIG);
        }
        Err(LoadingError::InvalidConfig(errs)) => {
            error!("Invalid configuration file:");
            for err in errs {
                error!("  - {}", err);
            }
            exit(exitcode::CONFIG);
        }
        Err(err) => {
            error!("{}", err);
            exit(exitcode::CONFIG);
        }
    };
}
