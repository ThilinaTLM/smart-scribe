//! Notification port interface

use async_trait::async_trait;
use thiserror::Error;

/// Notification errors
#[derive(Debug, Clone, Error)]
pub enum NotificationError {
    #[error("notify-send not found")]
    NotifySendNotFound,

    #[error("Failed to show notification: {0}")]
    SendFailed(String),
}

/// Notification icon types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationIcon {
    Info,
    Success,
    Warning,
    Error,
    Recording,
    Processing,
}

impl NotificationIcon {
    /// Get the freedesktop icon name
    pub const fn icon_name(&self) -> &'static str {
        match self {
            Self::Info => "dialog-information",
            Self::Success => "dialog-ok",
            Self::Warning => "dialog-warning",
            Self::Error => "dialog-error",
            Self::Recording => "audio-input-microphone",
            Self::Processing => "preferences-system",
        }
    }
}

/// Port for desktop notifications
#[async_trait]
pub trait Notifier: Send + Sync {
    /// Show a desktop notification.
    ///
    /// # Arguments
    /// * `title` - The notification title
    /// * `message` - The notification body
    /// * `icon` - The icon to display
    ///
    /// # Returns
    /// Ok(()) on success, error otherwise
    async fn notify(
        &self,
        title: &str,
        message: &str,
        icon: NotificationIcon,
    ) -> Result<(), NotificationError>;
}

/// Blanket implementation for boxed notifier types
#[async_trait]
impl Notifier for Box<dyn Notifier> {
    async fn notify(
        &self,
        title: &str,
        message: &str,
        icon: NotificationIcon,
    ) -> Result<(), NotificationError> {
        self.as_ref().notify(title, message, icon).await
    }
}
