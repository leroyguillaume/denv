// IMPORTS

use std::io::{self, Write};
#[cfg(test)]
use stub_trait::stub;

// TYPES

pub type Result = io::Result<()>;

// TRAITS

#[cfg_attr(test, stub)]
pub trait Downloader {
    fn download(&self, url: &str, out: &mut dyn Write) -> Result;
}

// STRUCTS

pub struct DefaultDownloader;

impl Downloader for DefaultDownloader {
    fn download(&self, _url: &str, _out: &mut dyn Write) -> Result {
        unimplemented!();
    }
}
