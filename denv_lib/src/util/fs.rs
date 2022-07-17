use log::{debug, trace};
use std::{
    fmt::{self, Display, Formatter},
    fs::{create_dir_all, File, OpenOptions},
    io,
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
            create_dir_all($path).map_err(|err| Error::new($path.clone(), err))
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
            .map_err(|err| Error::new($path, err))
    }};
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    path: PathBuf,
    source: io::Error,
}

impl Error {
    pub fn new(path: PathBuf, source: io::Error) -> Self {
        Self { path, source }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn source(&self) -> &io::Error {
        &self.source
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "I/O failed on {}: {}", self.path.display(), self.source)
    }
}

pub trait FileSystem {
    fn create_bin_file(&self, name: &str, version: &str) -> Result<(PathBuf, File)>;

    fn create_bin_symlink(&self, name: &str, version: &str) -> Result<()>;

    fn create_tmp_file(&self, filename: &str) -> Result<(PathBuf, File)>;

    fn root_dirpath(&self) -> &Path;

    fn tmp_dirpath(&self) -> &Path;
}

pub struct DefaultFileSystem {
    root_dirpath: PathBuf,
    tmp_dirpath: PathBuf,
}

impl DefaultFileSystem {
    pub fn new(root_dirpath: PathBuf, tmp_dirpath: PathBuf) -> Self {
        Self {
            root_dirpath,
            tmp_dirpath,
        }
    }

    fn tool_dirpath(&self, name: &str, version: &str) -> PathBuf {
        self.root_dirpath.join("tools").join(name).join(version)
    }
}

impl FileSystem for DefaultFileSystem {
    fn create_bin_file(&self, name: &str, version: &str) -> Result<(PathBuf, File)> {
        let dirpath = self.tool_dirpath(name, version);
        ensure_dir!(&dirpath)?;
        open_file!(dirpath.join(name))
    }

    fn create_bin_symlink(&self, name: &str, version: &str) -> Result<()> {
        let src_filepath = self.tool_dirpath(name, version).join(name);
        let dest_dirpath = self.root_dirpath.join("bin");
        ensure_dir!(&dest_dirpath)?;
        let dest_filepath = dest_dirpath.join(name);
        debug!(
            "Creating symlink from {} to {}",
            src_filepath.display(),
            dest_dirpath.display()
        );
        symlink(src_filepath, dest_filepath).map_err(|err| Error::new(dest_dirpath, err))
    }

    fn create_tmp_file(&self, filename: &str) -> Result<(PathBuf, File)> {
        open_file!(self.tmp_dirpath.join(filename))
    }

    fn root_dirpath(&self) -> &Path {
        &self.root_dirpath
    }

    fn tmp_dirpath(&self) -> &Path {
        &self.tmp_dirpath
    }
}

#[cfg(test)]
type CreateBinFileFn = dyn Fn(&str, &str) -> Result<(PathBuf, File)>;

#[cfg(test)]
type CreateBinSymlinkFn = dyn Fn(&str, &str) -> Result<()>;

#[cfg(test)]
type CreateTmpFileFn = dyn Fn(&str) -> Result<(PathBuf, File)>;

#[cfg(test)]
#[derive(Default)]
pub struct StubFs {
    create_bin_file_fn: Option<Box<CreateBinFileFn>>,
    create_bin_symlink_fn: Option<Box<CreateBinSymlinkFn>>,
    create_tmp_file_fn: Option<Box<CreateTmpFileFn>>,
}

#[cfg(test)]
impl StubFs {
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

    pub fn with_create_bin_symlink_fn<F: Fn(&str, &str) -> Result<()> + 'static>(
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
}

#[cfg(test)]
impl FileSystem for StubFs {
    fn create_bin_file(&self, name: &str, version: &str) -> Result<(PathBuf, File)> {
        match &self.create_bin_file_fn {
            Some(create_bin_file_fn) => create_bin_file_fn(name, version),
            None => unimplemented!(),
        }
    }

    fn create_bin_symlink(&self, name: &str, version: &str) -> Result<()> {
        match &self.create_bin_symlink_fn {
            Some(create_bin_symlink_fn) => create_bin_symlink_fn(name, version),
            None => unimplemented!(),
        }
    }

    fn create_tmp_file(&self, filename: &str) -> Result<(PathBuf, File)> {
        match &self.create_tmp_file_fn {
            Some(create_tmp_file_fn) => create_tmp_file_fn(filename),
            None => unimplemented!(),
        }
    }

    fn root_dirpath(&self) -> &Path {
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

    mod error {
        use super::*;

        mod new {
            use super::*;

            #[test]
            fn should_return_error() {
                let path = PathBuf::from("/error");
                let source_kind = io::ErrorKind::PermissionDenied;
                let source = io::Error::from(source_kind);
                let err = Error::new(path.clone(), source);
                assert_eq!(err.path(), path);
                assert_eq!(err.source().kind(), source_kind);
            }
        }

        mod to_string {
            use super::*;

            #[test]
            fn should_return_string() {
                let path = PathBuf::from("/error");
                let source = io::Error::from(io::ErrorKind::PermissionDenied);
                let expected = format!("I/O failed on {}: {}", path.display(), source);
                let err = Error::new(path, source);
                assert_eq!(err.to_string(), expected);
            }
        }
    }

    mod default_file_system {
        use super::*;

        mod new {
            use super::*;

            #[test]
            fn should_return_fs() {
                let root_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let fs = DefaultFileSystem::new(root_dirpath.clone(), tmp_dirpath.clone());
                assert_eq!(fs.root_dirpath(), root_dirpath);
                assert_eq!(fs.tmp_dirpath(), tmp_dirpath);
            }
        }

        mod create_bin_file {
            use super::*;

            #[test]
            fn should_return_err() {
                let root_dirpath = tempdir().unwrap().into_path().join("root");
                let tmp_dirpath = tempdir().unwrap().into_path();
                write(&root_dirpath, "").unwrap();
                let fs = DefaultFileSystem::new(root_dirpath, tmp_dirpath);
                if fs.create_bin_file("terraform", "1.2.3").is_ok() {
                    panic!("should fail");
                }
            }

            #[test]
            fn should_return_filepath_and_file() {
                let name = "terraform";
                let version = "1.2.3";
                let root_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let expected = root_dirpath
                    .join("tools")
                    .join(name)
                    .join(version)
                    .join(name);
                let fs = DefaultFileSystem::new(root_dirpath, tmp_dirpath);
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
                let root_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let filepath = root_dirpath.join("bin").join(name);
                create_dir_all(filepath.parent().unwrap()).unwrap();
                write(filepath, "").unwrap();
                let fs = DefaultFileSystem::new(root_dirpath, tmp_dirpath);
                if fs.create_bin_symlink(name, "1.2.3").is_ok() {
                    panic!("should fail");
                }
            }

            #[test]
            fn should_create_symlink() {
                let name = "terraform";
                let version = "1.2.3";
                let root_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let src_filepath = root_dirpath
                    .join("tools")
                    .join(name)
                    .join(version)
                    .join(name);
                let dest_filepath = root_dirpath.join("bin").join(name);
                let fs = DefaultFileSystem::new(root_dirpath, tmp_dirpath);
                fs.create_bin_symlink(name, version).unwrap();
                assert!(dest_filepath.is_symlink());
                assert_eq!(read_link(dest_filepath).unwrap(), src_filepath);
            }
        }

        mod create_tmp_file {
            use super::*;

            #[test]
            fn should_return_err() {
                let root_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path().join("tmp");
                write(&tmp_dirpath, "").unwrap();
                let fs = DefaultFileSystem::new(root_dirpath, tmp_dirpath);
                if fs.create_tmp_file("terraform-1.2.3.zip").is_ok() {
                    panic!("should fail");
                }
            }

            #[test]
            fn should_return_file() {
                let root_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let filename = "terraform-1.2.3.zip";
                let expected = tmp_dirpath.join(filename);
                let fs = DefaultFileSystem::new(root_dirpath, tmp_dirpath);
                let (filepath, mut file) = fs.create_tmp_file(filename).unwrap();
                assert_eq!(filepath, expected);
                write!(file, "test").unwrap();
            }
        }
    }
}
