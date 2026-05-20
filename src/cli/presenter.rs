//! CLI presenter for output formatting

use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;

use super::args::OutputFormatArg;

/// Presenter for CLI output formatting
pub struct Presenter {
    output_format: OutputFormatArg,
    spinner: Option<ProgressBar>,
    is_spinner_active: Arc<AtomicBool>,
}

impl Presenter {
    /// Create a new presenter
    pub fn new(output_format: OutputFormatArg) -> Self {
        Self {
            output_format,
            spinner: None,
            is_spinner_active: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Selected output format
    pub const fn output_format(&self) -> OutputFormatArg {
        self.output_format
    }

    /// Whether machine-readable JSON output is enabled
    pub const fn is_json(&self) -> bool {
        self.output_format.is_json()
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

    /// Output text to stdout
    pub fn output(&self, text: &str) {
        println!("{}", text);
    }

    /// Output JSON to stdout
    pub fn output_json<T: Serialize>(&self, value: &T) {
        match serde_json::to_string(value) {
            Ok(json) => println!("{}", json),
            Err(e) => {
                self.error(&format!("Failed to serialize JSON output: {}", e));
            }
        }
    }

    /// Output text to stdout without newline
    pub fn output_inline(&self, text: &str) {
        print!("{}", text);
        let _ = io::stdout().flush();
    }

    /// Output raw bytes already encoded as a single line
    pub fn output_line(&self, line: &str) {
        print!("{}", line);
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

    /// Return a [`WarningSink`](crate::application::WarningSink) closure that
    /// emits messages through this presenter's standard warning channel.
    ///
    /// Used to feed application-layer warnings back into the CLI without
    /// requiring the application layer to know how to format them.
    pub fn warning_sink(&self) -> crate::application::WarningSink {
        // The closure only needs to write "⚠ msg" to stderr; it does not
        // share any Presenter state, so we don't need the Presenter to be
        // Clone (or Sync).
        std::sync::Arc::new(|msg: &str| {
            eprintln!("{} {}", "⚠".yellow(), msg);
        })
    }
}

impl Default for Presenter {
    fn default() -> Self {
        Self::new(OutputFormatArg::Text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_progress_at_start() {
        let presenter = Presenter::new(OutputFormatArg::Text);
        let progress = presenter.format_progress(0, 10000);
        assert!(progress.contains("0s / 10s"));
    }

    #[test]
    fn format_progress_at_half() {
        let presenter = Presenter::new(OutputFormatArg::Text);
        let progress = presenter.format_progress(5000, 10000);
        assert!(progress.contains("5s / 10s"));
    }

    #[test]
    fn format_progress_at_end() {
        let presenter = Presenter::new(OutputFormatArg::Text);
        let progress = presenter.format_progress(10000, 10000);
        assert!(progress.contains("10s / 10s"));
    }

    #[test]
    fn presenter_tracks_json_mode() {
        let presenter = Presenter::new(OutputFormatArg::Json);
        assert!(presenter.is_json());
    }
}
