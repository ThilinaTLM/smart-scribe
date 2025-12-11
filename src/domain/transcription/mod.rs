//! Transcription domain module

mod audio_data;
mod domain_preset;
mod system_prompt;

pub use audio_data::{AudioData, AudioMimeType};
pub use domain_preset::DomainId;
pub use system_prompt::SystemPrompt;
