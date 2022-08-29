// IMPORTS

use super::{
    installer::{ArchiveArtifactInstaller, Artifact, DefaultArchiveArtifactInstaller, Symlink},
    Error, Kind, Result, Software,
};
use crate::fs::FileSystem;
use std::{env, path::Path};

// CONSTS

const CT_BIN_NAME: &str = "ct";
const CT_SOFT_NAME: &str = "chart-testing";

// STRUCTS

pub struct ChartTesting {
    installer: Box<dyn ArchiveArtifactInstaller>,
    version: String,
}

impl ChartTesting {
    pub fn new(version: String) -> Self {
        Self {
            installer: Box::new(DefaultArchiveArtifactInstaller::default()),
            version,
        }
    }

    #[inline]
    fn arch() -> Result<&'static str> {
        match env::consts::ARCH {
            "x86_64" => Ok("amd64"),
            "arm" => Ok("armv6"),
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

impl Software for ChartTesting {
    fn install(&self, project_dirpath: &Path, fs: &dyn FileSystem) -> Result<()> {
        let env_dirpath = fs
            .ensure_env_dir_is_present(project_dirpath)
            .map_err(Error::Io)?;
        let home_dirpath = fs.home_dirpath().map_err(Error::Io)?;
        let artifact = Artifact {
            name: CT_SOFT_NAME,
            symlinks: vec![
                Symlink {
                    dest: env_dirpath.join(CT_BIN_NAME),
                    src: Path::new(CT_BIN_NAME),
            }, Symlink {
                dest: home_dirpath.join(".ct/chart_schema.yaml"),
                src: Path::new("etc/chart-schema.yaml"),
            }, Symlink {
                dest: home_dirpath.join(".ct/lintconf.yaml"),
                src: Path::new("etc/lintconf.yaml"),
            }
            ],
            url: format!(
                "https://github.com/helm/chart-testing/releases/download/v{}/chart-testing_{}_{}_{}.tar.gz",
                self.version, self.version, Self::os()?, Self::arch()?,
            ),
            version: &self.version,
        };
        self.installer.install_targz(&artifact, fs)
    }

    fn kind(&self) -> Kind {
        Kind::ChartTesting(self)
    }

    fn name(&self) -> &str {
        CT_SOFT_NAME
    }

    fn version(&self) -> &str {
        &self.version
    }
}

// TESTS

#[cfg(test)]
mod chart_testing_test {
    use super::*;
    use crate::{fs::StubFileSystem, soft::installer::StubArchiveArtifactInstaller};
    use std::io;

    mod new {
        use super::*;

        #[test]
        fn should_return_soft() {
            let version = "3.7.0";
            let soft = ChartTesting::new(version.into());
            assert_eq!(soft.name(), CT_SOFT_NAME);
            assert_eq!(soft.version(), version);
            match soft.kind() {
                Kind::ChartTesting(_) => {}
                _ => panic!(),
            }
        }
    }

    mod install {
        use super::*;

        struct Data {
            env_dirpath: &'static Path,
            home_dirpath: &'static Path,
            project_dirpath: &'static Path,
            version: &'static str,
        }

        impl Default for Data {
            fn default() -> Self {
                Self {
                    env_dirpath: Path::new("/env"),
                    home_dirpath: Path::new("/home"),
                    project_dirpath: Path::new("/project"),
                    version: "3.7.0",
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
                let home_dirpath = data.home_dirpath;
                let expected_project_dirpath = data.project_dirpath;
                let version = data.version;
                let mut stubs = Self {
                    installer: StubArchiveArtifactInstaller::default(),
                    fs: StubFileSystem::default(),
                };
                stubs
                    .fs
                    .stub_ensure_env_dir_is_present_fn(move |project_dirpath| {
                        assert_eq!(project_dirpath, expected_project_dirpath);
                        Ok(env_dirpath.to_path_buf())
                    });
                stubs
                    .fs
                    .stub_home_dirpath_fn(|| Ok(home_dirpath.to_path_buf()));
                stubs.installer.stub_install_targz_fn(move |artifact, _| {
                    let expected_artifact = Artifact {
                        name: CT_SOFT_NAME,
                        symlinks: vec![Symlink {
                            dest: env_dirpath.join(CT_BIN_NAME),
                            src: Path::new(CT_BIN_NAME),
                        }, Symlink {
                            dest: home_dirpath.join(".ct/chart_schema.yaml"),
                            src: Path::new("etc/chart-schema.yaml"),
                        }, Symlink {
                            dest: home_dirpath.join(".ct/lintconf.yaml"),
                            src: Path::new("etc/lintconf.yaml"),
                        }],
                        url: format!(
                            "https://github.com/helm/chart-testing/releases/download/v{}/chart-testing_{}_{}_{}.tar.gz",
                            version, version, ChartTesting::os().unwrap(), ChartTesting::arch().unwrap(),
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
        fn should_return_io_err_if_ensure_env_dir_is_present_failed() {
            let data = Data::default();
            let mut stubs = Stubs::new(&data);
            stubs.fs.stub_ensure_env_dir_is_present_fn(|_| {
                Err(io::Error::from(io::ErrorKind::PermissionDenied))
            });
            test(&data, stubs, |res| match res.unwrap_err() {
                Error::Io(_) => {}
                err => panic!("{}", err),
            });
        }

        #[test]
        fn should_return_io_err_if_home_dirpath_failed() {
            let data = Data::default();
            let mut stubs = Stubs::new(&data);
            stubs
                .fs
                .stub_home_dirpath_fn(|| Err(io::Error::from(io::ErrorKind::PermissionDenied)));
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
                .stub_install_targz_fn(|_, _| Err(Error::UnsupportedSystem));
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
            let soft = ChartTesting {
                installer: Box::new(stubs.installer),
                version: data.version.into(),
            };
            let res = soft.install(data.project_dirpath, &stubs.fs);
            assert_fn(res);
        }
    }
}
