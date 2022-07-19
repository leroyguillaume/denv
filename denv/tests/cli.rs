use assert_cmd::prelude::*;
use std::process::Command;

const APP_NAME: &str = env!("CARGO_PKG_NAME");

#[test]
fn should_fail_if_cfg_file_does_not_exist() {
    let mut cmd = Command::cargo_bin(APP_NAME).unwrap();
    cmd.assert().failure().code(exitcode::CONFIG);
}
