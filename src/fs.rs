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

    fn delete_env_dir(&self, project_dirpath: &Path) -> Result<()>;

    fn ensure_env_dir(&self, project_dirpath: &Path) -> Result<PathBuf>;

    fn ensure_symlink(&self, src: &Path, dest: &Path) -> Result<()>;

    fn home_dirpath(&self) -> Result<PathBuf>;
}

// STRUCTS

pub struct DefaultFileSystem;

impl FileSystem for DefaultFileSystem {
    fn cwd(&self) -> Result<PathBuf> {
        unimplemented!();
    }

    fn delete_env_dir(&self, _project_dirpath: &Path) -> Result<()> {
        unimplemented!();
    }

    fn ensure_env_dir(&self, _project_dirpath: &Path) -> Result<PathBuf> {
        unimplemented!();
    }

    fn ensure_symlink(&self, _src: &Path, _dest: &Path) -> Result<()> {
        unimplemented!();
    }

    fn home_dirpath(&self) -> Result<PathBuf> {
        unimplemented!();
    }
}
