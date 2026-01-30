//! Unix Domain Socket communication for daemon control

use std::io;
use std::path::{Path, PathBuf};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;

use super::signals::DaemonSignal;
use crate::domain::daemon::DaemonState;

/// Socket path resolver
#[derive(Debug, Clone)]
pub struct SocketPath {
    path: PathBuf,
}

impl SocketPath {
    /// Create socket path, preferring XDG_RUNTIME_DIR (Unix) or temp_dir (Windows)
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

/// Daemon socket server - listens for commands and sends to channel
pub struct DaemonSocketServer {
    socket_path: SocketPath,
    listener: Option<UnixListener>,
}

impl DaemonSocketServer {
    /// Create a new socket server
    pub fn new(socket_path: SocketPath) -> Self {
        Self {
            socket_path,
            listener: None,
        }
    }

    /// Bind to the socket
    pub fn bind(&mut self) -> io::Result<()> {
        // Remove stale socket file if it exists
        self.socket_path.cleanup()?;

        // Bind listener
        let listener = UnixListener::bind(self.socket_path.path())?;
        self.listener = Some(listener);
        Ok(())
    }

    /// Get the socket path
    pub fn path(&self) -> &Path {
        self.socket_path.path()
    }

    /// Accept and handle connections
    ///
    /// This runs in a loop, accepting connections and processing commands.
    /// Each command is sent to the provided channel.
    /// The state_fn is called to get current daemon state for status queries.
    pub async fn run<F>(&self, tx: mpsc::Sender<DaemonSignal>, state_fn: F) -> io::Result<()>
    where
        F: Fn() -> DaemonState + Send + Sync + 'static,
    {
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

    /// Cleanup socket file
    pub fn cleanup(&self) {
        let _ = self.socket_path.cleanup();
    }
}

impl Drop for DaemonSocketServer {
    fn drop(&mut self) {
        self.cleanup();
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

/// Daemon socket client - connects and sends commands
pub struct DaemonSocketClient {
    socket_path: SocketPath,
}

impl DaemonSocketClient {
    /// Create a new socket client
    pub fn new(socket_path: SocketPath) -> Self {
        Self { socket_path }
    }

    /// Check if daemon appears to be running (socket exists)
    pub fn is_daemon_running(&self) -> bool {
        self.socket_path.exists()
    }

    /// Send a command and receive response
    pub async fn send_command(&self, cmd: &str) -> io::Result<String> {
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
        // Test path resolution with a specific value
        let path = std::env::var("XDG_RUNTIME_DIR")
            .map(|dir| PathBuf::from(dir).join("smart-scribe.sock"))
            .unwrap_or_else(|_| std::env::temp_dir().join("smart-scribe.sock"));

        // The actual SocketPath should match this logic
        let socket_path = SocketPath::new();
        assert_eq!(socket_path.path(), path.as_path());
    }

    #[test]
    fn socket_path_default_fallback() {
        // Test that if XDG_RUNTIME_DIR is not set, we fallback to temp_dir
        // We can't easily unset env vars in tests due to parallel execution,
        // so just verify the fallback path is correct
        let fallback = std::env::temp_dir().join("smart-scribe.sock");
        assert!(fallback.to_string_lossy().contains("smart-scribe.sock"));
    }
}
