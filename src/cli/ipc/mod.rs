//! IPC (Inter-Process Communication) module for daemon control
//!
//! Provides platform-specific implementations:
//! - Unix (Linux/macOS): Unix Domain Sockets
//! - Windows: Named Pipes

#[cfg(windows)]
mod named_pipe;
#[cfg(unix)]
mod unix_socket;

#[cfg(windows)]
pub use named_pipe::{NamedPipeClient, NamedPipeServer, PipePath};
#[cfg(unix)]
pub use unix_socket::{SocketPath, UnixSocketClient, UnixSocketServer};

use std::io;
use tokio::sync::mpsc;

use super::signals::DaemonSignal;
use crate::domain::daemon::DaemonState;

/// State function type for IPC servers
pub type StateFn = Box<dyn Fn() -> DaemonState + Send + Sync>;

/// Trait for IPC servers that listen for daemon commands
#[async_trait::async_trait]
pub trait IpcServer: Send + Sync {
    /// Bind to the IPC endpoint
    fn bind(&mut self) -> io::Result<()>;

    /// Get the path/name of the IPC endpoint
    fn path(&self) -> String;

    /// Accept and handle connections
    ///
    /// This runs in a loop, accepting connections and processing commands.
    /// Each command is sent to the provided channel.
    /// The state_fn is called to get current daemon state for status queries.
    async fn run(&self, tx: mpsc::Sender<DaemonSignal>, state_fn: StateFn) -> io::Result<()>;

    /// Cleanup IPC resources
    fn cleanup(&self);
}

/// Trait for IPC clients that send commands to the daemon
#[async_trait::async_trait]
pub trait IpcClient: Send + Sync {
    /// Check if daemon appears to be running (endpoint exists)
    fn is_daemon_running(&self) -> bool;

    /// Send a command and receive response
    async fn send_command(&self, cmd: &str) -> io::Result<String>;
}

/// Create the appropriate IPC server for the current platform
#[cfg(unix)]
pub fn create_ipc_server() -> Box<dyn IpcServer> {
    Box::new(UnixSocketServer::new(SocketPath::new()))
}

#[cfg(windows)]
pub fn create_ipc_server() -> Box<dyn IpcServer> {
    Box::new(NamedPipeServer::new(PipePath::new()))
}

/// Create the appropriate IPC client for the current platform
#[cfg(unix)]
pub fn create_ipc_client() -> Box<dyn IpcClient> {
    Box::new(UnixSocketClient::new(SocketPath::new()))
}

#[cfg(windows)]
pub fn create_ipc_client() -> Box<dyn IpcClient> {
    Box::new(NamedPipeClient::new(PipePath::new()))
}
