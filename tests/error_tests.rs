//! Error scenario integration tests

use std::process::Command;

fn smart_scribe_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_smart-scribe"))
}

// Skipped on Windows: this test redirects the config dir via XDG_CONFIG_HOME
// and HOME, but on Windows `dirs::config_dir()` resolves %APPDATA% through
// SHGetKnownFolderPath and ignores env vars, so the binary loads the runner's
// real config instead of the tmp `auth = api_key` one. Linux + macOS coverage
// exercises the same code path.
#[cfg(not(windows))]
#[test]
fn missing_api_key_in_api_mode_errors_quickly() {
    // Force `auth = api_key` via a tmp config so the missing-key check fires
    // before any recording starts.
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg_dir = dir.path().join("smart-scribe");
    std::fs::create_dir_all(&cfg_dir).unwrap();
    std::fs::write(
        cfg_dir.join("config.toml"),
        "auth = \"api_key\"\nduration = \"1s\"\n",
    )
    .unwrap();

    let output = smart_scribe_bin()
        .env_remove("OPENAI_API_KEY")
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path())
        .args(["-d", "1s"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("OPENAI_API_KEY") || stderr.contains("openai_api_key"),
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
fn config_set_invalid_auth_mode() {
    let output = smart_scribe_bin()
        .args(["config", "set", "auth", "cookies"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid") || stderr.contains("auth") || stderr.contains("oauth"),
        "Expected error about invalid auth mode, got: {}",
        stderr
    );
}

#[test]
fn config_set_legacy_keys_rejected() {
    for key in ["api_key", "backend", "domain", "chatgpt_cookie_file"] {
        let output = smart_scribe_bin()
            .args(["config", "set", key, "x"])
            .output()
            .expect("Failed to execute command");
        assert!(
            !output.status.success(),
            "expected legacy key `{key}` to be rejected"
        );
    }
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
    let output = smart_scribe_bin()
        .args(["config", "list"])
        .env("HOME", "/nonexistent")
        .env("XDG_CONFIG_HOME", "/nonexistent")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("not set") || stdout.contains("auth"),
        "Expected config list output, got: {}",
        stdout
    );
}
