//! OAuth / token management for ChatGPT subscription access
//!
//! Smart-scribe piggybacks on OpenAI's public Codex CLI OAuth client to obtain
//! a Bearer token that can be used against `https://chatgpt.com/backend-api/transcribe`.
//! The PKCE flow, refresh, JWT decoding and on-disk persistence all live in
//! this module.

mod codex_import;
mod error;
mod oauth_client;
mod oauth_store;
mod oauth_token;

pub use codex_import::import_from_codex;
pub use error::OAuthError;
pub use oauth_client::{exchange_code, refresh, run_pkce_login, CALLBACK_PORT, CLIENT_ID};
pub use oauth_store::OAuthStore;
pub use oauth_token::{decode_jwt_exp_and_account_id, OAuthToken};
