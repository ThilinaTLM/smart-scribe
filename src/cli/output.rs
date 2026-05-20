//! Structured CLI output types

use serde::{Deserialize, Serialize};

use crate::application::{DaemonOutput, TranscribeOutput};
use crate::domain::daemon::{DaemonState, StateUpdate};

/// Format a byte count as a short human-readable string
/// (e.g. `"500 B"`, `"2.0 KB"`, `"2.0 MB"`).
///
/// Lives in the CLI layer because the chosen format is a presentation concern
/// (different output formats / locales may render differently).
pub fn format_audio_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OneshotResponse {
    pub ok: bool,
    pub mode: &'static str,
    pub text: String,
    pub audio_size: String,
    pub clipboard_copied: bool,
    pub keystroke_sent: bool,
    pub paste_sent: bool,
}

impl From<TranscribeOutput> for OneshotResponse {
    fn from(output: TranscribeOutput) -> Self {
        Self {
            ok: true,
            mode: "oneshot",
            text: output.text,
            audio_size: format_audio_size(output.audio_size_bytes),
            clipboard_copied: output.clipboard_copied,
            keystroke_sent: output.keystroke_sent,
            paste_sent: output.paste_sent,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DaemonCommandAck {
    pub ok: bool,
    pub command: &'static str,
    pub accepted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DaemonStatusCommandResponse {
    pub ok: bool,
    pub command: &'static str,
    pub state: DaemonState,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatusPayload {
    pub state: DaemonState,
    pub elapsed_ms: u64,
}

impl DaemonStatusPayload {
    pub fn to_json_line(&self) -> String {
        format!("{}\n", serde_json::to_string(self).unwrap_or_default())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonEvent {
    State {
        state: DaemonState,
        elapsed_ms: u64,
    },
    #[serde(rename = "result")]
    Result {
        text: String,
        audio_size: String,
        clipboard_copied: bool,
        keystroke_sent: bool,
        paste_sent: bool,
    },
    Cancelled,
    Error {
        stage: String,
        message: String,
    },
    Shutdown,
}

impl DaemonEvent {
    pub fn state(state: DaemonState, elapsed_ms: u64) -> Self {
        Self::State { state, elapsed_ms }
    }

    pub fn error(stage: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Error {
            stage: stage.into(),
            message: message.into(),
        }
    }

    pub fn to_json_line(&self) -> String {
        format!("{}\n", serde_json::to_string(self).unwrap_or_default())
    }
}

impl From<StateUpdate> for DaemonEvent {
    fn from(update: StateUpdate) -> Self {
        Self::state(update.state, update.elapsed_ms)
    }
}

impl From<DaemonOutput> for DaemonEvent {
    fn from(output: DaemonOutput) -> Self {
        Self::Result {
            text: output.text,
            audio_size: format_audio_size(output.audio_size_bytes),
            clipboard_copied: output.clipboard_copied,
            keystroke_sent: output.keystroke_sent,
            paste_sent: output.paste_sent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_audio_size_thresholds() {
        assert_eq!(format_audio_size(500), "500 B");
        assert_eq!(format_audio_size(2048), "2.0 KB");
        assert_eq!(format_audio_size(2 * 1024 * 1024), "2.0 MB");
    }

    #[test]
    fn oneshot_response_includes_transcript() {
        let response = OneshotResponse::from(TranscribeOutput {
            text: "hello".to_string(),
            clipboard_copied: true,
            keystroke_sent: false,
            paste_sent: false,
            audio_size_bytes: 10 * 1024,
        });

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"text\":\"hello\""));
        assert!(json.contains("\"mode\":\"oneshot\""));
    }

    #[test]
    fn daemon_event_result_serializes_transcript() {
        let event = DaemonEvent::from(DaemonOutput {
            text: "transcript".to_string(),
            clipboard_copied: false,
            keystroke_sent: true,
            paste_sent: false,
            audio_size_bytes: 42 * 1024,
        });

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"result\""));
        assert!(json.contains("\"text\":\"transcript\""));
    }

    #[test]
    fn daemon_status_payload_serializes_elapsed() {
        let payload = DaemonStatusPayload {
            state: DaemonState::Recording,
            elapsed_ms: 1234,
        };

        let json = payload.to_json_line();
        assert!(json.contains("\"state\":\"recording\""));
        assert!(json.contains("\"elapsed_ms\":1234"));
    }
}
