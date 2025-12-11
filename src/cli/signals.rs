//! Signal handlers for one-shot and daemon modes

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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
    /// Toggle recording (SIGUSR1)
    Toggle,
    /// Cancel recording (SIGUSR2)
    Cancel,
    /// Shutdown daemon (SIGINT/SIGTERM)
    Shutdown,
}

/// Daemon signal handler
pub struct DaemonSignalHandler {
    receiver: mpsc::Receiver<DaemonSignal>,
}

impl DaemonSignalHandler {
    /// Create a new daemon signal handler and start listening
    pub async fn new() -> Result<Self, std::io::Error> {
        let (tx, rx) = mpsc::channel(10);

        // Setup SIGUSR1 handler (toggle)
        let tx_usr1 = tx.clone();
        let mut sigusr1 = signal(SignalKind::user_defined1())?;
        tokio::spawn(async move {
            loop {
                sigusr1.recv().await;
                let _ = tx_usr1.send(DaemonSignal::Toggle).await;
            }
        });

        // Setup SIGUSR2 handler (cancel)
        let tx_usr2 = tx.clone();
        let mut sigusr2 = signal(SignalKind::user_defined2())?;
        tokio::spawn(async move {
            loop {
                sigusr2.recv().await;
                let _ = tx_usr2.send(DaemonSignal::Cancel).await;
            }
        });

        // Setup SIGINT handler (shutdown)
        let tx_int = tx.clone();
        let mut sigint = signal(SignalKind::interrupt())?;
        tokio::spawn(async move {
            sigint.recv().await;
            let _ = tx_int.send(DaemonSignal::Shutdown).await;
        });

        // Setup SIGTERM handler (shutdown)
        let tx_term = tx;
        let mut sigterm = signal(SignalKind::terminate())?;
        tokio::spawn(async move {
            sigterm.recv().await;
            let _ = tx_term.send(DaemonSignal::Shutdown).await;
        });

        Ok(Self { receiver: rx })
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
