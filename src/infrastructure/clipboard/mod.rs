//! Clipboard infrastructure module
//!
//! Provides cross-platform clipboard support using arboard (primary)
//! or platform-specific tools as fallback.

mod arboard;
mod wayland;

pub use arboard::ArboardClipboard;
pub use wayland::WaylandClipboard;

use crate::application::ports::Clipboard;

/// Create the default clipboard adapter for the current platform
///
/// Uses arboard (cross-platform) as the primary option.
pub fn create_clipboard() -> Box<dyn Clipboard> {
    Box::new(ArboardClipboard::new())
}
