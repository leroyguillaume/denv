// IMPORTS

use super::{
    installer::{ArchiveArtifactInstaller, Artifact, DefaultArchiveArtifactInstaller, Symlink},
    Error, Kind, Result, Software,
};
use crate::fs::FileSystem;
use std::{env, path::Path};

// CONSTS

const TF_BIN_NAME: &str = "terraform";
const TF_SOFT_NAME: &str = "terraform";

// STRUCTS

pub struct Terraform {
    installer: Box<dyn ArchiveArtifactInstaller>,
    version: String,
}

impl Terraform {
    pub fn new(version: String) -> Self {
        Self {
            installer: Box::new(DefaultArchiveArtifactInstaller::default()),
            version,
        }
    }

    #[inline]
    fn arch() -> Result<&'static str> {
        match env::consts::ARCH {
            "x86" => Ok("386"),
            "x86_64" => Ok("amd64"),
            "arm" => Ok("arm"),
            "aarch64" => Ok("arm64"),
            _ => Err(Error::UnsupportedSystem),
        }
    }

    #[inline]
    fn os() -> Result<&'static str> {
        match env::consts::OS {
            "macos" => Ok("darwin"),
            "linux" => Ok("linux"),
            _ => Err(Error::UnsupportedSystem),
        }
    }
}

impl Software for Terraform {
    fn install(&self, project_dirpath: &Path, fs: &dyn FileSystem) -> Result<()> {
        let os = Self::os()?;
        let arch = Self::arch()?;
        let env_dirpath = fs.ensure_env_dir(project_dirpath).map_err(Error::Io)?;
        let artifact = Artifact {
            name: TF_SOFT_NAME,
            symlinks: vec![Symlink {
                dest: env_dirpath.join(TF_BIN_NAME),
                src: Path::new(TF_BIN_NAME),
            }],
            url: format!(
                "https://releases.hashicorp.com/terraform/{}/terraform_{}_{}_{}.zip",
                self.version, self.version, os, arch,
            ),
            version: &self.version,
        };
        self.installer.install_zip(&artifact, fs)
    }

    fn kind(&self) -> Kind {
        Kind::Terraform(self)
    }

    fn name(&self) -> &str {
        TF_SOFT_NAME
    }

    fn version(&self) -> &str {
        &self.version
    }
}

// TESTS

#[cfg(test)]
mod terraform_test {
    use super::*;
    use crate::{fs::StubFileSystem, soft::installer::StubArchiveArtifactInstaller};
    use std::io;

    mod new {
        use super::*;

        #[test]
        fn should_return_soft() {
            let version = "1.2.3";
            let soft = Terraform::new(version.into());
            assert_eq!(soft.name(), TF_SOFT_NAME);
            assert_eq!(soft.version(), version);
            match soft.kind() {
                Kind::Terraform(_) => {}
                _ => panic!(),
            }
        }
    }

    mod install {
        use super::*;

        struct Data {
            env_dirpath: &'static Path,
            project_dirpath: &'static Path,
            version: &'static str,
        }

        impl Default for Data {
            fn default() -> Self {
                Self {
                    env_dirpath: Path::new("/env"),
                    project_dirpath: Path::new("/project"),
                    version: "1.2.3",
                }
            }
        }

        struct Stubs {
            installer: StubArchiveArtifactInstaller,
            fs: StubFileSystem,
        }

        impl Stubs {
            fn new(data: &Data) -> Self {
                let env_dirpath = data.env_dirpath;
                let expected_project_dirpath = data.project_dirpath;
                let version = data.version;
                let mut stubs = Self {
                    installer: StubArchiveArtifactInstaller::default(),
                    fs: StubFileSystem::default(),
                };
                stubs.fs.stub_ensure_env_dir_fn(move |project_dirpath| {
                    assert_eq!(project_dirpath, expected_project_dirpath);
                    Ok(env_dirpath.to_path_buf())
                });
                stubs.installer.stub_install_zip_fn(move |artifact, _| {
                    let expected_artifact = Artifact {
                        name: TF_SOFT_NAME,
                        symlinks: vec![Symlink {
                            dest: env_dirpath.join(TF_BIN_NAME),
                            src: Path::new(TF_BIN_NAME),
                        }],
                        url: format!(
                            "https://releases.hashicorp.com/terraform/{}/terraform_{}_{}_{}.zip",
                            version,
                            version,
                            Terraform::os().unwrap(),
                            Terraform::arch().unwrap(),
                        ),
                        version,
                    };
                    assert_eq!(*artifact, expected_artifact);
                    Ok(())
                });
                stubs
            }
        }

        #[test]
        fn should_return_io_err_if_ensure_env_dir_failed() {
            let data = Data::default();
            let mut stubs = Stubs::new(&data);
            stubs
                .fs
                .stub_ensure_env_dir_fn(|_| Err(io::Error::from(io::ErrorKind::PermissionDenied)));
            test(&data, stubs, |res| match res.unwrap_err() {
                Error::Io(_) => {}
                err => panic!("{}", err),
            });
        }

        #[test]
        fn should_return_err_if_install_zip_failed() {
            let data = Data::default();
            let mut stubs = Stubs::new(&data);
            stubs
                .installer
                .stub_install_zip_fn(|_, _| Err(Error::UnsupportedSystem));
            test(&data, stubs, |res| match res.unwrap_err() {
                Error::UnsupportedSystem => {}
                err => panic!("{}", err),
            });
        }

        #[test]
        fn should_return_ok() {
            let data = Data::default();
            let stubs = Stubs::new(&data);
            test(&data, stubs, |res| {
                res.unwrap();
            });
        }

        #[inline]
        fn test<F: Fn(Result<()>)>(data: &Data, stubs: Stubs, assert_fn: F) {
            let soft = Terraform {
                installer: Box::new(stubs.installer),
                version: data.version.into(),
            };
            let res = soft.install(data.project_dirpath, &stubs.fs);
            assert_fn(res);
        }
    }
}
