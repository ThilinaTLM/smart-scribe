//! Notification infrastructure module
//!
//! Provides cross-platform notification support using notify-rust (primary)
//! or platform-specific tools as fallback.

mod notify_rust;
mod notify_send;

pub use notify_rust::NotifyRustNotifier;
pub use notify_send::NotifySendNotifier;

use crate::application::ports::Notifier;

/// Create the default notifier for the current platform
///
/// Uses notify-rust (cross-platform) as the primary option.
pub fn create_notifier() -> Box<dyn Notifier> {
    Box::new(NotifyRustNotifier::new())
}
