//! Transcription infrastructure module

mod chatgpt_oauth;
mod openai_api;

pub use chatgpt_oauth::ChatGptOAuthTranscriber;
pub use openai_api::OpenAiApiTranscriber;
