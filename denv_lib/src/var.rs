#[derive(Debug, Eq, PartialEq)]
pub struct Var {
    name: String,
    value: String,
}

impl Var {
    pub fn new(name: String, value: String) -> Self {
        Self { name, value }
    }

    pub fn export_statement(&self) -> String {
        format!("export {}=\"{}\"", self.name, self.value)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

#[cfg(test)]
mod test {
    use super::*;

    mod var {
        use super::*;

        mod new {
            use super::*;

            #[test]
            fn should_return_var() {
                let expected = Var {
                    name: "PATH".into(),
                    value: "$PATH:/sbin".into(),
                };
                let var = Var::new(expected.name.clone(), expected.value.clone());
                assert_eq!(var, expected);
                assert_eq!(var.name(), expected.name);
                assert_eq!(var.value(), expected.value);
            }
        }

        mod export_statement {
            use super::*;

            #[test]
            fn should_return_string() {
                let var = Var::new("PATH".into(), "$PATH:/sbin".into());
                let expected = format!("export {}=\"{}\"", var.name, var.value);
                assert_eq!(var.export_statement(), expected);
            }
        }
    }
}
