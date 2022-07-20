#[cfg(test)]
use crate::internal::fs::StubFileSystem;
use crate::{
    internal::{
        downloader::*,
        fs::{DefaultFileSystem, FileSystem},
        unzip::*,
    },
    software::terraform::*,
    software::*,
};
use hex::encode;
use jsonschema::JSONSchema;
use log::debug;
use sha2::{Digest, Sha256};
use std::{
    env::temp_dir,
    fmt::{self, Display, Formatter},
    fs::read_to_string,
    io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum LoadingError {
    FileOpeningFailed(io::Error),
    InvalidYaml(serde_yaml::Error),
    InvalidConfig(Vec<String>),
}

impl Display for LoadingError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::FileOpeningFailed(err) => write!(f, "{}", err),
            Self::InvalidYaml(err) => write!(f, "Invalid YAML syntax: {}", err),
            Self::InvalidConfig(_) => write!(f, "Invalid configuration file"),
        }
    }
}

pub struct Config {
    softwares: Vec<Box<dyn Software>>,
    pub(crate) fs: Box<dyn FileSystem>,
    pub(crate) downloader: Box<dyn Downloader>,
    pub(crate) unzipper: Box<dyn Unzipper>,
}

impl Config {
    pub fn load(filepath: &Path, denv_dirpath: PathBuf) -> Result<Self, LoadingError> {
        debug!("Loading configuration from {}", filepath.display());
        let cfg = read_to_string(filepath).map_err(LoadingError::FileOpeningFailed)?;
        let cfg =
            serde_yaml::from_str::<serde_json::Value>(&cfg).map_err(LoadingError::InvalidYaml)?;
        let schema = include_str!("../config.schema.json");
        let schema = serde_json::from_str(schema).unwrap();
        let schema = JSONSchema::compile(&schema).unwrap();
        if let Err(err_iter) = schema.validate(&cfg) {
            let errs = err_iter.map(|err| err.to_string()).collect();
            return Err(LoadingError::InvalidConfig(errs));
        }
        let mut softwares: Vec<Box<dyn Software>> = vec![];
        if let Some(cfg_softwares) = cfg.get("softwares") {
            if let Some(cfg_softwares) = cfg_softwares.get("terraform") {
                softwares.push(Box::new(Terraform(cfg_softwares.as_str().unwrap().into())));
            }
        }
        let cfg = Self {
            softwares,
            fs: Box::new(DefaultFileSystem::new(denv_dirpath, temp_dir())),
            downloader: Box::new(DefaultDownloader),
            unzipper: Box::new(DefaultUnzipper),
        };
        Ok(cfg)
    }

    pub fn sha256(&self) -> String {
        let mut hasher = Sha256::new();
        for software in &self.softwares {
            hasher.update(software.name());
            hasher.update(software.version());
        }
        let sha256 = hasher.finalize();
        encode(sha256)
    }

    pub fn softwares(&self) -> &[Box<dyn Software>] {
        &self.softwares
    }
}

#[cfg(test)]
impl Config {
    pub fn stub(fs: StubFileSystem, downloader: StubDownloader, unziper: StubUnzipper) -> Self {
        Self {
            softwares: vec![],
            downloader: Box::new(downloader),
            fs: Box::new(fs),
            unzipper: Box::new(unziper),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{fs::File, io::Write};
    use tempfile::tempdir;

    mod loading_error {
        use super::*;

        mod to_string {
            use super::*;

            mod file_opening_failed {
                use super::*;

                #[test]
                fn should_return_string() {
                    let err = io::Error::from(io::ErrorKind::PermissionDenied);
                    let expected = err.to_string();
                    let err = LoadingError::FileOpeningFailed(err);
                    assert_eq!(err.to_string(), expected);
                }
            }

            mod invalid_yaml {
                use super::*;

                #[test]
                fn should_return_string() {
                    let err = serde_yaml::from_str::<serde_yaml::Value>("{").unwrap_err();
                    let expected = format!("Invalid YAML syntax: {}", err);
                    let err = LoadingError::InvalidYaml(err);
                    assert_eq!(err.to_string(), expected);
                }
            }

            mod invalid_config {
                use super::*;

                #[test]
                fn should_return_string() {
                    let err = LoadingError::InvalidConfig(vec![]);
                    assert_eq!(err.to_string(), "Invalid configuration file");
                }
            }
        }
    }

    mod config {
        use super::*;

        mod load {
            use super::*;

            #[test]
            fn should_return_file_opening_failed() {
                match Config::load(Path::new("denv.yaml"), PathBuf::from(".denv")) {
                    Ok(_) => panic!("should fail"),
                    Err(LoadingError::FileOpeningFailed(_)) => {}
                    Err(err) => panic!("{}", err),
                }
            }

            #[test]
            fn should_return_invalid_yaml_err() {
                let dirpath = tempdir().unwrap().into_path();
                let filepath = dirpath.join("denv.yml");
                let mut file = File::create(&filepath).unwrap();
                write!(file, "{{").unwrap();
                match Config::load(Path::new(&filepath), PathBuf::from(".denv")) {
                    Ok(_) => panic!("should fail"),
                    Err(LoadingError::InvalidYaml(_)) => {}
                    Err(err) => panic!("{}", err),
                }
            }

            #[test]
            fn should_return_invalid_config_err() {
                let dirpath = tempdir().unwrap().into_path();
                let filepath = dirpath.join("denv.yml");
                let mut file = File::create(&filepath).unwrap();
                write!(file, "softwares: terraform").unwrap();
                match Config::load(Path::new(&filepath), PathBuf::from(".denv")) {
                    Ok(_) => panic!("should fail"),
                    Err(LoadingError::InvalidConfig(_)) => {}
                    Err(err) => panic!("{}", err),
                }
            }

            #[test]
            fn should_return_config() {
                let softwares: Vec<Box<dyn Software>> = vec![Box::new(Terraform("1.2.3".into()))];
                let denv_dirpath = PathBuf::from(".denv");
                let cfg =
                    Config::load(Path::new("../examples/denv.yml"), denv_dirpath.clone()).unwrap();
                assert_eq!(cfg.softwares(), softwares);
                assert_eq!(cfg.fs.denv_dirpath(), denv_dirpath);
            }
        }

        mod sha256 {
            use super::*;

            #[test]
            fn should_return_sha256_hex_string() {
                let mut cfg = Config::stub(
                    StubFileSystem::new(),
                    StubDownloader::new(),
                    StubUnzipper::new(),
                );
                let software1 = DummySoftware("1.2.3");
                let software2 = DummySoftware("1.2.4");
                let mut hasher = Sha256::new();
                hasher.update(software1.name());
                hasher.update(software1.version());
                hasher.update(software2.name());
                hasher.update(software2.version());
                let expected = encode(hasher.finalize());
                cfg.softwares = vec![Box::new(software1), Box::new(software2)];
                assert_eq!(cfg.sha256(), expected);
            }
        }

        mod softwares {
            use super::*;

            #[test]
            fn should_return_softwares() {
                let mut cfg = Config::stub(
                    StubFileSystem::new(),
                    StubDownloader::new(),
                    StubUnzipper::new(),
                );
                let software1 = DummySoftware("1.2.3");
                let software2 = DummySoftware("1.2.4");
                cfg.softwares = vec![Box::new(software1), Box::new(software2)];
                assert_eq!(cfg.softwares(), cfg.softwares);
            }
        }
    }
}
