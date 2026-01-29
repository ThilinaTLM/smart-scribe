//! Keystroke infrastructure module

mod factory;
mod noop;
mod wtype;
mod xdotool;
mod ydotool;

pub use factory::{create_keystroke, KeystrokeTool};
pub use noop::NoOpKeystroke;
pub use wtype::WtypeKeystroke;
pub use xdotool::XdotoolKeystroke;
pub use ydotool::YdotoolKeystroke;
