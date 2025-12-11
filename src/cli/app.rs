//! Main app runner for one-shot mode

use std::env;
use std::process::ExitCode;
use std::sync::Arc;

use crate::application::ports::ConfigStore;
use crate::application::{TranscribeCallbacks, TranscribeInput, TranscribeRecordingUseCase};
use crate::domain::config::AppConfig;
use crate::infrastructure::{
    FfmpegRecorder, GeminiTranscriber, NotifySendNotifier, WaylandClipboard, XdgConfigStore,
    XdotoolKeystroke,
};

use super::args::TranscribeOptions;
use super::presenter::Presenter;
use super::signals::ShutdownSignal;

/// Exit codes
pub const EXIT_SUCCESS: u8 = 0;
pub const EXIT_ERROR: u8 = 1;
pub const EXIT_USAGE_ERROR: u8 = 2;

/// Run the one-shot transcription
pub async fn run_oneshot(options: TranscribeOptions) -> ExitCode {
    let presenter = Presenter::new();

    // Load API key from config or environment
    let api_key = match get_api_key().await {
        Ok(key) => key,
        Err(e) => {
            presenter.error(&e);
            return ExitCode::from(EXIT_ERROR);
        }
    };

    // Setup signal handler
    let shutdown = ShutdownSignal::new();
    if let Err(e) = shutdown.setup().await {
        presenter.error(&format!("Failed to setup signal handler: {}", e));
        return ExitCode::from(EXIT_ERROR);
    }

    // Create adapters
    let recorder = FfmpegRecorder::new();
    let transcriber = GeminiTranscriber::new(api_key);
    let clipboard = WaylandClipboard::new();
    let keystroke = XdotoolKeystroke::new();
    let notifier = NotifySendNotifier::new();

    // Create use case
    let use_case = TranscribeRecordingUseCase::new(
        recorder,
        transcriber,
        clipboard,
        keystroke,
        notifier,
    );

    // Create input
    let input = TranscribeInput {
        duration: options.duration,
        domain: options.domain,
        enable_clipboard: options.clipboard,
        enable_keystroke: options.keystroke,
        enable_notify: options.notify,
    };

    // Create callbacks (simplified - use eprintln for status)
    let callbacks = TranscribeCallbacks {
        on_progress: Some(Arc::new(move |_elapsed, _total| {
            // Progress handled by spinner
        })),
        on_recording_start: Some(Box::new(|| {
            eprintln!("{} Recording...", "⠋".to_string());
        })),
        on_recording_end: Some(Box::new(|size: &str| {
            eprintln!("{} Recording complete ({})", "✓", size);
        })),
        on_transcribing_start: Some(Box::new(|| {
            eprintln!("{} Transcribing...", "⠋".to_string());
        })),
        on_transcribing_end: Some(Box::new(|| {
            eprintln!("{} Transcription complete", "✓");
        })),
    };

    // Execute
    match use_case.execute(input, callbacks).await {
        Ok(output) => {
            // Output transcription to stdout
            presenter.output(&output.text);

            // Show status for clipboard/keystroke
            if output.clipboard_copied {
                presenter.info("Copied to clipboard");
            }
            if output.keystroke_sent {
                presenter.info("Typed into window");
            }

            ExitCode::from(EXIT_SUCCESS)
        }
        Err(e) => {
            presenter.error(&e.to_string());
            ExitCode::from(EXIT_ERROR)
        }
    }
}

/// Get API key from environment or config file
pub async fn get_api_key() -> Result<String, String> {
    // Check environment first
    if let Ok(key) = env::var("GEMINI_API_KEY") {
        if !key.is_empty() {
            return Ok(key);
        }
    }

    // Check config file
    let store = XdgConfigStore::new();
    let config = store.load().await.unwrap_or_else(|_| AppConfig::empty());

    config.api_key.ok_or_else(|| {
        "Missing API key. Set GEMINI_API_KEY environment variable or run 'smart-scribe config set api_key <key>'".to_string()
    })
}

/// Load and merge configuration from file, env, and CLI
pub async fn load_merged_config(cli_config: AppConfig) -> AppConfig {
    let store = XdgConfigStore::new();
    let file_config = store.load().await.unwrap_or_else(|_| AppConfig::empty());

    // Build env config
    let env_config = AppConfig {
        api_key: env::var("GEMINI_API_KEY").ok().filter(|s| !s.is_empty()),
        ..Default::default()
    };

    // Merge: defaults < file < env < cli
    AppConfig::defaults()
        .merge(file_config)
        .merge(env_config)
        .merge(cli_config)
}
