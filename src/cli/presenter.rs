//! CLI presenter for output formatting

use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use colored::*;
use indicatif::{ProgressBar, ProgressStyle};

/// Presenter for CLI output formatting
pub struct Presenter {
    spinner: Option<ProgressBar>,
    is_spinner_active: Arc<AtomicBool>,
}

impl Presenter {
    /// Create a new presenter
    pub fn new() -> Self {
        Self {
            spinner: None,
            is_spinner_active: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start a spinner with message
    pub fn start_spinner(&mut self, message: &str) {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        spinner.set_message(message.to_string());
        spinner.enable_steady_tick(std::time::Duration::from_millis(80));
        self.spinner = Some(spinner);
        self.is_spinner_active.store(true, Ordering::SeqCst);
    }

    /// Update spinner message
    pub fn update_spinner(&self, message: &str) {
        if let Some(ref spinner) = self.spinner {
            spinner.set_message(message.to_string());
        }
    }

    /// Mark spinner as success and finish
    pub fn spinner_success(&mut self, message: &str) {
        if let Some(spinner) = self.spinner.take() {
            spinner.finish_with_message(format!("{} {}", "✓".green(), message));
        }
        self.is_spinner_active.store(false, Ordering::SeqCst);
    }

    /// Mark spinner as failed and finish
    pub fn spinner_fail(&mut self, message: &str) {
        if let Some(spinner) = self.spinner.take() {
            spinner.finish_with_message(format!("{} {}", "✗".red(), message));
        }
        self.is_spinner_active.store(false, Ordering::SeqCst);
    }

    /// Stop spinner without status
    pub fn stop_spinner(&mut self) {
        if let Some(spinner) = self.spinner.take() {
            spinner.finish_and_clear();
        }
        self.is_spinner_active.store(false, Ordering::SeqCst);
    }

    /// Print info message to stderr
    pub fn info(&self, message: &str) {
        eprintln!("{} {}", "ℹ".cyan(), message);
    }

    /// Print success message to stderr
    pub fn success(&self, message: &str) {
        eprintln!("{} {}", "✓".green(), message);
    }

    /// Print warning message to stderr
    pub fn warn(&self, message: &str) {
        eprintln!("{} {}", "⚠".yellow(), message);
    }

    /// Print error message to stderr
    pub fn error(&self, message: &str) {
        eprintln!("{} {}", "✗".red(), message);
    }

    /// Output text to stdout (the actual transcription output)
    pub fn output(&self, text: &str) {
        println!("{}", text);
    }

    /// Output text to stdout without newline
    pub fn output_inline(&self, text: &str) {
        print!("{}", text);
        let _ = io::stdout().flush();
    }

    /// Format recording progress bar
    pub fn format_progress(&self, elapsed_ms: u64, total_ms: u64) -> String {
        let elapsed_secs = elapsed_ms / 1000;
        let total_secs = total_ms / 1000;
        let percent = if total_ms > 0 {
            (elapsed_ms as f64 / total_ms as f64 * 100.0).min(100.0)
        } else {
            0.0
        };

        // Build progress bar
        let bar_width = 20;
        let filled = ((percent / 100.0) * bar_width as f64) as usize;
        let empty = bar_width - filled;

        format!(
            "[{}{}] {:>3}s / {}s",
            "█".repeat(filled).cyan(),
            "░".repeat(empty),
            elapsed_secs,
            total_secs
        )
    }

    /// Show a progress bar for recording
    pub fn show_recording_progress(&mut self, message: &str) {
        self.start_spinner(message);
    }

    /// Update recording progress
    pub fn update_recording_progress(&self, elapsed_ms: u64, total_ms: u64) {
        let progress = self.format_progress(elapsed_ms, total_ms);
        self.update_spinner(&format!("Recording... {}", progress));
    }

    /// Print daemon status
    pub fn daemon_status(&self, state: &str) {
        eprintln!("{} Daemon: {}", "●".cyan(), state);
    }

    /// Print a key-value pair (for config list)
    pub fn key_value(&self, key: &str, value: &str) {
        println!("{}: {}", key.cyan(), value);
    }
}

impl Default for Presenter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_progress_at_start() {
        let presenter = Presenter::new();
        let progress = presenter.format_progress(0, 10000);
        assert!(progress.contains("0s / 10s"));
    }

    #[test]
    fn format_progress_at_half() {
        let presenter = Presenter::new();
        let progress = presenter.format_progress(5000, 10000);
        assert!(progress.contains("5s / 10s"));
    }

    #[test]
    fn format_progress_at_end() {
        let presenter = Presenter::new();
        let progress = presenter.format_progress(10000, 10000);
        assert!(progress.contains("10s / 10s"));
    }
}
