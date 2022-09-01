// IMPORTS

use flate2::read::GzDecoder;
use log::debug;
use std::{
    fs::File,
    io::{self, BufReader, Error, ErrorKind},
    path::Path,
};
#[cfg(test)]
use stub_trait::stub;
use tar::Archive;
use zip::ZipArchive;

// TYPES

pub type Result = io::Result<()>;

// TRAITS

#[cfg_attr(test, stub)]
pub trait Unarchiver {
    fn untar(&self, archive_filepath: &Path, dest: &Path) -> Result;

    fn unzip(&self, archive_filepath: &Path, dest: &Path) -> Result;
}

// STRUCTS

pub struct DefaultUnarchiver;

impl Unarchiver for DefaultUnarchiver {
    fn untar(&self, archive_filepath: &Path, dest: &Path) -> Result {
        debug!(
            "Extracting {} into {}",
            archive_filepath.display(),
            dest.display(),
        );
        let tar_file = File::open(archive_filepath)?;
        let decoder = GzDecoder::new(BufReader::new(tar_file));
        let mut tar = Archive::new(decoder);
        tar.unpack(dest)
    }

    fn unzip(&self, archive_filepath: &Path, dest: &Path) -> Result {
        debug!(
            "Extracting {} into {}",
            archive_filepath.display(),
            dest.display(),
        );
        let zip_file = File::open(archive_filepath)?;
        let zip_file_buf = BufReader::new(zip_file);
        let mut zip = ZipArchive::new(zip_file_buf)
            .map_err(|err| Error::new(ErrorKind::InvalidInput, err))?;
        zip.extract(dest)
            .map_err(|err| Error::new(ErrorKind::Other, err))
    }
}

#[cfg(test)]
mod default_unarchiver_test {
    use super::*;
    use flate2::{write::GzEncoder, Compression};
    use std::{fs, io::Write, path::PathBuf};
    use tar::Builder;
    use zip::{write::FileOptions, ZipWriter};

    mod untar {
        use super::*;

        struct Data {
            archive_filepath: PathBuf,
            archived_file_content: &'static str,
            archived_filepath: &'static Path,
            dest: PathBuf,
        }

        impl Default for Data {
            fn default() -> Self {
                Self {
                    archive_filepath: tempfile::tempdir()
                        .unwrap()
                        .into_path()
                        .join("archive.tar.gz"),
                    archived_file_content: "Hello world!",
                    archived_filepath: Path::new("dir/file"),
                    dest: tempfile::tempdir().unwrap().into_path(),
                }
            }
        }

        #[test]
        fn should_return_err_if_file_does_not_exist() {
            let data = Data::default();
            test(&data, |res| {
                res.unwrap_err();
            })
        }

        #[test]
        fn should_return_err_if_archive_is_invalid() {
            let data = Data::default();
            File::create(&data.archive_filepath).unwrap();
            test(&data, |res| {
                res.unwrap_err();
            })
        }

        #[test]
        fn should_return_err_if_dest_is_not_directory() {
            let data = Data::default();
            create_tgz(&data);
            fs::remove_dir_all(&data.dest).unwrap();
            File::create(&data.dest).unwrap();
            test(&data, |res| {
                res.unwrap_err();
            })
        }

        #[test]
        fn should_return_ok() {
            let data = Data::default();
            create_tgz(&data);
            test(&data, |res| {
                res.unwrap();
                let unarchived_filepath = data.dest.join(data.archived_filepath);
                let content = fs::read_to_string(&unarchived_filepath).unwrap();
                assert_eq!(content, data.archived_file_content);
            })
        }

        #[inline]
        fn create_tgz(data: &Data) {
            let temp_dirpath = tempfile::tempdir().unwrap().into_path();
            let archived_filepath = temp_dirpath.join("test");
            let mut archived_file = File::create(&archived_filepath).unwrap();
            write!(archived_file, "{}", data.archived_file_content).unwrap();
            drop(archived_file);
            let mut archived_file = File::open(&archived_filepath).unwrap();
            let tar_file = File::create(&data.archive_filepath).unwrap();
            let encoder = GzEncoder::new(tar_file, Compression::default());
            let mut tar = Builder::new(encoder);
            tar.append_file(data.archived_filepath, &mut archived_file)
                .unwrap();
            tar.finish().unwrap();
        }

        #[inline]
        fn test<F: Fn(Result)>(data: &Data, assert_fn: F) {
            let unarchiver = DefaultUnarchiver;
            let res = unarchiver.untar(&data.archive_filepath, &data.dest);
            assert_fn(res);
        }
    }

    mod unzip {
        use super::*;

        struct Data {
            archive_filepath: PathBuf,
            archived_file_content: &'static str,
            archived_filepath: &'static str,
            dest: PathBuf,
        }

        impl Default for Data {
            fn default() -> Self {
                Self {
                    archive_filepath: tempfile::tempdir().unwrap().into_path().join("archive.zip"),
                    archived_file_content: "Hello world!",
                    archived_filepath: "dir/file",
                    dest: tempfile::tempdir().unwrap().into_path(),
                }
            }
        }

        #[test]
        fn should_return_err_if_file_does_not_exist() {
            let data = Data::default();
            test(&data, |res| {
                res.unwrap_err();
            })
        }

        #[test]
        fn should_return_err_if_archive_is_invalid() {
            let data = Data::default();
            File::create(&data.archive_filepath).unwrap();
            test(&data, |res| {
                let err = res.unwrap_err();
                match err.kind() {
                    ErrorKind::InvalidInput => {}
                    kind => panic!("{}", kind),
                }
            })
        }

        #[test]
        fn should_return_err_if_dest_is_not_directory() {
            let data = Data::default();
            create_zip(&data);
            fs::remove_dir_all(&data.dest).unwrap();
            File::create(&data.dest).unwrap();
            test(&data, |res| {
                res.unwrap_err();
            })
        }

        #[test]
        fn should_return_ok() {
            let data = Data::default();
            create_zip(&data);
            test(&data, |res| {
                res.unwrap();
                let unarchived_filepath = data.dest.join(data.archived_filepath);
                let content = fs::read_to_string(&unarchived_filepath).unwrap();
                assert_eq!(content, data.archived_file_content);
            })
        }

        #[inline]
        fn create_zip(data: &Data) {
            let zip_file = File::create(&data.archive_filepath).unwrap();
            let mut zip = ZipWriter::new(zip_file);
            zip.start_file(data.archived_filepath, FileOptions::default())
                .unwrap();
            zip.write_all(data.archived_file_content.as_bytes())
                .unwrap();
            zip.finish().unwrap();
        }

        #[inline]
        fn test<F: Fn(Result)>(data: &Data, assert_fn: F) {
            let unarchiver = DefaultUnarchiver;
            let res = unarchiver.unzip(&data.archive_filepath, &data.dest);
            assert_fn(res);
        }
    }
}
