//! Transcription infrastructure module

mod chatgpt;
mod gemini;

pub use chatgpt::ChatGptTranscriber;
pub use gemini::GeminiTranscriber;
