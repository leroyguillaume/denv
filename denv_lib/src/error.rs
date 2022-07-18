use reqwest::blocking::Response;
#[cfg(test)]
use std::io::Write;
use std::{
    fmt::{self, Display, Formatter},
    io,
    path::{Path, PathBuf},
};
use zip::result::ZipError;

#[derive(Debug)]
pub enum DownloadError {
    RequestProcessingFailed(reqwest::Error),
    RequestFailed(Response),
    WritingFailed(reqwest::Error),
}

impl Display for DownloadError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::RequestProcessingFailed(err) => write!(
                f,
                "GET {} failed: {}",
                err.url()
                    .map(|url| url.to_string())
                    .unwrap_or_else(|| "?".into()),
                err
            ),
            Self::RequestFailed(resp) => {
                write!(
                    f,
                    "GET {} returned an error {}",
                    resp.url(),
                    resp.status().as_u16()
                )
            }
            Self::WritingFailed(err) => write!(f, "I/O failed: {}", err),
        }
    }
}

#[derive(Debug)]
pub struct FileSystemError {
    path: PathBuf,
    source: io::Error,
}

impl FileSystemError {
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

impl Display for FileSystemError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "I/O failed on {}: {}", self.path.display(), self.source)
    }
}

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

#[cfg(test)]
pub(crate) struct WriteFailer;

#[cfg(test)]
impl Write for WriteFailer {
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::from(io::ErrorKind::BrokenPipe))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tempfile::tempfile;
    use zip::ZipArchive;

    mod download_error {
        use super::*;
        use reqwest::blocking::get;

        mod to_string {
            use super::*;

            mod request_processing_failed {
                use super::*;

                #[test]
                fn should_return_string() {
                    let url = "htpp://localhost:1234";
                    let err = get(url).unwrap_err();
                    let expected = format!("GET {} failed: {}", url, err);
                    let err = DownloadError::RequestProcessingFailed(err);
                    assert_eq!(err.to_string(), expected);
                }
            }

            mod request_failed {
                use super::*;

                #[test]
                fn should_return_string() {
                    let resp = get("https://fr.archive.ubuntu.com/ubuntu2/").unwrap();
                    let expected = format!(
                        "GET {} returned an error {}",
                        resp.url(),
                        resp.status().as_u16()
                    );
                    let err = DownloadError::RequestFailed(resp);
                    assert_eq!(err.to_string(), expected);
                }
            }

            mod writing_failed {
                use super::*;

                #[test]
                fn should_return_string() {
                    let mut out = WriteFailer;
                    let err = get("https://google.fr")
                        .unwrap()
                        .copy_to(&mut out)
                        .unwrap_err();
                    let expected = format!("I/O failed: {}", err);
                    let err = DownloadError::WritingFailed(err);
                    assert_eq!(err.to_string(), expected);
                }
            }
        }
    }

    mod file_system_error {
        use super::*;

        mod new {
            use super::*;

            #[test]
            fn should_return_error() {
                let path = PathBuf::from("/error");
                let source_kind = io::ErrorKind::PermissionDenied;
                let source = io::Error::from(source_kind);
                let err = FileSystemError::new(path.clone(), source);
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
                let err = FileSystemError::new(path, source);
                assert_eq!(err.to_string(), expected);
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
