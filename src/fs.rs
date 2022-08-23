// IMPORTS

use std::{
    io::Result,
    path::{Path, PathBuf},
};
#[cfg(test)]
use stub_trait::stub;

// TRAITS

#[cfg_attr(test, stub)]
pub trait FileSystem {
    fn cwd(&self) -> Result<PathBuf>;

    fn ensure_env_dir(&self) -> Result<PathBuf>;

    fn ensure_symlink(&self, src: &Path, dest: &Path) -> Result<()>;
}

// STRUCTS

pub struct DefaultFileSystem;

impl FileSystem for DefaultFileSystem {
    fn cwd(&self) -> Result<PathBuf> {
        unimplemented!();
    }

    fn ensure_env_dir(&self) -> Result<PathBuf> {
        unimplemented!();
    }

    fn ensure_symlink(&self, _src: &Path, _dest: &Path) -> Result<()> {
        unimplemented!();
    }
}
