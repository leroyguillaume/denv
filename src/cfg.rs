// IMPORTS

use jsonschema::JSONSchema;
use log::debug;
use serde_json::Value;
use std::{
    fmt::{self, Display, Formatter},
    fs::File,
    io,
    path::PathBuf,
};

// MACROS

macro_rules! add_software_definition_if_present {
    ($key:literal, $kind:expr, $value:expr, $cfg:expr) => {
        if let Some(soft_version) = $value.get($key) {
            let soft_version = soft_version.as_str().unwrap();
            let soft_def = SoftwareDefinition {
                kind: $kind,
                version: soft_version.into(),
            };
            $cfg.soft_defs.push(soft_def);
        }
    };
}

// TYPES

pub type Result = std::result::Result<Config, Error>;

// ENUMS

#[derive(Debug)]
pub enum Error {
    Config(Vec<String>),
    Io(io::Error),
    Version(Option<Value>),
    YamlSyntax(serde_yaml::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(_) => write!(f, "Invalid configuration"),
            Self::Io(err) => write!(f, "{}", err),
            Self::Version(version) => match version {
                Some(version) => write!(f, "{} is not a valid configuration version", version),
                None => write!(f, "Missing configuration version"),
            },
            Self::YamlSyntax(err) => write!(f, "{}", err),
        }
    }
}

// ENUMS

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SoftwareDefinitionKind {
    Terraform,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VarDefinitionKind {
    Literal(String),
}

// DATA STRUCTS

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub path: PathBuf,
    pub soft_defs: Vec<SoftwareDefinition>,
    pub var_defs: Vec<VarDefinition>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SoftwareDefinition {
    pub kind: SoftwareDefinitionKind,
    pub version: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VarDefinition {
    pub kind: VarDefinitionKind,
    pub name: String,
}

// TRAITS

pub trait ConfigLoader {
    fn load(&self, path: PathBuf) -> Result;
}

// STRUCTS

pub struct DefaultConfigLoader;

impl DefaultConfigLoader {
    #[inline]
    fn load_v1(path: PathBuf, json: Value) -> Result {
        let schema = include_str!("../resources/main/config/v1.schema.json");
        let schema: Value = serde_json::from_str(schema).unwrap();
        let schema = JSONSchema::compile(&schema).unwrap();
        if let Err(errs) = schema.validate(&json) {
            let err = Error::Config(errs.map(|err| err.to_string()).collect());
            return Err(err);
        }
        let mut config = Config {
            path,
            soft_defs: vec![],
            var_defs: vec![],
        };
        if let Some(softs) = json.get("softwares") {
            add_software_definition_if_present!(
                "terraform",
                SoftwareDefinitionKind::Terraform,
                softs,
                config
            );
        }
        if let Some(vars) = json.get("set") {
            let vars = vars.as_array().unwrap();
            for var in vars {
                let var_name = var.get("name").unwrap().as_str().unwrap();
                if let Some(var_value) = var.get("value") {
                    let var_value = match var_value {
                        Value::Bool(var_value) => var_value.to_string(),
                        Value::Number(var_value) => var_value.to_string(),
                        Value::String(var_value) => var_value.to_string(),
                        _ => unreachable!(),
                    };
                    let var_def = VarDefinition {
                        kind: VarDefinitionKind::Literal(var_value),
                        name: var_name.into(),
                    };
                    config.var_defs.push(var_def);
                }
            }
        }
        Ok(config)
    }
}

impl ConfigLoader for DefaultConfigLoader {
    fn load(&self, path: PathBuf) -> Result {
        debug!("Loading configuration from {}", path.display());
        let file = File::open(&path).map_err(Error::Io)?;
        let json: Value = serde_yaml::from_reader(file).map_err(Error::YamlSyntax)?;
        let json_version = json.get("version").ok_or(Error::Version(None))?;
        let version = json_version
            .as_str()
            .ok_or_else(|| Error::Version(Some(json_version.clone())))?;
        match version {
            "v1" => Self::load_v1(path, json),
            _ => Err(Error::Version(Some(json_version.clone()))),
        }
    }
}

#[cfg(test)]
pub struct StubConfigLoader;

#[cfg(test)]
impl ConfigLoader for StubConfigLoader {
    fn load(&self, _path: PathBuf) -> Result {
        unimplemented!();
    }
}

// TESTS

#[cfg(test)]
mod error_test {
    use super::*;

    mod to_string {
        use super::*;

        mod config {
            use super::*;

            #[test]
            fn should_return_str() {
                let str = "Invalid configuration";
                let err = Error::Config(vec![]);
                assert_eq!(err.to_string(), str);
            }
        }

        mod io {
            use super::*;

            #[test]
            fn should_return_str() {
                let err = ::std::io::Error::from(std::io::ErrorKind::PermissionDenied);
                let str = err.to_string();
                let err = Error::Io(err);
                assert_eq!(err.to_string(), str);
            }
        }

        mod version_missing {
            use super::*;

            #[test]
            fn should_return_str() {
                let str = "Missing configuration version";
                let err = Error::Version(None);
                assert_eq!(err.to_string(), str);
            }
        }

        mod version_invalid {
            use super::*;

            #[test]
            fn should_return_str() {
                let version = Value::from(1);
                let str = format!("{} is not a valid configuration version", version);
                let err = Error::Version(Some(version));
                assert_eq!(err.to_string(), str);
            }
        }

        mod yaml_syntax {
            use super::*;

            #[test]
            fn should_return_str() {
                let err = serde_yaml::from_str::<Value>("{").unwrap_err();
                let str = err.to_string();
                let err = Error::YamlSyntax(err);
                assert_eq!(err.to_string(), str);
            }
        }
    }
}

#[cfg(test)]
mod default_config_loader_test {
    use super::*;
    use std::path::Path;

    mod load {
        use super::*;

        #[test]
        fn should_return_io_err() {
            test(Path::new("notfound"), |res| match res.unwrap_err() {
                Error::Io(_) => {}
                err => panic!("{}", err),
            });
        }

        #[test]
        fn should_return_yaml_syntax_err() {
            test(Path::new("README.md"), |res| match res.unwrap_err() {
                Error::YamlSyntax(_) => {}
                err => panic!("{}", err),
            });
        }

        #[test]
        fn should_return_version_err_if_missing() {
            test(
                Path::new("resources/test/config/empty.yml"),
                |res| match res.unwrap_err() {
                    Error::Version(version) => assert!(version.is_none()),
                    err => panic!("{}", err),
                },
            );
        }

        #[test]
        fn should_return_version_err_if_invalid() {
            test(
                Path::new("resources/test/config/invalid-version.yml"),
                |res| match res.unwrap_err() {
                    Error::Version(version) => assert_eq!(version.unwrap(), Value::from(1)),
                    err => panic!("{}", err),
                },
            );
        }

        #[test]
        fn should_return_config_err() {
            test(
                Path::new("resources/test/config/invalid-v1.yml"),
                |res| match res.unwrap_err() {
                    Error::Config(errs) => assert_eq!(errs.len(), 3),
                    err => panic!("{}", err),
                },
            );
        }

        #[test]
        fn should_return_ok_if_v1() {
            let path = Path::new("resources/test/config/v1.yml");
            test(path, |res| {
                let cfg = Config {
                    path: path.to_path_buf(),
                    soft_defs: vec![SoftwareDefinition {
                        kind: SoftwareDefinitionKind::Terraform,
                        version: "1.2.3".into(),
                    }],
                    var_defs: vec![
                        VarDefinition {
                            kind: VarDefinitionKind::Literal("value".into()),
                            name: "VAR_STR".into(),
                        },
                        VarDefinition {
                            kind: VarDefinitionKind::Literal("1".into()),
                            name: "VAR_INT".into(),
                        },
                        VarDefinition {
                            kind: VarDefinitionKind::Literal("1.1".into()),
                            name: "VAR_NB".into(),
                        },
                        VarDefinition {
                            kind: VarDefinitionKind::Literal("true".into()),
                            name: "VAR_BOOL".into(),
                        },
                    ],
                };
                assert_eq!(res.unwrap(), cfg);
            });
        }

        #[inline]
        fn test<F: Fn(Result)>(path: &Path, assert_fn: F) {
            let loader = DefaultConfigLoader;
            let res = loader.load(path.to_path_buf());
            assert_fn(res);
        }
    }
}
