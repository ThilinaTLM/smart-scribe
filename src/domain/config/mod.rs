//! Configuration domain module

mod app_config;

pub use app_config::{
    AppConfig, AuthMode, LinuxConfig, WindowsConfig, DEFAULT_OPENAI_TRANSCRIBE_MODEL,
};
