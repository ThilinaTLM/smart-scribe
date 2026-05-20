//! Daemon app runner

use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::time::Duration as StdDuration;

use tokio::sync::broadcast;
use tokio::time::timeout;

use crate::application::ports::{AudioCue, AudioCueType};
use crate::application::{DaemonConfig, DaemonTranscriptionUseCase};
use crate::domain::config::AppConfig;
use crate::domain::daemon::{DaemonState, StateUpdate};

use super::args::DaemonOptions;
use super::auth_cmd::describe_auth;
use super::exit_codes;
use super::ipc::create_ipc_server;
use super::output::DaemonEvent;
use super::pid_file::{PidFile, PidFileError};
use super::presenter::Presenter;
use super::runtime::{build_adapters, BuildError, RuntimeOptions};
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
    event_tx: &'a broadcast::Sender<DaemonEvent>,
    audio_cue: &'a Arc<dyn AudioCue>,
}

/// Run daemon mode
pub async fn run_daemon(options: DaemonOptions, config: &AppConfig) -> ExitCode {
    let presenter = Presenter::new(options.output);

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
        return ExitCode::from(exit_codes::ERROR);
    }

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

    let enable_paste = options.paste;

    let daemon_config = DaemonConfig {
        max_duration: options.max_duration,
        enable_clipboard: options.clipboard,
        enable_keystroke: options.keystroke,
        enable_paste,
        enable_notify: options.notify,
        warning_sink: Some(presenter.warning_sink()),
    };

    let audio_cue: Arc<dyn AudioCue> = bundle.audio_cue;

    let use_case = DaemonTranscriptionUseCase::new(
        crate::application::UseCaseDeps {
            recorder: bundle.recorder,
            transcriber: bundle.transcriber,
            clipboard: bundle.clipboard,
            keystroke: bundle.keystroke,
            notifier: bundle.notifier,
            smart_paste: bundle.smart_paste,
        },
        daemon_config,
    );

    // Setup signal handler (returns handler + sender for socket server)
    let (mut signals, signal_tx) = match DaemonSignalHandler::new().await {
        Ok(s) => s,
        Err(e) => {
            presenter.error(&format!("Failed to setup signal handler: {}", e));
            return ExitCode::from(exit_codes::ERROR);
        }
    };

    // Setup IPC server (Unix socket on Linux/macOS, named pipe on Windows)
    let mut ipc_server = create_ipc_server();
    let ipc_path = ipc_server.path();

    if let Err(e) = ipc_server.bind() {
        presenter.error(&format!("Failed to bind IPC: {}", e));
        return ExitCode::from(exit_codes::ERROR);
    }

    // Wrap state and elapsed time in Arc<Mutex> for sharing with IPC server
    let state = Arc::new(Mutex::new(DaemonState::Idle));
    let elapsed = Arc::new(Mutex::new(0u64));
    let state_for_ipc = Arc::clone(&state);
    let elapsed_for_ipc = Arc::clone(&elapsed);

    // Create broadcast channels for indicator and external subscribers
    let (state_tx, _state_rx) = broadcast::channel::<StateUpdate>(STATE_BROADCAST_CAPACITY);
    let (event_tx, event_rx) = broadcast::channel::<DaemonEvent>(STATE_BROADCAST_CAPACITY);

    // Spawn indicator thread if enabled (platform-specific UI).
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

    #[cfg(target_os = "windows")]
    if options.indicator {
        let indicator_rx = state_tx.subscribe();
        let signal_tx_for_tray = signal_tx.clone();
        std::thread::spawn(move || {
            if let Err(e) = crate::gui::run_indicator(indicator_rx, signal_tx_for_tray) {
                eprintln!("Indicator error: {} (tray icon unavailable)", e);
            }
        });
        presenter.info("Tray indicator enabled");
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
                event_rx,
            )
            .await;
    });

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
        event_tx: &event_tx,
        audio_cue: &audio_cue,
    };
    let result = daemon_loop(&use_case, &mut signals, &ctx).await;

    // Cleanup (IPC server Drop will clean up resources)
    let _ = pid_file.release();

    if result {
        ExitCode::from(exit_codes::SUCCESS)
    } else {
        ExitCode::from(exit_codes::ERROR)
    }
}

async fn daemon_loop<R, T, C, K, N, P>(
    use_case: &DaemonTranscriptionUseCase<R, T, C, K, N, P>,
    signals: &mut DaemonSignalHandler,
    ctx: &DaemonLoopContext<'_>,
) -> bool
where
    R: crate::application::ports::UnboundedRecorder,
    T: crate::application::ports::Transcriber,
    C: crate::application::ports::Clipboard,
    K: crate::application::ports::Keystroke,
    N: crate::application::ports::Notifier,
    P: crate::application::ports::SmartPaste,
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

        let update = StateUpdate::new(state, elapsed_ms);

        // Broadcast to subscribers (ignore if no receivers)
        let _ = ctx.state_tx.send(update.clone());
        let _ = ctx.event_tx.send(DaemonEvent::from(update));
    };

    let emit_event = |event: DaemonEvent| {
        let _ = ctx.event_tx.send(event);
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
                            emit_event(DaemonEvent::error("start", e.to_string()));
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
                                let audio_size =
                                    super::output::format_audio_size(audio.size_bytes() as u64);
                                ctx.presenter
                                    .daemon_status(&format!("Processing ({})...", audio_size));
                                broadcast_state(DaemonState::Processing, final_elapsed);

                                let _ = ctx.audio_cue.play(AudioCueType::RecordingStop).await;

                                // Now transcribe
                                match use_case.transcribe_audio(audio).await {
                                    Ok(output) => {
                                        if ctx.presenter.is_json() {
                                            let event = DaemonEvent::from(output.clone());
                                            ctx.presenter.output_json(&event);
                                        } else {
                                            ctx.presenter.output(&output.text);
                                        }
                                        emit_event(DaemonEvent::from(output));
                                        ctx.presenter.daemon_status("Idle");
                                        broadcast_state(DaemonState::Idle, 0);
                                    }
                                    Err(e) => {
                                        ctx.presenter
                                            .error(&format!("Transcription failed: {}", e));
                                        emit_event(DaemonEvent::error("transcribe", e.to_string()));
                                        ctx.presenter.daemon_status("Idle (error)");
                                        broadcast_state(DaemonState::Idle, 0);
                                    }
                                }
                            }
                            Err(e) => {
                                ctx.presenter
                                    .error(&format!("Failed to stop recording: {}", e));
                                emit_event(DaemonEvent::error("stop", e.to_string()));
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
                        emit_event(DaemonEvent::error("cancel", e.to_string()));
                    } else {
                        let _ = ctx.audio_cue.play(AudioCueType::RecordingCancel).await;
                        emit_event(DaemonEvent::Cancelled);
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
                emit_event(DaemonEvent::Shutdown);
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
