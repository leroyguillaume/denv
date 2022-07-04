macro_rules! debug_err {
    ($result:expr) => {
        if log::log_enabled!(log::Level::Debug) {
            match $result {
                Ok(res) => Ok(res),
                Err(err) => {
                    log::debug!("{}", err);
                    Err(err)
                }
            }
        } else {
            $result
        }
    };
}
pub(crate) use debug_err;

macro_rules! map_debug_err {
    ($result:expr, $map:expr) => {
        match $result {
            Ok(res) => Ok(res),
            Err(err) => {
                let err = $map(err);
                log::debug!("{}", err);
                Err(err)
            }
        }
    };
}
pub(crate) use map_debug_err;

pub mod cfg;
pub mod tool;
pub mod util;
