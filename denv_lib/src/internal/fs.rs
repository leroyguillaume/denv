use crate::error::*;
use log::{debug, trace};
use std::{
    fs::{create_dir_all, metadata, remove_file, set_permissions, File},
    os::unix::fs::{symlink, PermissionsExt},
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
        std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&$path)
            .map(|file| ($path, file))
            .map_err(|err| FileSystemError::new($path, err))
    }};
}

macro_rules! software_bin_filepath {
    ($denv_dirpath:expr, $name:expr, $version:expr) => {
        software_dirpath!($denv_dirpath, $name, $version).join($name)
    };
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
const ENVIRONMENTS_DIRNAME: &str = "environments";
const ENV_FILENAME: &str = "env";

pub type Result<T> = std::result::Result<T, FileSystemError>;

pub trait FileSystem {
    fn create_bin_file(&self, name: &str, version: &str) -> Result<(PathBuf, File)>;

    fn create_bin_symlink(&self, env_id: &str, name: &str, version: &str) -> Result<()>;

    fn create_env_file(&self, env_id: &str) -> Result<(PathBuf, File)>;

    fn create_tmp_file(&self, filename: &str) -> Result<(PathBuf, File)>;

    fn denv_dirpath(&self) -> &Path;

    fn env_dirpath(&self, env_id: &str) -> PathBuf;

    fn is_installed_software(&self, name: &str, version: &str) -> bool;

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
        let (filepath, file) =
            open_file!(software_bin_filepath!(self.denv_dirpath, name, version))?;
        let file_metadata =
            metadata(&filepath).map_err(|err| FileSystemError::new(filepath.clone(), err))?;
        let mut perms = file_metadata.permissions();
        println!("{}", perms.mode());
        perms.set_mode(0o755);
        set_permissions(&filepath, perms)
            .map_err(|err| FileSystemError::new(filepath.clone(), err))?;
        Ok((filepath, file))
    }

    fn create_bin_symlink(&self, env_id: &str, name: &str, version: &str) -> Result<()> {
        let src_filepath = software_bin_filepath!(self.denv_dirpath, name, version);
        let dest_dirpath = env_dirpath!(self.denv_dirpath, env_id);
        ensure_dir!(&dest_dirpath)?;
        let dest_filepath = dest_dirpath.join(name);
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

    fn env_dirpath(&self, env_id: &str) -> PathBuf {
        env_dirpath!(self.denv_dirpath, env_id)
    }

    fn is_installed_software(&self, name: &str, version: &str) -> bool {
        software_dirpath!(self.denv_dirpath, name, version).is_dir()
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
type CreateEnvFileFn = dyn Fn(&str) -> Result<(PathBuf, File)>;

#[cfg(test)]
type CreateTmpFileFn = dyn Fn(&str) -> Result<(PathBuf, File)>;

#[cfg(test)]
type EnvDirpathFn = dyn Fn(&str) -> PathBuf;

#[cfg(test)]
type IsInstalledSoftwareFn = dyn Fn(&str, &str) -> bool;

#[cfg(test)]
#[derive(Default)]
pub struct StubFileSystem {
    create_bin_file_fn: Option<Box<CreateBinFileFn>>,
    create_bin_symlink_fn: Option<Box<CreateBinSymlinkFn>>,
    create_env_file_fn: Option<Box<CreateEnvFileFn>>,
    env_dirpath_fn: Option<Box<EnvDirpathFn>>,
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

    pub fn with_env_dirpath_fn<F: Fn(&str) -> PathBuf + 'static>(
        mut self,
        env_dirpath_fn: F,
    ) -> Self {
        self.env_dirpath_fn = Some(Box::new(env_dirpath_fn));
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

    fn create_bin_symlink(&self, env_id: &str, name: &str, version: &str) -> Result<()> {
        match &self.create_bin_symlink_fn {
            Some(create_bin_symlink_fn) => create_bin_symlink_fn(env_id, name, version),
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

    fn env_dirpath(&self, env_id: &str) -> PathBuf {
        match &self.env_dirpath_fn {
            Some(env_dirpath_fn) => env_dirpath_fn(env_id),
            None => unimplemented!(),
        }
    }

    fn is_installed_software(&self, name: &str, version: &str) -> bool {
        match &self.is_installed_software_fn {
            Some(is_installed_software_fn) => is_installed_software_fn(name, version),
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
                let denv_dirpath = tempdir().unwrap().into_path().join("root");
                let tmp_dirpath = tempdir().unwrap().into_path();
                write(&denv_dirpath, "").unwrap();
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                if fs.create_bin_file("software", "1.2.3").is_ok() {
                    panic!("should fail");
                }
            }

            #[test]
            fn should_return_filepath_and_file() {
                let name = "software";
                let version = "1.2.3";
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let expected = software_bin_filepath!(denv_dirpath, name, version);
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                let (filepath, mut file) = fs.create_bin_file(name, version).unwrap();
                assert_eq!(filepath, expected);
                write!(file, "test").unwrap();
                let perms = metadata(filepath).unwrap().permissions();
                assert_eq!(perms.mode(), 0o100755);
            }

            #[test]
            fn should_return_filepath_and_file_if_dir_does_not_exit() {
                let name = "software";
                let version = "1.2.3";
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                remove_dir_all(&denv_dirpath).unwrap();
                let expected = software_bin_filepath!(denv_dirpath, name, version);
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                let (filepath, mut file) = fs.create_bin_file(name, version).unwrap();
                assert_eq!(filepath, expected);
                write!(file, "test").unwrap();
                let perms = metadata(filepath).unwrap().permissions();
                assert_eq!(perms.mode(), 0o100755);
            }
        }

        mod create_bin_symlink {
            use super::*;

            #[test]
            fn should_return_err() {
                let name = "software";
                let env_id = "env_id";
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let filepath = env_dirpath!(denv_dirpath, env_id).join(name);
                create_dir_all(filepath.parent().unwrap()).unwrap();
                write(filepath, "").unwrap();
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                if fs.create_bin_symlink(env_id, name, "1.2.3").is_ok() {
                    panic!("should fail");
                }
            }

            #[test]
            fn should_create_symlink() {
                let name = "software";
                let version = "1.2.3";
                let env_id = "env_id";
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let src_filepath = software_bin_filepath!(denv_dirpath, name, version);
                let dest_dirpath = env_dirpath!(denv_dirpath, env_id);
                create_dir_all(&dest_dirpath).unwrap();
                let dest_filepath = dest_dirpath.join(name);
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                fs.create_bin_symlink(env_id, name, version).unwrap();
                assert!(dest_filepath.is_symlink());
                assert_eq!(read_link(dest_filepath).unwrap(), src_filepath);
            }

            #[test]
            fn should_create_symlink_if_it_already_exists() {
                let name = "software";
                let version = "1.2.3";
                let env_id = "env_id";
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let src_filepath = software_bin_filepath!(denv_dirpath, name, version);
                let dest_dirpath = env_dirpath!(denv_dirpath, env_id);
                create_dir_all(&dest_dirpath).unwrap();
                let dest_filepath = dest_dirpath.join(name);
                symlink(&src_filepath, &dest_filepath).unwrap();
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                fs.create_bin_symlink(env_id, name, version).unwrap();
                assert!(dest_filepath.is_symlink());
                assert_eq!(read_link(dest_filepath).unwrap(), src_filepath);
            }

            #[test]
            fn should_create_symlink_if_dir_does_not_exit() {
                let name = "software";
                let version = "1.2.3";
                let env_id = "env_id";
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let src_filepath = software_bin_filepath!(denv_dirpath, name, version);
                let dest_filepath = env_dirpath!(denv_dirpath, env_id).join(name);
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                fs.create_bin_symlink(env_id, name, version).unwrap();
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

        mod env_dirpath {
            use super::*;

            #[test]
            fn should_return_env_dirpath() {
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let env_id = "env_id";
                let expected = env_dirpath!(denv_dirpath, env_id);
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                let env_dirpath = fs.env_dirpath(env_id);
                assert_eq!(env_dirpath, expected);
            }
        }

        mod is_installed_software {
            use super::*;

            #[test]
            fn should_return_false() {
                let name = "software";
                let version = "1.2.3";
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                let is_installed = fs.is_installed_software(name, version);
                assert!(!is_installed);
            }

            #[test]
            fn should_return_true() {
                let name = "software";
                let version = "1.2.3";
                let denv_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                create_dir_all(software_dirpath!(denv_dirpath, name, version)).unwrap();
                let fs = DefaultFileSystem::new(denv_dirpath, tmp_dirpath);
                let is_installed = fs.is_installed_software(name, version);
                assert!(is_installed);
            }
        }
    }
}
