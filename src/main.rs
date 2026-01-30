//! SmartScribe CLI entry point

use std::process::ExitCode;

use clap::Parser;

use smart_scribe::cli::{
    app::{load_merged_config, run_oneshot, EXIT_ERROR, EXIT_USAGE_ERROR},
    args::{Cli, Commands},
    config_cmd::handle_config_command,
    daemon_app::run_daemon,
    daemon_cmd::handle_daemon_command,
    presenter::Presenter,
    DaemonOptions, TranscribeOptions,
};
use smart_scribe::domain::config::{AppConfig, LinuxConfig};
use smart_scribe::domain::recording::Duration;
use smart_scribe::infrastructure::XdgConfigStore;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let presenter = Presenter::new();

    // Handle subcommands
    match cli.command {
        Some(Commands::Config { action }) => {
            let store = XdgConfigStore::new();
            if let Err(e) = handle_config_command(action, &store, &presenter).await {
                presenter.error(&e.to_string());
                return ExitCode::from(EXIT_ERROR);
            }
            return ExitCode::SUCCESS;
        }
        Some(Commands::Daemon { action }) => {
            if let Err(e) = handle_daemon_command(action, &presenter).await {
                presenter.error(&e);
                return ExitCode::from(EXIT_ERROR);
            }
            return ExitCode::SUCCESS;
        }
        None => {}
    }

    // Build CLI config from args
    let cli_config = AppConfig {
        api_key: None, // API key comes from env/file only
        duration: cli.duration.clone(),
        max_duration: cli.max_duration.clone(),
        domain: cli
            .domain
            .map(|d| smart_scribe::domain::transcription::DomainId::from(d).to_string()),
        clipboard: if cli.clipboard { Some(true) } else { None },
        keystroke: if cli.keystroke { Some(true) } else { None },
        notify: if cli.notify { Some(true) } else { None },
        linux: cli.keystroke_tool.clone().map(|tool| LinuxConfig {
            keystroke_tool: Some(tool),
        }),
    };

    // Merge config
    let config = load_merged_config(cli_config).await;

    // Route to appropriate handler
    if cli.daemon {
        // Parse max duration
        let max_duration = match config.max_duration.as_ref() {
            Some(s) => match s.parse::<Duration>() {
                Ok(d) => d,
                Err(e) => {
                    presenter.error(&format!("Invalid max-duration: {}", e));
                    return ExitCode::from(EXIT_USAGE_ERROR);
                }
            },
            None => Duration::default_max_duration(),
        };

        let options = DaemonOptions {
            max_duration,
            domain: config.domain_or_default(),
            clipboard: config.clipboard_or_default(),
            keystroke: config.keystroke_or_default(),
            keystroke_tool: Some(config.keystroke_tool_or_default().to_string()),
            notify: config.notify_or_default(),
        };

        run_daemon(options).await
    } else {
        // Parse duration
        let duration = match config.duration.as_ref() {
            Some(s) => match s.parse::<Duration>() {
                Ok(d) => d,
                Err(e) => {
                    presenter.error(&format!("Invalid duration: {}", e));
                    return ExitCode::from(EXIT_USAGE_ERROR);
                }
            },
            None => Duration::default_duration(),
        };

        let options = TranscribeOptions {
            duration,
            domain: config.domain_or_default(),
            clipboard: config.clipboard_or_default(),
            keystroke: config.keystroke_or_default(),
            keystroke_tool: Some(config.keystroke_tool_or_default().to_string()),
            notify: config.notify_or_default(),
        };

        run_oneshot(options).await
    }
}
