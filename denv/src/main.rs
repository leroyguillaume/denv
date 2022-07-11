mod logger;

use denv_lib::cfg::Config;
use log::Level;
use logger::Logger;
use std::path::Path;

macro_rules! exit_if_err {
    ($result:expr, $code:expr) => {
        match $result {
            Ok(res) => res,
            Err(err) => {
                log::error!("{}", err);
                std::process::exit($code);
            }
        }
    };
}

fn main() {
    Logger::init(Level::Trace).unwrap();
    let _cfg = exit_if_err!(Config::load(Path::new("denv.yaml")), exitcode::CONFIG);
}
