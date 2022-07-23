pub mod cfg;
pub mod error;
mod internal;
pub mod software;
pub mod var;

use crate::{cfg::*, error::*, var::*};
use hex::encode;
use log::{debug, error, info};
use sha2::{Digest, Sha256};
use std::{io::Write, path::PathBuf};

const PATH_VARNAME: &str = "PATH";

#[derive(Debug, Eq, PartialEq)]
pub struct Environment(String);

impl Environment {
    pub fn new(path: PathBuf) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(path.to_string_lossy().as_bytes());
        let sha256 = hasher.finalize();
        Self(encode(sha256))
    }

    pub fn id(&self) -> &str {
        &self.0
    }

    pub fn load(&self, cfg: &Config) -> Result<(), EnvironmentLoadError> {
        let mut install_errs: Vec<(String, InstallError)> = vec![];
        let mut symlink_errs: Vec<(String, FileSystemError)> = vec![];
        for software in cfg.softwares() {
            let software = software.as_ref();
            if cfg.fs.is_installed_software(software) {
                debug!("{} is already installed", software);
            } else if let Err(err) = software.install(cfg) {
                error!("Unable to install {}: {}", software, err);
                install_errs.push((software.to_string(), err));
                continue;
            }
            if let Err(err) = cfg.fs.create_bin_symlink(&self.0, software) {
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
        let env_dirpath = cfg.fs.env_dirpath(&self.0);
        let path_var = Var::new(
            PATH_VARNAME.into(),
            format!("{}:${}", env_dirpath.display(), PATH_VARNAME),
        );
        let (env_filepath, mut env_file) = cfg
            .fs
            .create_env_file(&self.0)
            .map_err(EnvironmentLoadError::EnvFileWritingFailed)?;
        writeln!(env_file, "{}", path_var.export_statement()).map_err(|err| {
            EnvironmentLoadError::EnvFileWritingFailed(FileSystemError::new(env_filepath, err))
        })
    }

    pub fn path(&self, cfg: &Config) -> PathBuf {
        cfg.fs.env_dirpath(&self.0)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{internal::fs::*, software::*};
    use std::{
        fs::{read_to_string, File},
        io,
    };
    use tempfile::tempdir;

    mod environment {
        use super::*;

        mod new {
            use super::*;

            #[test]
            fn should_return_env() {
                let dirpath = PathBuf::from("/denv");
                let mut hasher = Sha256::new();
                hasher.update(dirpath.to_string_lossy().as_bytes());
                let sha256 = hasher.finalize();
                let expected = Environment(encode(sha256));
                let denv = Environment::new(dirpath);
                assert_eq!(denv, expected);
                assert_eq!(denv.id(), expected.0);
            }
        }

        mod load {
            use super::*;

            #[test]
            fn should_return_install_failed_err() {
                let dirpath = PathBuf::from("/denv");
                let env = Environment::new(dirpath);
                let epxected_env_id = env.0.clone();
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
                    .with_create_bin_symlink_fn(move |env_id, software| {
                        assert_eq!(env_id, epxected_env_id);
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
                    err => panic!("{}", err),
                }
            }

            #[test]
            fn should_return_env_file_writing_failed_if_env_file_opening_failed() {
                let dirpath = PathBuf::from("/denv");
                let env = Environment::new(dirpath);
                let expected_env_id = env.0.clone();
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
                    .with_create_bin_symlink_fn({
                        let expected_env_id = expected_env_id.clone();
                        move |env_id, software| {
                            let name = software.name();
                            assert_eq!(env_id, expected_env_id);
                            if name == software1_name {
                                assert_eq!(software.version(), software1_version);
                            } else if name == software2_name {
                                assert_eq!(software.version(), software2_version);
                            } else {
                                panic!()
                            }
                            Ok(())
                        }
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
                    })
                    .with_env_dirpath_fn({
                        let expected_env_id = expected_env_id.clone();
                        move |env_id| {
                            assert_eq!(env_id, expected_env_id);
                            PathBuf::from("/env")
                        }
                    })
                    .with_create_env_file_fn(move |env_id| {
                        assert_eq!(env_id, expected_env_id);
                        Err(FileSystemError::new(
                            PathBuf::from("/error"),
                            io::Error::from(io::ErrorKind::PermissionDenied),
                        ))
                    });
                let cfg = Config::stub()
                    .with_softwares(vec![software1, software2])
                    .with_fs(fs);
                match env.load(&cfg).unwrap_err() {
                    EnvironmentLoadError::EnvFileWritingFailed(_) => {}
                    err => panic!("{}", err),
                }
            }

            #[test]
            fn should_return_env_file_writing_failed_if_env_file_writing_failed() {
                let dirpath = PathBuf::from("/denv");
                let env = Environment::new(dirpath);
                let expected_env_id = env.0.clone();
                let env_dirpath = tempdir().unwrap().into_path();
                let env_filepath = env_dirpath.join("env");
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
                    .with_create_bin_symlink_fn({
                        let expected_env_id = expected_env_id.clone();
                        move |env_id, software| {
                            let name = software.name();
                            assert_eq!(env_id, expected_env_id);
                            if name == software1_name {
                                assert_eq!(software.version(), software1_version);
                            } else if name == software2_name {
                                assert_eq!(software.version(), software2_version);
                            } else {
                                panic!()
                            }
                            Ok(())
                        }
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
                    })
                    .with_env_dirpath_fn({
                        let expected_env_id = expected_env_id.clone();
                        move |env_id| {
                            assert_eq!(env_id, expected_env_id);
                            env_dirpath.clone()
                        }
                    })
                    .with_create_env_file_fn({
                        File::create(&env_filepath).unwrap();
                        move |env_id| {
                            assert_eq!(env_id, expected_env_id);
                            Ok((env_filepath.clone(), File::open(&env_filepath).unwrap()))
                        }
                    });
                let cfg = Config::stub()
                    .with_softwares(vec![software1, software2])
                    .with_fs(fs);
                match env.load(&cfg).unwrap_err() {
                    EnvironmentLoadError::EnvFileWritingFailed(_) => {}
                    err => panic!("{}", err),
                }
            }

            #[test]
            fn should_return_ok() {
                let dirpath = PathBuf::from("/denv");
                let env = Environment::new(dirpath);
                let expected_env_id = env.0.clone();
                let env_dirpath = tempdir().unwrap().into_path();
                let env_filepath = env_dirpath.join("env");
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
                    .with_create_bin_symlink_fn({
                        let expected_env_id = expected_env_id.clone();
                        move |env_id, software| {
                            let name = software.name();
                            assert_eq!(env_id, expected_env_id);
                            if name == software1_name {
                                assert_eq!(software.version(), software1_version);
                            } else if name == software2_name {
                                assert_eq!(software.version(), software2_version);
                            } else {
                                panic!()
                            }
                            Ok(())
                        }
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
                    })
                    .with_env_dirpath_fn({
                        let expected_env_id = expected_env_id.clone();
                        let env_dirpath = env_dirpath.clone();
                        move |env_id| {
                            assert_eq!(env_id, expected_env_id);
                            env_dirpath.clone()
                        }
                    })
                    .with_create_env_file_fn({
                        let env_filepath = env_filepath.clone();
                        move |env_id| {
                            assert_eq!(env_id, expected_env_id);
                            Ok((env_filepath.clone(), File::create(&env_filepath).unwrap()))
                        }
                    });
                let cfg = Config::stub()
                    .with_softwares(vec![software1, software2])
                    .with_fs(fs);
                env.load(&cfg).unwrap();
                let path_var = Var::new(
                    PATH_VARNAME.into(),
                    format!("{}:${}", env_dirpath.display(), PATH_VARNAME),
                );
                let env_file_content = read_to_string(env_filepath).unwrap();
                let expected_env_file_content = format!("{}\n", path_var.export_statement());
                assert_eq!(env_file_content, expected_env_file_content);
            }
        }

        mod path {
            use super::*;

            #[test]
            fn should_return_path() {
                let env = Environment::new(PathBuf::from("/denv"));
                let expected_env_id = env.0.clone();
                let expected = PathBuf::from("/env");
                let fs = StubFileSystem::new().with_env_dirpath_fn({
                    let expected = expected.clone();
                    move |env_id| {
                        assert_eq!(env_id, expected_env_id);
                        expected.clone()
                    }
                });
                let cfg = Config::stub().with_fs(fs);
                assert_eq!(env.path(&cfg), expected);
            }
        }
    }
}
