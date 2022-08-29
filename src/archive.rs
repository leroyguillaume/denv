// IMPORTS

use std::{io, path::Path};
#[cfg(test)]
use stub_trait::stub;

// TYPES

pub type Result = io::Result<()>;

// TRAITS

#[cfg_attr(test, stub)]
pub trait Unarchiver {
    fn untar(&self, archive_filepath: &Path, dest: &Path) -> Result;

    fn unzip(&self, archive_filepath: &Path, dest: &Path) -> Result;
}

// STRUCTS

pub struct DefaultUnarchiver;

impl Unarchiver for DefaultUnarchiver {
    fn untar(&self, _archive_filepath: &Path, _dest: &Path) -> Result {
        unimplemented!();
    }

    fn unzip(&self, _archive_filepath: &Path, _dest: &Path) -> Result {
        unimplemented!();
    }
}
