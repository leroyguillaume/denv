mod logger;

use denv_lib::cfg::{Config, LoadingError};
use log::{debug, error, info, Level};
use logger::Logger;
use std::{path::Path, process::exit};

fn main() {
    Logger::init(Level::Trace).unwrap();
    let cfg_filepath = Path::new("denv.yaml");
    let cfg = match Config::load(cfg_filepath) {
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
    for software in cfg.softwares() {
        if software.is_installed(&cfg) {
            debug!("{} is already installed", software);
        } else {
            match software.install(&cfg) {
                Ok(_) => {}
                Err(err) => error!("Unable to install {}: {}", software, err),
            };
        }
        match software.add_to_path(&cfg) {
            Ok(()) => info!("{}", software),
            Err(err) => error!("Unable to add {} to path: {}", software, err),
        }
    }
}
