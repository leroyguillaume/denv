use log::debug;
use std::{
    fs::create_dir_all,
    io,
    path::{Path, PathBuf},
};

pub trait Fs {
    fn ensure_tool_dir(&self, name: &str, version: &str) -> io::Result<PathBuf>;

    fn root_dirpath(&self) -> &Path;
}

pub struct DefaultFs(PathBuf);

impl DefaultFs {
    pub fn new(root_dirpath: &Path) -> Self {
        Self(root_dirpath.into())
    }
}

impl Fs for DefaultFs {
    fn ensure_tool_dir(&self, name: &str, version: &str) -> io::Result<PathBuf> {
        let path = self.0.join(name).join(version);
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
        &self.0
    }
}

#[cfg(test)]
pub struct StubFs {
    root_dirpath: PathBuf,
    ensure_tool_dir_fn: Option<Box<EnsureToolDirFn>>,
}

#[cfg(test)]
impl StubFs {
    pub fn new(root_dirpath: &Path) -> Self {
        Self {
            root_dirpath: root_dirpath.into(),
            ensure_tool_dir_fn: None,
        }
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
    fn ensure_tool_dir(&self, name: &str, version: &str) -> io::Result<PathBuf> {
        match &self.ensure_tool_dir_fn {
            Some(ensure_tool_dir_fn) => ensure_tool_dir_fn(name, version),
            None => unimplemented!(),
        }
    }

    fn root_dirpath(&self) -> &Path {
        &self.root_dirpath
    }
}

#[cfg(test)]
type EnsureToolDirFn = dyn Fn(&str, &str) -> io::Result<PathBuf>;

#[cfg(test)]
mod test {
    use super::*;
    use std::fs::write;
    use tempfile::tempdir;

    mod default_fs {
        use super::*;

        mod new {
            use super::*;

            #[test]
            fn should_return_fs() {
                let expected = tempdir().unwrap().into_path();
                let fs = DefaultFs::new(&expected);
                assert_eq!(fs.0, expected);
                assert_eq!(fs.root_dirpath(), expected);
            }
        }

        mod ensure_tool_dir {
            use super::*;

            #[test]
            fn should_return_err() {
                let name = "terraform";
                let version = "1.2.3";
                let tmp_dirpath = tempdir().unwrap().into_path();
                let path = tmp_dirpath.join(name).join(version);
                let fs = DefaultFs::new(&tmp_dirpath);
                create_dir_all(path.parent().unwrap()).unwrap();
                write(path, "").unwrap();
                if fs.ensure_tool_dir(name, version).is_ok() {
                    panic!("should fail");
                }
            }

            #[test]
            fn should_do_nothing() {
                let name = "terraform";
                let version = "1.2.3";
                let tmp_dirpath = tempdir().unwrap().into_path();
                let expected = tmp_dirpath.join(name).join(version);
                let fs = DefaultFs::new(&tmp_dirpath);
                create_dir_all(&expected).unwrap();
                let path = fs.ensure_tool_dir(name, version).unwrap();
                assert_eq!(path, expected);
            }

            #[test]
            fn should_create_dir() {
                let name = "terraform";
                let version = "1.2.3";
                let tmp_dirpath = tempdir().unwrap().into_path();
                let expected = tmp_dirpath.join(name).join(version);
                let fs = DefaultFs::new(&tmp_dirpath);
                let path = fs.ensure_tool_dir(name, version).unwrap();
                assert!(expected.is_dir());
                assert_eq!(path, expected);
            }
        }
    }
}
