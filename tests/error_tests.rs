//! Error scenario integration tests

use std::process::Command;

fn smart_scribe_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_smart-scribe"))
}

#[test]
fn missing_api_key_error() {
    // Remove API key from environment and test with a short timeout
    // The app should fail fast with a clear error message about missing API key
    let output = smart_scribe_bin()
        .env_remove("GEMINI_API_KEY")
        .env("HOME", "/nonexistent") // Prevent reading config file
        .env("XDG_CONFIG_HOME", "/nonexistent")
        .output()
        .expect("Failed to execute command");

    // Note: This may hang if the API key check happens after recording starts
    // In our implementation, API key is checked early so it should fail fast
    // If this test hangs, it means the API key check timing needs to be revisited
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("API") || stderr.contains("api_key") || stderr.contains("key"),
        "Expected error about missing API key, got: {}",
        stderr
    );
}

#[test]
fn config_get_unknown_key() {
    let output = smart_scribe_bin()
        .args(["config", "get", "unknown_key"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Unknown") || stderr.contains("unknown") || stderr.contains("Valid"),
        "Expected error about unknown key, got: {}",
        stderr
    );
}

#[test]
fn config_set_unknown_key() {
    let output = smart_scribe_bin()
        .args(["config", "set", "unknown_key", "value"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Unknown") || stderr.contains("unknown") || stderr.contains("Valid"),
        "Expected error about unknown key, got: {}",
        stderr
    );
}

#[test]
fn config_set_invalid_duration() {
    let output = smart_scribe_bin()
        .args(["config", "set", "duration", "invalid"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid") || stderr.contains("invalid") || stderr.contains("duration"),
        "Expected error about invalid duration, got: {}",
        stderr
    );
}

#[test]
fn config_set_invalid_domain() {
    let output = smart_scribe_bin()
        .args(["config", "set", "domain", "invalid_domain"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid") || stderr.contains("invalid") || stderr.contains("domain"),
        "Expected error about invalid domain, got: {}",
        stderr
    );
}

#[test]
fn config_set_invalid_boolean() {
    let output = smart_scribe_bin()
        .args(["config", "set", "clipboard", "maybe"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("true") || stderr.contains("false") || stderr.contains("boolean"),
        "Expected error about invalid boolean, got: {}",
        stderr
    );
}

#[test]
fn config_list_with_no_file() {
    // Test that config list works even without a config file (uses empty config)
    let output = smart_scribe_bin()
        .args(["config", "list"])
        .env("HOME", "/nonexistent")
        .env("XDG_CONFIG_HOME", "/nonexistent")
        .output()
        .expect("Failed to execute command");

    // Should succeed with defaults shown as "(not set)"
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("not set") || stdout.contains("api_key"),
        "Expected config list output, got: {}",
        stdout
    );
}
