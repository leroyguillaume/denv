use crate::{error::*, software::*};
use log::{debug, trace};
use std::{
    fs::{create_dir_all, remove_file, File, OpenOptions},
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

macro_rules! ensure_dir {
    ($path:expr) => {
        if $path.is_dir() {
            trace!("Directory {} already exists", $path.display());
            Ok(())
        } else {
            debug!("Creating directory {}", $path.display());
            create_dir_all($path).map_err(|err| FileSystemError::new($path.clone(), err))
        }
    };
}

macro_rules! env_dirpath {
    ($denv_dirpath:expr, $env_id:expr) => {
        $denv_dirpath
            .join(ENVIRONMENTS_DIRNAME)
            .join(Path::new($env_id))
    };
}

macro_rules! open_file {
    ($path:expr) => {{
        if let Some(parent_path) = $path.parent() {
            ensure_dir!(parent_path.to_path_buf())?;
        }
        OpenOptions::new()
            .create(true)
            .write(true)
            .open(&$path)
            .map(|file| ($path, file))
            .map_err(|err| FileSystemError::new($path, err))
    }};
}

macro_rules! software_bin_filepath {
    ($denv_dirpath:expr, $software:expr) => {
        software_dirpath!($denv_dirpath, $software).join($software.name())
    };
}

macro_rules! software_dirpath {
    ($denv_dirpath:expr, $software:expr) => {
        $denv_dirpath
            .join(SOFTWARES_DIRNAME)
            .join($software.name())
            .join($software.version())
    };
}

const SOFTWARES_DIRNAME: &str = "softwares";
const ENVIRONMENTS_DIRNAME: &str = "environments";
const ENV_FILENAME: &str = "env";

pub type Result<T> = std::result::Result<T, FileSystemError>;

pub trait FileSystem {
    fn create_bin_file(&self, software: &dyn Software) -> Result<(PathBuf, File)>;

    fn create_bin_symlink(&self, env_id: &str, software: &dyn Software) -> Result<()>;

    fn create_env_file(&self, env_id: &str) -> Result<(PathBuf, File)>;

    fn create_tmp_file(&self, filename: &str) -> Result<(PathBuf, File)>;

    fn denv_dirpath(&self) -> &Path;

    fn is_installed_software(&self, software: &dyn Software) -> bool;

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
    fn create_bin_file(&self, software: &dyn Software) -> Result<(PathBuf, File)> {
        open_file!(software_bin_filepath!(self.denv_dirpath, software))
    }

    fn create_bin_symlink(&self, env_id: &str, software: &dyn Software) -> Result<()> {
        let src_filepath = software_bin_filepath!(self.denv_dirpath, software);
        let dest_dirpath = env_dirpath!(self.denv_dirpath, env_id);
        ensure_dir!(&dest_dirpath)?;
        let dest_filepath = dest_dirpath.join(software.name());
        if dest_filepath.is_symlink() {
            trace!("Deleting {}", dest_filepath.display());
            remove_file(&dest_filepath)
                .map_err(|err| FileSystemError::new(dest_filepath.clone(), err))?;
        }
        debug!(
            "Creating symlink from {} to {}",
            src_filepath.display(),
            dest_dirpath.display()
        );
        symlink(src_filepath, &dest_filepath)
            .map_err(|err| FileSystemError::new(dest_filepath, err))
    }

    fn create_env_file(&self, env_id: &str) -> Result<(PathBuf, File)> {
        open_file!(env_dirpath!(self.denv_dirpath, env_id).join(ENV_FILENAME))
    }

    fn create_tmp_file(&self, filename: &str) -> Result<(PathBuf, File)> {
        open_file!(self.tmp_dirpath.join(filename))
    }

    fn denv_dirpath(&self) -> &Path {
        &self.denv_dirpath
    }

    fn is_installed_software(&self, software: &dyn Software) -> bool {
        software_dirpath!(self.denv_dirpath, software).is_dir()
    }

    fn tmp_dirpath(&self) -> &Path {
        &self.tmp_dirpath
    }
}

#[cfg(test)]
type CreateBinFileFn = dyn Fn(&dyn Software) -> Result<(PathBuf, File)>;

#[cfg(test)]
type CreateBinSymlinkFn = dyn Fn(&str, &dyn Software) -> Result<()>;

#[cfg(test)]
type CreateEnvFileFn = dyn Fn(&str) -> Result<(PathBuf, File)>;

#[cfg(test)]
type CreateTmpFileFn = dyn Fn(&str) -> Result<(PathBuf, File)>;

#[cfg(test)]
type IsInstalledSoftwareFn = dyn Fn(&dyn Software) -> bool;

#[cfg(test)]
#[derive(Default)]
pub struct StubFileSystem {
    create_bin_file_fn: Option<Box<CreateBinFileFn>>,
    create_bin_symlink_fn: Option<Box<CreateBinSymlinkFn>>,
    create_env_file_fn: Option<Box<CreateEnvFileFn>>,
    create_tmp_file_fn: Option<Box<CreateTmpFileFn>>,
    is_installed_software_fn: Option<Box<IsInstalledSoftwareFn>>,
}

#[cfg(test)]
impl StubFileSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_create_bin_file_fn<F: Fn(&dyn Software) -> Result<(PathBuf, File)> + 'static>(
        mut self,
        create_bin_file_fn: F,
    ) -> Self {
        self.create_bin_file_fn = Some(Box::new(create_bin_file_fn));
        self
    }

    pub fn with_create_bin_symlink_fn<F: Fn(&str, &dyn Software) -> Result<()> + 'static>(
        mut self,
        create_bin_symlink_fn: F,
    ) -> Self {
        self.create_bin_symlink_fn = Some(Box::new(create_bin_symlink_fn));
        self
    }

    pub fn with_create_env_file_fn<F: Fn(&str) -> Result<(PathBuf, File)> + 'static>(
        mut self,
        create_env_file_fn: F,
    ) -> Self {
        self.create_env_file_fn = Some(Box::new(create_env_file_fn));
        self
    }

    pub fn with_create_tmp_file_fn<F: Fn(&str) -> Result<(PathBuf, File)> + 'static>(
        mut self,
        create_tmp_file_fn: F,
    ) -> Self {
        self.create_tmp_file_fn = Some(Box::new(create_tmp_file_fn));
        self
    }

    pub fn with_is_installed_software_fn<F: Fn(&dyn Software) -> bool + 'static>(
        mut self,
        is_installed_software_fn: F,
    ) -> Self {
        self.is_installed_software_fn = Some(Box::new(is_installed_software_fn));
        self
    }
}

#[cfg(test)]
impl FileSystem for StubFileSystem {
    fn create_bin_file(&self, software: &dyn Software) -> Result<(PathBuf, File)> {
        match &self.create_bin_file_fn {
            Some(create_bin_file_fn) => create_bin_file_fn(software),
            None => unimplemented!(),
        }
    }

    fn create_bin_symlink(&self, env_id: &str, software: &dyn Software) -> Result<()> {
        match &self.create_bin_symlink_fn {
            Some(create_bin_symlink_fn) => create_bin_symlink_fn(env_id, software),
            None => unimplemented!(),
        }
    }

    fn create_env_file(&self, env_id: &str) -> Result<(PathBuf, File)> {
        match &self.create_env_file_fn {
            Some(create_bin_file_fn) => create_bin_file_fn(env_id),
            None => unimplemented!(),
        }
    }

    fn create_tmp_file(&self, filename: &str) -> Result<(PathBuf, File)> {
        match &self.create_tmp_file_fn {
            Some(create_tmp_file_fn) => create_tmp_file_fn(filename),
            None => unimplemented!(),
        }
    }

    fn denv_dirpath(&self) -> &Path {
        Path::new("root")
    }

    fn is_installed_software(&self, software: &dyn Software) -> bool {
        match &self.is_installed_software_fn {
            Some(is_installed_software_fn) => is_installed_software_fn(software),
            None => unimplemented!(),
        }
    }

    fn tmp_dirpath(&self) -> &Path {
        Path::new("tmp")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{
        fs::{read_link, remove_dir_all, write},
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
                let software = StubSoftware::new("software", "1.2.3");
                let denv_dirpath = tempdir().unwrap().into_path().join("root");
                let tmp_dirpath = tempdir().unwrap().into_path();
                write(&denv_dirpath, "").unwrap();
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                if fs.create_bin_file(&software).is_ok() {
                    panic!("should fail");
                }
            }

            #[test]
            fn should_return_filepath_and_file() {
                let software = StubSoftware::new("software", "1.2.3");
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let expected = software_bin_filepath!(denv_dirpath, software);
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                let (filepath, mut file) = fs.create_bin_file(&software).unwrap();
                assert_eq!(filepath, expected);
                write!(file, "test").unwrap();
            }

            #[test]
            fn should_return_filepath_and_file_if_dir_does_not_exit() {
                let software = StubSoftware::new("software", "1.2.3");
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                remove_dir_all(&denv_dirpath).unwrap();
                let expected = software_bin_filepath!(denv_dirpath, software);
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                let (filepath, mut file) = fs.create_bin_file(&software).unwrap();
                assert_eq!(filepath, expected);
                write!(file, "test").unwrap();
            }
        }

        mod create_bin_symlink {
            use super::*;

            #[test]
            fn should_return_err() {
                let software = StubSoftware::new("software", "1.2.3");
                let env_id = "env_id";
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let filepath = env_dirpath!(denv_dirpath, env_id).join(software.name());
                create_dir_all(filepath.parent().unwrap()).unwrap();
                write(filepath, "").unwrap();
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                if fs.create_bin_symlink(env_id, &software).is_ok() {
                    panic!("should fail");
                }
            }

            #[test]
            fn should_create_symlink() {
                let software = StubSoftware::new("software", "1.2.3");
                let env_id = "env_id";
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let src_filepath = software_bin_filepath!(denv_dirpath, software);
                let dest_dirpath = env_dirpath!(denv_dirpath, env_id);
                create_dir_all(&dest_dirpath).unwrap();
                let dest_filepath = dest_dirpath.join(software.name());
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                fs.create_bin_symlink(env_id, &software).unwrap();
                assert!(dest_filepath.is_symlink());
                assert_eq!(read_link(dest_filepath).unwrap(), src_filepath);
            }

            #[test]
            fn should_create_symlink_if_it_already_exists() {
                let software = StubSoftware::new("software", "1.2.3");
                let env_id = "env_id";
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let src_filepath = software_bin_filepath!(denv_dirpath, software);
                let dest_dirpath = env_dirpath!(denv_dirpath, env_id);
                create_dir_all(&dest_dirpath).unwrap();
                let dest_filepath = dest_dirpath.join(software.name());
                symlink(&src_filepath, &dest_filepath).unwrap();
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                fs.create_bin_symlink(env_id, &software).unwrap();
                assert!(dest_filepath.is_symlink());
                assert_eq!(read_link(dest_filepath).unwrap(), src_filepath);
            }

            #[test]
            fn should_create_symlink_if_dir_does_not_exit() {
                let software = StubSoftware::new("software", "1.2.3");
                let env_id = "env_id";
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let src_filepath = software_bin_filepath!(denv_dirpath, software);
                let dest_filepath = env_dirpath!(denv_dirpath, env_id).join(software.name());
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                fs.create_bin_symlink(env_id, &software).unwrap();
                assert!(dest_filepath.is_symlink());
                assert_eq!(read_link(dest_filepath).unwrap(), src_filepath);
            }
        }

        mod create_env_file {
            use super::*;

            #[test]
            fn should_return_err() {
                let env_id = "env_id";
                let denv_dirpath = tempdir().unwrap().into_path().join("root");
                let tmp_dirpath = tempdir().unwrap().into_path();
                write(&denv_dirpath, "").unwrap();
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                if fs.create_env_file(env_id).is_ok() {
                    panic!("should fail");
                }
            }

            #[test]
            fn should_return_filepath_and_file() {
                let env_id = "env_id";
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let expected = env_dirpath!(denv_dirpath, env_id).join(ENV_FILENAME);
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                let (filepath, mut file) = fs.create_env_file(env_id).unwrap();
                assert_eq!(filepath, expected);
                write!(file, "test").unwrap();
            }

            #[test]
            fn should_return_filepath_and_file_if_dir_does_not_exit() {
                let env_id = "env_id";
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                remove_dir_all(&denv_dirpath).unwrap();
                let expected = env_dirpath!(denv_dirpath, env_id).join(ENV_FILENAME);
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                let (filepath, mut file) = fs.create_env_file(env_id).unwrap();
                assert_eq!(filepath, expected);
                write!(file, "test").unwrap();
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

            #[test]
            fn should_return_file_if_dir_does_not_exist() {
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                remove_dir_all(&tmp_dirpath).unwrap();
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
                let software = StubSoftware::new("software", "1.2.3");
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                let is_installed = fs.is_installed_software(&software);
                assert!(!is_installed);
            }

            #[test]
            fn should_return_true() {
                let software = StubSoftware::new("software", "1.2.3");
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                create_dir_all(software_dirpath!(denv_dirpath, software)).unwrap();
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                let is_installed = fs.is_installed_software(&software);
                assert!(is_installed);
            }
        }
    }
}
