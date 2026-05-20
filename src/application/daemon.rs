//! Daemon transcription use case

use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

use crate::domain::daemon::{DaemonSession, DaemonState, InvalidStateTransition};
use crate::domain::recording::Duration;

use super::output_dispatcher::{dispatch as dispatch_output, OutputOptions};
use super::ports::{
    Clipboard, Keystroke, NotificationIcon, Notifier, RecordingError, SmartPaste, Transcriber,
    TranscriptionError, UnboundedRecorder,
};
use super::{warn, UseCaseDeps, WarningSink};

/// Errors from the daemon use case
#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("Recording failed: {0}")]
    Recording(#[from] RecordingError),

    #[error("Transcription failed: {0}")]
    Transcription(#[from] TranscriptionError),

    #[error("Invalid state transition: {0}")]
    InvalidState(#[from] InvalidStateTransition),
}

/// Configuration for daemon mode
#[derive(Clone)]
pub struct DaemonConfig {
    /// Maximum recording duration (safety limit)
    pub max_duration: Duration,
    /// Whether to copy result to clipboard
    pub enable_clipboard: bool,
    /// Whether to type result into focused window
    pub enable_keystroke: bool,
    /// Whether to use smart paste (Linux KDE Wayland only)
    pub enable_paste: bool,
    /// Whether to show notifications
    pub enable_notify: bool,
    /// Optional callback for non-fatal warnings. CLI plugs the presenter in;
    /// tests leave `None` to discard.
    pub warning_sink: Option<WarningSink>,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            max_duration: Duration::default_max_duration(),
            enable_clipboard: false,
            enable_keystroke: false,
            enable_paste: false,
            enable_notify: false,
            warning_sink: None,
        }
    }
}

impl std::fmt::Debug for DaemonConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DaemonConfig")
            .field("max_duration", &self.max_duration)
            .field("enable_clipboard", &self.enable_clipboard)
            .field("enable_keystroke", &self.enable_keystroke)
            .field("enable_paste", &self.enable_paste)
            .field("enable_notify", &self.enable_notify)
            .field("warning_sink", &self.warning_sink.is_some())
            .finish()
    }
}

/// Output from daemon transcription
#[derive(Debug, Clone)]
pub struct DaemonOutput {
    /// The transcribed text
    pub text: String,
    /// Whether clipboard copy succeeded
    pub clipboard_copied: bool,
    /// Whether keystroke injection succeeded
    pub keystroke_sent: bool,
    /// Whether smart paste succeeded
    pub paste_sent: bool,
    /// Audio file size in bytes. Presentation layer formats it.
    pub audio_size_bytes: u64,
}

/// Daemon transcription use case
pub struct DaemonTranscriptionUseCase<R, T, C, K, N, P>
where
    R: UnboundedRecorder,
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
    session: Arc<Mutex<DaemonSession>>,
    config: DaemonConfig,
}

impl<R, T, C, K, N, P> DaemonTranscriptionUseCase<R, T, C, K, N, P>
where
    R: UnboundedRecorder,
    T: Transcriber,
    C: Clipboard,
    K: Keystroke,
    N: Notifier,
    P: SmartPaste,
{
    /// Create a new daemon use case from a [`UseCaseDeps`] bundle plus a
    /// [`DaemonConfig`].
    pub fn new(deps: UseCaseDeps<R, T, C, K, N, P>, config: DaemonConfig) -> Self {
        Self {
            recorder: deps.recorder,
            transcriber: deps.transcriber,
            clipboard: deps.clipboard,
            keystroke: deps.keystroke,
            notifier: deps.notifier,
            smart_paste: deps.smart_paste,
            session: Arc::new(Mutex::new(DaemonSession::new())),
            config,
        }
    }

    /// Get current daemon state
    pub async fn state(&self) -> DaemonState {
        self.session.lock().await.state()
    }

    /// Start recording (toggle from idle).
    ///
    /// Ordering matters here: we run the fallible side-effects (recorder
    /// start) *before* we promise the world that we're recording, so the
    /// observable state stays Idle if anything blows up mid-sequence.
    pub async fn start_recording(&self) -> Result<(), DaemonError> {
        // Pre-flight check so we surface InvalidState early without
        // starting the recorder. We re-verify under the lock below.
        {
            let session = self.session.lock().await;
            if !session.is_idle() {
                return Err(InvalidStateTransition {
                    current_state: session.state(),
                    action: "start recording".to_string(),
                }
                .into());
            }
        }

        // 1. Capture active window for smart paste. Warning-only — if this
        //    fails we still want to record; the user just loses paste.
        if self.config.enable_paste {
            if let Err(e) = self.smart_paste.capture_active_window().await {
                warn(
                    self.config.warning_sink.as_ref(),
                    &format!("failed to capture active window: {}", e),
                );
            }
        }

        // 2. Start the recorder. Fatal failure mode — if this errors the
        //    session never transitions to Recording, so callers see Idle
        //    plus a returned error.
        self.recorder.start().await?;

        // 3. Transition the session. If we lose the race against another
        //    caller we have to roll back the recorder we just started.
        {
            let mut session = self.session.lock().await;
            if let Err(e) = session.start_recording() {
                drop(session);
                let _ = self.recorder.cancel().await;
                return Err(e.into());
            }
        }

        // 4. Notify (best-effort, never fatal).
        if self.config.enable_notify {
            let _ = self
                .notifier
                .notify(
                    "SmartScribe",
                    "Recording started...",
                    NotificationIcon::Recording,
                )
                .await;
        }

        Ok(())
    }

    /// Stop recording and return the audio data.
    ///
    /// Call [`transcribe_audio`](Self::transcribe_audio) afterwards to
    /// complete the transcription. We stop the recorder *before* the state
    /// transition: if the recorder fails the session stays Recording (so
    /// the user can retry / cancel) rather than getting stuck in
    /// Processing with no audio buffer.
    pub async fn stop_recording(
        &self,
    ) -> Result<crate::domain::transcription::AudioData, DaemonError> {
        // Verify pre-state without holding the lock across the stop call.
        {
            let session = self.session.lock().await;
            if !session.is_recording() {
                return Err(InvalidStateTransition {
                    current_state: session.state(),
                    action: "stop recording".to_string(),
                }
                .into());
            }
        }

        let audio = self.recorder.stop().await?;

        {
            let mut session = self.session.lock().await;
            session.stop_recording()?;
        }

        Ok(audio)
    }

    /// Transcribe the audio data and perform output actions
    pub async fn transcribe_audio(
        &self,
        audio: crate::domain::transcription::AudioData,
    ) -> Result<DaemonOutput, DaemonError> {
        let audio_size_bytes = audio.size_bytes() as u64;

        // Notify transcription start
        if self.config.enable_notify {
            let _ = self
                .notifier
                .notify(
                    "SmartScribe",
                    "Transcribing...",
                    NotificationIcon::Processing,
                )
                .await;
        }

        // Transcribe. If this fails we roll back the session to Idle so
        // the daemon doesn't get stuck in Processing forever.
        let text = match self.transcriber.transcribe(&audio).await {
            Ok(t) => t,
            Err(e) => {
                let mut session = self.session.lock().await;
                let _ = session.fail_processing();
                return Err(e.into());
            }
        };

        let opts = OutputOptions {
            clipboard: self.config.enable_clipboard,
            keystroke: self.config.enable_keystroke,
            paste: self.config.enable_paste,
        };
        let result = dispatch_output(
            &self.clipboard,
            &self.keystroke,
            &self.smart_paste,
            &text,
            opts,
            self.config.warning_sink.as_ref(),
        )
        .await;

        // Complete processing
        {
            let mut session = self.session.lock().await;
            session.complete_processing()?;
        }

        // Notify completion
        if self.config.enable_notify {
            let _ = self
                .notifier
                .notify(
                    "SmartScribe",
                    "Transcription complete!",
                    NotificationIcon::Success,
                )
                .await;
        }

        Ok(DaemonOutput {
            text,
            clipboard_copied: result.clipboard_copied,
            keystroke_sent: result.keystroke_sent,
            paste_sent: result.paste_sent,
            audio_size_bytes,
        })
    }

    /// Stop recording and transcribe (convenience method)
    pub async fn stop_and_transcribe(&self) -> Result<DaemonOutput, DaemonError> {
        let audio = self.stop_recording().await?;
        self.transcribe_audio(audio).await
    }

    /// Cancel recording without transcription
    pub async fn cancel(&self) -> Result<(), DaemonError> {
        {
            let mut session = self.session.lock().await;
            session.cancel_recording()?;
        }

        // Cancel the recording
        self.recorder.cancel().await?;

        // Notify cancellation
        if self.config.enable_notify {
            let _ = self
                .notifier
                .notify(
                    "SmartScribe",
                    "Recording cancelled",
                    NotificationIcon::Warning,
                )
                .await;
        }

        Ok(())
    }

    /// Check if recording has exceeded max duration
    pub fn check_max_duration(&self) -> bool {
        let elapsed = self.recorder.elapsed_ms();
        elapsed >= self.config.max_duration.as_millis()
    }

    /// Get elapsed recording time in milliseconds
    pub fn elapsed_ms(&self) -> u64 {
        self.recorder.elapsed_ms()
    }

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        self.recorder.is_recording()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::{
        ClipboardError, KeystrokeError, NotificationError, SmartPasteError,
    };
    use crate::domain::transcription::AudioData;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    struct MockUnboundedRecorder {
        recording: AtomicBool,
        elapsed: AtomicU64,
    }

    impl MockUnboundedRecorder {
        fn new() -> Self {
            Self {
                recording: AtomicBool::new(false),
                elapsed: AtomicU64::new(0),
            }
        }
    }

    #[async_trait]
    impl UnboundedRecorder for MockUnboundedRecorder {
        async fn start(&self) -> Result<(), RecordingError> {
            self.recording.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn stop(&self) -> Result<AudioData, RecordingError> {
            self.recording.store(false, Ordering::SeqCst);
            Ok(AudioData::new(vec![0u8; 100], Default::default()))
        }

        async fn cancel(&self) -> Result<(), RecordingError> {
            self.recording.store(false, Ordering::SeqCst);
            Ok(())
        }

        fn is_recording(&self) -> bool {
            self.recording.load(Ordering::SeqCst)
        }

        fn elapsed_ms(&self) -> u64 {
            self.elapsed.load(Ordering::SeqCst)
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
        ) -> Result<(), NotificationError> {
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
    async fn start_recording_from_idle() {
        let use_case = DaemonTranscriptionUseCase::new(
            UseCaseDeps {
                recorder: MockUnboundedRecorder::new(),
                transcriber: MockTranscriber,
                clipboard: MockClipboard,
                keystroke: MockKeystroke,
                notifier: MockNotifier,
                smart_paste: MockSmartPaste,
            },
            DaemonConfig::default(),
        );

        assert_eq!(use_case.state().await, DaemonState::Idle);
        use_case.start_recording().await.unwrap();
        assert_eq!(use_case.state().await, DaemonState::Recording);
    }

    #[tokio::test]
    async fn full_cycle() {
        let use_case = DaemonTranscriptionUseCase::new(
            UseCaseDeps {
                recorder: MockUnboundedRecorder::new(),
                transcriber: MockTranscriber,
                clipboard: MockClipboard,
                keystroke: MockKeystroke,
                notifier: MockNotifier,
                smart_paste: MockSmartPaste,
            },
            DaemonConfig::default(),
        );

        // Start recording
        use_case.start_recording().await.unwrap();
        assert_eq!(use_case.state().await, DaemonState::Recording);

        // Stop and transcribe
        let output = use_case.stop_and_transcribe().await.unwrap();
        assert_eq!(output.text, "Test transcription");
        assert_eq!(use_case.state().await, DaemonState::Idle);
    }

    #[tokio::test]
    async fn cancel_recording() {
        let use_case = DaemonTranscriptionUseCase::new(
            UseCaseDeps {
                recorder: MockUnboundedRecorder::new(),
                transcriber: MockTranscriber,
                clipboard: MockClipboard,
                keystroke: MockKeystroke,
                notifier: MockNotifier,
                smart_paste: MockSmartPaste,
            },
            DaemonConfig::default(),
        );

        use_case.start_recording().await.unwrap();
        use_case.cancel().await.unwrap();
        assert_eq!(use_case.state().await, DaemonState::Idle);
    }

    #[tokio::test]
    async fn start_recording_from_recording_fails() {
        let use_case = DaemonTranscriptionUseCase::new(
            UseCaseDeps {
                recorder: MockUnboundedRecorder::new(),
                transcriber: MockTranscriber,
                clipboard: MockClipboard,
                keystroke: MockKeystroke,
                notifier: MockNotifier,
                smart_paste: MockSmartPaste,
            },
            DaemonConfig::default(),
        );

        use_case.start_recording().await.unwrap();
        let result = use_case.start_recording().await;
        assert!(result.is_err());
    }

    /// A recorder whose `start` always fails. Used to confirm that the
    /// session stays Idle when the recorder rejects start.
    struct FailingRecorder;

    #[async_trait]
    impl UnboundedRecorder for FailingRecorder {
        async fn start(&self) -> Result<(), RecordingError> {
            Err(RecordingError::StartFailed("simulated failure".to_string()))
        }
        async fn stop(&self) -> Result<AudioData, RecordingError> {
            Err(RecordingError::RecordingFailed("not recording".to_string()))
        }
        async fn cancel(&self) -> Result<(), RecordingError> {
            Ok(())
        }
        fn is_recording(&self) -> bool {
            false
        }
        fn elapsed_ms(&self) -> u64 {
            0
        }
    }

    /// A transcriber whose `transcribe` always fails. Used to confirm the
    /// processing state rolls back to Idle on transcription error.
    struct FailingTranscriber;

    #[async_trait]
    impl Transcriber for FailingTranscriber {
        async fn transcribe(&self, _audio: &AudioData) -> Result<String, TranscriptionError> {
            Err(TranscriptionError::ApiError("simulated".to_string()))
        }
    }

    #[tokio::test]
    async fn start_recording_failure_leaves_session_idle() {
        let use_case = DaemonTranscriptionUseCase::new(
            UseCaseDeps {
                recorder: FailingRecorder,
                transcriber: MockTranscriber,
                clipboard: MockClipboard,
                keystroke: MockKeystroke,
                notifier: MockNotifier,
                smart_paste: MockSmartPaste,
            },
            DaemonConfig::default(),
        );

        assert_eq!(use_case.state().await, DaemonState::Idle);
        let result = use_case.start_recording().await;
        assert!(result.is_err(), "expected start to fail");
        assert_eq!(
            use_case.state().await,
            DaemonState::Idle,
            "session must not transition when recorder fails"
        );
    }

    #[tokio::test]
    async fn transcription_failure_rolls_session_back_to_idle() {
        let use_case = DaemonTranscriptionUseCase::new(
            UseCaseDeps {
                recorder: MockUnboundedRecorder::new(),
                transcriber: FailingTranscriber,
                clipboard: MockClipboard,
                keystroke: MockKeystroke,
                notifier: MockNotifier,
                smart_paste: MockSmartPaste,
            },
            DaemonConfig::default(),
        );

        use_case.start_recording().await.unwrap();
        let result = use_case.stop_and_transcribe().await;
        assert!(result.is_err(), "expected transcribe to fail");
        assert_eq!(
            use_case.state().await,
            DaemonState::Idle,
            "session must roll back to Idle on transcription failure"
        );
    }
}
