//! Unix Domain Socket communication for daemon control
//!
//! Used on Linux and macOS.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{broadcast, mpsc};

use super::{ElapsedFn, IpcClient, IpcServer, StateFn};
use crate::cli::signals::DaemonSignal;
use crate::domain::daemon::{DaemonState, StateUpdate};

/// Socket path resolver
#[derive(Debug, Clone)]
pub struct SocketPath {
    path: PathBuf,
}

impl SocketPath {
    /// Create socket path, preferring XDG_RUNTIME_DIR
    pub fn new() -> Self {
        let path = std::env::var("XDG_RUNTIME_DIR")
            .map(|dir| PathBuf::from(dir).join("smart-scribe.sock"))
            .unwrap_or_else(|_| std::env::temp_dir().join("smart-scribe.sock"));
        Self { path }
    }

    /// Get the socket path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Check if socket file exists
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Remove socket file if it exists
    pub fn cleanup(&self) -> io::Result<()> {
        if self.path.exists() {
            std::fs::remove_file(&self.path)?;
        }
        Ok(())
    }
}

impl Default for SocketPath {
    fn default() -> Self {
        Self::new()
    }
}

/// Unix Domain Socket server for daemon commands
pub struct UnixSocketServer {
    socket_path: SocketPath,
    listener: Option<UnixListener>,
}

impl UnixSocketServer {
    /// Create a new socket server
    pub fn new(socket_path: SocketPath) -> Self {
        Self {
            socket_path,
            listener: None,
        }
    }
}

impl Drop for UnixSocketServer {
    fn drop(&mut self) {
        self.cleanup();
    }
}

#[async_trait]
impl IpcServer for UnixSocketServer {
    fn bind(&mut self) -> io::Result<()> {
        // Remove stale socket file if it exists
        self.socket_path.cleanup()?;

        // Bind listener
        let listener = UnixListener::bind(self.socket_path.path())?;
        self.listener = Some(listener);
        Ok(())
    }

    fn path(&self) -> String {
        self.socket_path.path().to_string_lossy().to_string()
    }

    async fn run(
        &self,
        tx: mpsc::Sender<DaemonSignal>,
        state_fn: StateFn,
        elapsed_fn: ElapsedFn,
        state_rx: broadcast::Receiver<StateUpdate>,
    ) -> io::Result<()> {
        let listener = self
            .listener
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "Socket not bound"))?;

        // Wrap functions in Arc for sharing across connections
        let state_fn = Arc::new(state_fn);
        let elapsed_fn = Arc::new(elapsed_fn);

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let tx = tx.clone();
                    let state_fn = Arc::clone(&state_fn);
                    let elapsed_fn = Arc::clone(&elapsed_fn);
                    let state_rx = state_rx.resubscribe();
                    tokio::spawn(async move {
                        if let Err(e) =
                            handle_connection(stream, tx, state_fn, elapsed_fn, state_rx).await
                        {
                            // Don't log BrokenPipe errors - they're expected when clients disconnect
                            if e.kind() != io::ErrorKind::BrokenPipe {
                                eprintln!("Socket connection error: {}", e);
                            }
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Socket accept error: {}", e);
                }
            }
        }
    }

    fn cleanup(&self) {
        let _ = self.socket_path.cleanup();
    }
}

/// Handle a single client connection
async fn handle_connection(
    stream: UnixStream,
    tx: mpsc::Sender<DaemonSignal>,
    state_fn: Arc<StateFn>,
    elapsed_fn: Arc<ElapsedFn>,
    mut state_rx: broadcast::Receiver<StateUpdate>,
) -> io::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    // Read command
    reader.read_line(&mut line).await?;
    let cmd = line.trim();

    // Process command
    match cmd {
        "toggle" => {
            let _ = tx.send(DaemonSignal::Toggle).await;
            writer.write_all(b"ok\n").await?;
            writer.flush().await?;
        }
        "cancel" => {
            let _ = tx.send(DaemonSignal::Cancel).await;
            writer.write_all(b"ok\n").await?;
            writer.flush().await?;
        }
        "status" => {
            let current_state = state_fn();
            let response = match current_state {
                DaemonState::Idle => "idle\n",
                DaemonState::Recording => "recording\n",
                DaemonState::Processing => "processing\n",
            };
            writer.write_all(response.as_bytes()).await?;
            writer.flush().await?;
        }
        "subscribe" => {
            // Send initial state
            let initial = StateUpdate::new(state_fn(), elapsed_fn());
            writer.write_all(initial.to_json_line().as_bytes()).await?;
            writer.flush().await?;

            // Stream state updates until client disconnects
            loop {
                match state_rx.recv().await {
                    Ok(update) => {
                        if let Err(e) = writer.write_all(update.to_json_line().as_bytes()).await {
                            // Client disconnected
                            if e.kind() == io::ErrorKind::BrokenPipe {
                                break;
                            }
                            return Err(e);
                        }
                        if let Err(e) = writer.flush().await {
                            if e.kind() == io::ErrorKind::BrokenPipe {
                                break;
                            }
                            return Err(e);
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        // Subscriber lagged behind, send current state to catch up
                        let current = StateUpdate::new(state_fn(), elapsed_fn());
                        if let Err(e) = writer.write_all(current.to_json_line().as_bytes()).await {
                            if e.kind() == io::ErrorKind::BrokenPipe {
                                break;
                            }
                            return Err(e);
                        }
                    }
                }
            }
        }
        _ => {
            writer.write_all(b"error: unknown command\n").await?;
            writer.flush().await?;
        }
    }

    Ok(())
}

/// Unix Domain Socket client for sending commands to daemon
pub struct UnixSocketClient {
    socket_path: SocketPath,
}

impl UnixSocketClient {
    /// Create a new socket client
    pub fn new(socket_path: SocketPath) -> Self {
        Self { socket_path }
    }
}

#[async_trait]
impl IpcClient for UnixSocketClient {
    fn is_daemon_running(&self) -> bool {
        self.socket_path.exists()
    }

    async fn send_command(&self, cmd: &str) -> io::Result<String> {
        let stream = UnixStream::connect(self.socket_path.path()).await?;
        let (reader, mut writer) = stream.into_split();

        // Send command
        writer.write_all(format!("{}\n", cmd).as_bytes()).await?;
        writer.flush().await?;

        // Read response
        let mut reader = BufReader::new(reader);
        let mut response = String::new();
        reader.read_line(&mut response).await?;

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_path_uses_xdg_runtime_dir() {
        let path = std::env::var("XDG_RUNTIME_DIR")
            .map(|dir| PathBuf::from(dir).join("smart-scribe.sock"))
            .unwrap_or_else(|_| std::env::temp_dir().join("smart-scribe.sock"));

        let socket_path = SocketPath::new();
        assert_eq!(socket_path.path(), path.as_path());
    }

    #[test]
    fn socket_path_default_fallback() {
        let fallback = std::env::temp_dir().join("smart-scribe.sock");
        assert!(fallback.to_string_lossy().contains("smart-scribe.sock"));
    }
}
