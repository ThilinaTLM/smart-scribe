use std::process::Command;

use serde_json::Value;

fn smart_scribe_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_smart-scribe"))
}

#[test]
fn config_path_supports_json_output() {
    let output = smart_scribe_bin()
        .args(["--output", "json", "config", "path"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(stdout.trim()).expect("stdout should be valid JSON");

    assert_eq!(json["ok"], true);
    assert_eq!(json["action"], "path");
    assert!(json["path"].as_str().unwrap().contains("smart-scribe"));
}

#[test]
fn config_list_supports_json_output() {
    let output = smart_scribe_bin()
        .args(["--output", "json", "config", "list"])
        .env("HOME", "/nonexistent")
        .env("XDG_CONFIG_HOME", "/nonexistent")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(stdout.trim()).expect("stdout should be valid JSON");

    assert_eq!(json["ok"], true);
    assert_eq!(json["action"], "list");
    assert!(json["values"].get("auth").is_some());
    assert!(json["values"].get("openai_api_key").is_some());
}
