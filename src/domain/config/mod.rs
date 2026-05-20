//! Configuration domain module.
//!
//! Two complementary types live here:
//!
//! - [`RawAppConfig`] — on-disk schema (all-Optional, serialised to TOML by
//!   the infrastructure adapter). Suitable for layered merging
//!   (`defaults → file → env → CLI`) and for `config get/set/list`.
//! - [`AppConfig`] — runtime value object with concrete, validated values.
//!   Built by `AppConfig::try_from(raw)`; the only place parsing /
//!   validation happens.

mod app_config;
mod platform;
mod raw;

pub use app_config::{AppConfig, AuthMode, DEFAULT_OPENAI_TRANSCRIBE_MODEL};
pub use platform::PlatformConfig;
pub use raw::{RawAppConfig, RawLinuxConfig, RawWindowsConfig};
