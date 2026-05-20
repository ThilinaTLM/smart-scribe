//! CLI integration tests

use std::process::Command;

fn smart_scribe_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_smart-scribe"))
}

#[test]
fn help_output() {
    let output = smart_scribe_bin()
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("transcription"));
    assert!(stdout.contains("--output"));
    assert!(stdout.contains("--duration"));
    assert!(stdout.contains("--daemon"));
    assert!(stdout.contains("--clipboard"));
    assert!(stdout.contains("--keystroke"));
    assert!(stdout.contains("--notify"));
    assert!(stdout.contains("login"));
    assert!(stdout.contains("logout"));
    assert!(stdout.contains("auth"));
}

#[test]
fn version_output() {
    let output = smart_scribe_bin()
        .arg("--version")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("smart-scribe"));
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn config_path_command() {
    let output = smart_scribe_bin()
        .args(["config", "path"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("smart-scribe"));
    assert!(stdout.contains("config.toml"));
}

#[test]
fn daemon_help() {
    let output = smart_scribe_bin()
        .args(["daemon", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("toggle"));
    assert!(stdout.contains("cancel"));
    assert!(stdout.contains("status"));
    assert!(stdout.contains("subscribe"));
}

#[test]
fn config_help() {
    let output = smart_scribe_bin()
        .args(["config", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("init"));
    assert!(stdout.contains("set"));
    assert!(stdout.contains("get"));
    assert!(stdout.contains("list"));
    assert!(stdout.contains("path"));
}

#[test]
fn auth_status_runs() {
    let output = smart_scribe_bin()
        .args(["auth", "status"])
        .env("HOME", "/nonexistent")
        .env("XDG_CONFIG_HOME", "/nonexistent")
        .env_remove("OPENAI_API_KEY")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("auth") || stdout.contains("oauth"),
        "expected auth status output, got: {stdout}"
    );
}

#[test]
fn invalid_duration_error() {
    let output = smart_scribe_bin()
        .args(["--duration", "invalid"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid duration") || stderr.contains("invalid"),
        "Expected error about invalid duration, got: {}",
        stderr
    );
}

#[test]
fn daemon_duration_conflict() {
    let output = smart_scribe_bin()
        .args(["--daemon", "--duration", "30s"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot be used with") || stderr.contains("conflict"),
        "Expected conflict error, got: {}",
        stderr
    );
}

#[test]
fn rejects_legacy_domain_flag() {
    let output = smart_scribe_bin()
        .args(["--domain", "dev"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unexpected argument") || stderr.contains("--domain"),
        "Expected error about removed --domain flag, got: {}",
        stderr
    );
}

#[test]
fn rejects_legacy_backend_flag() {
    let output = smart_scribe_bin()
        .args(["--backend", "chatgpt"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
}
