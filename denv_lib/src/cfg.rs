use crate::util::{downloader::*, fs::*, zip::*};
use std::{env::temp_dir, path::Path};

pub struct Config {
    pub(crate) fs: Box<dyn Fs>,
    pub(crate) downloader: Box<dyn Downloader>,
    pub(crate) unziper: Box<dyn Unziper>,
}

impl Config {
    pub fn new(home_dirpath: &Path) -> Self {
        Self {
            fs: Box::new(DefaultFs::new(home_dirpath, &temp_dir())),
            downloader: Box::new(DefaultDownloader),
            unziper: Box::new(DefaultUnziper),
        }
    }
}

#[cfg(test)]
impl Config {
    pub fn stub(fs: StubFs, downloader: StubDownloader, unziper: StubUnziper) -> Self {
        Self {
            downloader: Box::new(downloader),
            fs: Box::new(fs),
            unziper: Box::new(unziper),
        }
    }
}
