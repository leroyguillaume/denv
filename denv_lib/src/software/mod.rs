pub mod terraform;

use crate::{cfg::Config, error::*};
use std::fmt::{self, Debug, Display, Formatter};

pub trait Software: Debug {
    fn install(&self, cfg: &Config) -> Result<(), InstallError>;

    fn name(&self) -> &'static str;

    fn version(&self) -> &str;
}

impl Display for dyn Software {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} v{}", self.name(), self.version())
    }
}

impl PartialEq for dyn Software {
    fn eq(&self, software: &dyn Software) -> bool {
        self.name() == software.name() && self.version() == software.version()
    }
}

#[cfg(test)]
type InstallFn = dyn Fn(&Config) -> Result<(), InstallError>;

#[cfg(test)]
pub struct StubSoftware {
    name: &'static str,
    version: &'static str,
    install_fn: Option<Box<InstallFn>>,
}

#[cfg(test)]
impl StubSoftware {
    pub fn new(name: &'static str, version: &'static str) -> Self {
        Self {
            name,
            version,
            install_fn: None,
        }
    }

    pub fn with_install_fn<F: Fn(&Config) -> Result<(), InstallError> + 'static>(
        mut self,
        install_fn: F,
    ) -> Self {
        self.install_fn = Some(Box::new(install_fn));
        self
    }
}

#[cfg(test)]
impl Debug for StubSoftware {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StubSoftware")
            .field("name", &self.name)
            .field("version", &self.version)
            .finish()
    }
}

#[cfg(test)]
impl Software for StubSoftware {
    fn install(&self, cfg: &Config) -> Result<(), InstallError> {
        match &self.install_fn {
            Some(install_fn) => install_fn(cfg),
            None => unimplemented!(),
        }
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn version(&self) -> &str {
        self.version
    }
}

#[cfg(test)]
mod test {
    use super::*;

    mod software {
        use super::*;

        mod eq {
            use super::*;

            #[test]
            fn should_return_false() {
                let software1: Box<dyn Software> = Box::new(StubSoftware::new("stub", "1.2.3"));
                let software2: Box<dyn Software> = Box::new(StubSoftware::new("stub", "1.2.4"));
                assert!(software1 != software2);
            }

            #[test]
            fn should_return_true() {
                let software1: Box<dyn Software> = Box::new(StubSoftware::new("stub", "1.2.3"));
                let software2: Box<dyn Software> = Box::new(StubSoftware::new("stub", "1.2.3"));
                assert!(software1 == software2);
            }
        }

        mod to_string {
            use super::*;

            #[test]
            fn should_return_string() {
                let software: Box<dyn Software> = Box::new(StubSoftware::new("stub", "1.2.3"));
                let expected = format!("{} v{}", software.name(), software.version());
                assert_eq!(software.to_string(), expected);
            }
        }
    }
}
