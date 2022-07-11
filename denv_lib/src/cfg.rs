use crate::util::{downloader::*, fs::*, zip::*};
use home::home_dir;
use jsonschema::JSONSchema;
use log::debug;
use std::{
    env::temp_dir,
    fmt::{self, Display, Formatter},
    fs::read_to_string,
    io,
    path::Path,
};

#[derive(Debug)]
pub enum LoadingError<'a> {
    FileOpeningFailed(&'a Path, io::Error),
    InvalidYaml(serde_yaml::Error),
    InvalidConfig(Vec<String>),
    HomeDirNotFound,
}

impl Display for LoadingError<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::FileOpeningFailed(path, err) => {
                write!(f, "Unable to open {}: {}", path.display(), err)
            }
            Self::InvalidYaml(err) => write!(f, "Invalid YAML syntax: {}", err),
            Self::InvalidConfig(errs) => {
                write!(f, "Invalid configuration file:\n{}", errs.join("\n"))
            }
            Self::HomeDirNotFound => write!(f, "Unable to find user home directory"),
        }
    }
}

pub struct Config {
    tools: Vec<ToolConfig>,
    pub(crate) fs: Box<dyn Fs>,
    pub(crate) downloader: Box<dyn Downloader>,
    pub(crate) unzipper: Box<dyn Unzipper>,
}

impl Config {
    pub fn load(filepath: &Path) -> Result<Self, LoadingError> {
        debug!("Loading configuration from {}", filepath.display());
        let cfg = read_to_string(filepath)
            .map_err(|err| LoadingError::FileOpeningFailed(filepath, err))?;
        let cfg =
            serde_yaml::from_str::<serde_json::Value>(&cfg).map_err(LoadingError::InvalidYaml)?;
        let schema = include_str!("../resources/main/config.schema.json");
        let schema = serde_json::from_str(schema).unwrap();
        let schema = JSONSchema::compile(&schema).unwrap();
        if let Err(err_iter) = schema.validate(&cfg) {
            let errs: Vec<String> = err_iter.map(|err| err.to_string()).collect();
            let err = LoadingError::InvalidConfig(errs);
            return Err(err);
        }
        let mut tools = vec![];
        if let Some(cfg_tools) = cfg.get("tools") {
            if let Some(cfg_tool) = cfg_tools.get("terraform") {
                tools.push(ToolConfig::Terraform(cfg_tool.as_str().unwrap().into()));
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
            fs: Box::new(DefaultFs::new(fs_root_dirpath, temp_dir())),
            downloader: Box::new(DefaultDownloader),
            unzipper: Box::new(DefaultUnzipper),
        };
        Ok(cfg)
    }

    pub fn tools(&self) -> &[ToolConfig] {
        &self.tools
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ToolConfig {
    Terraform(String),
}

#[cfg(test)]
impl Config {
    pub fn stub(fs: StubFs, downloader: StubDownloader, unziper: StubUnzipper) -> Self {
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

    mod config {
        use super::*;

        mod load {
            use super::*;

            #[test]
            fn should_return_file_opening_failed() {
                let expected = Path::new("resources/tests/config/not-found.yml");
                match Config::load(expected) {
                    Ok(_) => panic!("should fail"),
                    Err(LoadingError::FileOpeningFailed(filepath, _)) => {
                        assert_eq!(filepath, expected)
                    }
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
                let expected = vec![ToolConfig::Terraform("1.2.3".into())];
                let cfg = Config::load(Path::new("resources/tests/config/denv.yml")).unwrap();
                assert_eq!(cfg.tools(), expected);
            }
        }
    }

    mod loading_error {
        use super::*;

        mod to_string {
            use super::*;

            mod file_opening_failed {
                use super::*;

                #[test]
                fn should_return_string() {
                    let filepath = Path::new("not-found.yml");
                    let err = io::Error::from(io::ErrorKind::PermissionDenied);
                    let expected = format!("Unable to open {}: {}", filepath.display(), err);
                    let err = LoadingError::FileOpeningFailed(filepath, err);
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
                    let cfg =
                        serde_yaml::from_str::<serde_json::Value>("tools: terraform\nfoo: bar")
                            .unwrap();
                    let schema = include_str!("../resources/main/config.schema.json");
                    let schema = serde_json::from_str(schema).unwrap();
                    let schema = JSONSchema::compile(&schema).unwrap();
                    let err_iter = schema.validate(&cfg).unwrap_err();
                    let errs: Vec<String> = err_iter.map(|err| err.to_string()).collect();
                    let err = LoadingError::InvalidConfig(errs.clone());
                    let expected = format!("Invalid configuration file:\n{}", errs.join("\n"));
                    assert_eq!(err.to_string(), expected);
                }
            }

            mod home_dir_not_found {
                use super::*;

                #[test]
                fn should_return_string() {
                    let expected = "Unable to find user home directory";
                    assert_eq!(LoadingError::HomeDirNotFound.to_string(), expected);
                }
            }
        }
    }
}
