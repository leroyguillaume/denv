// IMPORTS

use std::fmt::{self, Display, Formatter};
#[cfg(test)]
use stub_trait::stub;

// TYPES

pub type Result = std::result::Result<String, Error>;

// ENUMS

#[derive(Debug)]
pub enum Error {
    #[cfg(test)]
    Stub,
}

impl Display for Error {
    fn fmt(&self, _f: &mut Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

pub enum Kind<'a> {
    Literal(&'a Literal),
}

// TRAITS

#[cfg_attr(test, stub)]
pub trait Var {
    fn compute_value(&self) -> Result;

    fn kind(&self) -> Kind<'_>;

    fn name(&self) -> &str;
}

// STRUCTS

pub struct Literal {
    name: String,
    _value: String,
}

impl Literal {
    pub fn new(name: String, value: String) -> Self {
        Self {
            name,
            _value: value,
        }
    }

    #[cfg(test)]
    pub fn value(&self) -> &str {
        &self._value
    }
}

impl Var for Literal {
    fn compute_value(&self) -> Result {
        unimplemented!();
    }

    fn kind(&self) -> Kind {
        Kind::Literal(self)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// TESTS

#[cfg(test)]
mod literal_test {
    use super::*;

    mod new {
        use super::*;

        #[test]
        fn should_return_var() {
            let name = "var";
            let value = "value";
            let var = Literal::new(name.into(), value.into());
            assert_eq!(var.name(), name);
            assert_eq!(var.value(), value);
            match var.kind() {
                Kind::Literal(_) => {}
            }
        }
    }
}
