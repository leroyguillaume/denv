pub mod terraform;

use crate::{cfg::Config, util::downloader::*, util::zip::*};
use std::{
    collections::{HashMap, HashSet},
    env::consts::{ARCH, OS},
    fmt::{self, Display, Formatter},
    io,
};

macro_rules! supported_systems {
    ($(($os:expr, $($arch:expr),+)),+) => {{
        HashMap::from([$(($os, HashSet::from([$($arch),*]))),*])
    }};
}
pub(crate) use supported_systems;

pub type SupportedSystems = HashMap<&'static str, HashSet<&'static str>>;

#[derive(Debug)]
pub enum InstallError {
    UnsupportedOs(SupportedSystems),
    UnsupportedArch(SupportedSystems),
    IoFailed(io::Error),
    DownloadFailed(DownloadError),
    UnzipFailed(UnzipError),
}

impl InstallError {
    fn fmt_supported_system(&self, supported_systems: &SupportedSystems) -> String {
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
                self.fmt_supported_system(supported_systems)
            ),
            Self::UnsupportedArch(supported_systems) => write!(
                f,
                "Architecture '{}' is not supported for OS '{}' (must be one of {})",
                ARCH,
                OS,
                self.fmt_supported_system(supported_systems)
            ),
            Self::IoFailed(err) => write!(f, "{}", err),
            Self::DownloadFailed(err) => write!(f, "{}", err),
            Self::UnzipFailed(err) => write!(f, "{}", err),
        }
    }
}

pub trait Tool {
    fn install(&self, cfg: &Config) -> Result<(), InstallError>;

    fn name(&self) -> &str;

    fn supported_systems(&self) -> SupportedSystems;

    fn version(&self) -> &str;
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::PathBuf;

    mod install_error {
        use super::*;

        mod to_string {
            use super::*;

            mod unsupported_os {
                use super::*;

                #[test]
                fn should_return_string() {
                    let supported_systems =
                        supported_systems!(("linux", "x86", "x86_64"), ("macos", "arm", "aarch64"));
                    let err = InstallError::UnsupportedOs(supported_systems);
                    let expected = format!("OS '{}' is not supported (must be one of [linux x86, linux x86_64, macos aarch64, macos arm])", OS);
                    assert_eq!(err.to_string(), expected);
                }
            }

            mod unsupported_arch {
                use super::*;

                #[test]
                fn should_return_string() {
                    let supported_systems =
                        supported_systems!(("linux", "x86", "x86_64"), ("macos", "arm", "aarch64"));
                    let err = InstallError::UnsupportedArch(supported_systems);
                    let expected = format!("Architecture '{}' is not supported for OS '{}' (must be one of [linux x86, linux x86_64, macos aarch64, macos arm])", ARCH, OS);
                    assert_eq!(err.to_string(), expected);
                }
            }

            mod io_failed {
                use super::*;

                #[test]
                fn should_return_string() {
                    let err = io::Error::from(io::ErrorKind::PermissionDenied);
                    let expected = err.to_string();
                    let err = InstallError::IoFailed(err);
                    assert_eq!(err.to_string(), expected);
                }
            }

            mod download_failed {
                use super::*;

                #[test]
                fn should_return_string() {
                    let err = DownloadError::RequestFailed(404, "not found".into());
                    let expected = err.to_string();
                    let err = InstallError::DownloadFailed(err);
                    assert_eq!(err.to_string(), expected);
                }
            }

            mod unzip_failed {
                use super::*;

                #[test]
                fn should_return_string() {
                    let err = UnzipError::FileOpeningFailed(
                        PathBuf::from("terraform.zip"),
                        io::Error::from(io::ErrorKind::PermissionDenied),
                    );
                    let expected = err.to_string();
                    let err = InstallError::UnzipFailed(err);
                    assert_eq!(err.to_string(), expected);
                }
            }
        }
    }
}
