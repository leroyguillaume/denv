// IMPORTS

use super::Error;
use crate::{
    archive::{DefaultUnarchiver, Unarchiver},
    fs::FileSystem,
    net::{DefaultDownloader, Downloader},
};
use log::{debug, warn};
use std::{
    io,
    path::{Path, PathBuf},
};
#[cfg(test)]
use stub_trait::stub;

// TYPES

pub type Result = super::Result<()>;

// DATA STRUCTS

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Artifact<'a> {
    pub bin_filepaths: Vec<&'static Path>,
    pub name: &'a str,
    pub symlinks: Vec<Symlink>,
    pub url: String,
    pub version: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Symlink {
    pub dest: PathBuf,
    pub required: bool,
    pub src: &'static Path,
}

// TRAITS

#[cfg_attr(test, stub)]
pub trait ArchiveArtifactInstaller {
    fn install_targz(&self, artifact: &Artifact, fs: &dyn FileSystem) -> Result;

    fn install_zip(&self, artifact: &Artifact, fs: &dyn FileSystem) -> Result;
}

// STRUCTS

pub struct DefaultArchiveArtifactInstaller {
    downloader: Box<dyn Downloader>,
    unarchiver: Box<dyn Unarchiver>,
}

impl DefaultArchiveArtifactInstaller {
    #[inline]
    fn install<F: Fn(&Path, &Path) -> io::Result<()>>(
        &self,
        artifact: &Artifact,
        fs: &dyn FileSystem,
        unarchive_fn: F,
    ) -> Result {
        let soft_dirpath = fs
            .ensure_software_dir(artifact.name, artifact.version)
            .map_err(Error::Io)?;
        let installed = artifact
            .bin_filepaths
            .iter()
            .all(|path| fs.file_exists(&soft_dirpath.join(path)));
        if installed {
            debug!(
                "{} v{} is already installed",
                artifact.name, artifact.version
            );
        } else {
            debug!("Installing {} v{}", artifact.name, artifact.version);
            let mut archive_file = fs.create_temp_file().map_err(Error::Io)?;
            self.downloader
                .download(&artifact.url, &mut archive_file.file)
                .map_err(Error::Io)?;
            unarchive_fn(&archive_file.path, &soft_dirpath).map_err(Error::Io)?;
        }
        for bin_filepath in &artifact.bin_filepaths {
            let bin_filepath = soft_dirpath.join(bin_filepath);
            fs.make_executable(&bin_filepath).map_err(Error::Io)?;
        }
        for symlink in &artifact.symlinks {
            let src = soft_dirpath.join(symlink.src);
            match fs.ensure_symlink(&src, &symlink.dest) {
                Ok(()) => {}
                Err(err) => {
                    if symlink.required {
                        return Err(Error::Io(err));
                    } else {
                        warn!(
                            "{}: Unable to update symlink {}: {}",
                            artifact.name,
                            symlink.dest.display(),
                            err
                        );
                    }
                }
            }
        }
        debug!("{} v{} installed", artifact.name, artifact.version);
        Ok(())
    }
}

impl ArchiveArtifactInstaller for DefaultArchiveArtifactInstaller {
    fn install_targz(&self, artifact: &Artifact, fs: &dyn FileSystem) -> Result {
        self.install(artifact, fs, |archive_filepath, dest| {
            self.unarchiver.untar(archive_filepath, dest)
        })
    }

    fn install_zip(&self, artifact: &Artifact, fs: &dyn FileSystem) -> Result {
        self.install(artifact, fs, |archive_filepath, dest| {
            self.unarchiver.unzip(archive_filepath, dest)
        })
    }
}

impl Default for DefaultArchiveArtifactInstaller {
    fn default() -> Self {
        Self {
            downloader: Box::new(DefaultDownloader),
            unarchiver: Box::new(DefaultUnarchiver),
        }
    }
}

// TESTS

#[cfg(test)]
mod default_archive_artifact_installer {
    use super::*;
    use crate::{
        archive::StubUnarchiver,
        fs::{StubFileSystem, TempFile},
        net::StubDownloader,
    };

    mod install {
        use super::*;

        macro_rules! tests {
            ($ident:ident, $method:ident, $stub_method:ident) => {
                mod $ident {
                    use super::*;

                    struct Data {
                        archive_filepath: &'static Path,
                        artifact: Artifact<'static>,
                        soft_dirpath: &'static Path,
                        soft_is_installed: bool,
                    }

                    impl Default for Data {
                        fn default() -> Self {
                            Self {
                                archive_filepath: Path::new("/archive"),
                                artifact: Artifact {
                                    bin_filepaths: vec![Path::new("bin")],
                                    name: "soft",
                                    symlinks: vec![
                                        Symlink {
                                            dest: PathBuf::from("/dest1"),
                                            required: true,
                                            src: Path::new("bin1"),
                                        },
                                        Symlink {
                                            dest: PathBuf::from("/dest2"),
                                            required: false,
                                            src: Path::new("/bin2"),
                                        },
                                    ],
                                    url: "url".into(),
                                    version: "1.0.0",
                                },
                                soft_dirpath: Path::new("/soft"),
                                soft_is_installed: false,
                            }
                        }
                    }

                    struct Stubs {
                        downloader: StubDownloader,
                        fs: StubFileSystem,
                        unarchiver: StubUnarchiver,
                    }

                    impl Stubs {
                        fn new(data: &Data) -> Self {
                            let expected_archive_filepath = data.archive_filepath;
                            let bin_filepath = data.artifact.bin_filepaths[0];
                            let expected_name = data.artifact.name;
                            let soft_dirpath = data.soft_dirpath;
                            let soft_is_installed = data.soft_is_installed;
                            let symlink1 = data.artifact.symlinks[0].clone();
                            let symlink2 = data.artifact.symlinks[1].clone();
                            let expected_version = data.artifact.version;
                            let expected_url = data.artifact.url.clone();
                            let mut stubs = Self {
                                downloader: StubDownloader::default(),
                                fs: StubFileSystem::default(),
                                unarchiver: StubUnarchiver::default(),
                            };
                            stubs.fs.stub_ensure_software_dir_fn(move |name, version| {
                                assert_eq!(name, expected_name);
                                assert_eq!(version, expected_version);
                                Ok(soft_dirpath.to_path_buf())
                            });
                            stubs.fs.stub_file_exists_fn(move |path| {
                                assert_eq!(path, soft_dirpath.join(bin_filepath));
                                soft_is_installed
                            });
                            if !soft_is_installed {
                                stubs.fs.stub_create_temp_file_fn(|| {
                                    let file = TempFile {
                                        file: tempfile::tempfile().unwrap(),
                                        path: expected_archive_filepath.to_path_buf(),
                                    };
                                    Ok(file)
                                });
                                stubs.downloader.stub_download_fn(move |url, _| {
                                    assert_eq!(url, expected_url);
                                    Ok(())
                                });
                                stubs
                                    .unarchiver
                                    .$stub_method(move |archive_filepath, dest| {
                                        assert_eq!(archive_filepath, expected_archive_filepath);
                                        assert_eq!(dest, soft_dirpath);
                                        Ok(())
                                    });
                            }
                            stubs.fs.stub_make_executable_fn(move |path| {
                                assert_eq!(path, soft_dirpath.join(bin_filepath));
                                Ok(())
                            });
                            stubs.fs.stub_ensure_symlink_fn(move |src, dest| {
                                if src == soft_dirpath.join(symlink1.src) {
                                    assert_eq!(dest, symlink1.dest);
                                    Ok(())
                                } else if src == soft_dirpath.join(symlink2.src) {
                                    assert_eq!(dest, symlink2.dest);
                                    Err(io::Error::from(io::ErrorKind::PermissionDenied))
                                } else {
                                    panic!("Unexpected src: {}", src.display());
                                }
                            });
                            stubs
                        }
                    }

                    #[test]
                    fn should_return_io_err_if_ensure_software_dir_failed() {
                        let data = Data::default();
                        let mut stubs = Stubs::new(&data);
                        stubs.fs.stub_ensure_software_dir_fn(|_, _| {
                            Err(io::Error::from(io::ErrorKind::PermissionDenied))
                        });
                        test(&data, stubs, |res| match res.unwrap_err() {
                            Error::Io(_) => {}
                            err => panic!("{}", err),
                        })
                    }

                    #[test]
                    fn should_return_io_err_if_create_temp_file_failed() {
                        let data = Data::default();
                        let mut stubs = Stubs::new(&data);
                        stubs.fs.stub_create_temp_file_fn(|| {
                            Err(io::Error::from(io::ErrorKind::PermissionDenied))
                        });
                        test(&data, stubs, |res| match res.unwrap_err() {
                            Error::Io(_) => {}
                            err => panic!("{}", err),
                        })
                    }

                    #[test]
                    fn should_return_io_err_if_download_failed() {
                        let data = Data::default();
                        let mut stubs = Stubs::new(&data);
                        stubs.downloader.stub_download_fn(|_, _| {
                            Err(io::Error::from(io::ErrorKind::PermissionDenied))
                        });
                        test(&data, stubs, |res| match res.unwrap_err() {
                            Error::Io(_) => {}
                            err => panic!("{}", err),
                        })
                    }

                    #[test]
                    fn should_return_io_err_if_untar_failed() {
                        let data = Data::default();
                        let mut stubs = Stubs::new(&data);
                        stubs.unarchiver.$stub_method(|_, _| {
                            Err(io::Error::from(io::ErrorKind::PermissionDenied))
                        });
                        test(&data, stubs, |res| match res.unwrap_err() {
                            Error::Io(_) => {}
                            err => panic!("{}", err),
                        })
                    }

                    #[test]
                    fn should_return_io_err_if_make_executable_failed() {
                        let data = Data::default();
                        let mut stubs = Stubs::new(&data);
                        stubs.fs.stub_make_executable_fn(|_| {
                            Err(io::Error::from(io::ErrorKind::PermissionDenied))
                        });
                        test(&data, stubs, |res| match res.unwrap_err() {
                            Error::Io(_) => {}
                            err => panic!("{}", err),
                        })
                    }

                    #[test]
                    fn should_return_io_err_if_ensure_symlink_failed() {
                        let data = Data::default();
                        let mut stubs = Stubs::new(&data);
                        stubs.fs.stub_ensure_symlink_fn(|_, _| {
                            Err(io::Error::from(io::ErrorKind::PermissionDenied))
                        });
                        test(&data, stubs, |res| match res.unwrap_err() {
                            Error::Io(_) => {}
                            err => panic!("{}", err),
                        })
                    }

                    #[test]
                    fn should_return_ok_if_software_is_not_installed() {
                        let data = Data::default();
                        let stubs = Stubs::new(&data);
                        test(&data, stubs, |res| {
                            res.unwrap();
                        })
                    }

                    #[test]
                    fn should_return_ok_if_software_is_installed() {
                        let data = Data {
                            soft_is_installed: true,
                            ..Data::default()
                        };
                        let stubs = Stubs::new(&data);
                        test(&data, stubs, |res| {
                            res.unwrap();
                        })
                    }

                    #[inline]
                    fn test<F: Fn(Result)>(data: &Data, stubs: Stubs, assert_fn: F) {
                        let installer = DefaultArchiveArtifactInstaller {
                            downloader: Box::new(stubs.downloader),
                            unarchiver: Box::new(stubs.unarchiver),
                        };
                        let res = installer.$method(&data.artifact, &stubs.fs);
                        assert_fn(res);
                    }
                }
            };
        }

        tests!(targz, install_targz, stub_untar_fn);
        tests!(zip, install_zip, stub_unzip_fn);
    }
}
