pub mod terraform;

use crate::{cfg::Config, util::downloader::*, util::fs, util::zip::*};
use std::{
    collections::{HashMap, HashSet},
    env::consts::{ARCH, OS},
    fmt::{self, Display, Formatter},
    path::PathBuf,
};

pub type SupportedSystems = HashMap<&'static str, HashSet<&'static str>>;

#[derive(Debug)]
pub enum InstallError {
    UnsupportedOs(SupportedSystems),
    UnsupportedArch(SupportedSystems),
    FileSystemWritingFailed(fs::Error),
    DownloadFailed(DownloadError),
    UnzipFailed(PathBuf, String, UnzipError),
}

impl InstallError {
    fn fmt_supported_systems(&self, supported_systems: &SupportedSystems) -> String {
        let mut s = String::new();
        let mut systems = Vec::from_iter(supported_systems.keys());
        systems.sort();
        for system in systems {
            let mut archs = Vec::from_iter(supported_systems.get(system).unwrap());
            archs.sort();
            for arch in archs {
                s = format!("{}{} {}, ", s, system, arch);
            }
        }
        if s.is_empty() {
            "[]".into()
        } else {
            format!("[{}]", &s[..s.len() - 2])
        }
    }
}

impl Display for InstallError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::UnsupportedOs(supported_systems) => write!(
                f,
                "OS '{}' is not supported (must be one of {})",
                OS,
                self.fmt_supported_systems(supported_systems)
            ),
            Self::UnsupportedArch(supported_systems) => write!(
                f,
                "Architecture '{}' is not supported for OS '{}' (must be one of {})",
                ARCH,
                OS,
                self.fmt_supported_systems(supported_systems)
            ),
            Self::FileSystemWritingFailed(err) => write!(f, "{}", err),
            Self::DownloadFailed(err) => write!(f, "{}", err),
            Self::UnzipFailed(zip_filepath, filepath, err) => write!(
                f,
                "Unzip {} from {} failed: {}",
                filepath,
                zip_filepath.display(),
                err
            ),
        }
    }
}

pub trait Tool {
    fn install(&self, cfg: &Config) -> Result<(), InstallError>;

    fn version(&self) -> &str;
}

#[cfg(test)]
mod test {
    use super::*;
    use maplit::{hashmap, hashset};
    use reqwest::blocking::get;
    use std::{io, path::PathBuf};

    mod install_error {
        use super::*;

        mod to_string {
            use super::*;

            mod unsupported_os {
                use super::*;

                #[test]
                fn should_return_string() {
                    let supported_systems = hashmap! {
                        "linux" => hashset!("x86", "x86_64"),
                        "macos" => hashset!("arm", "aarch64"),
                    };
                    let err = InstallError::UnsupportedOs(supported_systems);
                    let expected = format!("OS '{}' is not supported (must be one of [linux x86, linux x86_64, macos aarch64, macos arm])", OS);
                    assert_eq!(err.to_string(), expected);
                }
            }

            mod unsupported_arch {
                use super::*;

                #[test]
                fn should_return_string() {
                    let supported_systems = hashmap! {
                        "linux" => hashset!("x86", "x86_64"),
                        "macos" => hashset!("arm", "aarch64"),
                    };
                    let err = InstallError::UnsupportedArch(supported_systems);
                    let expected = format!("Architecture '{}' is not supported for OS '{}' (must be one of [linux x86, linux x86_64, macos aarch64, macos arm])", ARCH, OS);
                    assert_eq!(err.to_string(), expected);
                }
            }

            mod file_system_writing_failed {
                use super::*;

                #[test]
                fn should_return_string() {
                    let err = fs::Error::new(
                        PathBuf::from("/error"),
                        io::Error::from(io::ErrorKind::PermissionDenied),
                    );
                    let expected = err.to_string();
                    let err = InstallError::FileSystemWritingFailed(err);
                    assert_eq!(err.to_string(), expected);
                }
            }

            mod download_failed {
                use super::*;

                #[test]
                fn should_return_string() {
                    let resp = get("https://fr.archive.ubuntu.com/ubuntu2/").unwrap();
                    let err = DownloadError::RequestFailed(resp);
                    let expected = err.to_string();
                    let err = InstallError::DownloadFailed(err);
                    assert_eq!(err.to_string(), expected);
                }
            }

            mod unzip_failed {
                use super::*;

                #[test]
                fn should_return_string() {
                    let zip_filepath = PathBuf::from("/error");
                    let filepath = "file";
                    let err = UnzipError::FileOpeningFailed(io::Error::from(
                        io::ErrorKind::PermissionDenied,
                    ));
                    let expected = format!(
                        "Unzip {} from {} failed: {}",
                        filepath,
                        zip_filepath.display(),
                        err
                    );
                    let err = InstallError::UnzipFailed(zip_filepath, filepath.into(), err);
                    assert_eq!(err.to_string(), expected);
                }
            }
        }
    }
}
