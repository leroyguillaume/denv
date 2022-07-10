use log::{debug, trace};
use std::{
    fs::{create_dir_all, File, OpenOptions},
    io,
    path::{Path, PathBuf},
};

macro_rules! trace_open_file_w {
    ($filepath:expr) => {{
        trace!("Opening {} in write mode", $filepath.display());
        OpenOptions::new()
            .create(true)
            .write(true)
            .open($filepath)
            .map(|file| ($filepath, file))
    }};
}

pub trait Fs {
    fn create_bin_file(&self, tool_name: &str, version: &str) -> io::Result<(PathBuf, File)>;

    fn create_tmp_file(&self, filename: &str) -> io::Result<(PathBuf, File)>;

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
    fn create_bin_file(&self, tool_name: &str, version: &str) -> io::Result<(PathBuf, File)> {
        let dirpath = self
            .root_dirpath
            .join("tools")
            .join(tool_name)
            .join(version);
        if dirpath.is_dir() {
            debug!("Direction {} already exists", dirpath.display());
        } else {
            debug!("Creation directory {}", dirpath.display());
            debug_err!(create_dir_all(&dirpath))?;
        }
        trace_open_file_w!(dirpath.join(tool_name))
    }

    fn create_tmp_file(&self, filename: &str) -> io::Result<(PathBuf, File)> {
        trace_open_file_w!(self.tmp_dirpath.join(filename))
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
    create_bin_file_fn: Option<Box<CreateBinFileFn>>,
    create_tmp_file_fn: Option<Box<CreateTmpFileFn>>,
}

#[cfg(test)]
impl StubFs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_create_bin_file_fn<F: Fn(&str, &str) -> io::Result<(PathBuf, File)> + 'static>(
        mut self,
        create_bin_file_fn: F,
    ) -> Self {
        self.create_bin_file_fn = Some(Box::new(create_bin_file_fn));
        self
    }

    pub fn with_create_tmp_file_fn<F: Fn(&str) -> io::Result<(PathBuf, File)> + 'static>(
        mut self,
        create_tmp_file_fn: F,
    ) -> Self {
        self.create_tmp_file_fn = Some(Box::new(create_tmp_file_fn));
        self
    }
}

#[cfg(test)]
impl Fs for StubFs {
    fn create_bin_file(&self, tool_name: &str, version: &str) -> io::Result<(PathBuf, File)> {
        match &self.create_bin_file_fn {
            Some(create_bin_file_fn) => create_bin_file_fn(tool_name, version),
            None => unimplemented!(),
        }
    }

    fn create_tmp_file(&self, filename: &str) -> io::Result<(PathBuf, File)> {
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
type CreateBinFileFn = dyn Fn(&str, &str) -> io::Result<(PathBuf, File)>;

#[cfg(test)]
type CreateTmpFileFn = dyn Fn(&str) -> io::Result<(PathBuf, File)>;

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

        mod create_bin_file {
            use super::*;

            #[test]
            fn should_return_err() {
                let root_dirpath = tempdir().unwrap().into_path().join("root");
                let tmp_dirpath = tempdir().unwrap().into_path();
                let fs = DefaultFs::new(&root_dirpath, &tmp_dirpath);
                write(&root_dirpath, "").unwrap();
                if fs.create_bin_file("terraform", "1.2.3").is_ok() {
                    panic!("should fail");
                }
            }

            #[test]
            fn should_return_filepath_and_file() {
                let root_dirpath = tempdir().unwrap().into_path();
                let tmp_dirpath = tempdir().unwrap().into_path();
                let tool_name = "terraform";
                let version = "1.2.3";
                let fs = DefaultFs::new(&root_dirpath, &tmp_dirpath);
                let (filepath, mut file) = fs.create_bin_file(tool_name, version).unwrap();
                let expected = root_dirpath
                    .join("tools")
                    .join(tool_name)
                    .join(version)
                    .join(tool_name);
                assert_eq!(filepath, expected);
                write!(file, "test").unwrap();
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
                let filename = "terraform-1.2.3.zip";
                let fs = DefaultFs::new(&root_dirpath, &tmp_dirpath);
                let (filepath, mut file) = fs.create_tmp_file(filename).unwrap();
                assert_eq!(filepath, tmp_dirpath.join(filename));
                write!(file, "test").unwrap();
            }
        }
    }
}
