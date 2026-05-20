//! SmartScribe CLI entry point

use std::process::ExitCode;

use clap::Parser;

#[cfg(target_os = "linux")]
use smart_scribe::cli::IndicatorPosition;
use smart_scribe::cli::{
    app::{load_merged_config, run_oneshot, EXIT_ERROR, EXIT_USAGE_ERROR},
    args::{AuthAction, Cli, Commands},
    auth_cmd::{run_auth_status, run_login, run_logout},
    config_cmd::handle_config_command,
    daemon_app::run_daemon,
    daemon_cmd::handle_daemon_command,
    presenter::Presenter,
    DaemonOptions, TranscribeOptions,
};
use smart_scribe::domain::config::AppConfig;
#[cfg(target_os = "linux")]
use smart_scribe::domain::config::LinuxConfig;
#[cfg(target_os = "windows")]
use smart_scribe::domain::config::WindowsConfig;
use smart_scribe::domain::recording::Duration;
use smart_scribe::infrastructure::XdgConfigStore;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let presenter = Presenter::new(cli.output);

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
        Some(Commands::Login { from_codex }) => {
            return run_login(from_codex, cli.output).await;
        }
        Some(Commands::Logout) => {
            return run_logout(cli.output).await;
        }
        Some(Commands::Auth {
            action: AuthAction::Status,
        }) => {
            let config = load_merged_config(AppConfig::empty()).await;
            return run_auth_status(&config, cli.output).await;
        }
        None => {}
    }

    // Build CLI config from args
    #[cfg(target_os = "linux")]
    let cli_config = {
        let indicator_position_str = cli.indicator_position.map(|p| {
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

        // Build LinuxConfig with indicator and paste settings
        let linux = Some(LinuxConfig {
            keystroke_tool: cli.keystroke_tool.clone(),
            indicator: if cli.indicator { Some(true) } else { None },
            indicator_position: indicator_position_str,
            paste: if cli.paste { Some(true) } else { None },
        });

        AppConfig {
            auth: None,
            openai_api_key: None,
            openai_transcribe_model: None,
            duration: cli.duration.clone(),
            max_duration: cli.max_duration.clone(),
            clipboard: if cli.clipboard { Some(true) } else { None },
            keystroke: if cli.keystroke { Some(true) } else { None },
            notify: if cli.notify { Some(true) } else { None },
            audio_cue: if cli.audio_cue { Some(true) } else { None },
            linux,
            windows: None,
        }
    };

    #[cfg(target_os = "windows")]
    let cli_config = {
        let windows = Some(WindowsConfig {
            indicator: if cli.indicator { Some(true) } else { None },
            show_balloon: None,
        });

        AppConfig {
            auth: None,
            openai_api_key: None,
            openai_transcribe_model: None,
            duration: cli.duration.clone(),
            max_duration: cli.max_duration.clone(),
            clipboard: if cli.clipboard { Some(true) } else { None },
            keystroke: if cli.keystroke { Some(true) } else { None },
            notify: if cli.notify { Some(true) } else { None },
            audio_cue: if cli.audio_cue { Some(true) } else { None },
            linux: None,
            windows,
        }
    };

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    let cli_config = AppConfig {
        auth: None,
        openai_api_key: None,
        openai_transcribe_model: None,
        duration: cli.duration.clone(),
        max_duration: cli.max_duration.clone(),
        clipboard: if cli.clipboard { Some(true) } else { None },
        keystroke: if cli.keystroke { Some(true) } else { None },
        notify: if cli.notify { Some(true) } else { None },
        audio_cue: if cli.audio_cue { Some(true) } else { None },
        linux: None,
        windows: None,
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

        #[cfg(target_os = "linux")]
        let indicator_position: IndicatorPosition = config
            .indicator_position_or_default()
            .parse()
            .unwrap_or_default();

        let options = DaemonOptions {
            output: cli.output,
            max_duration,
            clipboard: config.clipboard_or_default(),
            keystroke: config.keystroke_or_default(),
            keystroke_tool: Some(config.keystroke_tool_or_default().to_string()),
            #[cfg(target_os = "linux")]
            paste: config.paste_or_default(),
            notify: config.notify_or_default(),
            audio_cue: config.audio_cue_or_default(),
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            indicator: config.indicator_or_default(),
            #[cfg(target_os = "linux")]
            indicator_position,
        };

        run_daemon(options, &config).await
    } else {
        // Parse optional fixed duration for foreground mode
        let duration = match config.duration.as_ref() {
            Some(s) => match s.parse::<Duration>() {
                Ok(d) => Some(d),
                Err(e) => {
                    presenter.error(&format!("Invalid duration: {}", e));
                    return ExitCode::from(EXIT_USAGE_ERROR);
                }
            },
            None => None,
        };

        // Parse optional safety cap for dynamic foreground mode
        let max_duration = match config.max_duration.as_ref() {
            Some(s) => match s.parse::<Duration>() {
                Ok(d) => Some(d),
                Err(e) => {
                    presenter.error(&format!("Invalid max-duration: {}", e));
                    return ExitCode::from(EXIT_USAGE_ERROR);
                }
            },
            None => None,
        };

        let options = TranscribeOptions {
            output: cli.output,
            duration,
            max_duration,
            clipboard: config.clipboard_or_default(),
            keystroke: config.keystroke_or_default(),
            keystroke_tool: Some(config.keystroke_tool_or_default().to_string()),
            #[cfg(target_os = "linux")]
            paste: config.paste_or_default(),
            notify: config.notify_or_default(),
            audio_cue: config.audio_cue_or_default(),
        };

        run_oneshot(options, &config).await
    }
}
