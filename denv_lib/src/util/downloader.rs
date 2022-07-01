use bytes::Bytes;
use log::debug;
use reqwest::{self, blocking::get, StatusCode};
use std::fmt::{self, Display, Formatter};

macro_rules! map_err {
    ($call:expr, $map_err:expr) => {
        match $call {
            Ok(val) => Ok(val),
            Err(err) => {
                let err = $map_err(err);
                debug!("{}", err);
                Err(err)
            }
        }
    };
}

pub enum DownloadError {
    ProcessingRequest(reqwest::Error),
    Http(StatusCode, Option<Bytes>),
}

impl Display for DownloadError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::ProcessingRequest(err) => write!(f, "Unable to process request: {}", err),
            Self::Http(status, content) => match content {
                Some(content) => {
                    let content = String::from_utf8_lossy(content);
                    write!(f, "Server returned an error {}: {}", status, content)
                }
                None => write!(f, "Server returned an error {}", status),
            },
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
        let resp = map_err!(get(url), DownloadError::ProcessingRequest)?;
        let status = resp.status();
        let content = map_err!(resp.bytes(), |_| DownloadError::Http(status, None))?;
        if !status.is_success() {
            let err = DownloadError::Http(status, Some(content));
            debug!("{}", err);
            return Err(err);
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

            mod processing_request {
                use super::*;

                #[test]
                fn should_return_string() {
                    let err = get("htpp://localhost:1234").unwrap_err();
                    let expected = format!("Unable to process request: {}", err);
                    let err = DownloadError::ProcessingRequest(err);
                    assert_eq!(err.to_string(), expected);
                }
            }

            mod http {
                use super::*;

                #[test]
                fn should_return_string() {
                    let err = get("http://google.fr/notfound").unwrap();
                    let status = err.status();
                    let bytes = err.bytes().unwrap();
                    let content = String::from_utf8_lossy(&bytes);
                    let expected = format!("Server returned an error {}: {}", status, content);
                    let err = DownloadError::Http(status, Some(bytes));
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
            fn should_return_processing_request_err() {
                let url = "htpp://localhost:1234";
                match DefaultDownloader.download(url) {
                    Ok(_) => panic!("should fail"),
                    Err(DownloadError::ProcessingRequest(_)) => {}
                    Err(err) => panic!("{}", err),
                }
            }

            #[test]
            fn should_return_http_err() {
                let url = "https://apt.releases.hashicorp.com/gpg2";
                let expected = get(url).unwrap();
                match DefaultDownloader.download(url) {
                    Ok(_) => panic!("should fail"),
                    Err(DownloadError::Http(status, content)) => {
                        assert_eq!(status, expected.status());
                        assert_eq!(content, Some(expected.bytes().unwrap()));
                    }
                    Err(err) => panic!("{}", err),
                }
            }

            #[test]
            fn should_return_bytes() {
                let url = "https://apt.releases.hashicorp.com/gpg";
                let expected = get(url).unwrap();
                match DefaultDownloader.download(url) {
                    Ok(content) => assert_eq!(content, expected.bytes().unwrap()),
                    Err(err) => panic!("{}", err),
                }
            }
        }
    }
}
