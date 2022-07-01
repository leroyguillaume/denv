use crate::*;
use bytes::Bytes;
use log::debug;
use reqwest::{self, blocking::get};
use std::fmt::{self, Display, Formatter};

pub enum DownloadError {
    RequestProcessingFailed(reqwest::Error),
    RequestFailed(u16),
}

impl Display for DownloadError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::RequestProcessingFailed(err) => write!(f, "Unable to process request: {}", err),
            Self::RequestFailed(status) => write!(f, "Server returned an error {}", status),
        }
    }
}

pub trait Downloader {
    fn download(&self, url: &str) -> Result<Bytes, DownloadError>;
}

pub struct DefaultDownloader;

impl Downloader for DefaultDownloader {
    fn download(&self, url: &str) -> Result<Bytes, DownloadError> {
        debug!("Processing GET request on {}", url);
        let resp = map_debug_err!(get(url), DownloadError::RequestProcessingFailed)?;
        let status = resp.status();
        let content = map_debug_err!(resp.bytes(), |_| DownloadError::RequestFailed(
            status.as_u16()
        ))?;
        if !status.is_success() {
            return debug_err!(Err(DownloadError::RequestFailed(status.as_u16())));
        }
        Ok(content)
    }
}

#[cfg(test)]
mod test {
    use super::*;

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
                    let err = get("http://google.fr/notfound").unwrap();
                    let status = err.status().as_u16();
                    let expected = format!("Server returned an error {}", status);
                    let err = DownloadError::RequestFailed(status);
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
                match DefaultDownloader.download(url) {
                    Ok(_) => panic!("should fail"),
                    Err(DownloadError::RequestProcessingFailed(_)) => {}
                    Err(err) => panic!("{}", err),
                }
            }

            #[test]
            fn should_return_request_failed_err() {
                let url = "https://fr.archive.ubuntu.com/ubuntu2/";
                let expected = get(url).unwrap();
                match DefaultDownloader.download(url) {
                    Ok(_) => panic!("should fail"),
                    Err(DownloadError::RequestFailed(status)) => {
                        assert_eq!(status, expected.status().as_u16());
                    }
                    Err(err) => panic!("{}", err),
                }
            }

            #[test]
            fn should_return_bytes() {
                let url = "http://fr.archive.ubuntu.com/ubuntu/";
                let expected = get(url).unwrap();
                match DefaultDownloader.download(url) {
                    Ok(content) => assert_eq!(content, expected.bytes().unwrap()),
                    Err(err) => panic!("{}", err),
                }
            }
        }
    }
}
