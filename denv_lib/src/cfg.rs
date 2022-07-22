#[cfg(test)]
use crate::internal::fs::StubFileSystem;
use crate::{
    error::*,
    internal::{
        downloader::*,
        fs::{DefaultFileSystem, FileSystem},
        unzip::*,
    },
    software::terraform::*,
    software::*,
};
use jsonschema::JSONSchema;
use log::debug;
use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
};

pub struct Config {
    softwares: Vec<Box<dyn Software>>,
    pub(crate) fs: Box<dyn FileSystem>,
    pub(crate) downloader: Box<dyn Downloader>,
    pub(crate) unzipper: Box<dyn Unzipper>,
}

impl Config {
    pub fn load(
        filepath: &Path,
        denv_dirpath: PathBuf,
        tmp_dirpath: PathBuf,
    ) -> Result<Self, ConfigLoadError> {
        debug!("Loading configuration from {}", filepath.display());
        let cfg = read_to_string(filepath).map_err(ConfigLoadError::FileOpeningFailed)?;
        let cfg = serde_yaml::from_str::<serde_json::Value>(&cfg)
            .map_err(ConfigLoadError::InvalidYaml)?;
        let schema = include_str!("../config.schema.json");
        let schema = serde_json::from_str(schema).unwrap();
        let schema = JSONSchema::compile(&schema).unwrap();
        if let Err(err_iter) = schema.validate(&cfg) {
            let errs = err_iter.map(|err| err.to_string()).collect();
            return Err(ConfigLoadError::InvalidConfig(errs));
        }
        let mut softwares: Vec<Box<dyn Software>> = vec![];
        if let Some(cfg_softwares) = cfg.get("softwares") {
            if let Some(cfg_softwares) = cfg_softwares.get("terraform") {
                softwares.push(Box::new(Terraform(cfg_softwares.as_str().unwrap().into())));
            }
        }
        let cfg = Self {
            softwares,
            fs: Box::new(DefaultFileSystem::new(denv_dirpath, tmp_dirpath)),
            downloader: Box::new(DefaultDownloader),
            unzipper: Box::new(DefaultUnzipper),
        };
        Ok(cfg)
    }

    pub fn softwares(&self) -> &[Box<dyn Software>] {
        &self.softwares
    }
}

#[cfg(test)]
impl Config {
    pub fn stub() -> Self {
        Self {
            softwares: vec![],
            downloader: Box::new(StubDownloader::new()),
            fs: Box::new(StubFileSystem::new()),
            unzipper: Box::new(StubUnzipper::new()),
        }
    }

    pub fn with_softwares(mut self, softwares: Vec<Box<dyn Software>>) -> Self {
        self.softwares = softwares;
        self
    }

    pub fn with_downloader(mut self, downloader: StubDownloader) -> Self {
        self.downloader = Box::new(downloader);
        self
    }

    pub fn with_fs(mut self, fs: StubFileSystem) -> Self {
        self.fs = Box::new(fs);
        self
    }

    pub fn with_unzipper(mut self, unzipper: StubUnzipper) -> Self {
        self.unzipper = Box::new(unzipper);
        self
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{fs::File, io::Write};
    use tempfile::tempdir;

    mod config {
        use super::*;

        mod load {
            use super::*;

            #[test]
            fn should_return_file_opening_failed() {
                match Config::load(
                    Path::new("denv.yaml"),
                    PathBuf::from(".denv"),
                    PathBuf::from("/tmp/denv"),
                ) {
                    Ok(_) => panic!("should fail"),
                    Err(ConfigLoadError::FileOpeningFailed(_)) => {}
                    Err(err) => panic!("{}", err),
                }
            }

            #[test]
            fn should_return_invalid_yaml_err() {
                let dirpath = tempdir().unwrap().into_path();
                let filepath = dirpath.join("denv.yml");
                let mut file = File::create(&filepath).unwrap();
                write!(file, "{{").unwrap();
                match Config::load(
                    Path::new(&filepath),
                    PathBuf::from(".denv"),
                    PathBuf::from("/tmp/denv"),
                ) {
                    Ok(_) => panic!("should fail"),
                    Err(ConfigLoadError::InvalidYaml(_)) => {}
                    Err(err) => panic!("{}", err),
                }
            }

            #[test]
            fn should_return_invalid_config_err() {
                let dirpath = tempdir().unwrap().into_path();
                let filepath = dirpath.join("denv.yml");
                let mut file = File::create(&filepath).unwrap();
                write!(file, "softwares: terraform").unwrap();
                match Config::load(
                    Path::new(&filepath),
                    PathBuf::from(".denv"),
                    PathBuf::from("/tmp/denv"),
                ) {
                    Ok(_) => panic!("should fail"),
                    Err(ConfigLoadError::InvalidConfig(_)) => {}
                    Err(err) => panic!("{}", err),
                }
            }

            #[test]
            fn should_return_config() {
                let softwares: Vec<Box<dyn Software>> = vec![Box::new(Terraform("1.2.3".into()))];
                let denv_dirpath = PathBuf::from(".denv");
                let tmp_dirpath = PathBuf::from("/tmp/denv");
                let cfg = Config::load(
                    Path::new("../examples/denv.yml"),
                    denv_dirpath.clone(),
                    tmp_dirpath.clone(),
                )
                .unwrap();
                assert_eq!(cfg.softwares(), softwares);
                assert_eq!(cfg.fs.denv_dirpath(), denv_dirpath);
                assert_eq!(cfg.fs.tmp_dirpath(), tmp_dirpath);
            }
        }

        mod softwares {
            use super::*;

            #[test]
            fn should_return_softwares() {
                let mut cfg = Config::stub();
                let software1 = StubSoftware::new("stub", "1.2.3");
                let software2 = StubSoftware::new("stub", "1.2.4");
                cfg.softwares = vec![Box::new(software1), Box::new(software2)];
                assert_eq!(cfg.softwares(), cfg.softwares);
            }
        }
    }
}
