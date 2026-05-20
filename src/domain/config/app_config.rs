//! Validated application configuration (runtime value object).
//!
//! Concrete, validated values; **no Options for things that have defaults**.
//! Constructed from the on-disk [`RawAppConfig`](super::RawAppConfig) via
//! `TryFrom`, which is the single place where parsing/validation lives.

use std::fmt;
use std::str::FromStr;

use crate::domain::error::ConfigError;
use crate::domain::recording::Duration;

use super::platform::PlatformConfig;
use super::raw::RawAppConfig;

/// Default transcription model.
///
/// `gpt-4o-transcribe` is OpenAI's highest-accuracy speech-to-text model
/// (lower word error rate than `whisper-1` and `gpt-4o-mini-transcribe`).
/// Both auth paths accept it; OAuth users pay nothing extra, API-key users
/// pay the same per-minute rate as `whisper-1`.
pub const DEFAULT_OPENAI_TRANSCRIBE_MODEL: &str = "gpt-4o-transcribe";

/// Auth mode selecting which transcription backend to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AuthMode {
    /// ChatGPT subscription via the Codex OAuth client.
    #[default]
    Oauth,
    /// OpenAI API key against `api.openai.com/v1/audio/transcriptions`.
    ApiKey,
}

impl AuthMode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Oauth => "oauth",
            Self::ApiKey => "api_key",
        }
    }
}

impl fmt::Display for AuthMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AuthMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "oauth" | "chatgpt" => Ok(Self::Oauth),
            "api_key" | "api-key" | "apikey" | "openai" => Ok(Self::ApiKey),
            other => Err(format!(
                "Invalid auth mode '{other}'. Valid options: oauth, api_key"
            )),
        }
    }
}

/// Validated, runtime application configuration.
///
/// Constructed via [`AppConfig::try_from`] from a [`RawAppConfig`]. All
/// fields are concrete (no Options for items that have static defaults);
/// only fields the user can leave unset (api key, prompt, language hint,
/// custom durations) stay Option-typed.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub auth: AuthMode,
    pub openai_api_key: Option<String>,
    pub openai_transcribe_model: String,
    pub transcribe_prompt: Option<String>,
    pub transcribe_language: Option<String>,
    /// User-supplied one-shot recording duration, if any.
    pub duration: Option<Duration>,
    /// User-supplied maximum duration / daemon safety limit, if any.
    pub max_duration: Option<Duration>,
    pub clipboard: bool,
    pub keystroke: bool,
    pub notify: bool,
    pub audio_cue: bool,
    pub platform: PlatformConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            auth: AuthMode::default(),
            openai_api_key: None,
            openai_transcribe_model: DEFAULT_OPENAI_TRANSCRIBE_MODEL.to_string(),
            transcribe_prompt: None,
            transcribe_language: None,
            duration: None,
            max_duration: None,
            clipboard: false,
            keystroke: false,
            notify: false,
            audio_cue: false,
            platform: PlatformConfig::defaults(),
        }
    }
}

impl AppConfig {
    /// Return the optional transcribe prompt with empty/whitespace strings
    /// treated as unset.
    pub fn transcribe_prompt_some(&self) -> Option<&str> {
        self.transcribe_prompt
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
    }

    /// Return the optional language hint with empty/whitespace strings
    /// treated as unset.
    pub fn transcribe_language_some(&self) -> Option<&str> {
        self.transcribe_language
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
    }
}

impl TryFrom<RawAppConfig> for AppConfig {
    type Error = ConfigError;

    fn try_from(raw: RawAppConfig) -> Result<Self, Self::Error> {
        // --- auth --------------------------------------------------------
        let auth = match raw.auth.as_deref() {
            None | Some("") => AuthMode::default(),
            Some(s) => s
                .parse()
                .map_err(|msg: String| ConfigError::ValidationError {
                    key: "auth".to_string(),
                    message: msg,
                })?,
        };

        // --- durations ---------------------------------------------------
        let duration = parse_duration(raw.duration.as_deref(), "duration")?;
        let max_duration = parse_duration(raw.max_duration.as_deref(), "max_duration")?;

        // --- model -------------------------------------------------------
        let openai_transcribe_model = raw
            .openai_transcribe_model
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_OPENAI_TRANSCRIBE_MODEL.to_string());

        // --- platform sub-config (flat shape) ----------------------------
        let defaults = PlatformConfig::defaults();
        let linux = raw.linux.unwrap_or_default();
        let windows = raw.windows.unwrap_or_default();
        let platform = PlatformConfig {
            keystroke_tool: linux.keystroke_tool.unwrap_or(defaults.keystroke_tool),
            // `indicator` is read from whichever platform table is present;
            // both platforms use the same flag so the merge is just `or`.
            indicator: linux.indicator.or(windows.indicator).unwrap_or(false),
            indicator_position: linux
                .indicator_position
                .unwrap_or(defaults.indicator_position),
            linux_paste: linux.paste.unwrap_or(false),
            windows_show_balloon: windows.show_balloon.unwrap_or(false),
        };

        Ok(Self {
            auth,
            openai_api_key: raw.openai_api_key.filter(|s| !s.is_empty()),
            openai_transcribe_model,
            transcribe_prompt: raw.transcribe_prompt,
            transcribe_language: raw.transcribe_language,
            duration,
            max_duration,
            clipboard: raw.clipboard.unwrap_or(false),
            keystroke: raw.keystroke.unwrap_or(false),
            notify: raw.notify.unwrap_or(false),
            audio_cue: raw.audio_cue.unwrap_or(false),
            platform,
        })
    }
}

fn parse_duration(input: Option<&str>, key: &str) -> Result<Option<Duration>, ConfigError> {
    match input {
        None => Ok(None),
        Some(s) if s.trim().is_empty() => Ok(None),
        Some(s) => s
            .parse::<Duration>()
            .map(Some)
            .map_err(|e| ConfigError::ValidationError {
                key: key.to_string(),
                message: e.to_string(),
            }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_have_expected_values() {
        let config = AppConfig::default();
        assert_eq!(config.auth, AuthMode::Oauth);
        assert_eq!(
            config.openai_transcribe_model,
            DEFAULT_OPENAI_TRANSCRIBE_MODEL
        );
        assert!(config.openai_api_key.is_none());
        assert!(!config.clipboard);
        assert_eq!(config.platform.keystroke_tool, "enigo");
    }

    #[test]
    fn from_raw_defaults() {
        let config = AppConfig::try_from(RawAppConfig::defaults()).unwrap();
        assert_eq!(config.auth, AuthMode::Oauth);
        assert_eq!(
            config.openai_transcribe_model,
            DEFAULT_OPENAI_TRANSCRIBE_MODEL
        );
        assert!(!config.clipboard);
        assert_eq!(config.platform.keystroke_tool, "enigo");
        assert_eq!(config.platform.indicator_position, "top-right");
    }

    #[test]
    fn from_raw_rejects_invalid_auth() {
        let raw = RawAppConfig {
            auth: Some("nonsense".into()),
            ..Default::default()
        };
        let err = AppConfig::try_from(raw).unwrap_err();
        match err {
            ConfigError::ValidationError { key, .. } => assert_eq!(key, "auth"),
            other => panic!("expected ValidationError, got {other:?}"),
        }
    }

    #[test]
    fn from_raw_rejects_invalid_duration() {
        let raw = RawAppConfig {
            duration: Some("garbage".into()),
            ..Default::default()
        };
        let err = AppConfig::try_from(raw).unwrap_err();
        match err {
            ConfigError::ValidationError { key, .. } => assert_eq!(key, "duration"),
            other => panic!("expected ValidationError, got {other:?}"),
        }
    }

    #[test]
    fn from_raw_parses_duration() {
        let raw = RawAppConfig {
            duration: Some("30s".into()),
            ..Default::default()
        };
        let config = AppConfig::try_from(raw).unwrap();
        assert_eq!(config.duration.unwrap().as_secs(), 30);
    }

    #[test]
    fn from_raw_treats_empty_duration_as_unset() {
        let raw = RawAppConfig {
            duration: Some("".into()),
            ..Default::default()
        };
        let config = AppConfig::try_from(raw).unwrap();
        assert!(config.duration.is_none());
    }

    #[test]
    fn from_raw_treats_empty_api_key_as_unset() {
        let raw = RawAppConfig {
            openai_api_key: Some("".into()),
            ..Default::default()
        };
        let config = AppConfig::try_from(raw).unwrap();
        assert!(config.openai_api_key.is_none());
    }

    #[test]
    fn auth_mode_parses() {
        assert_eq!(AuthMode::from_str("oauth"), Ok(AuthMode::Oauth));
        assert_eq!(AuthMode::from_str("OAuth"), Ok(AuthMode::Oauth));
        assert_eq!(AuthMode::from_str("api_key"), Ok(AuthMode::ApiKey));
        assert_eq!(AuthMode::from_str("API-Key"), Ok(AuthMode::ApiKey));
        assert!(AuthMode::from_str("nope").is_err());
    }
}
