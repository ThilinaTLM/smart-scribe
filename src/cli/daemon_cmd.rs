//! Daemon command handler - sends commands to running daemon via socket

use super::args::DaemonAction;
use super::presenter::Presenter;
use super::socket::{DaemonSocketClient, SocketPath};

/// Handle daemon subcommand
pub async fn handle_daemon_command(
    action: DaemonAction,
    presenter: &Presenter,
) -> Result<(), String> {
    let socket_path = SocketPath::new();
    let client = DaemonSocketClient::new(socket_path.clone());

    // Check if daemon is running
    if !client.is_daemon_running() {
        return Err(format!(
            "No daemon running. Start with: smart-scribe --daemon\n\
             (Expected socket at: {})",
            socket_path.path().display()
        ));
    }

    let cmd = match action {
        DaemonAction::Toggle => "toggle",
        DaemonAction::Cancel => "cancel",
        DaemonAction::Status => "status",
    };

    let response = client
        .send_command(cmd)
        .await
        .map_err(|e| format!("Failed to communicate with daemon: {}", e))?;

    let response = response.trim();

    match action {
        DaemonAction::Status => {
            presenter.info(&format!("Daemon status: {}", response));
        }
        _ => {
            if let Some(stripped) = response.strip_prefix("error:") {
                return Err(stripped.trim().to_string());
            }
            presenter.info(&format!("Command sent: {}", cmd));
        }
    }

    Ok(())
}
