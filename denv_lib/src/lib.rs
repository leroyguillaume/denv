pub mod cfg;
pub mod error;
mod internal;
pub mod software;

use crate::{cfg::*, error::*};
use log::debug;
use std::path::PathBuf;

macro_rules! dir_id {
    ($path:expr) => {{
        use sha2::Digest;
        let mut hasher = sha2::Sha256::new();
        hasher.update($path.to_string_lossy().as_bytes());
        let sha256 = hasher.finalize();
        hex::encode(sha256)
    }};
}

#[derive(Debug, Eq, PartialEq)]
pub struct DEnv(PathBuf);

impl DEnv {
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    pub fn run(&self, cfg: &Config) -> Result<(), Vec<RunError>> {
        let mut errs: Vec<RunError> = vec![];
        let dir_id = dir_id!(self.0);
        for software in cfg.softwares() {
            if software.is_installed(cfg) {
                debug!("{} is already installed", software);
            } else if let Err(err) = software.install(cfg) {
                errs.push(RunError::InstallFailed(software.to_string(), err));
                continue;
            }
            if let Err(err) =
                cfg.fs
                    .create_bin_symlink(software.name(), software.version(), &dir_id)
            {
                errs.push(RunError::SymlinkCreationFailed(software.to_string(), err))
            }
        }
        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{internal::fs::*, software::*};
    use std::io;

    mod denv {
        use super::*;

        mod new {
            use super::*;

            #[test]
            fn should_return_denv() {
                let expected = DEnv(PathBuf::from("/denv"));
                let denv = DEnv::new(expected.0.clone());
                assert_eq!(denv, expected);
            }
        }

        mod run {
            use super::*;

            #[test]
            fn should_return_list_of_errs() {
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
                    .with_create_bin_symlink_fn(move |name, version, _| {
                        assert_eq!(name, software2_name);
                        assert_eq!(version, software2_version);
                        Err(FileSystemError::new(
                            PathBuf::from("/error"),
                            io::Error::from(io::ErrorKind::PermissionDenied),
                        ))
                    })
                    .with_is_installed_software_fn(move |name, version| {
                        if name == software1_name {
                            assert_eq!(version, software1_version);
                            false
                        } else if name == software2_name {
                            assert_eq!(version, software2_version);
                            true
                        } else {
                            panic!()
                        }
                    });
                let cfg = Config::stub()
                    .with_softwares(vec![software1, software2])
                    .with_fs(fs);
                let denv = DEnv::new(PathBuf::from("/denv"));
                let errs = denv.run(&cfg).unwrap_err();
                assert_eq!(errs.len(), 2);
                match &errs[0] {
                    RunError::InstallFailed(software, _) => {
                        assert_eq!(software.clone(), software1_str)
                    }
                    _ => panic!(),
                }
                match &errs[1] {
                    RunError::SymlinkCreationFailed(software, _) => {
                        assert_eq!(software.clone(), software2_str)
                    }
                    _ => panic!(),
                }
            }

            #[test]
            fn should_return_ok() {
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
                    .with_create_bin_symlink_fn(move |name, version, _| {
                        if name == software1_name {
                            assert_eq!(version, software1_version);
                        } else if name == software2_name {
                            assert_eq!(version, software2_version);
                        } else {
                            panic!()
                        }
                        Ok(())
                    })
                    .with_is_installed_software_fn(move |name, version| {
                        if name == software1_name {
                            assert_eq!(version, software1_version);
                            false
                        } else if name == software2_name {
                            assert_eq!(version, software2_version);
                            true
                        } else {
                            panic!()
                        }
                    });
                let cfg = Config::stub()
                    .with_softwares(vec![software1, software2])
                    .with_fs(fs);
                let denv = DEnv::new(PathBuf::from("/denv"));
                denv.run(&cfg).unwrap();
            }
        }
    }
}
