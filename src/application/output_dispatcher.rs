//! Shared output-action dispatcher.
//!
//! Once a transcript exists, both the one-shot and daemon flows want to:
//!
//! 1. Copy the text to the clipboard (optional).
//! 2. Type the text into the focused window via the keystroke adapter
//!    (optional).
//! 3. Paste the text into the previously captured window via smart paste
//!    (optional).
//!
//! Each step is best-effort: a failure is surfaced through the configured
//! [`WarningSink`](super::WarningSink) and the flow continues. The
//! [`OutputDispatcher`] centralises that logic so the two use cases stay free
//! of duplicated branches and tool-specific handling.
//!
//! The dispatcher does **not** know about notifications. Notifications run
//! at coarser milestones (record start, processing, complete) and are kept
//! in the use cases themselves.
//!
//! Implementation note: the dispatcher takes `&C`, `&K`, `&P` rather than
//! owning the adapters so the use cases keep ownership and we don't burden
//! callers with a second wrapping `Arc`.

use super::ports::{Clipboard, Keystroke, SmartPaste};
use super::{warn, WarningSink};

/// Per-call options selecting which output channels to dispatch.
#[derive(Debug, Clone, Copy, Default)]
pub struct OutputOptions {
    pub clipboard: bool,
    pub keystroke: bool,
    pub paste: bool,
}

/// Outcome of [`OutputDispatcher::dispatch`]: which channels actually
/// succeeded. Channels that were disabled report `false`.
#[derive(Debug, Clone, Copy, Default)]
pub struct OutputResult {
    pub clipboard_copied: bool,
    pub keystroke_sent: bool,
    pub paste_sent: bool,
}

/// Dispatch a transcribed text through the configured clipboard / keystroke /
/// smart-paste adapters. Returns which channels succeeded; failures route
/// through the [`WarningSink`].
///
/// The function is generic over adapter types (rather than dyn-boxed) so we
/// keep the static-dispatch advantages of the use-case types and stay
/// trait-object-free in production builds.
pub async fn dispatch<C, K, P>(
    clipboard: &C,
    keystroke: &K,
    smart_paste: &P,
    text: &str,
    opts: OutputOptions,
    warning_sink: Option<&WarningSink>,
) -> OutputResult
where
    C: Clipboard + ?Sized,
    K: Keystroke + ?Sized,
    P: SmartPaste + ?Sized,
{
    let clipboard_copied = if opts.clipboard {
        match clipboard.copy(text).await {
            Ok(()) => true,
            Err(e) => {
                warn(warning_sink, &format!("clipboard copy failed: {}", e));
                false
            }
        }
    } else {
        false
    };

    let keystroke_sent = if opts.keystroke {
        match keystroke.type_text(text).await {
            Ok(()) => true,
            Err(e) => {
                warn(warning_sink, &format!("keystroke failed: {}", e));
                false
            }
        }
    } else {
        false
    };

    let paste_sent = if opts.paste {
        match smart_paste.paste(text).await {
            Ok(()) => true,
            Err(e) => {
                warn(warning_sink, &format!("smart paste failed: {}", e));
                false
            }
        }
    } else {
        false
    };

    OutputResult {
        clipboard_copied,
        keystroke_sent,
        paste_sent,
    }
}
