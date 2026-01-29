//! No-op keystroke adapter

use async_trait::async_trait;

use crate::application::ports::{Keystroke, KeystrokeError};

/// No-op keystroke adapter that does nothing
///
/// Used when no keystroke tool is available or keystroke is disabled.
pub struct NoOpKeystroke;

impl NoOpKeystroke {
    /// Create a new no-op keystroke adapter
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoOpKeystroke {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Keystroke for NoOpKeystroke {
    async fn type_text(&self, _text: &str) -> Result<(), KeystrokeError> {
        // Do nothing
        Ok(())
    }
}
