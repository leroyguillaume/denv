use crate::{
    util::{downloader::*, fs::*, zip::*},
    *,
};
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
pub enum LoadingError {
    IoFailed(io::Error),
    InvalidYaml(serde_yaml::Error),
    InvalidConfig(Vec<String>),
    HomeDirNotFound,
}

impl Display for LoadingError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::IoFailed(err) => write!(f, "{}", err),
            Self::InvalidYaml(err) => write!(f, "{}", err),
            Self::InvalidConfig(errs) => write!(f, "{}", errs.join(", ")),
            Self::HomeDirNotFound => write!(f, "Unable to find user home directory"),
        }
    }
}

pub struct Config {
    tools: Vec<ToolConfig>,
    pub(crate) fs: Box<dyn Fs>,
    pub(crate) downloader: Box<dyn Downloader>,
    pub(crate) unziper: Box<dyn Unziper>,
}

impl Config {
    pub fn load(filepath: &Path) -> Result<Self, LoadingError> {
        let cfg = map_debug_err!(read_to_string(filepath), LoadingError::IoFailed)?;
        let cfg = map_debug_err!(
            serde_yaml::from_str::<serde_json::Value>(&cfg),
            LoadingError::InvalidYaml
        )?;
        let schema = include_str!("../resources/main/config.schema.json");
        let schema = serde_json::from_str(schema).unwrap();
        let schema = JSONSchema::compile(&schema).unwrap();
        if let Err(err_iter) = schema.validate(&cfg) {
            let errs: Vec<String> = err_iter.map(|err| err.to_string()).collect();
            let err = LoadingError::InvalidConfig(errs);
            debug!("{}", err);
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
            unziper: Box::new(DefaultUnziper),
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
    pub fn stub(fs: StubFs, downloader: StubDownloader, unziper: StubUnziper) -> Self {
        Self {
            tools: vec![],
            downloader: Box::new(downloader),
            fs: Box::new(fs),
            unziper: Box::new(unziper),
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

            macro_rules! should_return_err {
                ($ident:ident, $filename:expr, $expect:ident) => {
                    #[test]
                    fn $ident() {
                        let filepath = format!("resources/tests/config/{}.yml", $filename);
                        match Config::load(Path::new(&filepath)) {
                            Ok(_) => panic!("should fail"),
                            Err(LoadingError::$expect(_)) => {}
                            Err(err) => panic!("{}", err),
                        }
                    }
                };
            }

            should_return_err!(should_return_io_failed_err, "not-found", IoFailed);
            should_return_err!(should_return_invalid_yaml_err, "invalid-yaml", InvalidYaml);
            should_return_err!(
                should_return_invalid_config_err,
                "invalid-config",
                InvalidConfig
            );

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

            mod io_failed {
                use super::*;

                #[test]
                fn should_return_string() {
                    let err = io::Error::from(io::ErrorKind::PermissionDenied);
                    let expected = err.to_string();
                    let err = LoadingError::IoFailed(err);
                    assert_eq!(err.to_string(), expected);
                }
            }

            mod invalid_yaml {
                use super::*;

                #[test]
                fn should_return_string() {
                    let err = serde_yaml::from_str::<serde_yaml::Value>("{").unwrap_err();
                    let expected = err.to_string();
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
                    let expected = errs.join(", ");
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
