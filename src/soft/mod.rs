// IMPORTS

use crate::fs::FileSystem;
use std::path::PathBuf;
#[cfg(test)]
use stub_trait::stub;
use tf::Terraform;

// MODS

pub mod tf;

// TYPES

pub type Result = std::result::Result<(), Error>;

// ENUMS

#[derive(Debug)]
pub enum Error {
    #[cfg(test)]
    Stub,
}

pub enum Kind<'a> {
    Terraform(&'a Terraform),
}

// TRAITS

#[cfg_attr(test, stub)]
pub trait Software {
    fn binary_paths(&self, fs: &dyn FileSystem) -> Vec<PathBuf>;

    fn install(&self, fs: &dyn FileSystem) -> Result;

    fn is_installed(&self, fs: &dyn FileSystem) -> bool;

    fn kind(&self) -> Kind<'_>;

    fn name(&self) -> &str;

    fn version(&self) -> &str;
}
