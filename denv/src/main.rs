mod logger;

use clap::Parser;
use clap_verbosity_flag::Verbosity;
use denv_lib::cfg::{Config, LoadingError};
use log::{debug, error, info};
use logger::Logger;
use std::{path::PathBuf, process::exit};

#[derive(Parser)]
#[clap(author, version, about)]
struct Args {
    #[clap(short = 'f', long = "config", help = "Configuration file")]
    cfg_filepath: Option<PathBuf>,

    #[clap(long, help = "Disable logs color")]
    no_color: bool,

    #[clap(flatten)]
    verbose: Verbosity,
}

fn main() {
    let args = Args::parse();
    Logger::init(args.verbose.log_level_filter(), !args.no_color).unwrap();
    let cfg_filepath = args.cfg_filepath.unwrap_or_else(|| {
        let path = PathBuf::from("denv.yml");
        if path.exists() {
            path
        } else {
            PathBuf::from("denv.yaml")
        }
    });
    let cfg = match Config::load(&cfg_filepath) {
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
