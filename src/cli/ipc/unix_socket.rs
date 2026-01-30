//! Unix Domain Socket communication for daemon control
//!
//! Used on Linux and macOS.

use std::io;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;

use super::{IpcClient, IpcServer, StateFn};
use crate::cli::signals::DaemonSignal;
use crate::domain::daemon::DaemonState;

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

    async fn run(&self, tx: mpsc::Sender<DaemonSignal>, state_fn: StateFn) -> io::Result<()> {
        let listener = self
            .listener
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "Socket not bound"))?;

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let tx = tx.clone();
                    let state = state_fn();
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, tx, state).await {
                            eprintln!("Socket connection error: {}", e);
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
    current_state: DaemonState,
) -> io::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    // Read command
    reader.read_line(&mut line).await?;
    let cmd = line.trim();

    // Process command
    let response = match cmd {
        "toggle" => {
            let _ = tx.send(DaemonSignal::Toggle).await;
            "ok\n"
        }
        "cancel" => {
            let _ = tx.send(DaemonSignal::Cancel).await;
            "ok\n"
        }
        "status" => match current_state {
            DaemonState::Idle => "idle\n",
            DaemonState::Recording => "recording\n",
            DaemonState::Processing => "processing\n",
        },
        _ => "error: unknown command\n",
    };

    writer.write_all(response.as_bytes()).await?;
    writer.flush().await?;

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
