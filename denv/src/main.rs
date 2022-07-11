mod logger;

use log::{debug, error, info, trace, warn, Level};
use logger::Logger;

fn main() {
    Logger::init(Level::Trace).unwrap();
    trace!("Hello, world!");
    debug!("Hello, world!");
    info!("Hello, world!");
    warn!("Hello, world!");
    error!("Hello, world!");
}
