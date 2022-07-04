use log::debug;
use std::{
    fs::create_dir_all,
    io,
    path::{Path, PathBuf},
};

pub trait Cache {
    fn ensure_tool_dir(&self, name: &str, version: &str) -> io::Result<PathBuf>;

    fn path(&self) -> &Path;
}

pub struct DefaultCache(PathBuf);

impl DefaultCache {
    pub fn new(path: &Path) -> Self {
        Self(path.into())
    }
}

impl Cache for DefaultCache {
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

    fn path(&self) -> &Path {
        &self.0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs::write;
    use tempfile::tempdir;

    mod default_cache {
        use super::*;

        mod new {
            use super::*;

            #[test]
            fn should_return_cache() {
                let expected = tempdir().unwrap().into_path();
                let cache = DefaultCache::new(&expected);
                assert_eq!(cache.0, expected);
                assert_eq!(cache.path(), expected);
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
                let cache = DefaultCache::new(&tmp_dirpath);
                create_dir_all(path.parent().unwrap()).unwrap();
                write(path, "").unwrap();
                if cache.ensure_tool_dir(name, version).is_ok() {
                    panic!("should fail");
                }
            }

            #[test]
            fn should_do_nothing() {
                let name = "terraform";
                let version = "1.2.3";
                let tmp_dirpath = tempdir().unwrap().into_path();
                let expected = tmp_dirpath.join(name).join(version);
                let cache = DefaultCache::new(&tmp_dirpath);
                create_dir_all(&expected).unwrap();
                let path = cache.ensure_tool_dir(name, version).unwrap();
                assert_eq!(path, expected);
            }

            #[test]
            fn should_create_dir() {
                let name = "terraform";
                let version = "1.2.3";
                let tmp_dirpath = tempdir().unwrap().into_path();
                let expected = tmp_dirpath.join(name).join(version);
                let cache = DefaultCache::new(&tmp_dirpath);
                let path = cache.ensure_tool_dir(name, version).unwrap();
                assert!(expected.is_dir());
                assert_eq!(path, expected);
            }
        }
    }
}
