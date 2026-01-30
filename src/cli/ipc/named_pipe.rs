//! Named Pipe communication for daemon control on Windows

use std::io;

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::windows::named_pipe::{ClientOptions, ServerOptions};
use tokio::sync::mpsc;

use super::{IpcClient, IpcServer, StateFn};
use crate::cli::signals::DaemonSignal;
use crate::domain::daemon::DaemonState;

/// Named pipe path
const PIPE_NAME: &str = r"\\.\pipe\smart-scribe";

/// Named pipe path resolver
#[derive(Debug, Clone)]
pub struct PipePath {
    path: String,
}

impl PipePath {
    /// Create the default pipe path
    pub fn new() -> Self {
        Self {
            path: PIPE_NAME.to_string(),
        }
    }

    /// Get the pipe path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Check if named pipe exists (attempt to connect)
    pub fn exists(&self) -> bool {
        // On Windows, we try to open the pipe to check if it exists
        std::fs::metadata(&self.path).is_ok()
    }
}

impl Default for PipePath {
    fn default() -> Self {
        Self::new()
    }
}

/// Named Pipe server for daemon commands
pub struct NamedPipeServer {
    pipe_path: PipePath,
    bound: bool,
}

impl NamedPipeServer {
    /// Create a new pipe server
    pub fn new(pipe_path: PipePath) -> Self {
        Self {
            pipe_path,
            bound: false,
        }
    }
}

#[async_trait]
impl IpcServer for NamedPipeServer {
    fn bind(&mut self) -> io::Result<()> {
        // Named pipes on Windows are created when first listening
        self.bound = true;
        Ok(())
    }

    fn path(&self) -> String {
        self.pipe_path.path().to_string()
    }

    async fn run(&self, tx: mpsc::Sender<DaemonSignal>, state_fn: StateFn) -> io::Result<()> {
        if !self.bound {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "Pipe not bound",
            ));
        }

        loop {
            // Create a new pipe instance for this connection
            let server = ServerOptions::new()
                .first_pipe_instance(false)
                .create(&self.pipe_path.path)?;

            // Wait for a client to connect
            server.connect().await?;

            let tx = tx.clone();
            let state = state_fn();

            tokio::spawn(async move {
                if let Err(e) = handle_connection(server, tx, state).await {
                    eprintln!("Pipe connection error: {}", e);
                }
            });
        }
    }

    fn cleanup(&self) {
        // Named pipes are automatically cleaned up when the server drops
    }
}

/// Handle a single client connection
async fn handle_connection<T>(
    pipe: T,
    tx: mpsc::Sender<DaemonSignal>,
    current_state: DaemonState,
) -> io::Result<()>
where
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let (reader, mut writer) = tokio::io::split(pipe);
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
    writer.shutdown().await?;

    Ok(())
}

/// Named Pipe client for sending commands to daemon
pub struct NamedPipeClient {
    pipe_path: PipePath,
}

impl NamedPipeClient {
    /// Create a new pipe client
    pub fn new(pipe_path: PipePath) -> Self {
        Self { pipe_path }
    }
}

#[async_trait]
impl IpcClient for NamedPipeClient {
    fn is_daemon_running(&self) -> bool {
        self.pipe_path.exists()
    }

    async fn send_command(&self, cmd: &str) -> io::Result<String> {
        let client = ClientOptions::new().open(&self.pipe_path.path)?;

        let (reader, mut writer) = tokio::io::split(client);

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
    fn pipe_path_creates() {
        let path = PipePath::new();
        assert_eq!(path.path(), PIPE_NAME);
    }
}
