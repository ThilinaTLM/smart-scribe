//! No-op smart paste adapter (used when paste mode is disabled)

use async_trait::async_trait;

use crate::application::ports::{SmartPaste, SmartPasteError};

/// No-op smart paste that does nothing
pub struct NoOpSmartPaste;

impl NoOpSmartPaste {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoOpSmartPaste {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SmartPaste for NoOpSmartPaste {
    async fn capture_active_window(&self) -> Result<(), SmartPasteError> {
        Ok(())
    }

    async fn paste(&self, _text: &str) -> Result<(), SmartPasteError> {
        Ok(())
    }
}
