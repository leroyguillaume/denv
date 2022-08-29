// IMPORTS

use std::{
    fs::File,
    io::Result,
    path::{Path, PathBuf},
};
#[cfg(test)]
use stub_trait::stub;

// DATA STRUCTS

pub struct TempFile {
    pub file: File,
    pub path: PathBuf,
}

// TRAITS

#[cfg_attr(test, stub)]
pub trait FileSystem {
    fn create_temp_file(&self) -> Result<TempFile>;

    fn cwd(&self) -> Result<PathBuf>;

    fn delete_env_dir(&self, project_dirpath: &Path) -> Result<()>;

    fn ensure_env_dir(&self, project_dirpath: &Path) -> Result<PathBuf>;

    fn ensure_software_dir(&self, name: &str, version: &str) -> Result<PathBuf>;

    fn ensure_symlink(&self, src: &Path, dest: &Path) -> Result<()>;

    fn file_exists(&self, path: &Path) -> bool;

    fn home_dirpath(&self) -> Result<PathBuf>;

    fn make_executable(&self, path: &Path) -> Result<()>;
}

// STRUCTS

pub struct DefaultFileSystem;

impl FileSystem for DefaultFileSystem {
    fn create_temp_file(&self) -> Result<TempFile> {
        unimplemented!();
    }

    fn cwd(&self) -> Result<PathBuf> {
        unimplemented!();
    }

    fn delete_env_dir(&self, _project_dirpath: &Path) -> Result<()> {
        unimplemented!();
    }

    fn ensure_env_dir(&self, _project_dirpath: &Path) -> Result<PathBuf> {
        unimplemented!();
    }

    fn ensure_software_dir(&self, _name: &str, _version: &str) -> Result<PathBuf> {
        unimplemented!();
    }

    fn ensure_symlink(&self, _src: &Path, _dest: &Path) -> Result<()> {
        unimplemented!();
    }

    fn file_exists(&self, _path: &Path) -> bool {
        unimplemented!();
    }

    fn home_dirpath(&self) -> Result<PathBuf> {
        unimplemented!();
    }

    fn make_executable(&self, _path: &Path) -> Result<()> {
        unimplemented!();
    }
}
