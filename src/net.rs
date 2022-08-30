// IMPORTS

use log::debug;
use reqwest::blocking;
use std::io::{self, BufWriter, Error, ErrorKind, Write};
#[cfg(test)]
use stub_trait::stub;

// TYPES

pub type Result = io::Result<()>;

// TRAITS

#[cfg_attr(test, stub)]
pub trait Downloader {
    fn download(&self, url: &str, out: &mut dyn Write) -> Result;
}

// STRUCTS

pub struct DefaultDownloader;

impl Downloader for DefaultDownloader {
    fn download(&self, url: &str, out: &mut dyn Write) -> Result {
        let mut buf = BufWriter::new(out);
        debug!("Processing GET request on {}", url);
        let mut resp = blocking::get(url).map_err(|err| Error::new(ErrorKind::Other, err))?;
        let status = resp.status();
        debug!("Server sent status code {}", status.as_u16());
        if !status.is_success() {
            return Err(Error::new(
                ErrorKind::Other,
                format!("Server sent status code {}", status.as_u16()),
            ));
        }
        resp.copy_to(&mut buf)
            .map_err(|err| Error::new(ErrorKind::Other, err))?;
        Ok(())
    }
}

// TESTS

#[cfg(test)]
mod default_downloader_test {
    use super::*;
    use crate::test::WriteFailer;

    mod download {
        use super::*;

        #[test]
        fn should_return_err_if_get_failed() {
            test("not an url", vec![], |_, res| {
                let err = res.unwrap_err();
                assert_eq!(err.kind(), ErrorKind::Other);
            });
        }

        #[test]
        fn should_return_err_if_server_sent_error() {
            test(
                "https://fr.archive.ubuntu.com/ubuntu2/",
                vec![],
                |_, res| {
                    let err = res.unwrap_err();
                    assert_eq!(err.kind(), ErrorKind::Other);
                    assert_eq!(err.to_string(), "Server sent status code 404");
                },
            );
        }

        #[test]
        fn should_return_err_if_write_on_output_failed() {
            test(
                "https://fr.archive.ubuntu.com/ubuntu/",
                WriteFailer,
                |_, res| {
                    res.unwrap_err();
                },
            );
        }

        #[test]
        fn should_return_ok() {
            let url = "https://fr.archive.ubuntu.com/ubuntu/";
            test(url, vec![], |out, res| {
                res.unwrap();
                let content = blocking::get(url).unwrap().text().unwrap();
                let out = String::from_utf8(out).unwrap();
                assert_eq!(out, content);
            });
        }

        #[inline]
        fn test<W: Write, F: Fn(W, Result)>(url: &str, mut out: W, assert_fn: F) {
            let downloader = DefaultDownloader;
            let res = downloader.download(url, &mut out);
            assert_fn(out, res);
        }
    }
}
