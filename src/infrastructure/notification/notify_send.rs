//! notify-send notification adapter

use std::process::Stdio;

use async_trait::async_trait;
use tokio::process::Command;

use crate::application::ports::{NotificationError, NotificationIcon, Notifier};

/// notify-send notification adapter
pub struct NotifySendNotifier {
    /// Application name for notifications
    app_name: String,
}

impl NotifySendNotifier {
    /// Create a new notify-send notifier
    pub fn new() -> Self {
        Self {
            app_name: "SmartScribe".to_string(),
        }
    }

    /// Create with custom app name
    pub fn with_app_name(app_name: impl Into<String>) -> Self {
        Self {
            app_name: app_name.into(),
        }
    }
}

impl Default for NotifySendNotifier {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Notifier for NotifySendNotifier {
    async fn notify(
        &self,
        title: &str,
        message: &str,
        icon: NotificationIcon,
    ) -> Result<(), NotificationError> {
        let status = Command::new("notify-send")
            .args([
                "--app-name",
                &self.app_name,
                "--icon",
                icon.icon_name(),
                title,
                message,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    NotificationError::NotifySendNotFound
                } else {
                    NotificationError::SendFailed(e.to_string())
                }
            })?;

        if !status.success() {
            return Err(NotificationError::SendFailed(format!(
                "notify-send exited with status: {}",
                status
            )));
        }

        Ok(())
    }
}
