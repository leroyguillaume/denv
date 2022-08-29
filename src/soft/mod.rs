// IMPORTS

use crate::fs::FileSystem;
use k8s::ChartTesting;
use std::{
    fmt::{self, Display, Formatter},
    io,
    path::Path,
};
#[cfg(test)]
use stub_trait::stub;
use tf::Terraform;

// MODS

pub mod k8s;
pub mod tf;

mod installer;

// TYPES

pub type Result<T> = std::result::Result<T, Error>;

// ENUMS

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    UnsupportedSystem,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "{}", err),
            Self::UnsupportedSystem => write!(f, "This system is not supported"),
        }
    }
}

pub enum Kind<'a> {
    ChartTesting(&'a ChartTesting),
    Terraform(&'a Terraform),
}

// TRAITS

#[cfg_attr(test, stub)]
pub trait Software {
    fn install(&self, project_dirpath: &Path, fs: &dyn FileSystem) -> Result<()>;

    fn kind(&self) -> Kind<'_>;

    fn name(&self) -> &str;

    fn version(&self) -> &str;
}

// TESTS

#[cfg(test)]
mod error_test {
    use super::*;

    mod to_string {
        use super::*;

        mod unsupported_system {
            use super::*;

            #[test]
            fn should_return_str() {
                let str = "This system is not supported";
                let err = Error::UnsupportedSystem;
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
    }
}
