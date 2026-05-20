//! Shared composition root for the one-shot and daemon runners.
//!
//! `run_oneshot` and `run_daemon` used to instantiate clipboard, keystroke,
//! notifier, recorder, smart-paste and audio-cue adapters by themselves —
//! roughly 80 lines of identical setup duplicated between the two. This
//! module centralises that setup behind a single [`build_adapters`] entry
//! point so the runners can focus on flow control.
//!
//! The bundle owns trait-objects (`Box<dyn ...>`) for the adapters that have
//! multiple implementations selected at runtime (clipboard, keystroke, smart
//! paste, audio cue) and concrete types where only one is meaningful
//! (`CpalRecorder`, the `Transcriber` enum, `NotifyRustNotifier`).
//!
//! The CLI runners then wrap the bundle in their respective use cases.

use std::sync::Arc;

use crate::application::ports::{AudioCue, Clipboard, Keystroke, Notifier, SmartPaste};
use crate::domain::config::AppConfig;
use crate::infrastructure::{
    create_audio_cue, create_clipboard, create_keystroke, create_notifier, create_recorder,
    create_smart_paste, create_transcriber, CpalRecorder, KeystrokeToolPreference, NoOpKeystroke,
    NoOpSmartPaste, Transcriber,
};

use super::presenter::Presenter;

/// Runtime knobs that affect adapter construction but not the validated
/// [`AppConfig`] (clipboard/keystroke/paste/notify enablement, audio cue,
/// etc.). Sourced from [`TranscribeOptions`](super::args::TranscribeOptions)
/// or [`DaemonOptions`](super::args::DaemonOptions).
#[derive(Debug, Clone)]
pub struct RuntimeOptions {
    pub clipboard: bool,
    pub keystroke: bool,
    pub keystroke_tool: Option<String>,
    pub paste: bool,
    pub audio_cue: bool,
}

impl From<&super::args::TranscribeOptions> for RuntimeOptions {
    fn from(o: &super::args::TranscribeOptions) -> Self {
        Self {
            clipboard: o.clipboard,
            keystroke: o.keystroke,
            keystroke_tool: o.keystroke_tool.clone(),
            paste: o.paste,
            audio_cue: o.audio_cue,
        }
    }
}

impl From<&super::args::DaemonOptions> for RuntimeOptions {
    fn from(o: &super::args::DaemonOptions) -> Self {
        Self {
            clipboard: o.clipboard,
            keystroke: o.keystroke,
            keystroke_tool: o.keystroke_tool.clone(),
            paste: o.paste,
            audio_cue: o.audio_cue,
        }
    }
}

/// Bundle of fully-wired adapters ready to feed into a use case.
pub struct AdapterBundle {
    pub recorder: CpalRecorder,
    pub transcriber: Transcriber,
    pub clipboard: Box<dyn Clipboard>,
    pub keystroke: Box<dyn Keystroke>,
    pub notifier: Box<dyn Notifier>,
    pub smart_paste: Box<dyn SmartPaste>,
    pub audio_cue: Arc<dyn AudioCue>,
}

/// Failure to construct one of the runtime adapters. Variants are split so
/// the caller can pick a sensible exit code (`USAGE_ERROR` for missing
/// credentials, `ERROR` for environmental failures).
#[derive(Debug)]
pub enum BuildError {
    /// Transcriber could not be initialised (missing key, broken token store).
    Transcriber(String),
    /// Smart-paste was requested but not available on this system.
    SmartPaste(String),
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transcriber(s) => write!(f, "{}", s),
            Self::SmartPaste(s) => write!(f, "Paste mode unavailable: {}", s),
        }
    }
}

/// Build the full adapter set for a runner.
///
/// Emits informational lines (`Clipboard: using ...`, `Keystroke: using ...`)
/// through the presenter so the runner doesn't have to repeat them.
pub async fn build_adapters(
    config: &AppConfig,
    opts: &RuntimeOptions,
    presenter: &Presenter,
) -> Result<AdapterBundle, BuildError> {
    let transcriber = create_transcriber(config).map_err(BuildError::Transcriber)?;
    let recorder = create_recorder();
    let notifier = create_notifier();

    let (clipboard, clipboard_tool) = create_clipboard().await;
    if opts.clipboard {
        presenter.info(&format!("Clipboard: using {}", clipboard_tool));
    }

    let preference = opts
        .keystroke_tool
        .as_ref()
        .and_then(|s| s.parse::<KeystrokeToolPreference>().ok())
        .unwrap_or_default();
    let keystroke: Box<dyn Keystroke> = match create_keystroke(preference).await {
        Ok((ks, tool)) => {
            if opts.keystroke {
                presenter.info(&format!("Keystroke: using {}", tool));
            }
            ks
        }
        Err(e) => {
            if opts.keystroke {
                presenter.warn(&format!("Keystroke disabled: {}", e));
            }
            Box::new(NoOpKeystroke::new())
        }
    };

    // Smart-paste is meaningful only on Linux KDE Wayland; on other
    // platforms (or when the flag is off) we plug in the no-op.
    let smart_paste: Box<dyn SmartPaste> = build_smart_paste(opts.paste, presenter).await?;

    let audio_cue: Arc<dyn AudioCue> = Arc::from(create_audio_cue(opts.audio_cue));

    Ok(AdapterBundle {
        recorder,
        transcriber,
        clipboard,
        keystroke,
        notifier,
        smart_paste,
        audio_cue,
    })
}

#[cfg(target_os = "linux")]
async fn build_smart_paste(
    enabled: bool,
    presenter: &Presenter,
) -> Result<Box<dyn SmartPaste>, BuildError> {
    if enabled {
        match create_smart_paste().await {
            Ok(sp) => {
                presenter.info("Paste: using kdotool+wl-copy+ydotool");
                Ok(sp)
            }
            Err(e) => Err(BuildError::SmartPaste(e.to_string())),
        }
    } else {
        Ok(Box::new(NoOpSmartPaste::new()))
    }
}

#[cfg(not(target_os = "linux"))]
async fn build_smart_paste(
    _enabled: bool,
    _presenter: &Presenter,
) -> Result<Box<dyn SmartPaste>, BuildError> {
    // Off-Linux: always plug the no-op; the `paste` flag is accepted but
    // inert (kept for portable CLI surfaces).
    let _ = create_smart_paste; // suppress unused-import warning
    Ok(Box::new(NoOpSmartPaste::new()))
}
