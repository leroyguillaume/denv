// IMPORTS

use crate::fs::FileSystem;
use std::path::PathBuf;
use tf::Terraform;

// MODS

pub mod tf;

// TYPES

pub type Result = std::result::Result<(), Error>;

// ENUMS

#[derive(Debug)]
pub enum Error {}

pub enum Kind<'a> {
    Terraform(&'a Terraform),
}

// TRAITS

pub trait Software {
    fn binary_paths(&self, fs: &dyn FileSystem) -> Vec<PathBuf>;

    fn install(&self, fs: &dyn FileSystem) -> Result;

    fn is_installed(&self, fs: &dyn FileSystem) -> bool;

    fn kind(&self) -> Kind;

    fn name(&self) -> &str;

    fn version(&self) -> &str;
}
