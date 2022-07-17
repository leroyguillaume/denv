use log::{debug, trace};
use reqwest::{
    self,
    blocking::{get, Response},
};
use std::{
    fmt::{self, Display, Formatter},
    io::{BufWriter, Write},
};

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
        let mut resp = get(url).map_err(DownloadError::RequestProcessingFailed)?;
        let status = resp.status();
        debug!("Server sent status code {}", status.as_u16());
        if !status.is_success() {
            return Err(DownloadError::RequestFailed(resp));
        }
        let size = resp
            .copy_to(&mut buf)
            .map_err(DownloadError::WritingFailed)?;
        trace!("{} bytes written", size);
        Ok(())
    }
}

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
type DownloadFn = dyn Fn(&str, &mut dyn Write) -> Result<(), DownloadError>;

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
