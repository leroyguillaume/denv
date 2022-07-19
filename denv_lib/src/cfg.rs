#[cfg(test)]
use crate::internal::fs::StubFileSystem;
use crate::{
    internal::{
        downloader::*,
        fs::{DefaultFileSystem, FileSystem},
        zip::*,
    },
    tool::terraform::*,
    tool::*,
};
use hex::encode;
use home::home_dir;
use jsonschema::JSONSchema;
use log::debug;
use sha2::{Digest, Sha256};
use std::{
    env::temp_dir,
    fmt::{self, Display, Formatter},
    fs::read_to_string,
    io,
    path::Path,
};

#[derive(Debug)]
pub enum LoadingError {
    FileOpeningFailed(io::Error),
    InvalidYaml(serde_yaml::Error),
    InvalidConfig(Vec<String>),
    HomeDirNotFound,
}

impl Display for LoadingError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::FileOpeningFailed(err) => write!(f, "{}", err),
            Self::InvalidYaml(err) => write!(f, "Invalid YAML syntax: {}", err),
            Self::InvalidConfig(_) => write!(f, "Invalid configuration file"),
            Self::HomeDirNotFound => write!(f, "Unable to get user home directory"),
        }
    }
}

pub struct Config {
    tools: Vec<Box<dyn Tool>>,
    pub(crate) fs: Box<dyn FileSystem>,
    pub(crate) downloader: Box<dyn Downloader>,
    pub(crate) unzipper: Box<dyn Unzipper>,
}

impl Config {
    pub fn load(filepath: &Path) -> Result<Self, LoadingError> {
        debug!("Loading configuration from {}", filepath.display());
        let cfg = read_to_string(filepath).map_err(LoadingError::FileOpeningFailed)?;
        let cfg =
            serde_yaml::from_str::<serde_json::Value>(&cfg).map_err(LoadingError::InvalidYaml)?;
        let schema = include_str!("../resources/main/config.schema.json");
        let schema = serde_json::from_str(schema).unwrap();
        let schema = JSONSchema::compile(&schema).unwrap();
        if let Err(err_iter) = schema.validate(&cfg) {
            let errs = err_iter.map(|err| err.to_string()).collect();
            return Err(LoadingError::InvalidConfig(errs));
        }
        let mut tools: Vec<Box<dyn Tool>> = vec![];
        if let Some(cfg_tools) = cfg.get("tools") {
            if let Some(cfg_tool) = cfg_tools.get("terraform") {
                tools.push(Box::new(Terraform(cfg_tool.as_str().unwrap().into())));
            }
        }
        let fs_root_dirpath = match home_dir() {
            Some(home_dirpath) => home_dirpath,
            None => {
                let err = LoadingError::HomeDirNotFound;
                debug!("{}", err);
                return Err(err);
            }
        };
        let cfg = Self {
            tools,
            fs: Box::new(DefaultFileSystem::new(fs_root_dirpath, temp_dir())),
            downloader: Box::new(DefaultDownloader),
            unzipper: Box::new(DefaultUnzipper),
        };
        Ok(cfg)
    }

    pub fn sha256(&self) -> String {
        let mut hasher = Sha256::new();
        for tool in &self.tools {
            hasher.update(tool.name());
            hasher.update(tool.version());
        }
        let sha256 = hasher.finalize();
        encode(sha256)
    }

    pub fn tools(&self) -> &[Box<dyn Tool>] {
        &self.tools
    }
}

#[cfg(test)]
impl Config {
    pub fn stub(fs: StubFileSystem, downloader: StubDownloader, unziper: StubUnzipper) -> Self {
        Self {
            tools: vec![],
            downloader: Box::new(downloader),
            fs: Box::new(fs),
            unzipper: Box::new(unziper),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

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

            mod home_dir_not_found {
                use super::*;

                #[test]
                fn should_return_string() {
                    let expected = "Unable to get user home directory";
                    assert_eq!(LoadingError::HomeDirNotFound.to_string(), expected);
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
                let expected = Path::new("resources/tests/config/not-found.yml");
                match Config::load(expected) {
                    Ok(_) => panic!("should fail"),
                    Err(LoadingError::FileOpeningFailed(_)) => {}
                    Err(err) => panic!("{}", err),
                }
            }

            #[test]
            fn should_return_invalid_yaml_err() {
                let expected = Path::new("resources/tests/config/invalid-yaml.yml");
                match Config::load(expected) {
                    Ok(_) => panic!("should fail"),
                    Err(LoadingError::InvalidYaml(_)) => {}
                    Err(err) => panic!("{}", err),
                }
            }

            #[test]
            fn should_return_invalid_config_err() {
                let expected = Path::new("resources/tests/config/invalid-config.yml");
                match Config::load(expected) {
                    Ok(_) => panic!("should fail"),
                    Err(LoadingError::InvalidConfig(_)) => {}
                    Err(err) => panic!("{}", err),
                }
            }

            #[test]
            fn should_return_config() {
                let expected: Vec<Box<dyn Tool>> = vec![Box::new(Terraform("1.2.3".into()))];
                let cfg = Config::load(Path::new("resources/tests/config/denv.yml")).unwrap();
                assert_eq!(cfg.tools(), expected);
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
                let tool1 = DummyTool("1.2.3");
                let tool2 = DummyTool("1.2.4");
                let mut hasher = Sha256::new();
                hasher.update(tool1.name());
                hasher.update(tool1.version());
                hasher.update(tool2.name());
                hasher.update(tool2.version());
                let expected = encode(hasher.finalize());
                cfg.tools = vec![Box::new(tool1), Box::new(tool2)];
                assert_eq!(cfg.sha256(), expected);
            }
        }

        mod tools {
            use super::*;

            #[test]
            fn should_return_tools() {
                let mut cfg = Config::stub(
                    StubFileSystem::new(),
                    StubDownloader::new(),
                    StubUnzipper::new(),
                );
                let tool1 = DummyTool("1.2.3");
                let tool2 = DummyTool("1.2.4");
                cfg.tools = vec![Box::new(tool1), Box::new(tool2)];
                assert_eq!(cfg.tools(), cfg.tools);
            }
        }
    }
}
