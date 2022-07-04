use super::*;
use crate::*;
use log::trace;
use std::fs::OpenOptions;
use tempfile::tempdir;

pub struct Terraform;

impl Tool for Terraform {
    fn install(&self, version: &str, cfg: &Config) -> Result<(), InstallError> {
        let os = map_env_const!(
            OS,
            ("macos", "darwin"),
            ("windows", "windows"),
            ("linux", "linux")
        );
        let arch = map_env_const!(
            ARCH,
            ("x86", "386"),
            ("x86_64", "amd64"),
            ("arm", "arm"),
            ("aarch64", "arm64")
        );
        let filename = format!("terraform_{}_{}_{}.zip", version, os, arch);
        let url = format!(
            "https://releases.hashicorp.com/terraform/{}/{}",
            version, filename
        );
        let tmp_dir = map_debug_err!(tempdir(), InstallError::IoFailed)?;
        let zip_filepath = tmp_dir.path().join(filename);
        trace!("Opening {} with write mode", zip_filepath.display());
        let mut zip_file = map_debug_err!(
            OpenOptions::new().write(true).open(&zip_filepath),
            InstallError::IoFailed
        )?;
        cfg.downloader
            .download(&url, &mut zip_file)
            .map_err(InstallError::DownloadFailed)?;
        trace!("Closing {}", zip_filepath.display());
        Ok(())
    }

    fn name(&self) -> &str {
        "terraform"
    }

    fn supported_systems(&self) -> SupportedSystems {
        supported_systems!(
            ("macos", "x86_64", "aarch64"),
            ("windows", "x86", "x86_64"),
            ("linux", "x86", "x86_64", "arm", "aarch64")
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    mod terraform {
        use super::*;

        mod install {
            // use super::*;

            macro_rules! tests {
                ($($os:expr),*) => {
                    $(
                        #[test]
                        #[cfg(target_os = $os)]
                        fn should_install_terraform() {
                            // let cfg = Config {

                            // };
                            // Terraform.install("1.2.3", &cfg).unwrap();
                            // panic!();
                        }
                    )*
                };
            }

            tests!("macos", "windows", "linux");
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
