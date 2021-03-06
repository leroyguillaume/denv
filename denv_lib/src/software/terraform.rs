use super::*;
use log::debug;
use std::io::BufWriter;

macro_rules! arch {
    () => {
        match std::env::consts::ARCH {
            "x86" => Ok("386"),
            "x86_64" => Ok("amd64"),
            "arm" => Ok("arm"),
            "aarch64" => Ok("arm64"),
            _ => Err(InstallError::UnsupportedArch(supported_systems!())),
        }
    };
}

macro_rules! os {
    () => {
        match std::env::consts::OS {
            "macos" => Ok("darwin"),
            "linux" => Ok("linux"),
            _ => Err(InstallError::UnsupportedOs(supported_systems!())),
        }
    };
}

macro_rules! supported_systems {
    () => {
        maplit::hashmap! {
            "linux" => maplit::hashset!("x86", "x86_64", "arm", "aarch64"),
            "macos" => maplit::hashset!("x86_64", "aarch64"),
        }
    };
}

const SOFTWARE_NAME: &str = "terraform";

#[derive(Debug, Eq, PartialEq)]
pub struct Terraform(pub String);

impl Software for Terraform {
    fn install(&self, cfg: &Config) -> Result<(), InstallError> {
        debug!("Installing {}", self as &dyn Software);
        let os = os!()?;
        let arch = arch!()?;
        let filename = format!("terraform_{}_{}_{}.zip", self.0, os, arch);
        let url = format!(
            "https://releases.hashicorp.com/terraform/{}/{}",
            self.0, filename
        );
        let (zip_filepath, mut zip_file) = cfg
            .fs
            .create_tmp_file(&filename)
            .map_err(InstallError::FileSystemWritingFailed)?;
        cfg.downloader
            .download(&url, &mut zip_file)
            .map_err(InstallError::DownloadFailed)?;
        let (_, bin_file) = cfg
            .fs
            .create_bin_file(SOFTWARE_NAME, &self.0)
            .map_err(InstallError::FileSystemWritingFailed)?;
        let mut file_buf = BufWriter::new(bin_file);
        cfg.unzipper
            .unzip(&zip_filepath, SOFTWARE_NAME, &mut file_buf)
            .map_err(|err| InstallError::UnzipFailed(zip_filepath, SOFTWARE_NAME.into(), err))?;
        debug!("{} installed", self as &dyn Software);
        Ok(())
    }

    fn name(&self) -> &'static str {
        SOFTWARE_NAME
    }

    fn version(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::internal::{downloader::*, fs::*, unzip::*};
    use reqwest::blocking::get;
    use std::{fs::File, io, path::PathBuf};
    use tempfile::tempdir;

    mod terraform {
        use super::*;

        mod install {
            use super::*;

            macro_rules! tests {
                ($os:expr, $arch:expr) => {
                    #[test]
                    #[cfg(all(target_os = $os, target_arch = $arch))]
                    fn should_return_file_system_writing_failed_failed_err_if_tmp_file_creation_failed() {
                        let expected_version = "1.2.3";
                        let tf = Terraform(expected_version.into());
                        let os = os!().unwrap();
                        let arch = arch!().unwrap();
                        let fs = StubFileSystem::new()
                            .with_create_tmp_file_fn({
                                move |filename| {
                                    let expected = format!("terraform_{}_{}_{}.zip", expected_version, os, arch);
                                    assert_eq!(filename, expected);
                                    Err(FileSystemError::new(PathBuf::from("/error"), io::Error::from(io::ErrorKind::PermissionDenied)))
                                }
                            });
                        let cfg = Config::stub().with_fs(fs);
                        match tf.install(&cfg) {
                            Ok(_) => panic!("should fail"),
                            Err(InstallError::FileSystemWritingFailed(_)) => {}
                            Err(err) => panic!("{}", err),
                        }
                    }

                    #[test]
                    #[cfg(all(target_os = $os, target_arch = $arch))]
                    fn should_return_download_failed_err() {
                        let expected_version = "1.2.3";
                        let tf = Terraform(expected_version.into());
                        let os = os!().unwrap();
                        let arch = arch!().unwrap();
                        let zip_filepath = tempdir().unwrap().into_path().join("terraform.zip");
                        let fs = StubFileSystem::new()
                            .with_create_tmp_file_fn({
                                let zip_filepath = zip_filepath.clone();
                                move |filename| {
                                    let expected = format!("terraform_{}_{}_{}.zip", expected_version, os, arch);
                                    assert_eq!(filename, expected);
                                    Ok((
                                        zip_filepath.clone(),
                                        File::create(&zip_filepath).map_err(|err| FileSystemError::new(zip_filepath.clone(), err))?
                                    ))
                                }
                            });
                        let downloader = StubDownloader::new()
                            .with_download_fn(move |url, _| {
                                let expected_url = format!(
                                    "https://releases.hashicorp.com/terraform/{}/terraform_{}_{}_{}.zip",
                                    expected_version, expected_version, os, arch
                                );
                                assert_eq!(url, expected_url);
                                let resp = get("https://fr.archive.ubuntu.com/ubuntu2/").unwrap();
                                Err(DownloadError::RequestFailed(resp))
                            });
                        let cfg = Config::stub().with_fs(fs).with_downloader(downloader);
                        match tf.install(&cfg) {
                            Ok(_) => panic!("should fail"),
                            Err(InstallError::DownloadFailed(_)) => {}
                            Err(err) => panic!("{}", err),
                        }
                    }

                    #[test]
                    #[cfg(all(target_os = $os, target_arch = $arch))]
                    fn should_return_file_system_writing_failed_err_if_bin_file_creation_failed() {
                        let expected_version = "1.2.3";
                        let tf = Terraform(expected_version.into());
                        let os = os!().unwrap();
                        let arch = arch!().unwrap();
                        let zip_filepath = tempdir().unwrap().into_path().join("terraform.zip");
                        let fs = StubFileSystem::new()
                            .with_create_tmp_file_fn({
                                let zip_filepath = zip_filepath.clone();
                                move |filename| {
                                    let expected = format!("terraform_{}_{}_{}.zip", expected_version, os, arch);
                                    assert_eq!(filename, expected);
                                    Ok((
                                        zip_filepath.clone(),
                                        File::create(&zip_filepath).map_err(|err| FileSystemError::new(zip_filepath.clone(), err))?
                                    ))
                                }
                            })
                            .with_create_bin_file_fn(move |name, version| {
                                assert_eq!(name, SOFTWARE_NAME);
                                assert_eq!(version, expected_version);
                                Err(FileSystemError::new(PathBuf::from("/error"), io::Error::from(io::ErrorKind::PermissionDenied)))
                            });
                        let downloader = StubDownloader::new()
                            .with_download_fn(move |url, _| {
                                let expected_url = format!(
                                    "https://releases.hashicorp.com/terraform/{}/terraform_{}_{}_{}.zip",
                                    expected_version, expected_version, os, arch
                                );
                                assert_eq!(url, expected_url);
                                Ok(())
                            });
                        let cfg = Config::stub().with_fs(fs).with_downloader(downloader);
                        match tf.install(&cfg) {
                            Ok(_) => panic!("should fail"),
                            Err(InstallError::FileSystemWritingFailed(_)) => {}
                            Err(err) => panic!("{}", err),
                        }
                    }

                    #[test]
                    #[cfg(all(target_os = $os, target_arch = $arch))]
                    fn should_return_unzip_failed_err() {
                        let expected_version = "1.2.3";
                        let tf = Terraform(expected_version.into());
                        let os = os!().unwrap();
                        let arch = arch!().unwrap();
                        let expected_zip_filepath = tempdir().unwrap().into_path().join("terraform.zip");
                        let bin_filepath = tempdir().unwrap().into_path().join(SOFTWARE_NAME);
                        let fs = StubFileSystem::new()
                            .with_create_tmp_file_fn({
                                let expected_zip_filepath = expected_zip_filepath.clone();
                                move |filename| {
                                    let expected = format!("terraform_{}_{}_{}.zip", expected_version, os, arch);
                                    assert_eq!(filename, expected);
                                    Ok((
                                        expected_zip_filepath.clone(),
                                        File::create(&expected_zip_filepath).map_err(|err| FileSystemError::new(expected_zip_filepath.clone(), err))?
                                    ))
                                }
                            })
                            .with_create_bin_file_fn(move |name, version| {
                                assert_eq!(name, SOFTWARE_NAME);
                                assert_eq!(version, expected_version);
                                Ok((
                                    bin_filepath.clone(),
                                    File::create(&bin_filepath).map_err(|err| FileSystemError::new(bin_filepath.clone(), err))?
                                ))
                            });
                        let downloader = StubDownloader::new()
                            .with_download_fn(move |url, _| {
                                let expected_url = format!(
                                    "https://releases.hashicorp.com/terraform/{}/terraform_{}_{}_{}.zip",
                                    expected_version, expected_version, os, arch
                                );
                                assert_eq!(url, expected_url);
                                Ok(())
                            });
                        let unziper = StubUnzipper::new()
                            .with_unzip_fn({
                                let expected_zip_filepath = expected_zip_filepath.clone();
                                move |zip_filepath, filename, _| {
                                assert_eq!(zip_filepath, expected_zip_filepath);
                                assert_eq!(filename, SOFTWARE_NAME);
                                Err(UnzipError::FileReadingFailed(
                                    io::Error::from(io::ErrorKind::PermissionDenied)
                                ))
                            }});
                        let cfg = Config::stub().with_fs(fs).with_downloader(downloader).with_unzipper(unziper);
                        match tf.install(&cfg) {
                            Ok(_) => panic!("should fail"),
                            Err(InstallError::UnzipFailed(zip_filepath, filepath, _)) => {
                                assert_eq!(zip_filepath, expected_zip_filepath);
                                assert_eq!(filepath, SOFTWARE_NAME);
                            },
                            Err(err) => panic!("{}", err),
                        }
                    }

                    #[test]
                    #[cfg(all(target_os = $os, target_arch = $arch))]
                    fn should_install_terraform() {
                        let expected_version = "1.2.3";
                        let tf = Terraform(expected_version.into());
                        let os = os!().unwrap();
                        let arch = arch!().unwrap();
                        let zip_filepath = tempdir().unwrap().into_path().join("terraform.zip");
                        let bin_filepath = tempdir().unwrap().into_path().join(SOFTWARE_NAME);
                        let fs = StubFileSystem::new()
                            .with_create_tmp_file_fn({
                                let zip_filepath = zip_filepath.clone();
                                move |filename| {
                                    let expected = format!("terraform_{}_{}_{}.zip", expected_version, os, arch);
                                    assert_eq!(filename, expected);
                                    Ok((
                                        zip_filepath.clone(),
                                        File::create(&zip_filepath).map_err(|err| FileSystemError::new(zip_filepath.clone(), err))?
                                    ))
                                }
                            })
                            .with_create_bin_file_fn(move |name, version| {
                                assert_eq!(name, SOFTWARE_NAME);
                                assert_eq!(version, expected_version);
                                Ok((
                                    bin_filepath.clone(),
                                    File::create(&bin_filepath).map_err(|err| FileSystemError::new(bin_filepath.clone(), err))?
                                ))
                            });
                        let downloader = StubDownloader::new()
                            .with_download_fn(move |url, _| {
                                let expected_url = format!(
                                    "https://releases.hashicorp.com/terraform/{}/terraform_{}_{}_{}.zip",
                                    expected_version, expected_version, os, arch
                                );
                                assert_eq!(url, expected_url);
                                Ok(())
                            });
                        let unziper = StubUnzipper::new()
                            .with_unzip_fn(move |filepath, filename, _| {
                                assert_eq!(filepath, zip_filepath);
                                assert_eq!(filename, SOFTWARE_NAME);
                                Ok(())
                            });
                        let cfg = Config::stub().with_fs(fs).with_downloader(downloader).with_unzipper(unziper);
                        tf.install(&cfg).unwrap()
                    }
                };
            }

            tests!("macos", "x86_64");
            tests!("macos", "aarch64");
            tests!("linux", "x86");
            tests!("linux", "x86_64");
            tests!("linux", "arm");
            tests!("linux", "aarch64");
        }

        mod name {
            use super::*;

            #[test]
            fn should_return_name() {
                let tf = Terraform("1.2.3".into());
                assert_eq!(tf.name(), SOFTWARE_NAME);
            }
        }

        mod version {
            use super::*;

            #[test]
            fn should_return_version() {
                let version = "1.2.3";
                let tf = Terraform(version.into());
                assert_eq!(tf.version(), version);
            }
        }
    }
}
