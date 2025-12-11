//! Daemon transcription use case

use std::sync::Arc;
use tokio::sync::Mutex;
use thiserror::Error;

use crate::domain::daemon::{DaemonSession, DaemonState, InvalidStateTransition};
use crate::domain::recording::Duration;
use crate::domain::transcription::{DomainId, SystemPrompt};

use super::ports::{
    Clipboard, ClipboardError, Keystroke, KeystrokeError,
    Notifier, NotificationIcon, RecordingError,
    Transcriber, TranscriptionError, UnboundedRecorder,
};

/// Errors from the daemon use case
#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("Recording failed: {0}")]
    Recording(#[from] RecordingError),

    #[error("Transcription failed: {0}")]
    Transcription(#[from] TranscriptionError),

    #[error("Invalid state transition: {0}")]
    InvalidState(#[from] InvalidStateTransition),

    #[error("Missing API key. Set GEMINI_API_KEY or configure via 'smart-scribe config set api_key <key>'")]
    MissingApiKey,
}

/// Configuration for daemon mode
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// Domain for transcription context
    pub domain: DomainId,
    /// Maximum recording duration (safety limit)
    pub max_duration: Duration,
    /// Whether to copy result to clipboard
    pub enable_clipboard: bool,
    /// Whether to type result into focused window
    pub enable_keystroke: bool,
    /// Whether to show notifications
    pub enable_notify: bool,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            domain: DomainId::default(),
            max_duration: Duration::default_max_duration(),
            enable_clipboard: false,
            enable_keystroke: false,
            enable_notify: false,
        }
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
    /// Audio file size
    pub audio_size: String,
}

/// Daemon transcription use case
pub struct DaemonTranscriptionUseCase<R, T, C, K, N>
where
    R: UnboundedRecorder,
    T: Transcriber,
    C: Clipboard,
    K: Keystroke,
    N: Notifier,
{
    recorder: R,
    transcriber: T,
    clipboard: C,
    keystroke: K,
    notifier: N,
    session: Arc<Mutex<DaemonSession>>,
    config: DaemonConfig,
}

impl<R, T, C, K, N> DaemonTranscriptionUseCase<R, T, C, K, N>
where
    R: UnboundedRecorder,
    T: Transcriber,
    C: Clipboard,
    K: Keystroke,
    N: Notifier,
{
    /// Create a new daemon use case instance
    pub fn new(
        recorder: R,
        transcriber: T,
        clipboard: C,
        keystroke: K,
        notifier: N,
        config: DaemonConfig,
    ) -> Self {
        Self {
            recorder,
            transcriber,
            clipboard,
            keystroke,
            notifier,
            session: Arc::new(Mutex::new(DaemonSession::new())),
            config,
        }
    }

    /// Get current daemon state
    pub async fn state(&self) -> DaemonState {
        self.session.lock().await.state()
    }

    /// Start recording (toggle from idle)
    pub async fn start_recording(&self) -> Result<(), DaemonError> {
        {
            let mut session = self.session.lock().await;
            session.start_recording()?;
        }

        // Notify recording start
        if self.config.enable_notify {
            let _ = self.notifier.notify(
                "SmartScribe",
                "Recording started...",
                NotificationIcon::Recording,
            ).await;
        }

        // Start the actual recording
        self.recorder.start().await?;

        Ok(())
    }

    /// Stop recording and return the audio data
    /// Call `transcribe_audio` afterwards to complete the transcription
    pub async fn stop_recording(&self) -> Result<crate::domain::transcription::AudioData, DaemonError> {
        // Transition to processing state
        {
            let mut session = self.session.lock().await;
            session.stop_recording()?;
        }

        // Stop recording and get audio
        let audio = self.recorder.stop().await?;
        Ok(audio)
    }

    /// Transcribe the audio data and perform output actions
    pub async fn transcribe_audio(&self, audio: crate::domain::transcription::AudioData) -> Result<DaemonOutput, DaemonError> {
        let audio_size = audio.human_readable_size();

        // Notify transcription start
        if self.config.enable_notify {
            let _ = self.notifier.notify(
                "SmartScribe",
                "Transcribing...",
                NotificationIcon::Processing,
            ).await;
        }

        // Build prompt and transcribe
        let prompt = SystemPrompt::build(self.config.domain);
        let text = self.transcriber.transcribe(&audio, &prompt).await?;

        // Perform output actions
        let clipboard_copied = if self.config.enable_clipboard {
            match self.clipboard.copy(&text).await {
                Ok(()) => true,
                Err(ClipboardError::WlCopyNotFound) => false,
                Err(_) => false,
            }
        } else {
            false
        };

        let keystroke_sent = if self.config.enable_keystroke {
            match self.keystroke.type_text(&text).await {
                Ok(()) => true,
                Err(KeystrokeError::XdotoolNotFound) => false,
                Err(_) => false,
            }
        } else {
            false
        };

        // Complete processing
        {
            let mut session = self.session.lock().await;
            session.complete_processing()?;
        }

        // Notify completion
        if self.config.enable_notify {
            let _ = self.notifier.notify(
                "SmartScribe",
                "Transcription complete!",
                NotificationIcon::Success,
            ).await;
        }

        Ok(DaemonOutput {
            text,
            clipboard_copied,
            keystroke_sent,
            audio_size,
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
            let _ = self.notifier.notify(
                "SmartScribe",
                "Recording cancelled",
                NotificationIcon::Warning,
            ).await;
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
    use crate::domain::transcription::AudioData;
    use crate::application::ports::NotificationError;
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
        async fn transcribe(
            &self,
            _audio: &AudioData,
            _prompt: &SystemPrompt,
        ) -> Result<String, TranscriptionError> {
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

    #[tokio::test]
    async fn start_recording_from_idle() {
        let use_case = DaemonTranscriptionUseCase::new(
            MockUnboundedRecorder::new(),
            MockTranscriber,
            MockClipboard,
            MockKeystroke,
            MockNotifier,
            DaemonConfig::default(),
        );

        assert_eq!(use_case.state().await, DaemonState::Idle);
        use_case.start_recording().await.unwrap();
        assert_eq!(use_case.state().await, DaemonState::Recording);
    }

    #[tokio::test]
    async fn full_cycle() {
        let use_case = DaemonTranscriptionUseCase::new(
            MockUnboundedRecorder::new(),
            MockTranscriber,
            MockClipboard,
            MockKeystroke,
            MockNotifier,
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
            MockUnboundedRecorder::new(),
            MockTranscriber,
            MockClipboard,
            MockKeystroke,
            MockNotifier,
            DaemonConfig::default(),
        );

        use_case.start_recording().await.unwrap();
        use_case.cancel().await.unwrap();
        assert_eq!(use_case.state().await, DaemonState::Idle);
    }

    #[tokio::test]
    async fn start_recording_from_recording_fails() {
        let use_case = DaemonTranscriptionUseCase::new(
            MockUnboundedRecorder::new(),
            MockTranscriber,
            MockClipboard,
            MockKeystroke,
            MockNotifier,
            DaemonConfig::default(),
        );

        use_case.start_recording().await.unwrap();
        let result = use_case.start_recording().await;
        assert!(result.is_err());
    }
}
