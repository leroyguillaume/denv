use crate::util::{cache::*, downloader::*};
use std::path::Path;

pub struct Config {
    cache: Box<dyn Cache>,
    downloader: Box<dyn Downloader>,
}

impl Config {
    pub fn new(home_dirpath: &Path) -> Self {
        Self {
            cache: Box::new(DefaultCache::new(home_dirpath)),
            downloader: Box::new(DefaultDownloader),
        }
    }

    pub fn cache(&self) -> &dyn Cache {
        self.cache.as_ref()
    }

    pub fn downloader(&self) -> &dyn Downloader {
        self.downloader.as_ref()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    mod config {
        use super::*;

        mod new {
            use super::*;

            #[test]
            fn should_return_cfg() {
                let home_dirpath = Path::new("/");
                let cfg = Config::new(home_dirpath);
                assert_eq!(cfg.cache.path(), home_dirpath);
            }
        }
    }
}
