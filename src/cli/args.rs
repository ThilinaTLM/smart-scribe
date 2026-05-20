//! CLI argument definitions using Clap

use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

use crate::domain::recording::Duration;

/// SmartScribe - AI-powered voice to text transcription
#[derive(Parser, Debug)]
#[command(name = "smart-scribe")]
#[command(version)]
#[command(about = "AI-powered voice to text transcription")]
#[command(long_about = None)]
pub struct Cli {
    /// Output format (text for humans, json for machine-readable output)
    #[arg(
        long,
        value_enum,
        value_name = "FORMAT",
        global = true,
        default_value = "text"
    )]
    pub output: OutputFormatArg,

    /// Fixed recording duration (e.g., 10s, 1m, 2m30s). If omitted, recording runs until Ctrl+C.
    #[arg(short = 'd', long, value_name = "TIME", conflicts_with = "daemon")]
    pub duration: Option<String>,

    /// Copy transcription to clipboard
    #[arg(short = 'c', long)]
    pub clipboard: bool,

    /// Type transcription into focused window
    #[arg(short = 'k', long)]
    pub keystroke: bool,

    /// Keystroke tool to use (Linux: enigo, auto, ydotool, xdotool, wtype)
    #[arg(long, value_name = "TOOL")]
    pub keystroke_tool: Option<String>,

    /// Show desktop notifications
    #[arg(short = 'n', long)]
    pub notify: bool,

    /// Smart paste: capture window, transcribe, paste via clipboard (Linux/KDE Wayland)
    #[cfg(target_os = "linux")]
    #[arg(short = 'p', long, conflicts_with_all = ["clipboard", "keystroke"])]
    pub paste: bool,

    /// Play audio cues on recording events
    #[arg(short = 'a', long)]
    pub audio_cue: bool,

    /// Run as daemon (control via: smart-scribe daemon toggle/cancel/status)
    #[arg(long)]
    pub daemon: bool,

    /// Optional safety limit for dynamic recording and daemon mode
    #[arg(long, value_name = "TIME", conflicts_with = "duration")]
    pub max_duration: Option<String>,

    /// Show recording indicator (daemon mode only; Wayland overlay on Linux, system tray on Windows)
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[arg(long, requires = "daemon")]
    pub indicator: bool,

    /// Indicator position on screen (daemon mode only, Linux/Wayland)
    #[cfg(target_os = "linux")]
    #[arg(long, value_name = "POSITION", requires = "indicator")]
    pub indicator_position: Option<IndicatorPosition>,

    /// Config subcommand
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Subcommands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Send commands to running daemon
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },
    /// Authenticate via ChatGPT OAuth (opens a browser)
    Login {
        /// Import the refresh token from an existing Codex CLI install
        #[arg(long)]
        from_codex: bool,
    },
    /// Forget the cached OAuth token
    Logout,
    /// Show current authentication status
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },
}

/// Daemon control actions
#[derive(Subcommand, Debug, Clone, Copy)]
pub enum DaemonAction {
    /// Toggle recording (start if idle, stop if recording)
    Toggle,
    /// Cancel current recording without transcribing
    Cancel,
    /// Show daemon status
    Status,
    /// Subscribe to daemon events (JSON output only)
    Subscribe,
}

/// Auth subcommands
#[derive(Subcommand, Debug, Clone, Copy)]
pub enum AuthAction {
    /// Print current authentication state
    Status,
}

/// Config action subcommands
#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Create config file with defaults
    Init,
    /// Set a config value
    Set {
        /// Config key
        key: String,
        /// Config value
        value: String,
    },
    /// Get a config value
    Get {
        /// Config key
        key: String,
    },
    /// List all config values
    List,
    /// Show config file path
    Path,
}

/// Output format argument for clap ValueEnum
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormatArg {
    Text,
    Json,
}

impl OutputFormatArg {
    pub const fn is_json(self) -> bool {
        matches!(self, Self::Json)
    }
}

/// Indicator position on screen (Linux only)
#[cfg(target_os = "linux")]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum IndicatorPosition {
    #[value(name = "top-right")]
    TopRight,
    #[value(name = "top-left")]
    TopLeft,
    #[value(name = "top-center")]
    TopCenter,
    #[default]
    #[value(name = "bottom-center")]
    BottomCenter,
    #[value(name = "bottom-right")]
    BottomRight,
    #[value(name = "bottom-left")]
    BottomLeft,
}

#[cfg(target_os = "linux")]
impl std::str::FromStr for IndicatorPosition {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "top-right" => Ok(IndicatorPosition::TopRight),
            "top-left" => Ok(IndicatorPosition::TopLeft),
            "top-center" => Ok(IndicatorPosition::TopCenter),
            "bottom-center" => Ok(IndicatorPosition::BottomCenter),
            "bottom-right" => Ok(IndicatorPosition::BottomRight),
            "bottom-left" => Ok(IndicatorPosition::BottomLeft),
            _ => Err(format!("Invalid indicator position: {}", s)),
        }
    }
}

/// Parsed transcribe options (oneshot mode).
///
/// `paste` is always present in the struct (Linux-only feature; ignored on
/// other platforms) to keep this surface portable and to avoid
/// `#[cfg(target_os = ...)]` spreading into callers. The CLI flag is still
/// Linux-gated; non-Linux builds always set `paste = false`.
#[derive(Debug, Clone)]
pub struct TranscribeOptions {
    pub output: OutputFormatArg,
    pub duration: Option<Duration>,
    pub max_duration: Option<Duration>,
    pub clipboard: bool,
    pub keystroke: bool,
    pub keystroke_tool: Option<String>,
    pub paste: bool,
    pub notify: bool,
    pub audio_cue: bool,
}

/// Parsed daemon options. Same portability rationale as
/// [`TranscribeOptions`].
#[derive(Debug, Clone)]
pub struct DaemonOptions {
    pub output: OutputFormatArg,
    pub max_duration: Duration,
    pub clipboard: bool,
    pub keystroke: bool,
    pub keystroke_tool: Option<String>,
    pub paste: bool,
    pub notify: bool,
    pub audio_cue: bool,
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    pub indicator: bool,
    #[cfg(target_os = "linux")]
    pub indicator_position: IndicatorPosition,
}

// Configuration-key validation lives in [`super::config_schema`]; the CLI
// parser only needs to recognise free-form key strings here and delegate to
// the schema at run time.

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_parses_defaults() {
        let cli = Cli::parse_from(["smart-scribe"]);
        assert_eq!(cli.output, OutputFormatArg::Text);
        assert!(cli.duration.is_none());
        assert!(!cli.clipboard);
        assert!(!cli.keystroke);
        assert!(cli.keystroke_tool.is_none());
        assert!(!cli.notify);
        assert!(!cli.audio_cue);
        assert!(!cli.daemon);
    }

    #[test]
    fn cli_parses_duration() {
        let cli = Cli::parse_from(["smart-scribe", "-d", "30s"]);
        assert_eq!(cli.duration, Some("30s".to_string()));
    }

    #[test]
    fn cli_parses_flags() {
        let cli = Cli::parse_from(["smart-scribe", "-c", "-k", "-n", "-a"]);
        assert!(cli.clipboard);
        assert!(cli.keystroke);
        assert!(cli.notify);
        assert!(cli.audio_cue);
    }

    #[test]
    fn cli_parses_daemon() {
        let cli = Cli::parse_from(["smart-scribe", "--daemon"]);
        assert!(cli.daemon);
    }

    #[test]
    fn cli_parses_output_json() {
        let cli = Cli::parse_from(["smart-scribe", "--output", "json"]);
        assert_eq!(cli.output, OutputFormatArg::Json);
    }

    #[test]
    fn cli_parses_daemon_with_max_duration() {
        let cli = Cli::parse_from(["smart-scribe", "--daemon", "--max-duration", "5m"]);
        assert!(cli.daemon);
        assert_eq!(cli.max_duration, Some("5m".to_string()));
    }

    #[test]
    fn cli_parses_standalone_max_duration() {
        let cli = Cli::parse_from(["smart-scribe", "--max-duration", "5m"]);
        assert!(!cli.daemon);
        assert_eq!(cli.max_duration, Some("5m".to_string()));
    }

    #[test]
    fn cli_rejects_duration_and_max_duration_together() {
        assert!(
            Cli::try_parse_from(["smart-scribe", "--duration", "10s", "--max-duration", "5m"])
                .is_err()
        );
    }

    #[test]
    fn cli_parses_config_init() {
        let cli = Cli::parse_from(["smart-scribe", "config", "init"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Config {
                action: ConfigAction::Init
            })
        ));
    }

    #[test]
    fn cli_parses_config_set() {
        let cli = Cli::parse_from(["smart-scribe", "config", "set", "auth", "api_key"]);
        if let Some(Commands::Config {
            action: ConfigAction::Set { key, value },
        }) = cli.command
        {
            assert_eq!(key, "auth");
            assert_eq!(value, "api_key");
        } else {
            panic!("Expected Config Set command");
        }
    }

    #[test]
    fn cli_parses_login() {
        let cli = Cli::parse_from(["smart-scribe", "login"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Login { from_codex: false })
        ));

        let cli = Cli::parse_from(["smart-scribe", "login", "--from-codex"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Login { from_codex: true })
        ));
    }

    #[test]
    fn cli_parses_logout() {
        let cli = Cli::parse_from(["smart-scribe", "logout"]);
        assert!(matches!(cli.command, Some(Commands::Logout)));
    }

    #[test]
    fn cli_parses_auth_status() {
        let cli = Cli::parse_from(["smart-scribe", "auth", "status"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Auth {
                action: AuthAction::Status
            })
        ));
    }

    #[test]
    fn valid_config_keys() {
        use crate::cli::config_schema;
        // Portable schema: every documented key is valid on every platform.
        // Runtime gating still applies the appropriate feature per OS.
        assert!(config_schema::find("auth").is_some());
        assert!(config_schema::find("openai_api_key").is_some());
        assert!(config_schema::find("openai_transcribe_model").is_some());
        assert!(config_schema::find("duration").is_some());
        assert!(config_schema::find("linux.keystroke_tool").is_some());
        assert!(config_schema::find("linux.indicator").is_some());
        assert!(config_schema::find("linux.indicator_position").is_some());
        assert!(config_schema::find("linux.paste").is_some());
        assert!(config_schema::find("windows.indicator").is_some());
        assert!(config_schema::find("windows.show_balloon").is_some());
        // Legacy keys are unknown.
        assert!(config_schema::find("api_key").is_none());
        assert!(config_schema::find("backend").is_none());
        assert!(config_schema::find("domain").is_none());
        assert!(config_schema::find("chatgpt_cookie_file").is_none());
        assert!(config_schema::find("invalid_key").is_none());
    }

    #[test]
    fn cli_parses_keystroke_tool() {
        let cli = Cli::parse_from(["smart-scribe", "-k", "--keystroke-tool", "xdotool"]);
        assert!(cli.keystroke);
        assert_eq!(cli.keystroke_tool, Some("xdotool".to_string()));
    }

    #[test]
    fn cli_parses_daemon_subscribe() {
        let cli = Cli::parse_from(["smart-scribe", "daemon", "subscribe", "--output", "json"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Daemon {
                action: DaemonAction::Subscribe
            })
        ));
        assert_eq!(cli.output, OutputFormatArg::Json);
    }

    #[test]
    fn verify_cli() {
        // Verify the CLI definition is valid
        Cli::command().debug_assert();
    }
}
