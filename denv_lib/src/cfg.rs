use crate::util::{downloader::*, fs::*, zip::*};
use std::{env::temp_dir, path::Path};

pub struct Config {
    fs: Box<dyn Fs>,
    downloader: Box<dyn Downloader>,
    unziper: Box<dyn Unziper>,
}

impl Config {
    pub fn new(home_dirpath: &Path) -> Self {
        Self {
            fs: Box::new(DefaultFs::new(home_dirpath, &temp_dir())),
            downloader: Box::new(DefaultDownloader),
            unziper: Box::new(DefaultUnziper),
        }
    }

    pub fn downloader(&self) -> &dyn Downloader {
        self.downloader.as_ref()
    }

    pub fn fs(&self) -> &dyn Fs {
        self.fs.as_ref()
    }

    pub fn unziper(&self) -> &dyn Unziper {
        self.unziper.as_ref()
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
                assert_eq!(cfg.fs.root_dirpath(), home_dirpath);
            }
        }
    }
}
