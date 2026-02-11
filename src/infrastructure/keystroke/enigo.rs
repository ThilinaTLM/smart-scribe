//! Cross-platform keystroke adapter using enigo
//!
//! Works on Windows, macOS, and Linux (X11/Wayland).

use async_trait::async_trait;

use crate::application::ports::{Keystroke, KeystrokeError};

/// Cross-platform keystroke adapter using enigo
pub struct EnigoKeystroke;

impl EnigoKeystroke {
    /// Create a new enigo keystroke adapter
    pub fn new() -> Self {
        Self
    }
}

impl Default for EnigoKeystroke {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Keystroke for EnigoKeystroke {
    async fn type_text(&self, text: &str) -> Result<(), KeystrokeError> {
        let text = text.to_owned();

        // enigo operations are blocking, so run in spawn_blocking
        tokio::task::spawn_blocking(move || {
            use enigo::{Enigo, Keyboard, Settings};

            let mut enigo = Enigo::new(&Settings::default()).map_err(|e| {
                KeystrokeError::TypeFailed(format!("Failed to create enigo: {}", e))
            })?;

            // On Linux, enigo sends events via XWayland which drops keystrokes.
            // Type character-by-character with delays to prevent dropped input.
            #[cfg(target_os = "linux")]
            {
                use enigo::{Direction, Key};
                use std::thread;
                use std::time::Duration;

                // Initial delay lets XWayland focus settle
                thread::sleep(Duration::from_millis(50));

                for ch in text.chars() {
                    if ch.is_uppercase() {
                        // Workaround for enigo x11rb bug: keysym_to_keycode() only
                        // searches level 0 of the keymap (lowercase). Uppercase keysyms
                        // live at level 1 (Shift) so they're never found, and the
                        // fallback dynamic binding fails on XWayland — silently dropping
                        // the character. Instead, hold Shift and type the lowercase
                        // equivalent whose keycode IS found at level 0.
                        let lowercase = ch.to_lowercase().next().unwrap_or(ch);
                        enigo.key(Key::Shift, Direction::Press).map_err(|e| {
                            KeystrokeError::TypeFailed(format!("Failed to press Shift: {}", e))
                        })?;
                        enigo
                            .key(Key::Unicode(lowercase), Direction::Click)
                            .map_err(|e| {
                                KeystrokeError::TypeFailed(format!("Failed to type char: {}", e))
                            })?;
                        enigo.key(Key::Shift, Direction::Release).map_err(|e| {
                            KeystrokeError::TypeFailed(format!("Failed to release Shift: {}", e))
                        })?;
                    } else {
                        let s = ch.to_string();
                        enigo.text(&s).map_err(|e| {
                            KeystrokeError::TypeFailed(format!("Failed to type text: {}", e))
                        })?;
                    }
                    thread::sleep(Duration::from_millis(2));
                }

                Ok(())
            }

            #[cfg(not(target_os = "linux"))]
            {
                enigo
                    .text(&text)
                    .map_err(|e| KeystrokeError::TypeFailed(format!("Failed to type text: {}", e)))
            }
        })
        .await
        .map_err(|e| KeystrokeError::TypeFailed(format!("Task join error: {}", e)))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keystroke_creates_successfully() {
        let _keystroke = EnigoKeystroke::new();
    }

    #[test]
    fn keystroke_default_creates() {
        let _keystroke = EnigoKeystroke::default();
    }
}
