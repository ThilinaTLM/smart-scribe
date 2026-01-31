//! Daemon app runner

use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::time::Duration as StdDuration;

use tokio::sync::broadcast;
use tokio::time::timeout;

use crate::application::ports::{AudioCue, AudioCueType};
use crate::application::{DaemonConfig, DaemonTranscriptionUseCase};
use crate::domain::daemon::{DaemonState, StateUpdate};
use crate::infrastructure::{
    create_audio_cue, create_clipboard, create_keystroke, create_notifier, create_recorder,
    GeminiTranscriber, KeystrokeToolPreference, NoOpKeystroke,
};

use super::app::{get_api_key, EXIT_ERROR, EXIT_SUCCESS};
use super::args::DaemonOptions;
use super::ipc::create_ipc_server;
use super::pid_file::{PidFile, PidFileError};
use super::presenter::Presenter;
use super::signals::{DaemonSignal, DaemonSignalHandler};

/// Buffer size for state update broadcast channel
const STATE_BROADCAST_CAPACITY: usize = 16;

/// Context for the daemon loop to reduce argument count
struct DaemonLoopContext<'a> {
    presenter: &'a Presenter,
    max_duration_ms: u64,
    shared_state: &'a Arc<Mutex<DaemonState>>,
    shared_elapsed: &'a Arc<Mutex<u64>>,
    state_tx: &'a broadcast::Sender<StateUpdate>,
    audio_cue: &'a Arc<dyn AudioCue>,
}

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

    // Create adapters (using cross-platform implementations)
    let recorder = create_recorder();
    let transcriber = GeminiTranscriber::new(api_key);
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

    // Setup signal handler (returns handler + sender for socket server)
    let (mut signals, signal_tx) = match DaemonSignalHandler::new().await {
        Ok(s) => s,
        Err(e) => {
            presenter.error(&format!("Failed to setup signal handler: {}", e));
            return ExitCode::from(EXIT_ERROR);
        }
    };

    // Setup IPC server (Unix socket on Linux/macOS, named pipe on Windows)
    let mut ipc_server = create_ipc_server();
    let ipc_path = ipc_server.path();

    if let Err(e) = ipc_server.bind() {
        presenter.error(&format!("Failed to bind IPC: {}", e));
        return ExitCode::from(EXIT_ERROR);
    }

    // Wrap state and elapsed time in Arc<Mutex> for sharing with IPC server
    let state = Arc::new(Mutex::new(DaemonState::Idle));
    let elapsed = Arc::new(Mutex::new(0u64));
    let state_for_ipc = Arc::clone(&state);
    let elapsed_for_ipc = Arc::clone(&elapsed);

    // Create broadcast channel for state updates
    let (state_tx, state_rx) = broadcast::channel::<StateUpdate>(STATE_BROADCAST_CAPACITY);

    // Spawn indicator thread if enabled (Linux only, requires Wayland)
    #[cfg(target_os = "linux")]
    if options.indicator {
        let indicator_rx = state_tx.subscribe();
        let position = options.indicator_position;
        std::thread::spawn(move || {
            if let Err(e) = crate::gui::run_indicator(position, indicator_rx) {
                eprintln!(
                    "Indicator error: {} (requires Wayland with wlr-layer-shell)",
                    e
                );
            }
        });
        presenter.info("Indicator overlay enabled");
    }

    // Spawn IPC server task
    tokio::spawn(async move {
        let _ = ipc_server
            .run(
                signal_tx,
                Box::new(move || {
                    // Use std::sync::Mutex - safe because lock is very brief
                    *state_for_ipc.lock().unwrap_or_else(|e| e.into_inner())
                }),
                Box::new(move || *elapsed_for_ipc.lock().unwrap_or_else(|e| e.into_inner())),
                state_rx,
            )
            .await;
    });

    // Create audio cue adapter
    let audio_cue: Arc<dyn AudioCue> = Arc::from(create_audio_cue(options.audio_cue));

    presenter.daemon_status("Started, waiting for commands...");
    presenter.info(&format!(
        "PID: {} | IPC: {} | SIGINT: exit",
        std::process::id(),
        ipc_path
    ));

    // Main signal loop
    let ctx = DaemonLoopContext {
        presenter: &presenter,
        max_duration_ms: options.max_duration.as_millis(),
        shared_state: &state,
        shared_elapsed: &elapsed,
        state_tx: &state_tx,
        audio_cue: &audio_cue,
    };
    let result = daemon_loop(&use_case, &mut signals, &ctx).await;

    // Cleanup (IPC server Drop will clean up resources)
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
    ctx: &DaemonLoopContext<'_>,
) -> bool
where
    R: crate::application::ports::UnboundedRecorder,
    T: crate::application::ports::Transcriber,
    C: crate::application::ports::Clipboard,
    K: crate::application::ports::Keystroke,
    N: crate::application::ports::Notifier,
{
    // Helper to broadcast state updates
    let broadcast_state = |state: DaemonState, elapsed_ms: u64| {
        // Update shared state for status queries
        if let Ok(mut guard) = ctx.shared_state.lock() {
            *guard = state;
        }
        if let Ok(mut guard) = ctx.shared_elapsed.lock() {
            *guard = elapsed_ms;
        }
        // Broadcast to subscribers (ignore if no receivers)
        let _ = ctx.state_tx.send(StateUpdate::new(state, elapsed_ms));
    };

    loop {
        let state = use_case.state().await;
        let elapsed_ms = use_case.elapsed_ms();

        // Update shared state and broadcast
        broadcast_state(state, elapsed_ms);

        // If recording, use timeout for max duration check and periodic broadcasts
        let signal = if state == DaemonState::Recording {
            let remaining_ms = ctx.max_duration_ms.saturating_sub(elapsed_ms);
            if remaining_ms == 0 {
                // Max duration reached
                Some(DaemonSignal::Toggle)
            } else {
                // Use 500ms timeout for periodic state broadcasts during recording
                match timeout(
                    StdDuration::from_millis(remaining_ms.min(500)),
                    signals.recv(),
                )
                .await
                {
                    Ok(sig) => sig,
                    Err(_) => {
                        // Timeout - check if max duration reached
                        if use_case.check_max_duration() {
                            ctx.presenter.warn("Max duration reached, auto-stopping");
                            Some(DaemonSignal::Toggle)
                        } else {
                            // Periodic broadcast during recording - continue loop
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
                ctx.presenter
                    .info(&format!("Processing toggle, state={:?}", current_state));
                match current_state {
                    DaemonState::Idle => {
                        // Start recording
                        if let Err(e) = use_case.start_recording().await {
                            ctx.presenter
                                .error(&format!("Failed to start recording: {}", e));
                            continue;
                        }
                        let _ = ctx.audio_cue.play(AudioCueType::RecordingStart).await;
                        ctx.presenter.daemon_status("Recording...");
                        broadcast_state(DaemonState::Recording, 0);
                    }
                    DaemonState::Recording => {
                        // Stop recording first to get audio size
                        let final_elapsed = use_case.elapsed_ms();
                        match use_case.stop_recording().await {
                            Ok(audio) => {
                                let audio_size = audio.human_readable_size();
                                ctx.presenter
                                    .daemon_status(&format!("Processing ({})...", audio_size));
                                broadcast_state(DaemonState::Processing, final_elapsed);

                                let _ = ctx.audio_cue.play(AudioCueType::RecordingStop).await;

                                // Now transcribe
                                match use_case.transcribe_audio(audio).await {
                                    Ok(output) => {
                                        ctx.presenter.output(&output.text);
                                        ctx.presenter.daemon_status("Idle");
                                        broadcast_state(DaemonState::Idle, 0);
                                    }
                                    Err(e) => {
                                        ctx.presenter
                                            .error(&format!("Transcription failed: {}", e));
                                        ctx.presenter.daemon_status("Idle (error)");
                                        broadcast_state(DaemonState::Idle, 0);
                                    }
                                }
                            }
                            Err(e) => {
                                ctx.presenter
                                    .error(&format!("Failed to stop recording: {}", e));
                                ctx.presenter.daemon_status("Idle (error)");
                                broadcast_state(DaemonState::Idle, 0);
                            }
                        }
                    }
                    DaemonState::Processing => {
                        // Already processing, ignore
                        ctx.presenter.warn("Already processing, please wait");
                    }
                }
            }
            Some(DaemonSignal::Cancel) => {
                let current_state = use_case.state().await;
                ctx.presenter
                    .info(&format!("Processing cancel, state={:?}", current_state));
                if current_state == DaemonState::Recording {
                    if let Err(e) = use_case.cancel().await {
                        ctx.presenter.error(&format!("Failed to cancel: {}", e));
                    } else {
                        let _ = ctx.audio_cue.play(AudioCueType::RecordingCancel).await;
                        ctx.presenter.daemon_status("Recording cancelled");
                        broadcast_state(DaemonState::Idle, 0);
                    }
                } else {
                    ctx.presenter.warn("Not recording, nothing to cancel");
                }
            }
            Some(DaemonSignal::Shutdown) => {
                ctx.presenter.info("Processing shutdown");
                let current_state = use_case.state().await;
                if current_state == DaemonState::Recording {
                    // Cancel any in-progress recording
                    let _ = use_case.cancel().await;
                }
                ctx.presenter.daemon_status("Shutting down...");
                broadcast_state(DaemonState::Idle, 0);
                return true;
            }
            None => {
                // Channel closed
                return false;
            }
        }
    }
}
