//! Keystroke infrastructure module
//!
//! Provides cross-platform keystroke support using enigo (primary)
//! or platform-specific tools as fallback on Linux.

mod enigo;
mod factory;
mod noop;
mod wtype;
mod xdotool;
mod ydotool;

pub use enigo::EnigoKeystroke;
pub use factory::{create_keystroke, detect_keystroke_tool, KeystrokeTool};
pub use noop::NoOpKeystroke;
pub use wtype::WtypeKeystroke;
pub use xdotool::XdotoolKeystroke;
pub use ydotool::YdotoolKeystroke;
