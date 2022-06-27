use home::home_dir;
use std::path::PathBuf;

#[derive(Debug)]
pub struct HomeNotFoundError;

#[derive(Debug, Eq, PartialEq)]
pub struct Config {
    denv_dirpath: PathBuf,
}

impl Config {
    pub fn new() -> Result<Self, HomeNotFoundError> {
        Ok(Self {
            denv_dirpath: home_dir()
                .map(|path| path.join(".denv"))
                .ok_or(HomeNotFoundError)?,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    mod config {
        use super::*;

        mod new {
            use super::*;

            #[test]
            fn should_return_config() {
                let expected = Config {
                    denv_dirpath: home_dir().map(|path| path.join(".denv")).unwrap(),
                };
                let cfg = Config::new().unwrap();
                assert_eq!(cfg, expected);
            }
        }
    }
}
