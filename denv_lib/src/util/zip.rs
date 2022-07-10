use crate::*;
use log::trace;
use std::{
    fmt::{self, Display, Formatter},
    fs::File,
    io::{self, copy, BufReader, Write},
    path::Path,
};
use zip::{result::ZipError, ZipArchive};

#[derive(Debug)]
pub enum UnzipError {
    IoFailed(io::Error),
    UnzipFailed(ZipError),
}

impl Display for UnzipError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::IoFailed(err) => write!(f, "{}", err),
            Self::UnzipFailed(err) => write!(f, "{}", err),
        }
    }
}

pub trait Unziper {
    fn unzip(
        &self,
        zip_filepath: &Path,
        filepath: &str,
        dest: &mut dyn Write,
    ) -> Result<(), UnzipError>;
}

pub struct DefaultUnziper;

impl Unziper for DefaultUnziper {
    fn unzip(
        &self,
        zip_filepath: &Path,
        filepath: &str,
        dest: &mut dyn Write,
    ) -> Result<(), UnzipError> {
        trace!("Opening file {} in read mode", zip_filepath.display());
        let zip_file = map_debug_err!(File::open(zip_filepath), UnzipError::IoFailed)?;
        let zip_file_buf = BufReader::new(zip_file);
        let mut zip = map_debug_err!(ZipArchive::new(zip_file_buf), UnzipError::UnzipFailed)?;
        let mut tgt_file = map_debug_err!(zip.by_name(filepath), UnzipError::UnzipFailed)?;
        map_debug_err!(copy(&mut tgt_file, dest), UnzipError::IoFailed)?;
        Ok(())
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct StubUnziper {
    unzip_fn: Option<Box<UnzipFn>>,
}

#[cfg(test)]
impl StubUnziper {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_unzip_fn<F: Fn(&Path, &str, &mut dyn Write) -> Result<(), UnzipError> + 'static>(
        mut self,
        unzip_fn: F,
    ) -> Self {
        self.unzip_fn = Some(Box::new(unzip_fn));
        self
    }
}

#[cfg(test)]
impl Unziper for StubUnziper {
    fn unzip(
        &self,
        zip_filepath: &Path,
        filepath: &str,
        dest: &mut dyn Write,
    ) -> Result<(), UnzipError> {
        match &self.unzip_fn {
            Some(unzip_fn) => unzip_fn(zip_filepath, filepath, dest),
            None => unimplemented!(),
        }
    }
}

#[cfg(test)]
type UnzipFn = dyn Fn(&Path, &str, &mut dyn Write) -> Result<(), UnzipError>;

#[cfg(test)]
mod test {
    use super::*;
    use std::{
        env::temp_dir,
        fs::{create_dir_all, File},
    };
    use tempfile::tempdir;

    mod unziper {
        use super::*;

        mod unzip {
            use super::*;

            #[test]
            fn should_return_io_failed_err() {
                let filepath = temp_dir().join("test");
                let mut out = vec![];
                match DefaultUnziper.unzip(&filepath, "test", &mut out) {
                    Ok(_) => panic!("should fail"),
                    Err(UnzipError::IoFailed(_)) => {}
                    Err(err) => panic!("{}", err),
                }
            }

            #[test]
            fn should_return_unzip_failed_err() {
                let dirpath = tempdir().unwrap().into_path();
                create_dir_all(&dirpath).unwrap();
                let filepath = dirpath.join("test");
                let _ = File::create(&filepath).unwrap();
                let mut out = vec![];
                match DefaultUnziper.unzip(&filepath, "test", &mut out) {
                    Ok(_) => panic!("should fail"),
                    Err(UnzipError::UnzipFailed(_)) => {}
                    Err(err) => panic!("{}", err),
                }
            }

            #[test]
            fn should_extract_file() {
                let mut out = vec![];
                let filepath = Path::new("resources/test/test.zip");
                DefaultUnziper.unzip(filepath, "test", &mut out).unwrap();
                assert_eq!(String::from_utf8(out).unwrap(), "test\n");
            }
        }
    }
}
