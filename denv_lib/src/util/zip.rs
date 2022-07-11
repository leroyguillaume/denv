use log::debug;
use std::{
    fmt::{self, Display, Formatter},
    fs::File,
    io::{self, copy, BufReader, Write},
    path::{Path, PathBuf},
};
use zip::{result::ZipError, ZipArchive};

#[derive(Debug)]
pub enum UnzipError {
    FileOpeningFailed(PathBuf, io::Error),
    UnzipFailed(PathBuf, ZipError),
    UnzipFileFailed(PathBuf, String, ZipError),
    DestinationWritingFailed(io::Error),
}

impl Display for UnzipError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::FileOpeningFailed(path, err) => {
                write!(f, "Unable to open {}: {}", path.display(), err)
            }
            Self::UnzipFailed(path, err) => {
                write!(f, "Unable to unzip {}: {}", path.display(), err)
            }
            Self::UnzipFileFailed(path, filename, err) => write!(
                f,
                "Unable to unzip {} from {}: {}",
                filename,
                path.display(),
                err
            ),
            Self::DestinationWritingFailed(err) => {
                write!(f, "Unable to write unzipped file: {}", err)
            }
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
        filename: &str,
        dest: &mut dyn Write,
    ) -> Result<(), UnzipError> {
        debug!("Unzipping {} from {}", filename, zip_filepath.display());
        let zip_file = File::open(zip_filepath)
            .map_err(|err| UnzipError::FileOpeningFailed(zip_filepath.to_path_buf(), err))?;
        let zip_file_buf = BufReader::new(zip_file);
        let mut zip = ZipArchive::new(zip_file_buf)
            .map_err(|err| UnzipError::UnzipFailed(zip_filepath.to_path_buf(), err))?;
        let mut tgt_file = zip.by_name(filename).map_err(|err| {
            UnzipError::UnzipFileFailed(zip_filepath.to_path_buf(), filename.into(), err)
        })?;
        copy(&mut tgt_file, dest).map_err(UnzipError::DestinationWritingFailed)?;
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
    use tempfile::{tempdir, tempfile};

    mod unziper {
        use super::*;

        mod unzip {
            use super::*;

            #[test]
            fn should_return_file_opening_failed_err() {
                let expected = temp_dir().join("test");
                let mut out = vec![];
                match DefaultUnziper.unzip(&expected, "test", &mut out) {
                    Ok(_) => panic!("should fail"),
                    Err(UnzipError::FileOpeningFailed(filepath, _)) => {
                        assert_eq!(filepath, expected)
                    }
                    Err(err) => panic!("{}", err),
                }
            }

            #[test]
            fn should_return_unzip_failed_err() {
                let dirpath = tempdir().unwrap().into_path();
                create_dir_all(&dirpath).unwrap();
                let expected_zip_filepath = dirpath.join("test");
                let _ = File::create(&expected_zip_filepath).unwrap();
                let mut out = vec![];
                match DefaultUnziper.unzip(&expected_zip_filepath, "test", &mut out) {
                    Ok(_) => panic!("should fail"),
                    Err(UnzipError::UnzipFailed(zip_filepath, _)) => {
                        assert_eq!(zip_filepath, expected_zip_filepath);
                    }
                    Err(err) => panic!("{}", err),
                }
            }

            #[test]
            fn should_return_unzip_file_failed_err() {
                let expected_zip_filepath = Path::new("resources/tests/unziper/test.zip");
                let expected_filename = "test2";
                let mut out = vec![];
                match DefaultUnziper.unzip(expected_zip_filepath, expected_filename, &mut out) {
                    Ok(_) => panic!("should fail"),
                    Err(UnzipError::UnzipFileFailed(zip_filepath, filename, _)) => {
                        assert_eq!(zip_filepath, expected_zip_filepath);
                        assert_eq!(filename, expected_filename);
                    }
                    Err(err) => panic!("{}", err),
                }
            }

            #[test]
            fn should_extract_file() {
                let mut out = vec![];
                let filepath = Path::new("resources/tests/unziper/test.zip");
                DefaultUnziper.unzip(filepath, "test", &mut out).unwrap();
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
                    let filepath = Path::new("terraform.zip");
                    let err = io::Error::from(io::ErrorKind::PermissionDenied);
                    let expected = format!("Unable to open {}: {}", filepath.display(), err);
                    let err = UnzipError::FileOpeningFailed(filepath.to_path_buf(), err);
                    assert_eq!(err.to_string(), expected);
                }
            }

            mod unzip_failed {
                use super::*;

                #[test]
                fn should_return_string() {
                    let filepath = Path::new("terraform.zip");
                    let file = tempfile().unwrap();
                    let err = ZipArchive::new(file).unwrap_err();
                    let expected = format!("Unable to unzip {}: {}", filepath.display(), err);
                    let err = UnzipError::UnzipFailed(filepath.to_path_buf(), err);
                    assert_eq!(err.to_string(), expected);
                }
            }

            mod unzip_file_failed {
                use super::*;

                #[test]
                fn should_return_string() {
                    let filepath = Path::new("terraform.zip");
                    let filename = "terraform";
                    let file = tempfile().unwrap();
                    let err = ZipArchive::new(file).unwrap_err();
                    let expected = format!(
                        "Unable to unzip {} from {}: {}",
                        filename,
                        filepath.display(),
                        err
                    );
                    let err =
                        UnzipError::UnzipFileFailed(filepath.to_path_buf(), filename.into(), err);
                    assert_eq!(err.to_string(), expected);
                }
            }

            mod destination_writing_failed {
                use super::*;

                #[test]
                fn should_return_string() {
                    let err = io::Error::from(io::ErrorKind::PermissionDenied);
                    let expected = format!("Unable to write unzipped file: {}", err);
                    let err = UnzipError::DestinationWritingFailed(err);
                    assert_eq!(err.to_string(), expected);
                }
            }
        }
    }
}
