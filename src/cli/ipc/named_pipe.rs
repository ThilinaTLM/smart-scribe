//! Named Pipe communication for daemon control on Windows

use std::io;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::windows::named_pipe::{ClientOptions, ServerOptions};
use tokio::sync::{broadcast, mpsc};

use super::{ElapsedFn, IpcClient, IpcServer, StateFn};
use crate::cli::signals::DaemonSignal;
use crate::domain::daemon::{DaemonState, StateUpdate};

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

    async fn run(
        &self,
        tx: mpsc::Sender<DaemonSignal>,
        state_fn: StateFn,
        elapsed_fn: ElapsedFn,
        state_rx: broadcast::Receiver<StateUpdate>,
    ) -> io::Result<()> {
        if !self.bound {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "Pipe not bound",
            ));
        }

        // Wrap functions in Arc for sharing across connections
        let state_fn = Arc::new(state_fn);
        let elapsed_fn = Arc::new(elapsed_fn);

        loop {
            // Create a new pipe instance for this connection
            let server = ServerOptions::new()
                .first_pipe_instance(false)
                .create(&self.pipe_path.path)?;

            // Wait for a client to connect
            server.connect().await?;

            let tx = tx.clone();
            let state_fn = Arc::clone(&state_fn);
            let elapsed_fn = Arc::clone(&elapsed_fn);
            let state_rx = state_rx.resubscribe();

            tokio::spawn(async move {
                if let Err(e) = handle_connection(server, tx, state_fn, elapsed_fn, state_rx).await
                {
                    // Don't log BrokenPipe errors - they're expected when clients disconnect
                    if e.kind() != io::ErrorKind::BrokenPipe {
                        eprintln!("Pipe connection error: {}", e);
                    }
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
    state_fn: Arc<StateFn>,
    elapsed_fn: Arc<ElapsedFn>,
    mut state_rx: broadcast::Receiver<StateUpdate>,
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
    match cmd {
        "toggle" => {
            let _ = tx.send(DaemonSignal::Toggle).await;
            writer.write_all(b"ok\n").await?;
            writer.flush().await?;
            writer.shutdown().await?;
        }
        "cancel" => {
            let _ = tx.send(DaemonSignal::Cancel).await;
            writer.write_all(b"ok\n").await?;
            writer.flush().await?;
            writer.shutdown().await?;
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
            writer.shutdown().await?;
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
            // Don't shutdown here as the client may still want to read
        }
        _ => {
            writer.write_all(b"error: unknown command\n").await?;
            writer.flush().await?;
            writer.shutdown().await?;
        }
    }

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
