//! Daemon command handler - sends commands to running daemon via IPC

use tokio::io::AsyncBufReadExt;

use super::args::DaemonAction;
use super::ipc::create_ipc_client;
use super::output::{DaemonCommandAck, DaemonStatusCommandResponse, DaemonStatusPayload};
use super::presenter::Presenter;

/// Handle daemon subcommand
pub async fn handle_daemon_command(
    action: DaemonAction,
    presenter: &Presenter,
) -> Result<(), String> {
    let client = create_ipc_client();

    // Check if daemon is running
    if !client.is_daemon_running() {
        return Err("No daemon running. Start with: smart-scribe --daemon".to_string());
    }

    match action {
        DaemonAction::Toggle => {
            let response = client
                .send_command("toggle")
                .await
                .map_err(|e| format!("Failed to communicate with daemon: {}", e))?;
            let response = response.trim();

            if let Some(stripped) = response.strip_prefix("error:") {
                return Err(stripped.trim().to_string());
            }

            if presenter.is_json() {
                presenter.output_json(&DaemonCommandAck {
                    ok: true,
                    command: "toggle",
                    accepted: true,
                });
            } else {
                presenter.info("Command sent: toggle");
            }
        }
        DaemonAction::Cancel => {
            let response = client
                .send_command("cancel")
                .await
                .map_err(|e| format!("Failed to communicate with daemon: {}", e))?;
            let response = response.trim();

            if let Some(stripped) = response.strip_prefix("error:") {
                return Err(stripped.trim().to_string());
            }

            if presenter.is_json() {
                presenter.output_json(&DaemonCommandAck {
                    ok: true,
                    command: "cancel",
                    accepted: true,
                });
            } else {
                presenter.info("Command sent: cancel");
            }
        }
        DaemonAction::Status => {
            if presenter.is_json() {
                let response = client
                    .send_command("status-json")
                    .await
                    .map_err(|e| format!("Failed to communicate with daemon: {}", e))?;
                let payload: DaemonStatusPayload = serde_json::from_str(response.trim())
                    .map_err(|e| format!("Failed to parse daemon status: {}", e))?;

                presenter.output_json(&DaemonStatusCommandResponse {
                    ok: true,
                    command: "status",
                    state: payload.state,
                    elapsed_ms: payload.elapsed_ms,
                });
            } else {
                let response = client
                    .send_command("status")
                    .await
                    .map_err(|e| format!("Failed to communicate with daemon: {}", e))?;
                presenter.info(&format!("Daemon status: {}", response.trim()));
            }
        }
        DaemonAction::Subscribe => {
            if !presenter.is_json() {
                return Err("daemon subscribe requires --output json".to_string());
            }

            let mut reader = client
                .subscribe()
                .await
                .map_err(|e| format!("Failed to subscribe to daemon events: {}", e))?;

            loop {
                let mut line = String::new();
                let bytes = reader
                    .read_line(&mut line)
                    .await
                    .map_err(|e| format!("Failed to read daemon event: {}", e))?;

                if bytes == 0 {
                    break;
                }

                presenter.output_line(&line);
            }
        }
    }

    Ok(())
}
