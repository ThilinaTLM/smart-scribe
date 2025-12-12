//! Signal handlers for one-shot and daemon modes

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use colored::Colorize;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;

/// Shutdown signal for one-shot mode
pub struct ShutdownSignal {
    shutdown: Arc<AtomicBool>,
}

impl ShutdownSignal {
    /// Create a new shutdown signal handler
    pub fn new() -> Self {
        Self {
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get a clone of the shutdown flag
    pub fn flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.shutdown)
    }

    /// Check if shutdown was requested
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    /// Setup signal handler
    pub async fn setup(&self) -> Result<(), std::io::Error> {
        let shutdown = Arc::clone(&self.shutdown);

        // Handle SIGINT (Ctrl+C)
        let mut sigint = signal(SignalKind::interrupt())?;
        tokio::spawn(async move {
            sigint.recv().await;
            shutdown.store(true, Ordering::SeqCst);
        });

        Ok(())
    }
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self::new()
    }
}

/// Daemon signals
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonSignal {
    /// Toggle recording
    Toggle,
    /// Cancel recording
    Cancel,
    /// Shutdown daemon (SIGINT/SIGTERM)
    Shutdown,
}

/// Daemon signal handler
///
/// Handles OS shutdown signals (SIGINT/SIGTERM) and provides a channel
/// for receiving daemon commands from other sources (e.g., socket server).
pub struct DaemonSignalHandler {
    receiver: mpsc::Receiver<DaemonSignal>,
}

impl DaemonSignalHandler {
    /// Create a new daemon signal handler and start listening for shutdown signals.
    ///
    /// Returns the handler and a sender that can be used by other sources
    /// (like a socket server) to send commands to the daemon loop.
    pub async fn new() -> Result<(Self, mpsc::Sender<DaemonSignal>), std::io::Error> {
        let (tx, rx) = mpsc::channel(10);

        // Setup SIGINT handler (shutdown)
        let tx_int = tx.clone();
        let mut sigint = signal(SignalKind::interrupt())?;
        tokio::spawn(async move {
            sigint.recv().await;
            eprintln!("{} Received SIGINT (shutdown)", "↓".cyan());
            let _ = tx_int.send(DaemonSignal::Shutdown).await;
        });

        // Setup SIGTERM handler (shutdown)
        let tx_term = tx.clone();
        let mut sigterm = signal(SignalKind::terminate())?;
        tokio::spawn(async move {
            sigterm.recv().await;
            eprintln!("{} Received SIGTERM (shutdown)", "↓".cyan());
            let _ = tx_term.send(DaemonSignal::Shutdown).await;
        });

        Ok((Self { receiver: rx }, tx))
    }

    /// Wait for the next signal
    pub async fn recv(&mut self) -> Option<DaemonSignal> {
        self.receiver.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shutdown_signal_default_is_false() {
        let signal = ShutdownSignal::new();
        assert!(!signal.is_shutdown());
    }

    #[test]
    fn shutdown_signal_flag_can_be_set() {
        let signal = ShutdownSignal::new();
        let flag = signal.flag();
        flag.store(true, Ordering::SeqCst);
        assert!(signal.is_shutdown());
    }

    #[test]
    fn daemon_signal_equality() {
        assert_eq!(DaemonSignal::Toggle, DaemonSignal::Toggle);
        assert_ne!(DaemonSignal::Toggle, DaemonSignal::Cancel);
    }
}
