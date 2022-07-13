use super::*;
use log::info;
use std::{
    env::consts::{ARCH, OS},
    io::BufWriter,
};

macro_rules! supported_systems {
    () => {
        maplit::hashmap! {
            "linux" => maplit::hashset!("x86", "x86_64", "arm", "aarch64"),
            "macos" => maplit::hashset!("x86_64", "aarch64"),
        }
    };
}

const TOOL_NAME: &str = "terraform";

#[derive(Debug, Eq, PartialEq)]
pub struct Terraform(String);

impl Terraform {
    pub fn new(version: String) -> Self {
        Self(version)
    }

    fn arch(&self) -> Result<&'static str, InstallError> {
        match ARCH {
            "x86" => Ok("386"),
            "x86_64" => Ok("amd64"),
            "arm" => Ok("arm"),
            "aarch64" => Ok("arm64"),
            _ => Err(InstallError::UnsupportedArch(supported_systems!())),
        }
    }

    fn os(&self) -> Result<&'static str, InstallError> {
        match OS {
            "macos" => Ok("darwin"),
            "linux" => Ok("linux"),
            _ => Err(InstallError::UnsupportedOs(supported_systems!())),
        }
    }
}

impl Tool for Terraform {
    fn install(&self, cfg: &Config) -> Result<(), InstallError> {
        info!("Installing {} v{}", TOOL_NAME, self.0);
        let os = self.os()?;
        let arch = self.arch()?;
        let filename = format!("terraform_{}_{}_{}.zip", self.0, os, arch);
        let url = format!(
            "https://releases.hashicorp.com/terraform/{}/{}",
            self.0, filename
        );
        let (zip_filepath, mut zip_file) = cfg
            .fs
            .create_tmp_file(&filename)
            .map_err(InstallError::IoFailed)?;
        cfg.downloader
            .download(&url, &mut zip_file)
            .map_err(InstallError::DownloadFailed)?;
        let (_, bin_file) = cfg
            .fs
            .create_bin_file(TOOL_NAME, &self.0)
            .map_err(InstallError::IoFailed)?;
        let mut file_buf = BufWriter::new(bin_file);
        cfg.unzipper
            .unzip(&zip_filepath, TOOL_NAME, &mut file_buf)
            .map_err(InstallError::UnzipFailed)?;
        cfg.fs
            .create_bin_symlink(TOOL_NAME, &self.0)
            .map_err(InstallError::IoFailed)?;
        info!("{} v{} installed", TOOL_NAME, &self.0);
        Ok(())
    }

    fn version(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::util::{downloader::*, fs::*, zip::*};
    use std::{fs::File, path::PathBuf};
    use tempfile::tempdir;

    mod terraform {
        use super::*;

        mod new {
            use super::*;

            #[test]
            fn should_return_tool() {
                let expected = Terraform("1.2.3".into());
                let tf = Terraform::new(expected.0.clone());
                assert_eq!(tf, expected);
                assert_eq!(tf.version(), expected.0);
            }
        }

        mod arch {
            use super::*;

            macro_rules! should_return_arch {
                ($arch:expr, $expected:expr) => {
                    #[test]
                    #[cfg(target_arch = $arch)]
                    fn should_return_arch() {
                        let tf = Terraform("1.2.3".into());
                        let arch = tf.arch().unwrap();
                        assert_eq!(arch, $expected);
                    }
                };
            }

            should_return_arch!("x86", "386");
            should_return_arch!("x86_64", "amd64");
            should_return_arch!("arm", "arm");
            should_return_arch!("aarch64", "arm64");
        }

        mod os {
            use super::*;

            macro_rules! should_return_os {
                ($os:expr, $expected:expr) => {
                    #[test]
                    #[cfg(target_os = $os)]
                    fn should_return_os() {
                        let tf = Terraform("1.2.3".into());
                        let os = tf.os().unwrap();
                        assert_eq!(os, $expected);
                    }
                };
            }

            should_return_os!("macos", "darwin");
            should_return_os!("linux", "linux");
        }

        mod install {
            use super::*;

            macro_rules! tests {
                ($os:expr, $arch:expr) => {
                    #[test]
                    #[cfg(all(target_os = $os, target_arch = $arch))]
                    fn should_return_io_failed_err_if_tmp_file_creation_failed() {
                        let expected_version = "1.2.3";
                        let tf = Terraform::new(expected_version.into());
                        let os = tf.os().unwrap();
                        let arch = tf.arch().unwrap();
                        let fs = StubFs::new()
                            .with_create_tmp_file_fn({
                                move |filename| {
                                    let expected = format!("terraform_{}_{}_{}.zip", expected_version, os, arch);
                                    assert_eq!(filename, expected);
                                    Err(io::Error::from(io::ErrorKind::PermissionDenied))
                                }
                            });
                        let downloader = StubDownloader::new();
                        let unziper = StubUnzipper::new();
                        let cfg = Config::stub(fs, downloader, unziper);
                        match tf.install(&cfg) {
                            Ok(_) => panic!("should fail"),
                            Err(InstallError::IoFailed(_)) => {}
                            Err(err) => panic!("{}", err),
                        }
                    }

                    #[test]
                    #[cfg(all(target_os = $os, target_arch = $arch))]
                    fn should_return_download_failed_err() {
                        let expected_version = "1.2.3";
                        let tf = Terraform::new(expected_version.into());
                        let os = tf.os().unwrap();
                        let arch = tf.arch().unwrap();
                        let zip_filepath = tempdir().unwrap().into_path().join("terraform.zip");
                        let fs = StubFs::new()
                            .with_create_tmp_file_fn({
                                let zip_filepath = zip_filepath.clone();
                                move |filename| {
                                    let expected = format!("terraform_{}_{}_{}.zip", expected_version, os, arch);
                                    assert_eq!(filename, expected);
                                    Ok((
                                        zip_filepath.clone(),
                                        File::create(&zip_filepath)?
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
                                Err(DownloadError::RequestFailed(404, String::new()))
                            });
                        let unziper = StubUnzipper::new();
                        let cfg = Config::stub(fs, downloader, unziper);
                        match tf.install(&cfg) {
                            Ok(_) => panic!("should fail"),
                            Err(InstallError::DownloadFailed(_)) => {}
                            Err(err) => panic!("{}", err),
                        }
                    }

                    #[test]
                    #[cfg(all(target_os = $os, target_arch = $arch))]
                    fn should_return_io_failed_err_if_bin_file_creation_failed() {
                        let expected_version = "1.2.3";
                        let tf = Terraform::new(expected_version.into());
                        let os = tf.os().unwrap();
                        let arch = tf.arch().unwrap();
                        let zip_filepath = tempdir().unwrap().into_path().join("terraform.zip");
                        let fs = StubFs::new()
                            .with_create_tmp_file_fn({
                                let zip_filepath = zip_filepath.clone();
                                move |filename| {
                                    let expected = format!("terraform_{}_{}_{}.zip", expected_version, os, arch);
                                    assert_eq!(filename, expected);
                                    Ok((
                                        zip_filepath.clone(),
                                        File::create(&zip_filepath)?
                                    ))
                                }
                            })
                            .with_create_bin_file_fn(move |name, version| {
                                assert_eq!(name, TOOL_NAME);
                                assert_eq!(version, expected_version);
                                Err(io::Error::from(io::ErrorKind::PermissionDenied))
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
                        let unziper = StubUnzipper::new();
                        let cfg = Config::stub(fs, downloader, unziper);
                        match tf.install(&cfg) {
                            Ok(_) => panic!("should fail"),
                            Err(InstallError::IoFailed(_)) => {}
                            Err(err) => panic!("{}", err),
                        }
                    }

                    #[test]
                    #[cfg(all(target_os = $os, target_arch = $arch))]
                    fn should_return_unzip_failed_err() {
                        let expected_version = "1.2.3";
                        let tf = Terraform::new(expected_version.into());
                        let os = tf.os().unwrap();
                        let arch = tf.arch().unwrap();
                        let zip_filepath = tempdir().unwrap().into_path().join("terraform.zip");
                        let bin_filepath = tempdir().unwrap().into_path().join(TOOL_NAME);
                        let fs = StubFs::new()
                            .with_create_tmp_file_fn({
                                let zip_filepath = zip_filepath.clone();
                                move |filename| {
                                    let expected = format!("terraform_{}_{}_{}.zip", expected_version, os, arch);
                                    assert_eq!(filename, expected);
                                    Ok((
                                        zip_filepath.clone(),
                                        File::create(&zip_filepath)?
                                    ))
                                }
                            })
                            .with_create_bin_file_fn(move |name, version| {
                                assert_eq!(name, TOOL_NAME);
                                assert_eq!(version, expected_version);
                                Ok((
                                    bin_filepath.clone(),
                                    File::create(&bin_filepath)?
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
                                assert_eq!(filename, TOOL_NAME);
                                Err(UnzipError::FileOpeningFailed(
                                    PathBuf::from("terraform.zip"),
                                    io::Error::from(io::ErrorKind::PermissionDenied)
                                ))
                            });
                        let cfg = Config::stub(fs, downloader, unziper);
                        match tf.install(&cfg) {
                            Ok(_) => panic!("should fail"),
                            Err(InstallError::UnzipFailed(_)) => {}
                            Err(err) => panic!("{}", err),
                        }
                    }

                    #[test]
                    #[cfg(all(target_os = $os, target_arch = $arch))]
                    fn should_return_io_failed_err_if_bin_symlink_creation_failed() {
                        let expected_version = "1.2.3";
                        let tf = Terraform::new(expected_version.into());
                        let os = tf.os().unwrap();
                        let arch = tf.arch().unwrap();
                        let zip_filepath = tempdir().unwrap().into_path().join("terraform.zip");
                        let bin_filepath = tempdir().unwrap().into_path().join(TOOL_NAME);
                        let fs = StubFs::new()
                            .with_create_tmp_file_fn({
                                let zip_filepath = zip_filepath.clone();
                                move |filename| {
                                    let expected = format!("terraform_{}_{}_{}.zip", expected_version, os, arch);
                                    assert_eq!(filename, expected);
                                    Ok((
                                        zip_filepath.clone(),
                                        File::create(&zip_filepath)?
                                    ))
                                }
                            })
                            .with_create_bin_file_fn(move |name, version| {
                                assert_eq!(name, TOOL_NAME);
                                assert_eq!(version, expected_version);
                                Ok((
                                    bin_filepath.clone(),
                                    File::create(&bin_filepath)?
                                ))
                            })
                            .with_create_bin_symlink_fn(move |name, version| {
                                assert_eq!(name, TOOL_NAME);
                                assert_eq!(version, expected_version);
                                Err(io::Error::from(io::ErrorKind::PermissionDenied))
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
                                assert_eq!(filename, TOOL_NAME);
                                Ok(())
                            });
                        let cfg = Config::stub(fs, downloader, unziper);
                        match tf.install(&cfg) {
                            Ok(_) => panic!("should fail"),
                            Err(InstallError::IoFailed(_)) => {}
                            Err(err) => panic!("{}", err),
                        }
                    }

                    #[test]
                    #[cfg(all(target_os = $os, target_arch = $arch))]
                    fn should_install_terraform() {
                        let expected_version = "1.2.3";
                        let tf = Terraform::new(expected_version.into());
                        let os = tf.os().unwrap();
                        let arch = tf.arch().unwrap();
                        let zip_filepath = tempdir().unwrap().into_path().join("terraform.zip");
                        let bin_filepath = tempdir().unwrap().into_path().join(TOOL_NAME);
                        let fs = StubFs::new()
                            .with_create_tmp_file_fn({
                                let zip_filepath = zip_filepath.clone();
                                move |filename| {
                                    let expected = format!("terraform_{}_{}_{}.zip", expected_version, os, arch);
                                    assert_eq!(filename, expected);
                                    Ok((
                                        zip_filepath.clone(),
                                        File::create(&zip_filepath)?
                                    ))
                                }
                            })
                            .with_create_bin_file_fn(move |name, version| {
                                assert_eq!(name, TOOL_NAME);
                                assert_eq!(version, expected_version);
                                Ok((
                                    bin_filepath.clone(),
                                    File::create(&bin_filepath)?
                                ))
                            })
                            .with_create_bin_symlink_fn(move |name, version| {
                                assert_eq!(name, TOOL_NAME);
                                assert_eq!(version, expected_version);
                                Ok(())
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
                                assert_eq!(filename, TOOL_NAME);
                                Ok(())
                            });
                        let cfg = Config::stub(fs, downloader, unziper);
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
    }
}
