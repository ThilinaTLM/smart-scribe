//! CLI argument definitions using Clap

use clap::{Parser, Subcommand, ValueEnum};

use crate::domain::recording::Duration;
use crate::domain::transcription::DomainId;

/// SmartScribe - AI-powered voice to text transcription
#[derive(Parser, Debug)]
#[command(name = "smart-scribe")]
#[command(version = "2.0.0")]
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

    /// Show desktop notifications
    #[arg(short = 'n', long)]
    pub notify: bool,

    /// Run as daemon, controlled by signals
    #[arg(long)]
    pub daemon: bool,

    /// Max recording duration for daemon mode
    #[arg(long, value_name = "TIME", requires = "daemon")]
    pub max_duration: Option<String>,

    /// Config subcommand
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Config subcommands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
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
    pub notify: bool,
}

/// Parsed daemon options
#[derive(Debug, Clone)]
pub struct DaemonOptions {
    pub max_duration: Duration,
    pub domain: DomainId,
    pub clipboard: bool,
    pub keystroke: bool,
    pub notify: bool,
}

/// Valid config keys
pub const VALID_CONFIG_KEYS: &[&str] = &[
    "api_key",
    "duration",
    "max_duration",
    "domain",
    "clipboard",
    "keystroke",
    "notify",
];

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
        assert!(!cli.notify);
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
        let cli = Cli::parse_from(["smart-scribe", "-c", "-k", "-n"]);
        assert!(cli.clipboard);
        assert!(cli.keystroke);
        assert!(cli.notify);
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
            Some(Commands::Config { action: ConfigAction::Init })
        ));
    }

    #[test]
    fn cli_parses_config_set() {
        let cli = Cli::parse_from(["smart-scribe", "config", "set", "domain", "dev"]);
        if let Some(Commands::Config { action: ConfigAction::Set { key, value } }) = cli.command {
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
        assert!(!is_valid_config_key("invalid_key"));
    }

    #[test]
    fn verify_cli() {
        // Verify the CLI definition is valid
        Cli::command().debug_assert();
    }
}
