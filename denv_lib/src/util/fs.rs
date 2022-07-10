use log::{debug, trace};
use std::{
    fs::{create_dir_all, File, OpenOptions},
    io,
    path::{Path, PathBuf},
};

pub trait Fs {
    fn create_tmp_file(&self, filename: &str) -> io::Result<File>;

    fn ensure_tool_dir(&self, name: &str, version: &str) -> io::Result<PathBuf>;

    fn root_dirpath(&self) -> &Path;

    fn tmp_dirpath(&self) -> &Path;
}

pub struct DefaultFs {
    root_dirpath: PathBuf,
    tmp_dirpath: PathBuf,
}

impl DefaultFs {
    pub fn new(root_dirpath: &Path, tmp_dirpath: &Path) -> Self {
        Self {
            root_dirpath: root_dirpath.into(),
            tmp_dirpath: tmp_dirpath.into(),
        }
    }
}

impl Fs for DefaultFs {
    fn create_tmp_file(&self, filename: &str) -> io::Result<File> {
        let filepath = self.tmp_dirpath.join(filename);
        trace!("Opening {} in write mode", filepath.display());
        OpenOptions::new().create(true).write(true).open(filepath)
    }

    fn ensure_tool_dir(&self, name: &str, version: &str) -> io::Result<PathBuf> {
        let path = self.root_dirpath.join(name).join(version);
        if path.is_dir() {
            debug!("Directory {} already exists", path.display());
            Ok(path)
        } else {
            debug!("Creating directory {}", path.display());
            match create_dir_all(&path) {
                Ok(()) => Ok(path),
                Err(err) => {
                    debug!("Unable to create directory: {}", err);
                    Err(err)
                }
            }
        }
    }

    fn root_dirpath(&self) -> &Path {
        &self.root_dirpath
    }

    fn tmp_dirpath(&self) -> &Path {
        &self.tmp_dirpath
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct StubFs {
    create_tmp_file_fn: Option<Box<CreateTmpFileFn>>,
    ensure_tool_dir_fn: Option<Box<EnsureToolDirFn>>,
}

#[cfg(test)]
impl StubFs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_create_tmp_file_fn<F: Fn(&str) -> io::Result<File> + 'static>(
        mut self,
        create_tmp_file_fn: F,
    ) -> Self {
        self.create_tmp_file_fn = Some(Box::new(create_tmp_file_fn));
        self
    }

    pub fn with_ensure_tool_dir_fn<F: Fn(&str, &str) -> io::Result<PathBuf> + 'static>(
        mut self,
        ensure_tool_dir_fn: F,
    ) -> Self {
        self.ensure_tool_dir_fn = Some(Box::new(ensure_tool_dir_fn));
        self
    }
}

#[cfg(test)]
impl Fs for StubFs {
    fn create_tmp_file(&self, filename: &str) -> io::Result<File> {
        match &self.create_tmp_file_fn {
            Some(create_tmp_file_fn) => create_tmp_file_fn(filename),
            None => unimplemented!(),
        }
    }

    fn ensure_tool_dir(&self, name: &str, version: &str) -> io::Result<PathBuf> {
        match &self.ensure_tool_dir_fn {
            Some(ensure_tool_dir_fn) => ensure_tool_dir_fn(name, version),
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
type CreateTmpFileFn = dyn Fn(&str) -> io::Result<File>;

#[cfg(test)]
type EnsureToolDirFn = dyn Fn(&str, &str) -> io::Result<PathBuf>;

#[cfg(test)]
mod test {
    use super::*;
    use std::{fs::write, io::Write};
    use tempfile::tempdir;

    mod default_fs {
        use super::*;

        mod new {
            use super::*;

            #[test]
            fn should_return_fs() {
                let root_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let fs = DefaultFs::new(&root_dirpath, &tmp_dirpath);
                assert_eq!(fs.root_dirpath(), root_dirpath);
                assert_eq!(fs.tmp_dirpath(), tmp_dirpath);
            }
        }

        mod create_tmp_file {
            use super::*;

            #[test]
            fn should_return_err() {
                let root_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path().join("tmp");
                let fs = DefaultFs::new(&root_dirpath, &tmp_dirpath);
                write(&tmp_dirpath, "").unwrap();
                if fs.create_tmp_file("terraform-1.2.3.zip").is_ok() {
                    panic!("should fail");
                }
            }

            #[test]
            fn should_return_file() {
                let root_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let fs = DefaultFs::new(&root_dirpath, &tmp_dirpath);
                let mut file = fs.create_tmp_file("terraform-1.2.3.zip").unwrap();
                write!(file, "test").unwrap();
            }
        }

        mod ensure_tool_dir {
            use super::*;

            #[test]
            fn should_return_err() {
                let name = "terraform";
                let version = "1.2.3";
                let root_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let path = root_dirpath.join(name).join(version);
                let fs = DefaultFs::new(&root_dirpath, &tmp_dirpath);
                create_dir_all(path.parent().unwrap()).unwrap();
                write(&path, "").unwrap();
                if fs.ensure_tool_dir(name, version).is_ok() {
                    panic!("should fail");
                }
            }

            #[test]
            fn should_do_nothing() {
                let name = "terraform";
                let version = "1.2.3";
                let root_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let expected = root_dirpath.join(name).join(version);
                let fs = DefaultFs::new(&root_dirpath, &tmp_dirpath);
                create_dir_all(&expected).unwrap();
                let path = fs.ensure_tool_dir(name, version).unwrap();
                assert_eq!(path, expected);
            }

            #[test]
            fn should_create_dir() {
                let name = "terraform";
                let version = "1.2.3";
                let root_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let expected = root_dirpath.join(name).join(version);
                let fs = DefaultFs::new(&root_dirpath, &tmp_dirpath);
                let path = fs.ensure_tool_dir(name, version).unwrap();
                assert!(expected.is_dir());
                assert_eq!(path, expected);
            }
        }
    }
}
