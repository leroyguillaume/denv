use log::debug;
use std::{
    fmt::{self, Display, Formatter},
    fs::File,
    io::{self, copy, BufReader, Write},
    path::Path,
};
use zip::{result::ZipError, ZipArchive};

#[derive(Debug)]
pub enum UnzipError {
    FileOpeningFailed(io::Error),
    InvalidZipFile(ZipError),
    UnzipFailed(ZipError),
    DestinationWritingFailed(io::Error),
}

impl Display for UnzipError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::FileOpeningFailed(err) => write!(f, "{}", err),
            Self::InvalidZipFile(err) => write!(f, "{}", err),
            Self::UnzipFailed(err) => write!(f, "{}", err),
            Self::DestinationWritingFailed(err) => write!(f, "{}", err),
        }
    }
}

pub trait Unzipper {
    fn unzip(
        &self,
        zip_filepath: &Path,
        filepath: &str,
        dest: &mut dyn Write,
    ) -> Result<(), UnzipError>;
}

pub struct DefaultUnzipper;

impl Unzipper for DefaultUnzipper {
    fn unzip(
        &self,
        zip_filepath: &Path,
        filepath: &str,
        dest: &mut dyn Write,
    ) -> Result<(), UnzipError> {
        debug!("Unzipping {} from {}", filepath, zip_filepath.display());
        let zip_file = File::open(zip_filepath).map_err(UnzipError::FileOpeningFailed)?;
        let zip_file_buf = BufReader::new(zip_file);
        let mut zip = ZipArchive::new(zip_file_buf).map_err(UnzipError::InvalidZipFile)?;
        let mut tgt_file = zip.by_name(filepath).map_err(UnzipError::UnzipFailed)?;
        copy(&mut tgt_file, dest).map_err(UnzipError::DestinationWritingFailed)?;
        Ok(())
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct StubUnzipper {
    unzip_fn: Option<Box<UnzipFn>>,
}

#[cfg(test)]
impl StubUnzipper {
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
impl Unzipper for StubUnzipper {
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
    use tempfile::{tempdir, tempfile};

    mod unziper {
        use super::*;

        mod unzip {
            use super::*;

            #[test]
            fn should_return_file_opening_failed_err() {
                let zip_filepath = temp_dir().join("test");
                let mut out = vec![];
                match DefaultUnzipper.unzip(&zip_filepath, "test", &mut out) {
                    Ok(_) => panic!("should fail"),
                    Err(UnzipError::FileOpeningFailed(_)) => {}
                    Err(err) => panic!("{}", err),
                }
            }

            #[test]
            fn should_return_invalid_zip_file_err() {
                let dirpath = tempdir().unwrap().into_path();
                create_dir_all(&dirpath).unwrap();
                let zip_filepath = dirpath.join("test");
                let _ = File::create(&zip_filepath).unwrap();
                let mut out = vec![];
                match DefaultUnzipper.unzip(&zip_filepath, "test", &mut out) {
                    Ok(_) => panic!("should fail"),
                    Err(UnzipError::InvalidZipFile(_)) => {}
                    Err(err) => panic!("{}", err),
                }
            }

            #[test]
            fn should_return_unzip_failed_err() {
                let zip_filepath = Path::new("resources/tests/unziper/test.zip");
                let filepath = "test2";
                let mut out = vec![];
                match DefaultUnzipper.unzip(zip_filepath, filepath, &mut out) {
                    Ok(_) => panic!("should fail"),
                    Err(UnzipError::UnzipFailed(_)) => {}
                    Err(err) => panic!("{}", err),
                }
            }

            #[test]
            fn should_extract_file() {
                let mut out = vec![];
                let filepath = Path::new("resources/tests/unziper/test.zip");
                DefaultUnzipper.unzip(filepath, "test", &mut out).unwrap();
                assert_eq!(String::from_utf8(out).unwrap(), "test\n");
            }
        }
    }

    mod unzip_error {
        use super::*;

        mod to_string {
            use super::*;

            mod file_opening_failed {
                use super::*;

                #[test]
                fn should_return_string() {
                    let err = io::Error::from(io::ErrorKind::PermissionDenied);
                    let expected = err.to_string();
                    let err = UnzipError::FileOpeningFailed(err);
                    assert_eq!(err.to_string(), expected);
                }
            }

            mod invalid_zip_file {
                use super::*;

                #[test]
                fn should_return_string() {
                    let file = tempfile().unwrap();
                    let err = ZipArchive::new(file).unwrap_err();
                    let expected = err.to_string();
                    let err = UnzipError::InvalidZipFile(err);
                    assert_eq!(err.to_string(), expected);
                }
            }

            mod unzip_failed {
                use super::*;

                #[test]
                fn should_return_string() {
                    let file = tempfile().unwrap();
                    let err = ZipArchive::new(file).unwrap_err();
                    let expected = err.to_string();
                    let err = UnzipError::UnzipFailed(err);
                    assert_eq!(err.to_string(), expected);
                }
            }

            mod destination_writing_failed {
                use super::*;

                #[test]
                fn should_return_string() {
                    let err = io::Error::from(io::ErrorKind::PermissionDenied);
                    let expected = err.to_string();
                    let err = UnzipError::DestinationWritingFailed(err);
                    assert_eq!(err.to_string(), expected);
                }
            }
        }
    }
}
