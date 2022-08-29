// IMPORTS

use super::Result;
use crate::fs::FileSystem;
use std::path::{Path, PathBuf};
#[cfg(test)]
use stub_trait::stub;

// DATA STRUCTS

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Artifact<'a> {
    pub name: &'a str,
    pub symlinks: Vec<Symlink>,
    pub url: String,
    pub version: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Symlink {
    pub dest: PathBuf,
    pub src: &'static Path,
}

// TRAITS

#[cfg_attr(test, stub)]
pub trait ArchiveArtifactInstaller {
    fn install_targz(&self, artifact: &Artifact, fs: &dyn FileSystem) -> Result<()>;

    fn install_zip(&self, artifact: &Artifact, fs: &dyn FileSystem) -> Result<()>;
}

// STRUCTS

#[derive(Default)]
pub struct DefaultArchiveArtifactInstaller;

impl ArchiveArtifactInstaller for DefaultArchiveArtifactInstaller {
    fn install_targz(&self, _artifact: &Artifact, _fs: &dyn FileSystem) -> Result<()> {
        unimplemented!();
    }

    fn install_zip(&self, _artifact: &Artifact, _fs: &dyn FileSystem) -> Result<()> {
        unimplemented!();
    }
}
