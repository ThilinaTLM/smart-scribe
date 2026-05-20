//! Main app runner for one-shot mode

use std::env;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use async_trait::async_trait;
use tokio::time::timeout;

use crate::application::ports::{
    AudioCue, AudioCueType, ConfigStore, Transcriber, TranscriptionError,
};
use crate::application::{TranscribeCallbacks, TranscribeInput, TranscribeRecordingUseCase};
use crate::domain::config::{AppConfig, AuthMode};
use crate::domain::recording::Duration;
use crate::domain::transcription::AudioData;
use crate::infrastructure::{
    create_audio_cue, create_clipboard, create_keystroke, create_notifier, create_recorder,
    create_smart_paste, ChatGptOAuthTranscriber, KeystrokeToolPreference, NoOpKeystroke,
    NoOpSmartPaste, OAuthStore, OpenAiApiTranscriber, XdgConfigStore,
};

use super::args::TranscribeOptions;
use super::auth_cmd::describe_auth;
use super::output::OneshotResponse;
use super::presenter::Presenter;
use super::signals::DaemonSignalHandler;

/// Poll interval for foreground recording updates.
const FOREGROUND_POLL_MS: u64 = 200;

/// Enum wrapper for dynamic backend dispatch.
pub enum AnyTranscriber {
    Oauth(ChatGptOAuthTranscriber),
    ApiKey(OpenAiApiTranscriber),
}

#[async_trait]
impl Transcriber for AnyTranscriber {
    async fn transcribe(&self, audio: &AudioData) -> Result<String, TranscriptionError> {
        match self {
            AnyTranscriber::Oauth(t) => t.transcribe(audio).await,
            AnyTranscriber::ApiKey(t) => t.transcribe(audio).await,
        }
    }
}

/// Create a transcriber based on the merged config.
///
/// For OAuth mode we construct the transcriber even if no token is yet on
/// disk — the missing-token error is surfaced at the first transcribe call so
/// that `smart-scribe login` can still be used to populate it.
pub fn create_transcriber(config: &AppConfig) -> Result<AnyTranscriber, String> {
    let model = config.openai_transcribe_model_or_default().to_string();
    match config.auth_or_default() {
        AuthMode::Oauth => {
            let store = OAuthStore::new()
                .map_err(|e| format!("Could not initialize OAuth token store: {e}"))?;
            Ok(AnyTranscriber::Oauth(ChatGptOAuthTranscriber::new(
                store, model,
            )))
        }
        AuthMode::ApiKey => {
            let api_key = config.openai_api_key.as_ref().ok_or_else(|| {
                "Missing OpenAI API key. Set OPENAI_API_KEY or run \
                 'smart-scribe config set openai_api_key <key>'."
                    .to_string()
            })?;
            Ok(AnyTranscriber::ApiKey(OpenAiApiTranscriber::new(
                api_key, model,
            )))
        }
    }
}

/// Exit codes
pub const EXIT_SUCCESS: u8 = 0;
pub const EXIT_ERROR: u8 = 1;
pub const EXIT_USAGE_ERROR: u8 = 2;

/// Run the one-shot transcription
pub async fn run_oneshot(options: TranscribeOptions, config: &AppConfig) -> ExitCode {
    let mut presenter = Presenter::new(options.output);

    // Create transcriber from merged config
    let transcriber = match create_transcriber(config) {
        Ok(t) => t,
        Err(e) => {
            presenter.error(&e);
            return ExitCode::from(EXIT_ERROR);
        }
    };
    eprintln!("{}", describe_auth(config));

    // Create adapters (using cross-platform implementations)
    let recorder = create_recorder();
    let (clipboard, clipboard_tool) = create_clipboard().await;
    if options.clipboard {
        eprintln!("Clipboard: using {}", clipboard_tool);
    }
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

    // Create smart paste adapter (Linux only)
    #[cfg(target_os = "linux")]
    let smart_paste: Box<dyn crate::application::ports::SmartPaste> = if options.paste {
        match create_smart_paste().await {
            Ok(sp) => {
                eprintln!("Paste: using kdotool+wl-copy+ydotool");
                sp
            }
            Err(e) => {
                presenter.error(&format!("Paste mode unavailable: {}", e));
                return ExitCode::from(EXIT_ERROR);
            }
        }
    } else {
        Box::new(NoOpSmartPaste::new())
    };
    #[cfg(not(target_os = "linux"))]
    let smart_paste: Box<dyn crate::application::ports::SmartPaste> =
        Box::new(NoOpSmartPaste::new());

    // Create use case
    let use_case = TranscribeRecordingUseCase::new(
        recorder,
        transcriber,
        clipboard,
        keystroke,
        notifier,
        smart_paste,
    );

    // Create audio cue adapter
    let audio_cue: Arc<dyn AudioCue> = Arc::from(create_audio_cue(options.audio_cue));

    // Create input
    #[cfg(target_os = "linux")]
    let enable_paste = options.paste;
    #[cfg(not(target_os = "linux"))]
    let enable_paste = false;

    match options.duration {
        Some(duration) => {
            let input = TranscribeInput {
                duration,
                enable_clipboard: options.clipboard,
                enable_keystroke: options.keystroke,
                enable_paste,
                enable_notify: options.notify,
            };
            let callbacks = fixed_callbacks(Arc::clone(&audio_cue));

            match use_case.execute(input, callbacks).await {
                Ok(output) => present_output(&presenter, output),
                Err(e) => {
                    presenter.error(&e.to_string());
                    ExitCode::from(EXIT_ERROR)
                }
            }
        }
        None => {
            let input = TranscribeInput {
                duration: options
                    .max_duration
                    .unwrap_or_else(Duration::default_duration),
                enable_clipboard: options.clipboard,
                enable_keystroke: options.keystroke,
                enable_paste,
                enable_notify: options.notify,
            };
            let callbacks = TranscribeCallbacks {
                on_progress: None,
                on_recording_start: None,
                on_recording_end: None,
                on_transcribing_start: None,
                on_transcribing_end: None,
            };

            let (mut signals, _signal_tx) = match DaemonSignalHandler::new().await {
                Ok(s) => s,
                Err(e) => {
                    presenter.error(&format!("Failed to setup signal handler: {}", e));
                    return ExitCode::from(EXIT_ERROR);
                }
            };

            if let Err(e) = use_case.start_recording(&input, &callbacks).await {
                presenter.error(&e.to_string());
                return ExitCode::from(EXIT_ERROR);
            }

            let cue = Arc::clone(&audio_cue);
            tokio::spawn(async move {
                let _ = cue.play(AudioCueType::RecordingStart).await;
            });

            presenter.start_spinner(&foreground_recording_message(0, options.max_duration));

            loop {
                let elapsed_ms = use_case.elapsed_ms();

                if let Some(max_duration) = options.max_duration {
                    if elapsed_ms >= max_duration.as_millis() {
                        presenter.warn("Max duration reached, stopping recording");
                        break;
                    }
                }

                presenter.update_spinner(&foreground_recording_message(
                    elapsed_ms,
                    options.max_duration,
                ));

                let wait_ms = options
                    .max_duration
                    .map(|max| {
                        max.as_millis()
                            .saturating_sub(elapsed_ms)
                            .min(FOREGROUND_POLL_MS)
                    })
                    .unwrap_or(FOREGROUND_POLL_MS)
                    .max(1);

                match timeout(StdDuration::from_millis(wait_ms), signals.recv()).await {
                    Ok(Some(_)) => break,
                    Ok(None) => break,
                    Err(_) => continue,
                }
            }

            let stop_future = use_case.stop_recording();
            tokio::pin!(stop_future);

            let audio = tokio::select! {
                result = &mut stop_future => {
                    match result {
                        Ok(audio) => audio,
                        Err(e) => {
                            presenter.spinner_fail("Recording failed");
                            presenter.error(&e.to_string());
                            return ExitCode::from(EXIT_ERROR);
                        }
                    }
                }
                signal = signals.recv() => {
                    if signal.is_some() {
                        presenter.spinner_fail("Recording aborted");
                        return ExitCode::from(EXIT_ERROR);
                    }

                    match stop_future.await {
                        Ok(audio) => audio,
                        Err(e) => {
                            presenter.spinner_fail("Recording failed");
                            presenter.error(&e.to_string());
                            return ExitCode::from(EXIT_ERROR);
                        }
                    }
                }
            };

            presenter.spinner_success(&format!(
                "Recording complete ({})",
                audio.human_readable_size()
            ));

            let cue = Arc::clone(&audio_cue);
            tokio::spawn(async move {
                let _ = cue.play(AudioCueType::RecordingStop).await;
            });

            presenter.start_spinner("Transcribing... Press Ctrl+C to abort");

            let transcribe_future = use_case.finalize_dynamic_recording(&input, &callbacks, audio);
            tokio::pin!(transcribe_future);

            let output = tokio::select! {
                result = &mut transcribe_future => {
                    match result {
                        Ok(output) => output,
                        Err(e) => {
                            presenter.spinner_fail("Transcription failed");
                            presenter.error(&e.to_string());
                            return ExitCode::from(EXIT_ERROR);
                        }
                    }
                }
                signal = signals.recv() => {
                    if signal.is_some() {
                        presenter.spinner_fail("Transcription aborted");
                        return ExitCode::from(EXIT_ERROR);
                    }

                    match transcribe_future.await {
                        Ok(output) => output,
                        Err(e) => {
                            presenter.spinner_fail("Transcription failed");
                            presenter.error(&e.to_string());
                            return ExitCode::from(EXIT_ERROR);
                        }
                    }
                }
            };

            presenter.spinner_success("Transcription complete");
            present_output(&presenter, output)
        }
    }
}

fn fixed_callbacks(audio_cue: Arc<dyn AudioCue>) -> TranscribeCallbacks {
    TranscribeCallbacks {
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
    }
}

fn foreground_recording_message(elapsed_ms: u64, max_duration: Option<Duration>) -> String {
    let elapsed = Duration::from_millis(elapsed_ms);

    match max_duration {
        Some(max) => format!("Recording... Press Ctrl+C to stop [{} / {}]", elapsed, max),
        None => format!("Recording... Press Ctrl+C to stop [{}]", elapsed),
    }
}

fn present_output(presenter: &Presenter, output: crate::application::TranscribeOutput) -> ExitCode {
    if presenter.is_json() {
        presenter.output_json(&OneshotResponse::from(output));
        return ExitCode::from(EXIT_SUCCESS);
    }

    presenter.output(&output.text);

    if output.clipboard_copied {
        presenter.info("Copied to clipboard");
    }
    if output.keystroke_sent {
        presenter.info("Typed into window");
    }
    if output.paste_sent {
        presenter.info("Pasted into window");
    }

    ExitCode::from(EXIT_SUCCESS)
}

/// Get the OpenAI API key from environment or config file (for `auth = api_key`).
pub async fn get_openai_api_key() -> Result<String, String> {
    if let Ok(key) = env::var("OPENAI_API_KEY") {
        if !key.is_empty() {
            return Ok(key);
        }
    }

    let store = XdgConfigStore::new();
    let config = store.load().await.unwrap_or_else(|_| AppConfig::empty());

    config.openai_api_key.ok_or_else(|| {
        "Missing OpenAI API key. Set OPENAI_API_KEY or run \
         'smart-scribe config set openai_api_key <key>'."
            .to_string()
    })
}

/// Load and merge configuration from file, env, and CLI
pub async fn load_merged_config(cli_config: AppConfig) -> AppConfig {
    let store = XdgConfigStore::new();
    let file_config = store.load().await.unwrap_or_else(|_| AppConfig::empty());

    // Build env config
    let env_config = AppConfig {
        openai_api_key: env::var("OPENAI_API_KEY").ok().filter(|s| !s.is_empty()),
        ..Default::default()
    };

    // Merge: defaults < file < env < cli
    AppConfig::defaults()
        .merge(file_config)
        .merge(env_config)
        .merge(cli_config)
}
