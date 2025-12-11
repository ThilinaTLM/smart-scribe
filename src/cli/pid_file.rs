//! PID file management for daemon mode

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process;

use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;

/// Default PID file location
const DEFAULT_PID_PATH: &str = "/tmp/smart-scribe.pid";

/// PID file for daemon mode
pub struct PidFile {
    path: PathBuf,
}

impl PidFile {
    /// Create a new PID file manager with default path
    pub fn new() -> Self {
        Self {
            path: PathBuf::from(DEFAULT_PID_PATH),
        }
    }

    /// Create with custom path
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Get the PID file path
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Check if another daemon is already running
    pub fn is_running(&self) -> Option<u32> {
        if !self.path.exists() {
            return None;
        }

        // Read existing PID
        let mut file = match File::open(&self.path) {
            Ok(f) => f,
            Err(_) => return None,
        };

        let mut contents = String::new();
        if file.read_to_string(&mut contents).is_err() {
            return None;
        }

        let pid: u32 = match contents.trim().parse() {
            Ok(p) => p,
            Err(_) => return None,
        };

        // Check if process is still alive using signal 0
        let pid_t = Pid::from_raw(pid as i32);
        match kill(pid_t, Signal::SIGCONT) {
            Ok(_) => Some(pid), // Process exists
            Err(nix::errno::Errno::ESRCH) => {
                // Process doesn't exist - stale PID file
                let _ = fs::remove_file(&self.path);
                None
            }
            Err(_) => None, // Other error - assume not running
        }
    }

    /// Acquire the PID file (fails if another daemon is running)
    pub fn acquire(&self) -> Result<(), PidFileError> {
        // Check for existing daemon
        if let Some(pid) = self.is_running() {
            return Err(PidFileError::AlreadyRunning(pid));
        }

        // Write our PID
        let mut file = File::create(&self.path).map_err(|e| {
            PidFileError::WriteFailed(format!("Failed to create PID file: {}", e))
        })?;

        let pid = process::id();
        write!(file, "{}", pid).map_err(|e| {
            PidFileError::WriteFailed(format!("Failed to write PID: {}", e))
        })?;

        Ok(())
    }

    /// Release the PID file
    pub fn release(&self) -> Result<(), PidFileError> {
        if self.path.exists() {
            fs::remove_file(&self.path).map_err(|e| {
                PidFileError::RemoveFailed(format!("Failed to remove PID file: {}", e))
            })?;
        }
        Ok(())
    }
}

impl Default for PidFile {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for PidFile {
    fn drop(&mut self) {
        // Best-effort cleanup
        let _ = self.release();
    }
}

/// PID file errors
#[derive(Debug, thiserror::Error)]
pub enum PidFileError {
    #[error("Another daemon is already running (PID: {0})")]
    AlreadyRunning(u32),

    #[error("Failed to write PID file: {0}")]
    WriteFailed(String),

    #[error("Failed to remove PID file: {0}")]
    RemoveFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn new_uses_default_path() {
        let pid_file = PidFile::new();
        assert_eq!(pid_file.path(), &PathBuf::from(DEFAULT_PID_PATH));
    }

    #[test]
    fn custom_path() {
        let pid_file = PidFile::with_path("/custom/path.pid");
        assert_eq!(pid_file.path(), &PathBuf::from("/custom/path.pid"));
    }

    #[test]
    fn is_running_returns_none_for_nonexistent_file() {
        let pid_file = PidFile::with_path(temp_dir().join("nonexistent.pid"));
        assert!(pid_file.is_running().is_none());
    }
}
