pub mod terraform;

use crate::{cfg::Config, error::*};
use std::{
    collections::{HashMap, HashSet},
    env::consts::{ARCH, OS},
    fmt::{self, Debug, Display, Formatter},
    path::PathBuf,
};

pub type SupportedSystems = HashMap<&'static str, HashSet<&'static str>>;

#[derive(Debug)]
pub enum InstallError {
    UnsupportedOs(SupportedSystems),
    UnsupportedArch(SupportedSystems),
    FileSystemWritingFailed(FileSystemError),
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

pub trait Software: Debug {
    fn add_to_path(&self, cfg: &Config) -> Result<(), FileSystemError> {
        cfg.fs
            .create_bin_symlink(self.name(), self.version(), &cfg.sha256())
    }

    fn install(&self, cfg: &Config) -> Result<(), InstallError>;

    fn is_installed(&self, cfg: &Config) -> bool {
        cfg.fs.is_installed_software(self.name(), self.version())
    }

    fn name(&self) -> &'static str;

    fn version(&self) -> &str;
}

impl Display for dyn Software {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} v{}", self.name(), self.version())
    }
}

impl PartialEq for dyn Software {
    fn eq(&self, software: &dyn Software) -> bool {
        self.name() == software.name() && self.version() == software.version()
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub struct DummySoftware(pub &'static str);

#[cfg(test)]
impl Software for DummySoftware {
    fn install(&self, _cfg: &Config) -> Result<(), InstallError> {
        Ok(())
    }

    fn name(&self) -> &'static str {
        "dummy"
    }

    fn version(&self) -> &str {
        self.0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::internal::{downloader::*, fs::*, unzip::*};
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
                    let err = FileSystemError::new(
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

    mod software {
        use super::*;

        mod add_to_path {
            use super::*;

            #[test]
            fn should_return_err() {
                let software = DummySoftware("1.2.3");
                let fs = StubFileSystem::new().with_create_bin_symlink_fn(
                    move |name, version, cfg_sha256| {
                        assert_eq!(name, software.name());
                        assert_eq!(version, software.version());
                        assert_eq!(
                            cfg_sha256,
                            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                        );
                        Err(FileSystemError::new(
                            PathBuf::from("/error"),
                            io::Error::from(io::ErrorKind::PermissionDenied),
                        ))
                    },
                );
                let cfg = Config::stub(fs, StubDownloader::new(), StubUnzipper::new());
                if software.add_to_path(&cfg).is_ok() {
                    panic!("should fail");
                }
            }

            #[test]
            fn should_create_symlink() {
                let software = DummySoftware("1.2.3");
                let fs = StubFileSystem::new().with_create_bin_symlink_fn(
                    move |name, version, cfg_sha256| {
                        assert_eq!(name, software.name());
                        assert_eq!(version, software.version());
                        assert_eq!(
                            cfg_sha256,
                            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                        );
                        Ok(())
                    },
                );
                let cfg = Config::stub(fs, StubDownloader::new(), StubUnzipper::new());
                software.add_to_path(&cfg).unwrap();
            }
        }

        mod eq {
            use super::*;

            #[test]
            fn should_return_false() {
                let software1: Box<dyn Software> = Box::new(DummySoftware("1.2.3"));
                let software2: Box<dyn Software> = Box::new(DummySoftware("1.2.4"));
                assert!(software1 != software2);
            }

            #[test]
            fn should_return_true() {
                let software1: Box<dyn Software> = Box::new(DummySoftware("1.2.3"));
                let software2: Box<dyn Software> = Box::new(DummySoftware("1.2.3"));
                assert!(software1 == software2);
            }
        }

        mod is_installed {
            use super::*;

            macro_rules! test {
                ($ident:ident, $expected:expr) => {
                    #[test]
                    fn $ident() {
                        let software = DummySoftware("1.2.3");
                        let fs = StubFileSystem::new().with_is_installed_software_fn(
                            move |name, version| {
                                assert_eq!(name, software.name());
                                assert_eq!(version, software.version());
                                $expected
                            },
                        );
                        let cfg = Config::stub(fs, StubDownloader::new(), StubUnzipper::new());
                        assert_eq!(software.is_installed(&cfg), $expected);
                    }
                };
            }

            test!(should_return_false, false);
            test!(should_return_true, true);
        }

        mod to_string {
            use super::*;

            #[test]
            fn should_return_string() {
                let software: Box<dyn Software> = Box::new(DummySoftware("1.2.3"));
                let expected = format!("{} v{}", software.name(), software.version());
                assert_eq!(software.to_string(), expected);
            }
        }
    }
}
