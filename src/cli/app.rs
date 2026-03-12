//! Main app runner for one-shot mode

use std::env;
use std::process::ExitCode;
use std::sync::Arc;

use async_trait::async_trait;

use crate::application::ports::{AudioCueType, ConfigStore, Transcriber, TranscriptionError};
use crate::application::{TranscribeCallbacks, TranscribeInput, TranscribeRecordingUseCase};
use crate::domain::config::AppConfig;
use crate::domain::transcription::{AudioData, SystemPrompt};
use crate::infrastructure::{
    create_audio_cue, create_clipboard, create_keystroke, create_notifier, create_recorder,
    ChatGptTranscriber, GeminiTranscriber, KeystrokeToolPreference, NoOpKeystroke, XdgConfigStore,
};

use super::args::TranscribeOptions;
use super::presenter::Presenter;
use super::signals::ShutdownSignal;

/// Enum wrapper for dynamic backend dispatch
pub enum AnyTranscriber {
    Gemini(GeminiTranscriber),
    ChatGpt(ChatGptTranscriber),
}

#[async_trait]
impl Transcriber for AnyTranscriber {
    async fn transcribe(
        &self,
        audio: &AudioData,
        prompt: &SystemPrompt,
    ) -> Result<String, TranscriptionError> {
        match self {
            AnyTranscriber::Gemini(t) => t.transcribe(audio, prompt).await,
            AnyTranscriber::ChatGpt(t) => t.transcribe(audio, prompt).await,
        }
    }
}

/// Create a transcriber based on the merged config
pub fn create_transcriber(config: &AppConfig) -> Result<AnyTranscriber, String> {
    match config.backend_or_default() {
        "chatgpt" => {
            let cookie_file = config.chatgpt_cookie_file_or_default();
            if !cookie_file.exists() {
                return Err(format!(
                    "ChatGPT cookie file not found: {}\nExport cookies from your browser to this file.",
                    cookie_file.display()
                ));
            }
            Ok(AnyTranscriber::ChatGpt(ChatGptTranscriber::new(
                cookie_file,
            )))
        }
        _ => {
            // Gemini (default)
            let api_key = config.api_key.as_ref().ok_or_else(|| {
                "Missing API key. Set GEMINI_API_KEY environment variable or run 'smart-scribe config set api_key <key>'".to_string()
            })?;
            Ok(AnyTranscriber::Gemini(GeminiTranscriber::new(api_key)))
        }
    }
}

/// Exit codes
pub const EXIT_SUCCESS: u8 = 0;
pub const EXIT_ERROR: u8 = 1;
pub const EXIT_USAGE_ERROR: u8 = 2;

/// Run the one-shot transcription
pub async fn run_oneshot(options: TranscribeOptions, config: &AppConfig) -> ExitCode {
    let presenter = Presenter::new();

    // Create transcriber from merged config
    let transcriber = match create_transcriber(config) {
        Ok(t) => t,
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

    // Create adapters (using cross-platform implementations)
    let recorder = create_recorder();
    let clipboard = create_clipboard();
    let notifier = create_notifier();

    // Parse keystroke tool preference
    let preference = options
        .keystroke_tool
        .as_ref()
        .and_then(|s| s.parse::<KeystrokeToolPreference>().ok())
        .unwrap_or_default();

    // Detect keystroke tool
    let keystroke: Box<dyn crate::application::ports::Keystroke> =
        match create_keystroke(preference).await {
            Ok((ks, tool)) => {
                eprintln!("Keystroke: using {}", tool);
                ks
            }
            Err(e) => {
                if options.keystroke {
                    presenter.warn(&format!("Keystroke disabled: {}", e));
                }
                Box::new(NoOpKeystroke::new())
            }
        };

    // Create use case
    let use_case =
        TranscribeRecordingUseCase::new(recorder, transcriber, clipboard, keystroke, notifier);

    // Create audio cue adapter
    let audio_cue = Arc::new(create_audio_cue(options.audio_cue));

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
        on_recording_start: Some(Box::new({
            let cue = Arc::clone(&audio_cue);
            move || {
                eprintln!("⠋ Recording...");
                let cue = Arc::clone(&cue);
                tokio::spawn(async move {
                    let _ = cue.play(AudioCueType::RecordingStart).await;
                });
            }
        })),
        on_recording_end: Some(Box::new({
            let cue = Arc::clone(&audio_cue);
            move |size: &str| {
                eprintln!("✓ Recording complete ({})", size);
                let cue = Arc::clone(&cue);
                tokio::spawn(async move {
                    let _ = cue.play(AudioCueType::RecordingStop).await;
                });
            }
        })),
        on_transcribing_start: Some(Box::new(|| {
            eprintln!("⠋ Transcribing...");
        })),
        on_transcribing_end: Some(Box::new(|| {
            eprintln!("✓ Transcription complete");
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
        chatgpt_cookie_file: env::var("CHATGPT_COOKIE_FILE")
            .ok()
            .filter(|s| !s.is_empty()),
        ..Default::default()
    };

    // Merge: defaults < file < env < cli
    AppConfig::defaults()
        .merge(file_config)
        .merge(env_config)
        .merge(cli_config)
}
