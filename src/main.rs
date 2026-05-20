//! SmartScribe CLI entry point.
//!
//! Translates the parsed [`Cli`] into a [`RawAppConfig`] overlay, merges it
//! with the file / env layers via [`load_merged_config`], then dispatches to
//! the one-shot or daemon runner.

use std::process::ExitCode;

use clap::Parser;

#[cfg(target_os = "linux")]
use smart_scribe::cli::IndicatorPosition;
use smart_scribe::cli::{
    app::{load_merged_config, run_oneshot},
    args::{AuthAction, Cli, Commands},
    auth_cmd::{run_auth_status, run_login, run_logout},
    config_cmd::handle_config_command,
    daemon_app::run_daemon,
    daemon_cmd::handle_daemon_command,
    exit_codes,
    presenter::Presenter,
    DaemonOptions, TranscribeOptions,
};
use smart_scribe::domain::config::{RawAppConfig, RawLinuxConfig, RawWindowsConfig};
use smart_scribe::infrastructure::XdgConfigStore;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let presenter = Presenter::new(cli.output);

    // Handle subcommands that don't need the merged AppConfig.
    match cli.command {
        Some(Commands::Config { action }) => {
            let store = XdgConfigStore::new();
            if let Err(e) = handle_config_command(action, &store, &presenter).await {
                presenter.error(&e.to_string());
                return ExitCode::from(exit_codes::ERROR);
            }
            return ExitCode::SUCCESS;
        }
        Some(Commands::Daemon { action }) => {
            if let Err(e) = handle_daemon_command(action, &presenter).await {
                presenter.error(&e);
                return ExitCode::from(exit_codes::ERROR);
            }
            return ExitCode::SUCCESS;
        }
        Some(Commands::Login { from_codex }) => {
            return run_login(from_codex, cli.output).await;
        }
        Some(Commands::Logout) => {
            return run_logout(cli.output).await;
        }
        Some(Commands::Auth {
            action: AuthAction::Status,
        }) => {
            let config = match load_merged_config(RawAppConfig::empty()).await {
                Ok(c) => c,
                Err(e) => {
                    presenter.error(&format!("Invalid configuration: {}", e));
                    return ExitCode::from(exit_codes::USAGE_ERROR);
                }
            };
            return run_auth_status(&config, cli.output).await;
        }
        None => {}
    }

    // Build the CLI overlay as a RawAppConfig (one place, no cfg blocks).
    let cli_config = cli_to_raw(&cli);

    let config = match load_merged_config(cli_config).await {
        Ok(c) => c,
        Err(e) => {
            presenter.error(&format!("Invalid configuration: {}", e));
            return ExitCode::from(exit_codes::USAGE_ERROR);
        }
    };

    if cli.daemon {
        // Daemon mode always needs a concrete max duration; fall back to the
        // domain default if neither config nor CLI supplied one.
        let max_duration = config
            .max_duration
            .unwrap_or_else(smart_scribe::domain::recording::Duration::default_max_duration);

        #[cfg(target_os = "linux")]
        let indicator_position: IndicatorPosition = config
            .platform
            .indicator_position
            .parse()
            .unwrap_or_default();

        let options = DaemonOptions {
            output: cli.output,
            max_duration,
            clipboard: config.clipboard,
            keystroke: config.keystroke,
            keystroke_tool: Some(config.platform.keystroke_tool.clone()),
            paste: config.platform.linux_paste,
            notify: config.notify,
            audio_cue: config.audio_cue,
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            indicator: config.platform.indicator,
            #[cfg(target_os = "linux")]
            indicator_position,
        };

        run_daemon(options, &config).await
    } else {
        let options = TranscribeOptions {
            output: cli.output,
            duration: config.duration,
            max_duration: config.max_duration,
            clipboard: config.clipboard,
            keystroke: config.keystroke,
            keystroke_tool: Some(config.platform.keystroke_tool.clone()),
            paste: config.platform.linux_paste,
            notify: config.notify,
            audio_cue: config.audio_cue,
        };

        run_oneshot(options, &config).await
    }
}

/// Translate the parsed CLI into the raw-config overlay layer.
///
/// Returns `None`-filled fields where the user didn't pass a flag (so the
/// merge step keeps file/env values intact). Platform-gated CLI flags are
/// the only `#[cfg]` blocks here; the rest is portable.
fn cli_to_raw(cli: &Cli) -> RawAppConfig {
    #[cfg(target_os = "linux")]
    let indicator_position = cli.indicator_position.map(|p| {
        match p {
            IndicatorPosition::TopRight => "top-right",
            IndicatorPosition::TopLeft => "top-left",
            IndicatorPosition::TopCenter => "top-center",
            IndicatorPosition::BottomCenter => "bottom-center",
            IndicatorPosition::BottomRight => "bottom-right",
            IndicatorPosition::BottomLeft => "bottom-left",
        }
        .to_string()
    });
    #[cfg(not(target_os = "linux"))]
    let indicator_position: Option<String> = None;

    #[cfg(target_os = "linux")]
    let cli_paste = cli.paste;
    #[cfg(not(target_os = "linux"))]
    let cli_paste = false;

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    let cli_indicator = cli.indicator;
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    let cli_indicator = false;

    let linux = Some(RawLinuxConfig {
        keystroke_tool: cli.keystroke_tool.clone(),
        indicator: if cli_indicator { Some(true) } else { None },
        indicator_position,
        paste: if cli_paste { Some(true) } else { None },
    });

    let windows = Some(RawWindowsConfig {
        indicator: if cli_indicator { Some(true) } else { None },
        show_balloon: None,
    });

    RawAppConfig {
        auth: None,
        openai_api_key: None,
        openai_transcribe_model: None,
        transcribe_prompt: None,
        transcribe_language: None,
        duration: cli.duration.clone(),
        max_duration: cli.max_duration.clone(),
        clipboard: if cli.clipboard { Some(true) } else { None },
        keystroke: if cli.keystroke { Some(true) } else { None },
        notify: if cli.notify { Some(true) } else { None },
        audio_cue: if cli.audio_cue { Some(true) } else { None },
        linux,
        windows,
    }
}
