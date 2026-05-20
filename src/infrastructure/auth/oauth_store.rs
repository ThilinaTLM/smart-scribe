//! On-disk persistence for OAuth tokens.
//!
//! Tokens live at `<config_dir>/smart-scribe/oauth.json`. On Unix the file is
//! written with mode 0600; on Windows we rely on the per-user `%APPDATA%` ACL.

use std::path::{Path, PathBuf};

use super::error::OAuthError;
use super::oauth_token::OAuthToken;

/// Default token storage filename within the smart-scribe config directory.
pub const OAUTH_FILE_NAME: &str = "oauth.json";

#[derive(Debug, Clone)]
pub struct OAuthStore {
    path: PathBuf,
}

impl OAuthStore {
    /// Construct the default store at `<config_dir>/smart-scribe/oauth.json`.
    pub fn new() -> Result<Self, OAuthError> {
        let dir = dirs::config_dir()
            .or_else(dirs::home_dir)
            .ok_or(OAuthError::ConfigDirMissing)?
            .join("smart-scribe");
        Ok(Self {
            path: dir.join(OAUTH_FILE_NAME),
        })
    }

    /// Construct a store at an explicit path (used by tests).
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Load the token from disk, returning `Ok(None)` if the file does not exist.
    pub fn load(&self) -> Result<Option<OAuthToken>, OAuthError> {
        if !self.path.exists() {
            return Ok(None);
        }
        let bytes = std::fs::read(&self.path)?;
        let token: OAuthToken = serde_json::from_slice(&bytes)
            .map_err(|e| OAuthError::Io(format!("parse oauth.json: {e}")))?;
        Ok(Some(token))
    }

    /// Persist the token, replacing any existing file.
    pub fn save(&self, token: &OAuthToken) -> Result<(), OAuthError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_vec_pretty(token)
            .map_err(|e| OAuthError::Io(format!("serialize oauth.json: {e}")))?;
        std::fs::write(&self.path, json)?;
        self.set_secure_permissions()?;
        Ok(())
    }

    /// Delete the token file. Idempotent.
    pub fn delete(&self) -> Result<(), OAuthError> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    /// `true` if the file exists on disk.
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    #[cfg(unix)]
    fn set_secure_permissions(&self) -> Result<(), OAuthError> {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&self.path, perms)?;
        Ok(())
    }

    #[cfg(not(unix))]
    fn set_secure_permissions(&self) -> Result<(), OAuthError> {
        // On Windows we rely on the per-user %APPDATA% ACL; no portable way
        // to set restrictive ACLs from the standard library.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_token() -> OAuthToken {
        OAuthToken {
            access_token: "access".into(),
            refresh_token: "refresh".into(),
            expires_at_unix: 1_000,
            account_id: "acc".into(),
        }
    }

    #[test]
    fn save_then_load_round_trip() {
        let dir = tempdir().unwrap();
        let store = OAuthStore::with_path(dir.path().join("oauth.json"));
        store.save(&sample_token()).unwrap();
        let loaded = store.load().unwrap().unwrap();
        assert_eq!(loaded.access_token, "access");
        assert_eq!(loaded.account_id, "acc");
    }

    #[test]
    fn load_returns_none_when_missing() {
        let dir = tempdir().unwrap();
        let store = OAuthStore::with_path(dir.path().join("oauth.json"));
        assert!(store.load().unwrap().is_none());
    }

    #[test]
    fn delete_is_idempotent() {
        let dir = tempdir().unwrap();
        let store = OAuthStore::with_path(dir.path().join("oauth.json"));
        store.delete().unwrap();
        store.save(&sample_token()).unwrap();
        store.delete().unwrap();
        assert!(!store.exists());
    }

    #[cfg(unix)]
    #[test]
    fn saved_file_has_0600_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let store = OAuthStore::with_path(dir.path().join("oauth.json"));
        store.save(&sample_token()).unwrap();
        let meta = std::fs::metadata(store.path()).unwrap();
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "expected 0600, got {:o}", mode);
    }
}
