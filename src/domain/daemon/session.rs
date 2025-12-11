//! Daemon session state machine

use std::fmt;
use thiserror::Error;

/// Daemon states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DaemonState {
    #[default]
    Idle,
    Recording,
    Processing,
}

impl DaemonState {
    /// Get the string representation
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Recording => "recording",
            Self::Processing => "processing",
        }
    }
}

impl fmt::Display for DaemonState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error when an invalid state transition is attempted
#[derive(Debug, Clone, Error)]
#[error("Invalid state transition: cannot {action} while in {current_state} state")]
pub struct InvalidStateTransition {
    pub current_state: DaemonState,
    pub action: String,
}

/// Daemon session entity.
/// Manages state transitions for the daemon lifecycle.
///
/// State machine:
///   IDLE -> RECORDING (start_recording)
///   RECORDING -> PROCESSING (stop_recording)
///   RECORDING -> IDLE (cancel_recording)
///   PROCESSING -> IDLE (complete_processing)
#[derive(Debug, Default)]
pub struct DaemonSession {
    state: DaemonState,
}

impl DaemonSession {
    /// Create a new daemon session in idle state
    pub fn new() -> Self {
        Self {
            state: DaemonState::Idle,
        }
    }

    /// Get the current state
    pub fn state(&self) -> DaemonState {
        self.state
    }

    /// Check if currently idle
    pub fn is_idle(&self) -> bool {
        self.state == DaemonState::Idle
    }

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        self.state == DaemonState::Recording
    }

    /// Check if currently processing
    pub fn is_processing(&self) -> bool {
        self.state == DaemonState::Processing
    }

    /// Transition from IDLE to RECORDING
    pub fn start_recording(&mut self) -> Result<(), InvalidStateTransition> {
        if self.state != DaemonState::Idle {
            return Err(InvalidStateTransition {
                current_state: self.state,
                action: "start recording".to_string(),
            });
        }
        self.state = DaemonState::Recording;
        Ok(())
    }

    /// Transition from RECORDING to PROCESSING
    pub fn stop_recording(&mut self) -> Result<(), InvalidStateTransition> {
        if self.state != DaemonState::Recording {
            return Err(InvalidStateTransition {
                current_state: self.state,
                action: "stop recording".to_string(),
            });
        }
        self.state = DaemonState::Processing;
        Ok(())
    }

    /// Transition from RECORDING to IDLE (cancel without transcription)
    pub fn cancel_recording(&mut self) -> Result<(), InvalidStateTransition> {
        if self.state != DaemonState::Recording {
            return Err(InvalidStateTransition {
                current_state: self.state,
                action: "cancel recording".to_string(),
            });
        }
        self.state = DaemonState::Idle;
        Ok(())
    }

    /// Transition from PROCESSING to IDLE
    pub fn complete_processing(&mut self) -> Result<(), InvalidStateTransition> {
        if self.state != DaemonState::Processing {
            return Err(InvalidStateTransition {
                current_state: self.state,
                action: "complete processing".to_string(),
            });
        }
        self.state = DaemonState::Idle;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_session_is_idle() {
        let session = DaemonSession::new();
        assert!(session.is_idle());
        assert!(!session.is_recording());
        assert!(!session.is_processing());
    }

    #[test]
    fn start_recording_from_idle() {
        let mut session = DaemonSession::new();
        assert!(session.start_recording().is_ok());
        assert!(session.is_recording());
    }

    #[test]
    fn start_recording_from_recording_fails() {
        let mut session = DaemonSession::new();
        session.start_recording().unwrap();

        let err = session.start_recording().unwrap_err();
        assert_eq!(err.current_state, DaemonState::Recording);
        assert!(err.action.contains("start recording"));
    }

    #[test]
    fn start_recording_from_processing_fails() {
        let mut session = DaemonSession::new();
        session.start_recording().unwrap();
        session.stop_recording().unwrap();

        let err = session.start_recording().unwrap_err();
        assert_eq!(err.current_state, DaemonState::Processing);
    }

    #[test]
    fn stop_recording_from_recording() {
        let mut session = DaemonSession::new();
        session.start_recording().unwrap();

        assert!(session.stop_recording().is_ok());
        assert!(session.is_processing());
    }

    #[test]
    fn stop_recording_from_idle_fails() {
        let mut session = DaemonSession::new();

        let err = session.stop_recording().unwrap_err();
        assert_eq!(err.current_state, DaemonState::Idle);
    }

    #[test]
    fn cancel_recording_from_recording() {
        let mut session = DaemonSession::new();
        session.start_recording().unwrap();

        assert!(session.cancel_recording().is_ok());
        assert!(session.is_idle());
    }

    #[test]
    fn cancel_recording_from_idle_fails() {
        let mut session = DaemonSession::new();

        let err = session.cancel_recording().unwrap_err();
        assert_eq!(err.current_state, DaemonState::Idle);
    }

    #[test]
    fn complete_processing_from_processing() {
        let mut session = DaemonSession::new();
        session.start_recording().unwrap();
        session.stop_recording().unwrap();

        assert!(session.complete_processing().is_ok());
        assert!(session.is_idle());
    }

    #[test]
    fn complete_processing_from_recording_fails() {
        let mut session = DaemonSession::new();
        session.start_recording().unwrap();

        let err = session.complete_processing().unwrap_err();
        assert_eq!(err.current_state, DaemonState::Recording);
    }

    #[test]
    fn full_cycle() {
        let mut session = DaemonSession::new();
        assert!(session.is_idle());

        session.start_recording().unwrap();
        assert!(session.is_recording());

        session.stop_recording().unwrap();
        assert!(session.is_processing());

        session.complete_processing().unwrap();
        assert!(session.is_idle());

        // Can start another cycle
        session.start_recording().unwrap();
        assert!(session.is_recording());
    }

    #[test]
    fn state_display() {
        assert_eq!(DaemonState::Idle.to_string(), "idle");
        assert_eq!(DaemonState::Recording.to_string(), "recording");
        assert_eq!(DaemonState::Processing.to_string(), "processing");
    }

    #[test]
    fn error_display() {
        let err = InvalidStateTransition {
            current_state: DaemonState::Processing,
            action: "start recording".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("start recording"));
        assert!(msg.contains("processing"));
    }
}
