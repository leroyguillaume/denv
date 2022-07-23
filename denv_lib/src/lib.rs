pub mod cfg;
pub mod error;
mod internal;
pub mod software;
pub mod var;

use crate::{cfg::*, error::*};
use log::{debug, error, info};
use std::path::PathBuf;

macro_rules! env_id {
    ($path:expr) => {{
        use sha2::Digest;
        let mut hasher = sha2::Sha256::new();
        hasher.update($path.to_string_lossy().as_bytes());
        let sha256 = hasher.finalize();
        hex::encode(sha256)
    }};
}

#[derive(Debug, Eq, PartialEq)]
pub struct Environment(PathBuf);

impl Environment {
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    pub fn load(&self, cfg: &Config) -> Result<(), EnvironmentLoadError> {
        let mut install_errs: Vec<(String, InstallError)> = vec![];
        let mut symlink_errs: Vec<(String, FileSystemError)> = vec![];
        let env_id = env_id!(self.0);
        for software in cfg.softwares() {
            let software = software.as_ref();
            if cfg.fs.is_installed_software(software) {
                debug!("{} is already installed", software);
            } else if let Err(err) = software.install(cfg) {
                error!("Unable to install {}: {}", software, err);
                install_errs.push((software.to_string(), err));
                continue;
            }
            if let Err(err) = cfg.fs.create_bin_symlink(&env_id, software) {
                error!("Unable to create symlink for {}: {}", software, err);
                symlink_errs.push((software.to_string(), err));
                continue;
            }
            info!("{}", software);
        }
        if !install_errs.is_empty() || !symlink_errs.is_empty() {
            return Err(EnvironmentLoadError::InstallFailed {
                install_errs,
                symlink_errs,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{internal::fs::*, software::*};
    use std::io;

    mod environment {
        use super::*;

        mod new {
            use super::*;

            #[test]
            fn should_return_env() {
                let expected = Environment(PathBuf::from("/denv"));
                let denv = Environment::new(expected.0.clone());
                assert_eq!(denv, expected);
            }
        }

        mod load {
            use super::*;

            #[test]
            fn should_return_list_of_errs() {
                let dirpath = PathBuf::from("/denv");
                let software1_name = "software1";
                let software1_version = "3.2.1";
                let software1 = StubSoftware::new(software1_name, software1_version)
                    .with_install_fn(|_| {
                        Err(InstallError::FileSystemWritingFailed(FileSystemError::new(
                            PathBuf::from("/error"),
                            io::Error::from(io::ErrorKind::PermissionDenied),
                        )))
                    });
                let software1: Box<dyn Software> = Box::new(software1);
                let software1_str = software1.to_string();
                let software2_name = "software2";
                let software2_version = "1.2.3";
                let software2 = StubSoftware::new(software2_name, software2_version)
                    .with_install_fn(|_| Ok(()));
                let software2: Box<dyn Software> = Box::new(software2);
                let software2_str = software2.to_string();
                let fs = StubFileSystem::new()
                    .with_create_bin_symlink_fn(move |dir_id, software| {
                        assert_eq!(dir_id, env_id!(dirpath));
                        assert_eq!(software.name(), software2_name);
                        assert_eq!(software.version(), software2_version);
                        Err(FileSystemError::new(
                            PathBuf::from("/error"),
                            io::Error::from(io::ErrorKind::PermissionDenied),
                        ))
                    })
                    .with_is_installed_software_fn(move |software| {
                        let name = software.name();
                        if name == software1_name {
                            assert_eq!(software.version(), software1_version);
                            false
                        } else if name == software2_name {
                            assert_eq!(software.version(), software2_version);
                            true
                        } else {
                            panic!()
                        }
                    });
                let cfg = Config::stub()
                    .with_softwares(vec![software1, software2])
                    .with_fs(fs);
                let env = Environment::new(PathBuf::from("/denv"));
                match env.load(&cfg).unwrap_err() {
                    EnvironmentLoadError::InstallFailed {
                        install_errs,
                        symlink_errs,
                    } => {
                        assert_eq!(install_errs.len(), 1);
                        assert_eq!(install_errs[0].0, software1_str);
                        assert_eq!(symlink_errs.len(), 1);
                        assert_eq!(symlink_errs[0].0, software2_str);
                    }
                }
            }

            #[test]
            fn should_return_ok() {
                let dirpath = PathBuf::from("/denv");
                let software1_name = "software1";
                let software1_version = "3.2.1";
                let software1 = StubSoftware::new(software1_name, software1_version)
                    .with_install_fn(|_| Ok(()));
                let software1: Box<dyn Software> = Box::new(software1);
                let software2_name = "software2";
                let software2_version = "1.2.3";
                let software2 = StubSoftware::new(software2_name, software2_version)
                    .with_install_fn(|_| Ok(()));
                let software2: Box<dyn Software> = Box::new(software2);
                let fs = StubFileSystem::new()
                    .with_create_bin_symlink_fn(move |dir_id, software| {
                        let name = software.name();
                        assert_eq!(dir_id, env_id!(dirpath));
                        if name == software1_name {
                            assert_eq!(software.version(), software1_version);
                        } else if name == software2_name {
                            assert_eq!(software.version(), software2_version);
                        } else {
                            panic!()
                        }
                        Ok(())
                    })
                    .with_is_installed_software_fn(move |software| {
                        let name = software.name();
                        if name == software1_name {
                            assert_eq!(software.version(), software1_version);
                            false
                        } else if name == software2_name {
                            assert_eq!(software.version(), software2_version);
                            true
                        } else {
                            panic!()
                        }
                    });
                let cfg = Config::stub()
                    .with_softwares(vec![software1, software2])
                    .with_fs(fs);
                let env = Environment::new(PathBuf::from("/denv"));
                env.load(&cfg).unwrap();
            }
        }
    }
}
