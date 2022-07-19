use assert_cmd::prelude::*;
use std::process::Command;

const APP_NAME: &str = env!("CARGO_PKG_NAME");

#[test]
fn should_fail_if_cfg_file_does_not_exist() {
    let mut cmd = Command::cargo_bin(APP_NAME).unwrap();
    cmd.assert().failure().code(exitcode::CONFIG);
}

#[test]
fn should_fail_if_cfg_file_is_invalid_yaml() {
    let mut cmd = Command::cargo_bin(APP_NAME).unwrap();
    cmd.arg("-f")
        .arg("../denv_lib/resources/tests/config/invalid-yaml.yml");
    cmd.assert().failure().code(exitcode::CONFIG);
}

#[test]
fn should_fail_if_cfg_file_is_invalid() {
    let mut cmd = Command::cargo_bin(APP_NAME).unwrap();
    cmd.arg("-f")
        .arg("../denv_lib/resources/tests/config/invalid-config.yml");
    cmd.assert().failure().code(exitcode::CONFIG);
}
