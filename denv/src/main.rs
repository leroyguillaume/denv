mod logger;

use clap::Parser;
use clap_verbosity_flag::Verbosity;
use denv_lib::{
    cfg::Config,
    error::{ConfigLoadError, EnvironmentLoadError},
    *,
};
use home::home_dir;
use log::error;
use logger::Logger;
use std::{
    env::{current_dir, temp_dir},
    path::PathBuf,
    process::exit,
};

#[derive(Parser)]
#[clap(name = "D-Env", author, version, about)]
struct Args {
    #[clap(long = "denv-directory", name = "DENV_DIR", help = "D-Env directory")]
    denv_dirpath: Option<PathBuf>,

    #[clap(short = 'p', long = "path", help = "Display environment path")]
    display_env_dirpath: bool,

    #[clap(
        short = 'f',
        long = "config",
        name = "CONFIG_FILE",
        help = "Configuration file"
    )]
    cfg_filepath: Option<PathBuf>,

    #[clap(long = "no-color", help = "Disable logs color")]
    logs_color_disabled: bool,

    #[clap(long = "tmp-directory", name = "TMP_DIR", help = "Temporary directory")]
    tmp_dirpath: Option<PathBuf>,

    #[clap(flatten)]
    verbose: Verbosity,
}

fn main() {
    let args = Args::parse();
    let cur_dirpath = match current_dir() {
        Ok(cur_dirpath) => cur_dirpath,
        Err(err) => {
            error!("Unable to get current working directory: {}", err);
            exit(exitcode::UNAVAILABLE);
        }
    };
    let denv_dirpath = args.denv_dirpath.unwrap_or_else(|| match home_dir() {
        Some(home_dirpath) => home_dirpath.join(".denv"),
        None => {
            error!("Unable to get user home directory");
            exit(exitcode::UNAVAILABLE);
        }
    });
    let tmp_dirpath = args.tmp_dirpath.unwrap_or_else(|| temp_dir().join("denv"));
    Logger::init(args.verbose.log_level_filter(), !args.logs_color_disabled).unwrap();
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
        Err(ConfigLoadError::FileReadingFailed(err)) => {
            error!("Unable to open {}: {}", cfg_filepath.display(), err);
            exit(exitcode::CONFIG);
        }
        Err(ConfigLoadError::InvalidConfig(errs)) => {
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
    let env = Environment::new(cur_dirpath);
    match env.load(&cfg) {
        Ok(()) => {
            if args.display_env_dirpath {
                println!("{}", env.path(&cfg).display());
            }
            exit(exitcode::OK)
        }
        Err(EnvironmentLoadError::EnvFileWritingFailed(err)) => {
            error!("Unable to write env file: {}", err);
            exit(exitcode::IOERR);
        }
        Err(_) => exit(exitcode::SOFTWARE),
    }
}
