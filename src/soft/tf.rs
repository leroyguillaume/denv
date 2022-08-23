// IMPORTS

use super::*;

// CONSTS

const TF_SOFT_NAME: &str = "terraform";

// STRUCTS

pub struct Terraform {
    version: String,
}

impl Terraform {
    pub fn new(version: String) -> Self {
        Self { version }
    }
}

impl Software for Terraform {
    fn binary_paths(&self, _fs: &dyn FileSystem) -> Vec<PathBuf> {
        unimplemented!();
    }

    fn install(&self, _fs: &dyn FileSystem) -> Result {
        unimplemented!();
    }

    fn is_installed(&self, _fs: &dyn FileSystem) -> bool {
        unimplemented!();
    }

    fn kind(&self) -> Kind {
        Kind::Terraform(self)
    }

    fn name(&self) -> &str {
        TF_SOFT_NAME
    }

    fn version(&self) -> &str {
        &self.version
    }
}

// TESTS

#[cfg(test)]
mod terraform_test {
    use super::*;

    mod new {
        use super::*;

        #[test]
        fn should_return_soft() {
            let version = "1.2.3";
            let soft = Terraform::new(version.into());
            assert_eq!(soft.name(), TF_SOFT_NAME);
            assert_eq!(soft.version(), version);
            match soft.kind() {
                Kind::Terraform(_) => {}
            }
        }
    }
}
