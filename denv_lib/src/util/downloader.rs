use crate::*;
use log::{debug, trace};
use reqwest::{self, blocking::get};
use std::{
    fmt::{self, Display, Formatter},
    io::{BufWriter, Write},
};

pub enum DownloadError {
    RequestProcessingFailed(reqwest::Error),
    RequestFailed(u16, String),
    WritingFailed(reqwest::Error),
}

impl Display for DownloadError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::RequestProcessingFailed(err) => write!(f, "Unable to process request: {}", err),
            Self::RequestFailed(status, content) => {
                write!(f, "Server sent an error {}: {}", status, content)
            }
            Self::WritingFailed(err) => {
                write!(f, "Unable to write response content to file: {}", err)
            }
        }
    }
}

pub trait Downloader {
    fn download(&self, url: &str, out: &mut dyn Write) -> Result<(), DownloadError>;
}

pub struct DefaultDownloader;

impl Downloader for DefaultDownloader {
    fn download(&self, url: &str, out: &mut dyn Write) -> Result<(), DownloadError> {
        let mut buf = BufWriter::new(out);
        debug!("Processing GET request on {}", url);
        let mut resp = map_debug_err!(get(url), DownloadError::RequestProcessingFailed)?;
        let status = resp.status();
        debug!("Server sent status code {}", status.as_u16());
        if !status.is_success() {
            let content = resp.text().unwrap_or_default();
            return debug_err!(Err(DownloadError::RequestFailed(status.as_u16(), content)));
        }
        let size = map_debug_err!(resp.copy_to(&mut buf), DownloadError::WritingFailed)?;
        trace!("{} bytes written", size);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io;

    struct WriteFailer;

    impl Write for WriteFailer {
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }

        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(io::ErrorKind::BrokenPipe))
        }
    }

    mod download_error {
        use super::*;

        mod to_string {
            use super::*;

            mod request_processing_failed {
                use super::*;

                #[test]
                fn should_return_string() {
                    let err = get("htpp://localhost:1234").unwrap_err();
                    let expected = format!("Unable to process request: {}", err);
                    let err = DownloadError::RequestProcessingFailed(err);
                    assert_eq!(err.to_string(), expected);
                }
            }

            mod request_failed {
                use super::*;

                #[test]
                fn should_return_string() {
                    let resp = get("https://fr.archive.ubuntu.com/ubuntu2/").unwrap();
                    let status = resp.status().as_u16();
                    let content = resp.text().unwrap();
                    let expected = format!("Server sent an error {}: {}", status, content);
                    let err = DownloadError::RequestFailed(status, content);
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
                    let expected = format!("Unable to write response content to file: {}", err);
                    let err = DownloadError::WritingFailed(err);
                    assert_eq!(err.to_string(), expected);
                }
            }
        }
    }

    mod default_downloader {
        use super::*;

        mod download {
            use super::*;

            #[test]
            fn should_return_request_processing_failed_err() {
                let url = "htpp://localhost:1234";
                let mut out = vec![];
                match DefaultDownloader.download(url, &mut out) {
                    Ok(_) => panic!("should fail"),
                    Err(DownloadError::RequestProcessingFailed(_)) => {}
                    Err(err) => panic!("{}", err),
                }
            }

            #[test]
            fn should_return_request_failed_err() {
                let url = "https://fr.archive.ubuntu.com/ubuntu2/";
                let mut out = vec![];
                let expected = get(url).unwrap();
                match DefaultDownloader.download(url, &mut out) {
                    Ok(_) => panic!("should fail"),
                    Err(DownloadError::RequestFailed(status, content)) => {
                        assert_eq!(status, expected.status().as_u16());
                        assert_eq!(content, expected.text().unwrap());
                        assert!(out.is_empty());
                    }
                    Err(err) => panic!("{}", err),
                }
            }

            #[test]
            fn should_return_writing_failed_err() {
                let url = "http://fr.archive.ubuntu.com/ubuntu/";
                let mut out = WriteFailer;
                match DefaultDownloader.download(url, &mut out) {
                    Ok(_) => panic!("should fail"),
                    Err(DownloadError::WritingFailed(_)) => {}
                    Err(err) => panic!("{}", err),
                }
            }

            #[test]
            fn should_write_bytes_in_file() {
                let url = "http://fr.archive.ubuntu.com/ubuntu/";
                let mut out = vec![];
                let expected = get(url).unwrap().text().unwrap();
                match DefaultDownloader.download(url, &mut out) {
                    Ok(_) => assert_eq!(String::from_utf8(out).unwrap(), expected),
                    Err(err) => panic!("{}", err),
                }
            }
        }
    }
}
