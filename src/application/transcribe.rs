//! Transcribe recording use case

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use thiserror::Error;

use crate::domain::recording::Duration;
use crate::domain::transcription::AudioData;

use super::ports::{
    AudioRecorder, Clipboard, ClipboardError, Keystroke, KeystrokeError, NotificationIcon,
    Notifier, ProgressCallback, RecordingError, SmartPaste, Transcriber, TranscriptionError,
    UnboundedRecorder,
};

/// Errors from the transcribe use case
#[derive(Debug, Error)]
pub enum TranscribeError {
    #[error("Recording failed: {0}")]
    Recording(#[from] RecordingError),

    #[error("Transcription failed: {0}")]
    Transcription(#[from] TranscriptionError),
}

/// Input parameters for the transcribe use case
#[derive(Debug, Clone)]
pub struct TranscribeInput {
    /// Recording duration
    pub duration: Duration,
    /// Whether to copy result to clipboard
    pub enable_clipboard: bool,
    /// Whether to type result into focused window
    pub enable_keystroke: bool,
    /// Whether to use smart paste (Linux KDE Wayland only)
    pub enable_paste: bool,
    /// Whether to show notifications
    pub enable_notify: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for TranscribeInput {
    fn default() -> Self {
        Self {
            duration: Duration::default_duration(),
            enable_clipboard: false,
            enable_keystroke: false,
            enable_paste: false,
            enable_notify: false,
        }
    }
}

/// Output from the transcribe use case
#[derive(Debug, Clone)]
pub struct TranscribeOutput {
    /// The transcribed text
    pub text: String,
    /// Whether clipboard copy succeeded (if enabled)
    pub clipboard_copied: bool,
    /// Whether keystroke injection succeeded (if enabled)
    pub keystroke_sent: bool,
    /// Whether smart paste succeeded (if enabled)
    pub paste_sent: bool,
    /// Audio file size in human-readable format
    pub audio_size: String,
}

/// Callbacks for progress and status updates
#[derive(Default)]
#[allow(clippy::type_complexity)]
pub struct TranscribeCallbacks {
    /// Called during recording with (elapsed_ms, total_ms)
    pub on_progress: Option<ProgressCallback>,
    /// Called when recording starts
    pub on_recording_start: Option<Box<dyn Fn() + Send + Sync>>,
    /// Called when recording ends
    pub on_recording_end: Option<Box<dyn Fn(&str) + Send + Sync>>,
    /// Called when transcription starts
    pub on_transcribing_start: Option<Box<dyn Fn() + Send + Sync>>,
    /// Called when transcription ends
    pub on_transcribing_end: Option<Box<dyn Fn() + Send + Sync>>,
}

/// One-shot transcription use case
pub struct TranscribeRecordingUseCase<R, T, C, K, N, P>
where
    R: AudioRecorder,
    T: Transcriber,
    C: Clipboard,
    K: Keystroke,
    N: Notifier,
    P: SmartPaste,
{
    recorder: R,
    transcriber: T,
    clipboard: C,
    keystroke: K,
    notifier: N,
    smart_paste: P,
    stop_flag: Arc<AtomicBool>,
}

impl<R, T, C, K, N, P> TranscribeRecordingUseCase<R, T, C, K, N, P>
where
    R: AudioRecorder,
    T: Transcriber,
    C: Clipboard,
    K: Keystroke,
    N: Notifier,
    P: SmartPaste,
{
    /// Create a new use case instance
    pub fn new(
        recorder: R,
        transcriber: T,
        clipboard: C,
        keystroke: K,
        notifier: N,
        smart_paste: P,
    ) -> Self {
        Self {
            recorder,
            transcriber,
            clipboard,
            keystroke,
            notifier,
            smart_paste,
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get the stop flag for external signal handling
    pub fn stop_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.stop_flag)
    }

    /// Signal to stop recording early
    pub fn stop_early(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
    }

    /// Execute the transcription workflow
    pub async fn execute(
        &self,
        input: TranscribeInput,
        callbacks: TranscribeCallbacks,
    ) -> Result<TranscribeOutput, TranscribeError> {
        // Reset stop flag
        self.stop_flag.store(false, Ordering::SeqCst);

        self.prepare_recording(&input, &callbacks, true).await;

        // Record audio
        let audio = self
            .recorder
            .record(input.duration, callbacks.on_progress.clone())
            .await?;

        self.finalize_recording(&input, &callbacks, audio).await
    }

    async fn prepare_recording(
        &self,
        input: &TranscribeInput,
        callbacks: &TranscribeCallbacks,
        include_duration_in_notification: bool,
    ) {
        // Capture active window for smart paste (before recording starts)
        if input.enable_paste {
            if let Err(e) = self.smart_paste.capture_active_window().await {
                eprintln!("Warning: failed to capture active window: {}", e);
            }
        }

        // Notify recording start
        if input.enable_notify {
            let body = if include_duration_in_notification {
                format!("Recording for {}...", input.duration)
            } else {
                "Recording started...".to_string()
            };
            let _ = self
                .notifier
                .notify("SmartScribe", &body, NotificationIcon::Recording)
                .await;
        }

        if let Some(ref cb) = callbacks.on_recording_start {
            cb();
        }
    }

    async fn finalize_recording(
        &self,
        input: &TranscribeInput,
        callbacks: &TranscribeCallbacks,
        audio: AudioData,
    ) -> Result<TranscribeOutput, TranscribeError> {
        let audio_size = audio.human_readable_size();

        if let Some(ref cb) = callbacks.on_recording_end {
            cb(&audio_size);
        }

        self.transcribe_audio(input, callbacks, audio).await
    }

    pub async fn transcribe_audio(
        &self,
        input: &TranscribeInput,
        callbacks: &TranscribeCallbacks,
        audio: AudioData,
    ) -> Result<TranscribeOutput, TranscribeError> {
        let audio_size = audio.human_readable_size();

        // Notify transcription start
        if input.enable_notify {
            let _ = self
                .notifier
                .notify(
                    "SmartScribe",
                    "Transcribing...",
                    NotificationIcon::Processing,
                )
                .await;
        }

        if let Some(ref cb) = callbacks.on_transcribing_start {
            cb();
        }

        // Transcribe
        let text = self.transcriber.transcribe(&audio).await?;

        if let Some(ref cb) = callbacks.on_transcribing_end {
            cb();
        }

        // Perform output actions (non-fatal)
        let clipboard_copied = if input.enable_clipboard {
            match self.clipboard.copy(&text).await {
                Ok(()) => true,
                Err(ClipboardError::WlCopyNotFound) => {
                    eprintln!("Warning: wl-copy not found, skipping clipboard");
                    false
                }
                Err(e) => {
                    eprintln!("Warning: clipboard copy failed: {}", e);
                    false
                }
            }
        } else {
            false
        };

        let keystroke_sent = if input.enable_keystroke {
            match self.keystroke.type_text(&text).await {
                Ok(()) => true,
                Err(KeystrokeError::NoToolAvailable) => {
                    eprintln!("Warning: no keystroke tool available, skipping keystroke");
                    false
                }
                Err(KeystrokeError::YdotoolNotAvailable) => {
                    eprintln!("Warning: ydotool not available, skipping keystroke");
                    false
                }
                Err(KeystrokeError::WtypeNotFound) => {
                    eprintln!("Warning: wtype not found, skipping keystroke");
                    false
                }
                Err(KeystrokeError::XdotoolNotFound) => {
                    eprintln!("Warning: xdotool not found, skipping keystroke");
                    false
                }
                Err(e) => {
                    eprintln!("Warning: keystroke failed: {}", e);
                    false
                }
            }
        } else {
            false
        };

        // Smart paste: paste into captured window via clipboard
        let paste_sent = if input.enable_paste {
            match self.smart_paste.paste(&text).await {
                Ok(()) => true,
                Err(e) => {
                    eprintln!("Warning: smart paste failed: {}", e);
                    false
                }
            }
        } else {
            false
        };

        // Notify completion
        if input.enable_notify {
            let _ = self
                .notifier
                .notify(
                    "SmartScribe",
                    "Transcription complete!",
                    NotificationIcon::Success,
                )
                .await;
        }

        Ok(TranscribeOutput {
            text,
            clipboard_copied,
            keystroke_sent,
            paste_sent,
            audio_size,
        })
    }
}

impl<R, T, C, K, N, P> TranscribeRecordingUseCase<R, T, C, K, N, P>
where
    R: AudioRecorder + UnboundedRecorder,
    T: Transcriber,
    C: Clipboard,
    K: Keystroke,
    N: Notifier,
    P: SmartPaste,
{
    /// Start an unbounded recording session for foreground mode.
    pub async fn start_recording(
        &self,
        input: &TranscribeInput,
        callbacks: &TranscribeCallbacks,
    ) -> Result<(), TranscribeError> {
        self.stop_flag.store(false, Ordering::SeqCst);
        self.prepare_recording(input, callbacks, false).await;
        self.recorder.start().await?;
        Ok(())
    }

    /// Stop an unbounded recording session and return the captured audio.
    pub async fn stop_recording(&self) -> Result<AudioData, TranscribeError> {
        Ok(self.recorder.stop().await?)
    }

    /// Cancel an in-progress unbounded recording session.
    pub async fn cancel_recording(&self) -> Result<(), TranscribeError> {
        self.recorder.cancel().await?;
        Ok(())
    }

    /// Complete transcription for audio captured by foreground mode.
    pub async fn finalize_dynamic_recording(
        &self,
        input: &TranscribeInput,
        callbacks: &TranscribeCallbacks,
        audio: AudioData,
    ) -> Result<TranscribeOutput, TranscribeError> {
        self.finalize_recording(input, callbacks, audio).await
    }

    /// Get elapsed recording time in milliseconds.
    pub fn elapsed_ms(&self) -> u64 {
        self.recorder.elapsed_ms()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::SmartPasteError;
    use crate::domain::transcription::AudioData;
    use async_trait::async_trait;

    // Mock implementations for testing
    struct MockRecorder;

    #[async_trait]
    impl AudioRecorder for MockRecorder {
        async fn record(
            &self,
            _duration: Duration,
            _on_progress: Option<ProgressCallback>,
        ) -> Result<AudioData, RecordingError> {
            Ok(AudioData::new(vec![0u8; 100], Default::default()))
        }
    }

    struct MockTranscriber;

    #[async_trait]
    impl Transcriber for MockTranscriber {
        async fn transcribe(&self, _audio: &AudioData) -> Result<String, TranscriptionError> {
            Ok("Test transcription".to_string())
        }
    }

    struct MockClipboard;

    #[async_trait]
    impl Clipboard for MockClipboard {
        async fn copy(&self, _text: &str) -> Result<(), ClipboardError> {
            Ok(())
        }
    }

    struct MockKeystroke;

    #[async_trait]
    impl Keystroke for MockKeystroke {
        async fn type_text(&self, _text: &str) -> Result<(), KeystrokeError> {
            Ok(())
        }
    }

    struct MockNotifier;

    #[async_trait]
    impl Notifier for MockNotifier {
        async fn notify(
            &self,
            _title: &str,
            _message: &str,
            _icon: NotificationIcon,
        ) -> Result<(), super::super::ports::NotificationError> {
            Ok(())
        }
    }

    struct MockSmartPaste;

    #[async_trait]
    impl SmartPaste for MockSmartPaste {
        async fn capture_active_window(&self) -> Result<(), SmartPasteError> {
            Ok(())
        }

        async fn paste(&self, _text: &str) -> Result<(), SmartPasteError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn execute_returns_transcription() {
        let use_case = TranscribeRecordingUseCase::new(
            MockRecorder,
            MockTranscriber,
            MockClipboard,
            MockKeystroke,
            MockNotifier,
            MockSmartPaste,
        );

        let input = TranscribeInput::default();
        let callbacks = TranscribeCallbacks::default();

        let output = use_case.execute(input, callbacks).await.unwrap();
        assert_eq!(output.text, "Test transcription");
        assert!(!output.clipboard_copied); // Not enabled
        assert!(!output.keystroke_sent); // Not enabled
    }

    #[tokio::test]
    async fn execute_with_clipboard_enabled() {
        let use_case = TranscribeRecordingUseCase::new(
            MockRecorder,
            MockTranscriber,
            MockClipboard,
            MockKeystroke,
            MockNotifier,
            MockSmartPaste,
        );

        let input = TranscribeInput {
            enable_clipboard: true,
            ..Default::default()
        };
        let callbacks = TranscribeCallbacks::default();

        let output = use_case.execute(input, callbacks).await.unwrap();
        assert!(output.clipboard_copied);
    }

    #[tokio::test]
    async fn execute_with_keystroke_enabled() {
        let use_case = TranscribeRecordingUseCase::new(
            MockRecorder,
            MockTranscriber,
            MockClipboard,
            MockKeystroke,
            MockNotifier,
            MockSmartPaste,
        );

        let input = TranscribeInput {
            enable_keystroke: true,
            ..Default::default()
        };
        let callbacks = TranscribeCallbacks::default();

        let output = use_case.execute(input, callbacks).await.unwrap();
        assert!(output.keystroke_sent);
    }
}
