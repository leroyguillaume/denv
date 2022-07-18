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

pub trait Tool: Debug {
    fn install(&self, cfg: &Config) -> Result<(), InstallError>;

    fn is_installed(&self, cfg: &Config) -> bool {
        cfg.fs.is_installed_tool(self.name(), self.version())
    }

    fn name(&self) -> &'static str;

    fn version(&self) -> &str;
}

impl Display for dyn Tool {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} v{}", self.name(), self.version())
    }
}

impl PartialEq for dyn Tool {
    fn eq(&self, tool: &dyn Tool) -> bool {
        self.name() == tool.name() && self.version() == tool.version()
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub struct DummyTool(pub &'static str);

#[cfg(test)]
impl Tool for DummyTool {
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
    use crate::util::{downloader::*, fs::*, zip::*};
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

    mod tool {
        use super::*;

        mod eq {
            use super::*;

            #[test]
            fn should_return_false() {
                let tool1: Box<dyn Tool> = Box::new(DummyTool("1.2.3"));
                let tool2: Box<dyn Tool> = Box::new(DummyTool("1.2.4"));
                assert!(tool1 != tool2);
            }

            #[test]
            fn should_return_true() {
                let tool1: Box<dyn Tool> = Box::new(DummyTool("1.2.3"));
                let tool2: Box<dyn Tool> = Box::new(DummyTool("1.2.3"));
                assert!(tool1 == tool2);
            }
        }

        mod is_installed {
            use super::*;

            macro_rules! test {
                ($ident:ident, $expected:expr) => {
                    #[test]
                    fn $ident() {
                        let tool = DummyTool("1.2.3");
                        let fs = StubFileSystem::new().with_is_installed_tool_fn(
                            move |name, version| {
                                assert_eq!(name, tool.name());
                                assert_eq!(version, tool.version());
                                $expected
                            },
                        );
                        let cfg = Config::stub(fs, StubDownloader::new(), StubUnzipper::new());
                        assert_eq!(tool.is_installed(&cfg), $expected);
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
                let tool: Box<dyn Tool> = Box::new(DummyTool("1.2.3"));
                let expected = format!("{} v{}", tool.name(), tool.version());
                assert_eq!(tool.to_string(), expected);
            }
        }
    }
}
