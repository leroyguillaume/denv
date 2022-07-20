use crate::error::*;
use log::{debug, trace};
use std::{
    fs::{create_dir_all, File, OpenOptions},
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

macro_rules! ensure_dir {
    ($path:expr) => {
        if $path.is_dir() {
            debug!("Directory {} already exists", $path.display());
            Ok(())
        } else {
            debug!("Creating directory {}", $path.display());
            create_dir_all($path).map_err(|err| FileSystemError::new($path.clone(), err))
        }
    };
}

macro_rules! open_file {
    ($path:expr) => {{
        trace!("Opening {} in write mode", $path.display());
        OpenOptions::new()
            .create(true)
            .write(true)
            .open(&$path)
            .map(|file| ($path, file))
            .map_err(|err| FileSystemError::new($path, err))
    }};
}

macro_rules! software_dirpath {
    ($denv_dirpath:expr, $name:expr, $version:expr) => {
        $denv_dirpath
            .join(SOFTWARES_DIRNAME)
            .join($name)
            .join($version)
    };
}

const SOFTWARES_DIRNAME: &str = "softwares";
const CONFIGURATIONS_DIRNAME: &str = "configurations";

pub type Result<T> = std::result::Result<T, FileSystemError>;

pub trait FileSystem {
    fn create_bin_file(&self, name: &str, version: &str) -> Result<(PathBuf, File)>;

    fn create_bin_symlink(&self, name: &str, version: &str, cfg_sha256: &str) -> Result<()>;

    fn create_tmp_file(&self, filename: &str) -> Result<(PathBuf, File)>;

    fn is_installed_software(&self, name: &str, version: &str) -> bool;

    fn denv_dirpath(&self) -> &Path;

    fn tmp_dirpath(&self) -> &Path;
}

pub struct DefaultFileSystem {
    denv_dirpath: PathBuf,
    tmp_dirpath: PathBuf,
}

impl DefaultFileSystem {
    pub fn new(denv_dirpath: PathBuf, tmp_dirpath: PathBuf) -> Self {
        Self {
            denv_dirpath,
            tmp_dirpath,
        }
    }
}

impl FileSystem for DefaultFileSystem {
    fn create_bin_file(&self, name: &str, version: &str) -> Result<(PathBuf, File)> {
        let dirpath = software_dirpath!(self.denv_dirpath, name, version);
        ensure_dir!(&dirpath)?;
        open_file!(dirpath.join(name))
    }

    fn create_bin_symlink(&self, name: &str, version: &str, cfg_sha256: &str) -> Result<()> {
        let src_filepath = software_dirpath!(self.denv_dirpath, name, version).join(name);
        let dest_dirpath = self
            .denv_dirpath
            .join(CONFIGURATIONS_DIRNAME)
            .join(cfg_sha256);
        ensure_dir!(&dest_dirpath)?;
        let dest_filepath = dest_dirpath.join(name);
        debug!(
            "Creating symlink from {} to {}",
            src_filepath.display(),
            dest_dirpath.display()
        );
        symlink(src_filepath, dest_filepath).map_err(|err| FileSystemError::new(dest_dirpath, err))
    }

    fn create_tmp_file(&self, filename: &str) -> Result<(PathBuf, File)> {
        open_file!(self.tmp_dirpath.join(filename))
    }

    fn is_installed_software(&self, name: &str, version: &str) -> bool {
        software_dirpath!(self.denv_dirpath, name, version).is_dir()
    }

    fn denv_dirpath(&self) -> &Path {
        &self.denv_dirpath
    }

    fn tmp_dirpath(&self) -> &Path {
        &self.tmp_dirpath
    }
}

#[cfg(test)]
type CreateBinFileFn = dyn Fn(&str, &str) -> Result<(PathBuf, File)>;

#[cfg(test)]
type CreateBinSymlinkFn = dyn Fn(&str, &str, &str) -> Result<()>;

#[cfg(test)]
type CreateTmpFileFn = dyn Fn(&str) -> Result<(PathBuf, File)>;

#[cfg(test)]
type IsInstalledSoftwareFn = dyn Fn(&str, &str) -> bool;

#[cfg(test)]
#[derive(Default)]
pub struct StubFileSystem {
    create_bin_file_fn: Option<Box<CreateBinFileFn>>,
    create_bin_symlink_fn: Option<Box<CreateBinSymlinkFn>>,
    create_tmp_file_fn: Option<Box<CreateTmpFileFn>>,
    is_installed_software_fn: Option<Box<IsInstalledSoftwareFn>>,
}

#[cfg(test)]
impl StubFileSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_create_bin_file_fn<F: Fn(&str, &str) -> Result<(PathBuf, File)> + 'static>(
        mut self,
        create_bin_file_fn: F,
    ) -> Self {
        self.create_bin_file_fn = Some(Box::new(create_bin_file_fn));
        self
    }

    pub fn with_create_bin_symlink_fn<F: Fn(&str, &str, &str) -> Result<()> + 'static>(
        mut self,
        create_bin_symlink_fn: F,
    ) -> Self {
        self.create_bin_symlink_fn = Some(Box::new(create_bin_symlink_fn));
        self
    }

    pub fn with_create_tmp_file_fn<F: Fn(&str) -> Result<(PathBuf, File)> + 'static>(
        mut self,
        create_tmp_file_fn: F,
    ) -> Self {
        self.create_tmp_file_fn = Some(Box::new(create_tmp_file_fn));
        self
    }

    pub fn with_is_installed_software_fn<F: Fn(&str, &str) -> bool + 'static>(
        mut self,
        is_installed_software_fn: F,
    ) -> Self {
        self.is_installed_software_fn = Some(Box::new(is_installed_software_fn));
        self
    }
}

#[cfg(test)]
impl FileSystem for StubFileSystem {
    fn create_bin_file(&self, name: &str, version: &str) -> Result<(PathBuf, File)> {
        match &self.create_bin_file_fn {
            Some(create_bin_file_fn) => create_bin_file_fn(name, version),
            None => unimplemented!(),
        }
    }

    fn create_bin_symlink(&self, name: &str, version: &str, cfg_sha256: &str) -> Result<()> {
        match &self.create_bin_symlink_fn {
            Some(create_bin_symlink_fn) => create_bin_symlink_fn(name, version, cfg_sha256),
            None => unimplemented!(),
        }
    }

    fn create_tmp_file(&self, filename: &str) -> Result<(PathBuf, File)> {
        match &self.create_tmp_file_fn {
            Some(create_tmp_file_fn) => create_tmp_file_fn(filename),
            None => unimplemented!(),
        }
    }

    fn is_installed_software(&self, name: &str, version: &str) -> bool {
        match &self.is_installed_software_fn {
            Some(is_installed_software_fn) => is_installed_software_fn(name, version),
            None => unimplemented!(),
        }
    }

    fn denv_dirpath(&self) -> &Path {
        Path::new("root")
    }

    fn tmp_dirpath(&self) -> &Path {
        Path::new("tmp")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{
        fs::{read_link, write},
        io::Write,
    };
    use tempfile::tempdir;

    mod default_file_system {
        use super::*;

        mod new {
            use super::*;

            #[test]
            fn should_return_fs() {
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let fs = DefaultFileSystem::new(denv_dirpath.clone(), tmp_dirpath.clone());
                assert_eq!(fs.denv_dirpath(), denv_dirpath);
                assert_eq!(fs.tmp_dirpath(), tmp_dirpath);
            }
        }

        mod create_bin_file {
            use super::*;

            #[test]
            fn should_return_err() {
                let denv_dirpath = tempdir().unwrap().into_path().join("root");
                let tmp_dirpath = tempdir().unwrap().into_path();
                write(&denv_dirpath, "").unwrap();
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                if fs.create_bin_file("terraform", "1.2.3").is_ok() {
                    panic!("should fail");
                }
            }

            #[test]
            fn should_return_filepath_and_file() {
                let name = "terraform";
                let version = "1.2.3";
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let expected = denv_dirpath
                    .join(SOFTWARES_DIRNAME)
                    .join(name)
                    .join(version)
                    .join(name);
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                let (filepath, mut file) = fs.create_bin_file(name, version).unwrap();
                assert_eq!(filepath, expected);
                write!(file, "test").unwrap();
            }
        }

        mod create_bin_symlink {
            use super::*;

            #[test]
            fn should_return_err() {
                let name = "terraform";
                let cfg_sha256 = "sha256";
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let filepath = denv_dirpath
                    .join(CONFIGURATIONS_DIRNAME)
                    .join(cfg_sha256)
                    .join(name);
                create_dir_all(filepath.parent().unwrap()).unwrap();
                write(filepath, "").unwrap();
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                if fs.create_bin_symlink(name, "1.2.3", cfg_sha256).is_ok() {
                    panic!("should fail");
                }
            }

            #[test]
            fn should_create_symlink() {
                let name = "terraform";
                let version = "1.2.3";
                let cfg_sha256 = "sha256";
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let src_filepath = denv_dirpath
                    .join(SOFTWARES_DIRNAME)
                    .join(name)
                    .join(version)
                    .join(name);
                let dest_filepath = denv_dirpath
                    .join(CONFIGURATIONS_DIRNAME)
                    .join(cfg_sha256)
                    .join(name);
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                fs.create_bin_symlink(name, version, cfg_sha256).unwrap();
                assert!(dest_filepath.is_symlink());
                assert_eq!(read_link(dest_filepath).unwrap(), src_filepath);
            }
        }

        mod create_tmp_file {
            use super::*;

            #[test]
            fn should_return_err() {
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path().join("tmp");
                write(&tmp_dirpath, "").unwrap();
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                if fs.create_tmp_file("terraform-1.2.3.zip").is_ok() {
                    panic!("should fail");
                }
            }

            #[test]
            fn should_return_file() {
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let filename = "terraform-1.2.3.zip";
                let expected = tmp_dirpath.join(filename);
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                let (filepath, mut file) = fs.create_tmp_file(filename).unwrap();
                assert_eq!(filepath, expected);
                write!(file, "test").unwrap();
            }
        }

        mod is_installed_software {
            use super::*;

            #[test]
            fn should_return_false() {
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                let is_installed = fs.is_installed_software("terraform", "1.2.3");
                assert!(!is_installed);
            }

            #[test]
            fn should_return_true() {
                let name = "terraform";
                let version = "1.2.3";
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                create_dir_all(
                    denv_dirpath
                        .join(SOFTWARES_DIRNAME)
                        .join(name)
                        .join(version),
                )
                .unwrap();
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                let is_installed = fs.is_installed_software(name, version);
                assert!(is_installed);
            }
        }
    }
}
