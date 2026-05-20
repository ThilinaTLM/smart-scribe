//! Main app runner for one-shot mode

use std::env;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use tokio::time::timeout;

use crate::application::ports::{AudioCue, AudioCueType, ConfigStore};
use crate::application::{TranscribeCallbacks, TranscribeInput, TranscribeRecordingUseCase};
use crate::domain::config::{AppConfig, RawAppConfig};
use crate::domain::error::ConfigError;
use crate::domain::recording::Duration;
use crate::infrastructure::XdgConfigStore;

// Re-export the transcriber factory at this path for backwards compatibility
// with `super::app::create_transcriber` callers (still used by daemon_app).
pub use crate::infrastructure::create_transcriber;

use super::args::TranscribeOptions;
use super::auth_cmd::describe_auth;
use super::exit_codes;
use super::output::OneshotResponse;
use super::presenter::Presenter;
use super::runtime::{build_adapters, BuildError, RuntimeOptions};
use super::signals::DaemonSignalHandler;

/// Poll interval for foreground recording updates.
const FOREGROUND_POLL_MS: u64 = 200;

/// Run the one-shot transcription
pub async fn run_oneshot(options: TranscribeOptions, config: &AppConfig) -> ExitCode {
    let mut presenter = Presenter::new(options.output);

    let runtime_opts = RuntimeOptions::from(&options);
    let bundle = match build_adapters(config, &runtime_opts, &presenter).await {
        Ok(b) => b,
        Err(BuildError::Transcriber(msg)) => {
            presenter.error(&msg);
            return ExitCode::from(exit_codes::ERROR);
        }
        Err(BuildError::SmartPaste(msg)) => {
            presenter.error(&format!("Paste mode unavailable: {}", msg));
            return ExitCode::from(exit_codes::ERROR);
        }
    };
    presenter.info(&describe_auth(config));

    let use_case = TranscribeRecordingUseCase::new(crate::application::UseCaseDeps {
        recorder: bundle.recorder,
        transcriber: bundle.transcriber,
        clipboard: bundle.clipboard,
        keystroke: bundle.keystroke,
        notifier: bundle.notifier,
        smart_paste: bundle.smart_paste,
    });
    let audio_cue: Arc<dyn AudioCue> = bundle.audio_cue;

    let enable_paste = options.paste;

    match options.duration {
        Some(duration) => {
            let input = TranscribeInput {
                duration,
                enable_clipboard: options.clipboard,
                enable_keystroke: options.keystroke,
                enable_paste,
                enable_notify: options.notify,
                warning_sink: Some(presenter.warning_sink()),
            };
            let callbacks = fixed_callbacks(Arc::clone(&audio_cue));

            match use_case.execute(input, callbacks).await {
                Ok(output) => present_output(&presenter, output),
                Err(e) => {
                    presenter.error(&e.to_string());
                    ExitCode::from(exit_codes::ERROR)
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
                warning_sink: Some(presenter.warning_sink()),
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
                    return ExitCode::from(exit_codes::ERROR);
                }
            };

            if let Err(e) = use_case.start_recording(&input, &callbacks).await {
                presenter.error(&e.to_string());
                return ExitCode::from(exit_codes::ERROR);
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
                            return ExitCode::from(exit_codes::ERROR);
                        }
                    }
                }
                signal = signals.recv() => {
                    if signal.is_some() {
                        presenter.spinner_fail("Recording aborted");
                        return ExitCode::from(exit_codes::ERROR);
                    }

                    match stop_future.await {
                        Ok(audio) => audio,
                        Err(e) => {
                            presenter.spinner_fail("Recording failed");
                            presenter.error(&e.to_string());
                            return ExitCode::from(exit_codes::ERROR);
                        }
                    }
                }
            };

            presenter.spinner_success(&format!(
                "Recording complete ({})",
                super::output::format_audio_size(audio.size_bytes() as u64)
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
                            return ExitCode::from(exit_codes::ERROR);
                        }
                    }
                }
                signal = signals.recv() => {
                    if signal.is_some() {
                        presenter.spinner_fail("Transcription aborted");
                        return ExitCode::from(exit_codes::ERROR);
                    }

                    match transcribe_future.await {
                        Ok(output) => output,
                        Err(e) => {
                            presenter.spinner_fail("Transcription failed");
                            presenter.error(&e.to_string());
                            return ExitCode::from(exit_codes::ERROR);
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
            move |size_bytes: u64| {
                eprintln!(
                    "✓ Recording complete ({})",
                    super::output::format_audio_size(size_bytes)
                );
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
        return ExitCode::from(exit_codes::SUCCESS);
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

    ExitCode::from(exit_codes::SUCCESS)
}

/// Get the OpenAI API key from environment or config file (for `auth = api_key`).
pub async fn get_openai_api_key() -> Result<String, String> {
    if let Ok(key) = env::var("OPENAI_API_KEY") {
        if !key.is_empty() {
            return Ok(key);
        }
    }

    let store = XdgConfigStore::new();
    let config = store.load().await.unwrap_or_else(|_| RawAppConfig::empty());

    config
        .openai_api_key
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            "Missing OpenAI API key. Set OPENAI_API_KEY or run \
         'smart-scribe config set openai_api_key <key>'."
                .to_string()
        })
}

/// Load and merge configuration from file, env, and CLI inputs.
///
/// Returns the validated [`AppConfig`]; surfaces validation errors
/// (`auth=garbage`, malformed durations, ...) as [`ConfigError::
/// ValidationError`].
pub async fn load_merged_config(cli_config: RawAppConfig) -> Result<AppConfig, ConfigError> {
    let store = XdgConfigStore::new();
    let file_config = store.load().await.unwrap_or_else(|_| RawAppConfig::empty());

    let env_config = RawAppConfig {
        openai_api_key: env::var("OPENAI_API_KEY").ok().filter(|s| !s.is_empty()),
        ..Default::default()
    };

    let merged = RawAppConfig::defaults()
        .merge(file_config)
        .merge(env_config)
        .merge(cli_config);

    AppConfig::try_from(merged)
}
