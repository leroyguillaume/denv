use super::*;
use std::{
    env::consts::{ARCH, OS},
    io::BufWriter,
};

pub struct Terraform;

impl Terraform {
    fn arch(&self) -> Result<&str, InstallError> {
        match ARCH {
            "x86" => Ok("386"),
            "x86_64" => Ok("amd64"),
            "arm" => Ok("arm"),
            "aarch64" => Ok("arm64"),
            _ => Err(InstallError::UnsupportedArch(self.supported_systems())),
        }
    }

    fn os(&self) -> Result<&str, InstallError> {
        match OS {
            "macos" => Ok("darwin"),
            "linux" => Ok("linux"),
            _ => Err(InstallError::UnsupportedOs(self.supported_systems())),
        }
    }
}

impl Tool for Terraform {
    fn install(&self, version: &str, cfg: &Config) -> Result<(), InstallError> {
        let os = self.os()?;
        let arch = self.arch()?;
        let filename = format!("terraform_{}_{}_{}.zip", version, os, arch);
        let url = format!(
            "https://releases.hashicorp.com/terraform/{}/{}",
            version, filename
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
            .create_bin_file(self.name(), version)
            .map_err(InstallError::IoFailed)?;
        let mut file_buf = BufWriter::new(bin_file);
        cfg.unzipper
            .unzip(&zip_filepath, "terraform", &mut file_buf)
            .map_err(InstallError::UnzipFailed)?;
        cfg.fs
            .create_bin_symlink(self.name(), version)
            .map_err(InstallError::IoFailed)
    }

    fn name(&self) -> &str {
        "terraform"
    }

    fn supported_systems(&self) -> SupportedSystems {
        supported_systems!(
            ("macos", "x86_64", "aarch64"),
            ("linux", "x86", "x86_64", "arm", "aarch64")
        )
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

        mod arch {
            use super::*;

            macro_rules! should_return_arch {
                ($arch:expr, $expected:expr) => {
                    #[test]
                    #[cfg(target_arch = $arch)]
                    fn should_return_arch() {
                        let arch = Terraform.arch().unwrap();
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
                        let os = Terraform.os().unwrap();
                        assert_eq!(os, $expected);
                    }
                };
            }

            should_return_os!("macos", "darwin");
            should_return_os!("linux", "linux");
        }

        mod install {
            use super::*;

            macro_rules! test {
                (
                    $ident:ident,
                    $create_tmp_file:expr,
                    $download:expr,
                    $create_bin_file:expr,
                    $unzip:expr,
                    $create_bin_symlink:expr,
                    $expect:expr
                ) => {
                    #[test]
                    fn $ident() {
                        let os = Terraform.os().unwrap();
                        let arch = Terraform.arch().unwrap();
                        let expected_version = "1.2.3";
                        let zip_filepath = tempdir().unwrap().into_path().join("terraform.zip");
                        let bin_filepath = tempdir().unwrap().into_path().join("terraform");
                        let fs = StubFs::new()
                            .with_create_tmp_file_fn({
                                let zip_filepath = zip_filepath.clone();
                                move |filename| {
                                    let expected = format!("terraform_{}_{}_{}.zip", expected_version, os, arch);
                                    assert_eq!(filename, expected);
                                    $create_tmp_file(&zip_filepath)
                                }
                            })
                            .with_create_bin_file_fn(move |name, version| {
                                assert_eq!(name, "terraform");
                                assert_eq!(version, expected_version);
                                $create_bin_file(&bin_filepath)
                            })
                            .with_create_bin_symlink_fn(move |name, version| {
                                assert_eq!(name, "terraform");
                                assert_eq!(version, expected_version);
                                $create_bin_symlink()
                            });
                        let downloader = StubDownloader::new()
                            .with_download_fn(move |url, _| {
                                let expected_url = format!(
                                    "https://releases.hashicorp.com/terraform/{}/terraform_{}_{}_{}.zip",
                                    expected_version, expected_version, os, arch
                                );
                                assert_eq!(url, expected_url);
                                $download()
                            });
                        let unziper = StubUnzipper::new()
                            .with_unzip_fn(move |filepath, filename, _| {
                                assert_eq!(filepath, zip_filepath);
                                assert_eq!(filename, "terraform");
                                $unzip()
                            });
                        let cfg = Config::stub(fs, downloader, unziper);
                        $expect(Terraform.install(expected_version, &cfg));
                    }
                };
            }

            macro_rules! tests {
                ($os:expr, $arch:expr) => {
                    #[cfg(all(target_os = $os, target_arch = $arch))]
                    test!(
                        should_return_io_failed_err_if_tmp_file_creation_failed,
                        |_| Err(io::Error::from(io::ErrorKind::PermissionDenied)),
                        || Ok(()),
                        |bin_filepath: &PathBuf| Ok((
                            bin_filepath.clone(),
                            File::create(bin_filepath)?
                        )),
                        || Ok(()),
                        || Ok(()),
                        |res| {
                            match res {
                                Ok(_) => panic!("should fail"),
                                Err(InstallError::IoFailed(_)) => {}
                                Err(err) => panic!("{}", err),
                            }
                        }
                    );

                    #[cfg(all(target_os = $os, target_arch = $arch))]
                    test!(
                        should_return_download_failed_err,
                        |zip_filepath: &PathBuf| Ok((
                            zip_filepath.clone(),
                            File::create(zip_filepath)?
                        )),
                        || Err(DownloadError::RequestFailed(404, String::new())),
                        |bin_filepath: &PathBuf| Ok((
                            bin_filepath.clone(),
                            File::create(bin_filepath)?
                        )),
                        || Ok(()),
                        || Ok(()),
                        |res| {
                            match res {
                                Ok(_) => panic!("should fail"),
                                Err(InstallError::DownloadFailed(_)) => {}
                                Err(err) => panic!("{}", err),
                            }
                        }
                    );

                    #[cfg(all(target_os = $os, target_arch = $arch))]
                    test!(
                        should_return_io_failed_err_if_bin_file_creation_failed,
                        |zip_filepath: &PathBuf| Ok((
                            zip_filepath.clone(),
                            File::create(zip_filepath)?
                        )),
                        || Ok(()),
                        |_| Err(io::Error::from(io::ErrorKind::PermissionDenied)),
                        || Ok(()),
                        || Ok(()),
                        |res| {
                            match res {
                                Ok(_) => panic!("should fail"),
                                Err(InstallError::IoFailed(_)) => {}
                                Err(err) => panic!("{}", err),
                            }
                        }
                    );

                    #[cfg(all(target_os = $os, target_arch = $arch))]
                    test!(
                        should_return_unzip_failed_err,
                        |zip_filepath: &PathBuf| Ok((
                            zip_filepath.clone(),
                            File::create(zip_filepath)?
                        )),
                        || Ok(()),
                        |bin_filepath: &PathBuf| Ok((
                            bin_filepath.clone(),
                            File::create(bin_filepath)?
                        )),
                        || Err(UnzipError::FileOpeningFailed(
                            PathBuf::from("terraform.zip"),
                            io::Error::from(io::ErrorKind::PermissionDenied)
                        )),
                        || Ok(()),
                        |res| {
                            match res {
                                Ok(_) => panic!("should fail"),
                                Err(InstallError::UnzipFailed(_)) => {}
                                Err(err) => panic!("{}", err),
                            }
                        }
                    );

                    #[cfg(all(target_os = $os, target_arch = $arch))]
                    test!(
                        should_return_io_failed_err_if_bin_symlink_creation_failed,
                        |zip_filepath: &PathBuf| Ok((
                            zip_filepath.clone(),
                            File::create(zip_filepath)?
                        )),
                        || Ok(()),
                        |bin_filepath: &PathBuf| Ok((
                            bin_filepath.clone(),
                            File::create(bin_filepath)?
                        )),
                        || Ok(()),
                        || Err(io::Error::from(io::ErrorKind::PermissionDenied)),
                        |res| {
                            match res {
                                Ok(_) => panic!("should fail"),
                                Err(InstallError::IoFailed(_)) => {}
                                Err(err) => panic!("{}", err),
                            }
                        }
                    );

                    #[cfg(all(target_os = $os, target_arch = $arch))]
                    test!(
                        should_install_terraform,
                        |zip_filepath: &PathBuf| Ok((
                            zip_filepath.clone(),
                            File::create(zip_filepath)?
                        )),
                        || Ok(()),
                        |bin_filepath: &PathBuf| Ok((
                            bin_filepath.clone(),
                            File::create(bin_filepath)?
                        )),
                        || Ok(()),
                        || Ok(()),
                        |res: Result<(), InstallError>| res.unwrap()
                    );
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
                assert_eq!(Terraform.name(), "terraform");
            }
        }
    }
}
