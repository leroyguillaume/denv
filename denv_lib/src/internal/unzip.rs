use crate::error::*;
use log::debug;
use std::{
    fs::File,
    io::{copy, BufReader, Write},
    path::Path,
};
use zip::ZipArchive;

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
        let zip_file = File::open(zip_filepath).map_err(UnzipError::FileReadingFailed)?;
        let zip_file_buf = BufReader::new(zip_file);
        let mut zip = ZipArchive::new(zip_file_buf).map_err(UnzipError::InvalidZipFile)?;
        let mut tgt_file = zip.by_name(filepath).map_err(UnzipError::UnzipFailed)?;
        copy(&mut tgt_file, dest).map_err(UnzipError::DestinationWritingFailed)?;
        Ok(())
    }
}

#[cfg(test)]
type UnzipFn = dyn Fn(&Path, &str, &mut dyn Write) -> Result<(), UnzipError>;

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
mod test {
    use super::*;
    use std::{
        env::temp_dir,
        fs::{create_dir_all, File},
    };
    use tempfile::tempdir;
    use zip::{write::FileOptions, ZipWriter};

    mod unzipper {
        use super::*;

        mod unzip {
            use super::*;

            #[test]
            fn should_return_file_reading_failed_err() {
                let zip_filepath = temp_dir().join("test");
                let mut out = vec![];
                match DefaultUnzipper.unzip(&zip_filepath, "test", &mut out) {
                    Ok(_) => panic!("should fail"),
                    Err(UnzipError::FileReadingFailed(_)) => {}
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
                let expected = "Hello world!";
                let dirpath = tempdir().unwrap().into_path();
                let zip_filepath = dirpath.join("terraform.zip");
                let file = File::create(&zip_filepath).unwrap();
                let mut zip = ZipWriter::new(file);
                zip.start_file("test", FileOptions::default()).unwrap();
                zip.write_all(expected.as_bytes()).unwrap();
                zip.finish().unwrap();
                let mut out = vec![];
                match DefaultUnzipper.unzip(&zip_filepath, "test2", &mut out) {
                    Ok(_) => panic!("should fail"),
                    Err(UnzipError::UnzipFailed(_)) => {}
                    Err(err) => panic!("{}", err),
                }
            }

            #[test]
            fn should_extract_file() {
                let filepath = "test";
                let expected = "Hello world!";
                let dirpath = tempdir().unwrap().into_path();
                let zip_filepath = dirpath.join("terraform.zip");
                let file = File::create(&zip_filepath).unwrap();
                let mut zip = ZipWriter::new(file);
                zip.start_file(filepath, FileOptions::default()).unwrap();
                zip.write_all(expected.as_bytes()).unwrap();
                zip.finish().unwrap();
                let mut out = vec![];
                DefaultUnzipper
                    .unzip(&zip_filepath, filepath, &mut out)
                    .unwrap();
                assert_eq!(String::from_utf8(out).unwrap(), expected);
            }
        }
    }
}
