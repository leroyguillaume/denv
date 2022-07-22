pub mod terraform;

use crate::{cfg::Config, error::*};
use std::fmt::{self, Debug, Display, Formatter};

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
type InstallFn = dyn Fn(&Config) -> Result<(), InstallError>;

#[cfg(test)]
pub struct StubSoftware {
    name: &'static str,
    version: &'static str,
    install_fn: Option<Box<InstallFn>>,
}

#[cfg(test)]
impl StubSoftware {
    pub fn new(name: &'static str, version: &'static str) -> Self {
        Self {
            name,
            version,
            install_fn: None,
        }
    }

    pub fn with_install_fn<F: Fn(&Config) -> Result<(), InstallError> + 'static>(
        mut self,
        install_fn: F,
    ) -> Self {
        self.install_fn = Some(Box::new(install_fn));
        self
    }
}

#[cfg(test)]
impl Debug for StubSoftware {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StubSoftware")
            .field("name", &self.name)
            .field("version", &self.version)
            .finish()
    }
}

#[cfg(test)]
impl Software for StubSoftware {
    fn install(&self, cfg: &Config) -> Result<(), InstallError> {
        match &self.install_fn {
            Some(install_fn) => install_fn(cfg),
            None => unimplemented!(),
        }
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn version(&self) -> &str {
        self.version
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::internal::fs::*;
    use std::{io, path::PathBuf};

    mod software {
        use super::*;

        mod add_to_path {
            use super::*;

            #[test]
            fn should_return_err() {
                let software_name = "stub";
                let software_version = "1.2.3";
                let software = StubSoftware::new(software_name, software_version);
                let fs = StubFileSystem::new().with_create_bin_symlink_fn(
                    move |name, version, cfg_sha256| {
                        assert_eq!(name, software_name);
                        assert_eq!(version, software_version);
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
                let cfg = Config::stub().with_fs(fs);
                if software.add_to_path(&cfg).is_ok() {
                    panic!("should fail");
                }
            }

            #[test]
            fn should_create_symlink() {
                let software_name = "stub";
                let software_version = "1.2.3";
                let software = StubSoftware::new(software_name, software_version);
                let fs = StubFileSystem::new().with_create_bin_symlink_fn(
                    move |name, version, cfg_sha256| {
                        assert_eq!(name, software_name);
                        assert_eq!(version, software_version);
                        assert_eq!(
                            cfg_sha256,
                            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                        );
                        Ok(())
                    },
                );
                let cfg = Config::stub().with_fs(fs);
                software.add_to_path(&cfg).unwrap();
            }
        }

        mod eq {
            use super::*;

            #[test]
            fn should_return_false() {
                let software1: Box<dyn Software> = Box::new(StubSoftware::new("stub", "1.2.3"));
                let software2: Box<dyn Software> = Box::new(StubSoftware::new("stub", "1.2.4"));
                assert!(software1 != software2);
            }

            #[test]
            fn should_return_true() {
                let software1: Box<dyn Software> = Box::new(StubSoftware::new("stub", "1.2.3"));
                let software2: Box<dyn Software> = Box::new(StubSoftware::new("stub", "1.2.3"));
                assert!(software1 == software2);
            }
        }

        mod is_installed {
            use super::*;

            macro_rules! test {
                ($ident:ident, $expected:expr) => {
                    #[test]
                    fn $ident() {
                        let software_name = "stub";
                        let software_version = "1.2.3";
                        let software = StubSoftware::new(software_name, software_version);
                        let fs = StubFileSystem::new().with_is_installed_software_fn(
                            move |name, version| {
                                assert_eq!(name, software_name);
                                assert_eq!(version, software_version);
                                $expected
                            },
                        );
                        let cfg = Config::stub().with_fs(fs);
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
                let software: Box<dyn Software> = Box::new(StubSoftware::new("stub", "1.2.3"));
                let expected = format!("{} v{}", software.name(), software.version());
                assert_eq!(software.to_string(), expected);
            }
        }
    }
}
