mod logger;

use clap::Parser;
use clap_verbosity_flag::Verbosity;
use denv_lib::cfg::{Config, LoadingError};
use home::home_dir;
use log::{debug, error, info};
use logger::Logger;
use std::{env::temp_dir, path::PathBuf, process::exit};

#[derive(Parser)]
#[clap(name = "D-Env", author, version, about)]
struct Args {
    #[clap(
        short = 'f',
        long = "config",
        name = "CONFIG_FILE",
        help = "Configuration file"
    )]
    cfg_filepath: Option<PathBuf>,

    #[clap(long, help = "Disable logs color")]
    no_color: bool,

    #[clap(long = "denv-directory", name = "DENV_DIR", help = "D-Env directory")]
    denv_dirpath: Option<PathBuf>,

    #[clap(long = "tmp-directory", name = "TMP_DIR", help = "Temporary directory")]
    tmp_dirpath: Option<PathBuf>,

    #[clap(flatten)]
    verbose: Verbosity,
}

fn main() {
    let args = Args::parse();
    let denv_dirpath = args.denv_dirpath.unwrap_or_else(|| match home_dir() {
        Some(home_dirpath) => home_dirpath.join(".denv"),
        None => {
            error!("Unable to get user home directory");
            exit(exitcode::UNAVAILABLE);
        }
    });
    let tmp_dirpath = args.tmp_dirpath.unwrap_or_else(|| temp_dir().join("denv"));
    Logger::init(args.verbose.log_level_filter(), !args.no_color).unwrap();
    let cfg_filepath = args.cfg_filepath.unwrap_or_else(|| {
        let path = PathBuf::from("denv.yml");
        if path.exists() {
            path
        } else {
            PathBuf::from("denv.yaml")
        }
    });
    let cfg = match Config::load(&cfg_filepath, denv_dirpath, tmp_dirpath) {
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
