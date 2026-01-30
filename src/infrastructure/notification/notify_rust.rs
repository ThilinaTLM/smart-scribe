//! Cross-platform notification adapter using notify-rust
//!
//! Works on Windows, macOS, and Linux.

use async_trait::async_trait;

use crate::application::ports::{NotificationError, NotificationIcon, Notifier};

/// Cross-platform notifier using notify-rust
pub struct NotifyRustNotifier {
    /// Application name for notifications
    app_name: String,
}

impl NotifyRustNotifier {
    /// Create a new notify-rust notifier
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

impl Default for NotifyRustNotifier {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Notifier for NotifyRustNotifier {
    async fn notify(
        &self,
        title: &str,
        message: &str,
        icon: NotificationIcon,
    ) -> Result<(), NotificationError> {
        let title = title.to_owned();
        let message = message.to_owned();
        let app_name = self.app_name.clone();
        let icon_name = icon.icon_name().to_string();

        // notify-rust operations can block, so run in spawn_blocking
        tokio::task::spawn_blocking(move || {
            notify_rust::Notification::new()
                .appname(&app_name)
                .summary(&title)
                .body(&message)
                .icon(&icon_name)
                .show()
                .map_err(|e| NotificationError::SendFailed(e.to_string()))?;

            Ok(())
        })
        .await
        .map_err(|e| NotificationError::SendFailed(format!("Task join error: {}", e)))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notifier_creates_successfully() {
        let _notifier = NotifyRustNotifier::new();
    }

    #[test]
    fn notifier_with_custom_app_name() {
        let notifier = NotifyRustNotifier::with_app_name("TestApp");
        assert_eq!(notifier.app_name, "TestApp");
    }

    #[test]
    fn notifier_default_creates() {
        let notifier = NotifyRustNotifier::default();
        assert_eq!(notifier.app_name, "SmartScribe");
    }
}
