//! CLI argument definitions using Clap

use clap::{Parser, Subcommand, ValueEnum};

use crate::domain::recording::Duration;
use crate::domain::transcription::DomainId;

/// SmartScribe - AI-powered voice to text transcription
#[derive(Parser, Debug)]
#[command(name = "smart-scribe")]
#[command(version)]
#[command(about = "AI-powered voice to text transcription using Google Gemini")]
#[command(long_about = None)]
pub struct Cli {
    /// Recording duration (e.g., 10s, 1m, 2m30s)
    #[arg(short = 'd', long, value_name = "TIME", conflicts_with = "daemon")]
    pub duration: Option<String>,

    /// Domain preset for transcription context
    #[arg(short = 'D', long, value_name = "DOMAIN")]
    pub domain: Option<DomainArg>,

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

    /// Play audio cues on recording events
    #[arg(short = 'a', long)]
    pub audio_cue: bool,

    /// Run as daemon (control via: smart-scribe daemon toggle/cancel/status)
    #[arg(long)]
    pub daemon: bool,

    /// Max recording duration for daemon mode
    #[arg(long, value_name = "TIME", requires = "daemon")]
    pub max_duration: Option<String>,

    /// Show recording indicator overlay (daemon mode only, Linux/Wayland)
    #[cfg(target_os = "linux")]
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

/// Domain argument for clap ValueEnum
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum DomainArg {
    General,
    Dev,
    Medical,
    Legal,
    Finance,
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

impl From<DomainArg> for DomainId {
    fn from(arg: DomainArg) -> Self {
        match arg {
            DomainArg::General => DomainId::General,
            DomainArg::Dev => DomainId::Dev,
            DomainArg::Medical => DomainId::Medical,
            DomainArg::Legal => DomainId::Legal,
            DomainArg::Finance => DomainId::Finance,
        }
    }
}

impl From<DomainId> for DomainArg {
    fn from(id: DomainId) -> Self {
        match id {
            DomainId::General => DomainArg::General,
            DomainId::Dev => DomainArg::Dev,
            DomainId::Medical => DomainArg::Medical,
            DomainId::Legal => DomainArg::Legal,
            DomainId::Finance => DomainArg::Finance,
        }
    }
}

/// Parsed transcribe options (oneshot mode)
#[derive(Debug, Clone)]
pub struct TranscribeOptions {
    pub duration: Duration,
    pub domain: DomainId,
    pub clipboard: bool,
    pub keystroke: bool,
    pub keystroke_tool: Option<String>,
    pub notify: bool,
    pub audio_cue: bool,
}

/// Parsed daemon options
#[derive(Debug, Clone)]
pub struct DaemonOptions {
    pub max_duration: Duration,
    pub domain: DomainId,
    pub clipboard: bool,
    pub keystroke: bool,
    pub keystroke_tool: Option<String>,
    pub notify: bool,
    pub audio_cue: bool,
    #[cfg(target_os = "linux")]
    pub indicator: bool,
    #[cfg(target_os = "linux")]
    pub indicator_position: IndicatorPosition,
}

/// Valid config keys (Linux includes indicator settings)
#[cfg(target_os = "linux")]
pub const VALID_CONFIG_KEYS: &[&str] = &[
    "api_key",
    "duration",
    "max_duration",
    "domain",
    "clipboard",
    "keystroke",
    "notify",
    "audio_cue",
    "linux.keystroke_tool",
    "linux.indicator",
    "linux.indicator_position",
];

/// Valid config keys (non-Linux)
#[cfg(not(target_os = "linux"))]
pub const VALID_CONFIG_KEYS: &[&str] = &[
    "api_key",
    "duration",
    "max_duration",
    "domain",
    "clipboard",
    "keystroke",
    "notify",
    "audio_cue",
];

/// Valid keystroke tool values for all platforms
pub const KEYSTROKE_TOOL_ENIGO: &str = "enigo";

/// Valid keystroke tool values (platform-aware)
#[cfg(target_os = "linux")]
pub const VALID_KEYSTROKE_TOOLS: &[&str] = &["enigo", "auto", "ydotool", "xdotool", "wtype"];

#[cfg(not(target_os = "linux"))]
pub const VALID_KEYSTROKE_TOOLS: &[&str] = &["enigo"];

/// Check if a config key is valid
pub fn is_valid_config_key(key: &str) -> bool {
    VALID_CONFIG_KEYS.contains(&key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_parses_defaults() {
        let cli = Cli::parse_from(["smart-scribe"]);
        assert!(cli.duration.is_none());
        assert!(cli.domain.is_none());
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
    fn cli_parses_domain() {
        let cli = Cli::parse_from(["smart-scribe", "-D", "dev"]);
        assert_eq!(cli.domain, Some(DomainArg::Dev));
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
    fn cli_parses_daemon_with_max_duration() {
        let cli = Cli::parse_from(["smart-scribe", "--daemon", "--max-duration", "5m"]);
        assert!(cli.daemon);
        assert_eq!(cli.max_duration, Some("5m".to_string()));
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
        let cli = Cli::parse_from(["smart-scribe", "config", "set", "domain", "dev"]);
        if let Some(Commands::Config {
            action: ConfigAction::Set { key, value },
        }) = cli.command
        {
            assert_eq!(key, "domain");
            assert_eq!(value, "dev");
        } else {
            panic!("Expected Config Set command");
        }
    }

    #[test]
    fn domain_arg_converts_to_domain_id() {
        assert_eq!(DomainId::from(DomainArg::General), DomainId::General);
        assert_eq!(DomainId::from(DomainArg::Dev), DomainId::Dev);
    }

    #[test]
    fn valid_config_keys() {
        assert!(is_valid_config_key("api_key"));
        assert!(is_valid_config_key("duration"));
        #[cfg(target_os = "linux")]
        assert!(is_valid_config_key("linux.keystroke_tool"));
        #[cfg(not(target_os = "linux"))]
        assert!(!is_valid_config_key("linux.keystroke_tool"));
        assert!(!is_valid_config_key("invalid_key"));
    }

    #[test]
    fn cli_parses_keystroke_tool() {
        let cli = Cli::parse_from(["smart-scribe", "-k", "--keystroke-tool", "xdotool"]);
        assert!(cli.keystroke);
        assert_eq!(cli.keystroke_tool, Some("xdotool".to_string()));
    }

    #[test]
    fn verify_cli() {
        // Verify the CLI definition is valid
        Cli::command().debug_assert();
    }
}
