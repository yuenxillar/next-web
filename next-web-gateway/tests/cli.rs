//! The command line contract of the binary.

mod support;

use std::process::Command;

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_next-web-gateway"))
}

fn write_config(name: &str, document: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "next-web-gateway-cli-{name}-{}.yaml",
        support::free_port()
    ));
    std::fs::write(&path, document).expect("write the test configuration");
    path
}

const VALID: &str = "\
next:
  gateway:
    routes:
      - id: users
        uri: \"http://127.0.0.1:8080\"
        predicates:
          - Path=/api/**
";

#[test]
fn check_accepts_a_valid_configuration() {
    let path = write_config("valid", VALID);

    let output = binary()
        .args(["--check", "--config"])
        .arg(&path)
        .output()
        .expect("run the binary");

    assert!(
        output.status.success(),
        "expected exit 0, stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("valid"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn check_exits_2_on_invalid_config_and_prints_the_path() {
    let broken = "\
next:
  gateway:
    routes:
      - id: users
        uri: \"http://127.0.0.1:8080\"
        filters:
          - StrpPrefix=1
";
    let path = write_config("invalid", broken);

    let output = binary()
        .args(["--check", "--config"])
        .arg(&path)
        .output()
        .expect("run the binary");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("StripPrefix"), "stderr was: {stderr}");

    let _ = std::fs::remove_file(path);
}

#[test]
fn missing_config_file_exits_2() {
    let output = binary()
        .args(["--check", "--config", "definitely-missing.yaml"])
        .output()
        .expect("run the binary");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("failed to read"));
}

#[test]
fn help_exits_0() {
    let output = binary().arg("--help").output().expect("run the binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--config"), "stdout was: {stdout}");
    assert!(stdout.contains("--check"), "stdout was: {stdout}");
}

#[test]
fn the_config_path_can_come_from_the_environment() {
    let path = write_config("env", VALID);

    let output = binary()
        .arg("--check")
        .env("NEXT_WEB_GATEWAY_CONFIG", &path)
        .output()
        .expect("run the binary");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = std::fs::remove_file(path);
}
