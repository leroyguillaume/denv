use crate::error::*;
use log::debug;
use reqwest::{self, blocking::get};
use std::io::{BufWriter, Write};

pub trait Downloader {
    fn download(&self, url: &str, out: &mut dyn Write) -> Result<(), DownloadError>;
}

pub struct DefaultDownloader;

impl Downloader for DefaultDownloader {
    fn download(&self, url: &str, out: &mut dyn Write) -> Result<(), DownloadError> {
        let mut buf = BufWriter::new(out);
        debug!("Processing GET request on {}", url);
        let mut resp = get(url).map_err(DownloadError::RequestProcessingFailed)?;
        let status = resp.status();
        debug!("Server sent status code {}", status.as_u16());
        if !status.is_success() {
            return Err(DownloadError::RequestFailed(resp));
        }
        resp.copy_to(&mut buf)
            .map_err(DownloadError::WritingFailed)?;
        Ok(())
    }
}

#[cfg(test)]
type DownloadFn = dyn Fn(&str, &mut dyn Write) -> Result<(), DownloadError>;

#[cfg(test)]
#[derive(Default)]
pub struct StubDownloader {
    download_fn: Option<Box<DownloadFn>>,
}

#[cfg(test)]
impl StubDownloader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_download_fn<F: Fn(&str, &mut dyn Write) -> Result<(), DownloadError> + 'static>(
        mut self,
        download_fn: F,
    ) -> Self {
        self.download_fn = Some(Box::new(download_fn));
        self
    }
}

#[cfg(test)]
impl Downloader for StubDownloader {
    fn download(&self, url: &str, out: &mut dyn Write) -> Result<(), DownloadError> {
        match &self.download_fn {
            Some(download_fn) => download_fn(url, out),
            None => unimplemented!(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

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
                match DefaultDownloader.download(url, &mut out) {
                    Ok(_) => panic!("should fail"),
                    Err(DownloadError::RequestFailed(_)) => {}
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
