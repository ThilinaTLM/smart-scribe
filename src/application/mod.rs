//! Application layer - Use cases and port interfaces
//!
//! Contains the core business operations and trait definitions
//! for external system interactions.

pub mod daemon;
pub mod output_dispatcher;
pub mod ports;
pub mod transcribe;

pub use output_dispatcher::{dispatch as dispatch_output, OutputOptions, OutputResult};

use std::sync::Arc;

/// Callback used by use cases to surface non-fatal warnings to the
/// presentation layer. The application never formats or prints itself.
///
/// Implementations should be cheap to call and must not block; the CLI
/// typically routes messages into [`crate::cli::Presenter::warn`].
pub type WarningSink = Arc<dyn Fn(&str) + Send + Sync>;

/// Helper: dispatch a warning to the sink if one is configured.
pub(crate) fn warn(sink: Option<&WarningSink>, message: &str) {
    if let Some(sink) = sink {
        sink(message);
    }
}

// Re-export use cases
pub use daemon::{DaemonConfig, DaemonError, DaemonOutput, DaemonTranscriptionUseCase};
pub use transcribe::{
    TranscribeCallbacks, TranscribeError, TranscribeInput, TranscribeOutput,
    TranscribeRecordingUseCase,
};

/// Bundle of adapters consumed by the use cases.
///
/// Both [`TranscribeRecordingUseCase`] and [`DaemonTranscriptionUseCase`]
/// used to take six positional arguments in identical order; threading them
/// through a struct removes that boilerplate and keeps the call sites
/// self-documenting.
pub struct UseCaseDeps<R, T, C, K, N, P> {
    pub recorder: R,
    pub transcriber: T,
    pub clipboard: C,
    pub keystroke: K,
    pub notifier: N,
    pub smart_paste: P,
}
