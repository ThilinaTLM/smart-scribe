//! Daemon app runner

use std::process::ExitCode;
use std::time::Duration as StdDuration;

use tokio::time::timeout;

use crate::application::{DaemonConfig, DaemonTranscriptionUseCase};
use crate::domain::daemon::DaemonState;
use crate::infrastructure::{
    FfmpegRecorder, GeminiTranscriber, NotifySendNotifier, WaylandClipboard, XdotoolKeystroke,
};

use super::app::{EXIT_ERROR, EXIT_SUCCESS, get_api_key};
use super::args::DaemonOptions;
use super::pid_file::{PidFile, PidFileError};
use super::presenter::Presenter;
use super::signals::{DaemonSignal, DaemonSignalHandler};

/// Run daemon mode
pub async fn run_daemon(options: DaemonOptions) -> ExitCode {
    let presenter = Presenter::new();

    // Acquire PID file
    let pid_file = PidFile::new();
    if let Err(e) = pid_file.acquire() {
        match e {
            PidFileError::AlreadyRunning(pid) => {
                presenter.error(&format!("Another daemon is already running (PID: {})", pid));
            }
            _ => {
                presenter.error(&e.to_string());
            }
        }
        return ExitCode::from(EXIT_ERROR);
    }

    // Load API key
    let api_key = match get_api_key().await {
        Ok(key) => key,
        Err(e) => {
            presenter.error(&e);
            return ExitCode::from(EXIT_ERROR);
        }
    };

    // Create adapters
    let recorder = FfmpegRecorder::new();
    let transcriber = GeminiTranscriber::new(api_key);
    let clipboard = WaylandClipboard::new();
    let keystroke = XdotoolKeystroke::new();
    let notifier = NotifySendNotifier::new();

    // Create daemon config
    let config = DaemonConfig {
        domain: options.domain,
        max_duration: options.max_duration,
        enable_clipboard: options.clipboard,
        enable_keystroke: options.keystroke,
        enable_notify: options.notify,
    };

    // Create use case
    let use_case = DaemonTranscriptionUseCase::new(
        recorder,
        transcriber,
        clipboard,
        keystroke,
        notifier,
        config,
    );

    // Setup signal handler
    let mut signals = match DaemonSignalHandler::new().await {
        Ok(s) => s,
        Err(e) => {
            presenter.error(&format!("Failed to setup signal handler: {}", e));
            return ExitCode::from(EXIT_ERROR);
        }
    };

    presenter.daemon_status("Started, waiting for signals...");
    presenter.info(&format!(
        "PID: {} | SIGUSR1: toggle | SIGUSR2: cancel | SIGINT: exit",
        std::process::id()
    ));

    // Main signal loop
    let max_duration_ms = options.max_duration.as_millis();
    let result = daemon_loop(&use_case, &mut signals, &presenter, max_duration_ms).await;

    // Cleanup
    let _ = pid_file.release();

    if result {
        ExitCode::from(EXIT_SUCCESS)
    } else {
        ExitCode::from(EXIT_ERROR)
    }
}

async fn daemon_loop<R, T, C, K, N>(
    use_case: &DaemonTranscriptionUseCase<R, T, C, K, N>,
    signals: &mut DaemonSignalHandler,
    presenter: &Presenter,
    max_duration_ms: u64,
) -> bool
where
    R: crate::application::ports::UnboundedRecorder,
    T: crate::application::ports::Transcriber,
    C: crate::application::ports::Clipboard,
    K: crate::application::ports::Keystroke,
    N: crate::application::ports::Notifier,
{
    loop {
        let state = use_case.state().await;

        // If recording, use timeout for max duration check
        let signal = if state == DaemonState::Recording {
            let remaining_ms = max_duration_ms.saturating_sub(use_case.elapsed_ms());
            if remaining_ms == 0 {
                // Max duration reached
                Some(DaemonSignal::Toggle)
            } else {
                match timeout(
                    StdDuration::from_millis(remaining_ms.min(100)),
                    signals.recv(),
                )
                .await
                {
                    Ok(sig) => sig,
                    Err(_) => {
                        // Timeout - check if max duration reached
                        if use_case.check_max_duration() {
                            presenter.warn("Max duration reached, auto-stopping");
                            Some(DaemonSignal::Toggle)
                        } else {
                            continue;
                        }
                    }
                }
            }
        } else {
            signals.recv().await
        };

        match signal {
            Some(DaemonSignal::Toggle) => {
                let current_state = use_case.state().await;
                match current_state {
                    DaemonState::Idle => {
                        // Start recording
                        if let Err(e) = use_case.start_recording().await {
                            presenter.error(&format!("Failed to start recording: {}", e));
                            continue;
                        }
                        presenter.daemon_status("Recording...");
                    }
                    DaemonState::Recording => {
                        // Stop and transcribe
                        presenter.daemon_status("Processing...");
                        match use_case.stop_and_transcribe().await {
                            Ok(output) => {
                                presenter.output(&output.text);
                                presenter.daemon_status(&format!(
                                    "Idle (last: {} audio)",
                                    output.audio_size
                                ));
                            }
                            Err(e) => {
                                presenter.error(&format!("Transcription failed: {}", e));
                                presenter.daemon_status("Idle (error)");
                            }
                        }
                    }
                    DaemonState::Processing => {
                        // Already processing, ignore
                        presenter.warn("Already processing, please wait");
                    }
                }
            }
            Some(DaemonSignal::Cancel) => {
                let current_state = use_case.state().await;
                if current_state == DaemonState::Recording {
                    if let Err(e) = use_case.cancel().await {
                        presenter.error(&format!("Failed to cancel: {}", e));
                    } else {
                        presenter.daemon_status("Recording cancelled");
                    }
                } else {
                    presenter.warn("Not recording, nothing to cancel");
                }
            }
            Some(DaemonSignal::Shutdown) => {
                let current_state = use_case.state().await;
                if current_state == DaemonState::Recording {
                    // Cancel any in-progress recording
                    let _ = use_case.cancel().await;
                }
                presenter.daemon_status("Shutting down...");
                return true;
            }
            None => {
                // Channel closed
                return false;
            }
        }
    }
}

