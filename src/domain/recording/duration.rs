//! Duration value object

use std::fmt;
use std::str::FromStr;
use std::time::Duration as StdDuration;

use crate::domain::error::DurationParseError;

/// Default recording duration (10 seconds)
pub const DEFAULT_DURATION_SECS: u64 = 10;

/// Default max duration for daemon mode (60 seconds)
pub const DEFAULT_MAX_DURATION_SECS: u64 = 60;

/// Value object representing a time duration.
/// Immutable and validated on creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration {
    milliseconds: u64,
}

impl Duration {
    /// Create a Duration from milliseconds
    pub const fn from_millis(ms: u64) -> Self {
        Self { milliseconds: ms }
    }

    /// Create a Duration from seconds
    pub const fn from_secs(secs: u64) -> Self {
        Self {
            milliseconds: secs * 1000,
        }
    }

    /// Default recording duration (10 seconds)
    pub const fn default_duration() -> Self {
        Self::from_secs(DEFAULT_DURATION_SECS)
    }

    /// Default max duration for daemon mode (60 seconds)
    pub const fn default_max_duration() -> Self {
        Self::from_secs(DEFAULT_MAX_DURATION_SECS)
    }

    /// Get duration in seconds
    pub const fn as_secs(&self) -> u64 {
        self.milliseconds / 1000
    }

    /// Get duration in milliseconds
    pub const fn as_millis(&self) -> u64 {
        self.milliseconds
    }

    /// Convert to std::time::Duration
    pub const fn as_std(&self) -> StdDuration {
        StdDuration::from_millis(self.milliseconds)
    }
}

impl FromStr for Duration {
    type Err = DurationParseError;

    /// Parse a duration string into a Duration value object.
    /// Supported formats: "30s", "1m", "2m30s", "90s"
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let input = s.trim().to_lowercase();

        // Regex-like pattern matching for formats like "30s", "1m", "2m30s"
        let mut minutes: u64 = 0;
        let mut seconds: u64 = 0;
        let mut current_num = String::new();
        let mut found_any = false;

        for ch in input.chars() {
            if ch.is_ascii_digit() {
                current_num.push(ch);
            } else if ch == 'm' && !current_num.is_empty() {
                minutes = current_num
                    .parse()
                    .map_err(|_| DurationParseError { input: s.to_string() })?;
                current_num.clear();
                found_any = true;
            } else if ch == 's' && !current_num.is_empty() {
                seconds = current_num
                    .parse()
                    .map_err(|_| DurationParseError { input: s.to_string() })?;
                current_num.clear();
                found_any = true;
            } else {
                return Err(DurationParseError { input: s.to_string() });
            }
        }

        // Handle case where there's leftover numbers (invalid format)
        if !current_num.is_empty() || !found_any {
            return Err(DurationParseError { input: s.to_string() });
        }

        let total_ms = (minutes * 60 + seconds) * 1000;

        if total_ms == 0 {
            return Err(DurationParseError { input: s.to_string() });
        }

        Ok(Self { milliseconds: total_ms })
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let total_secs = self.as_secs();
        let minutes = total_secs / 60;
        let seconds = total_secs % 60;

        if minutes == 0 {
            write!(f, "{}s", seconds)
        } else if seconds == 0 {
            write!(f, "{}m", minutes)
        } else {
            write!(f, "{}m{}s", minutes, seconds)
        }
    }
}

impl Default for Duration {
    fn default() -> Self {
        Self::default_duration()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_seconds_only() {
        let d: Duration = "30s".parse().unwrap();
        assert_eq!(d.as_secs(), 30);
        assert_eq!(d.as_millis(), 30000);
    }

    #[test]
    fn parse_minutes_only() {
        let d: Duration = "2m".parse().unwrap();
        assert_eq!(d.as_secs(), 120);
    }

    #[test]
    fn parse_minutes_and_seconds() {
        let d: Duration = "2m30s".parse().unwrap();
        assert_eq!(d.as_secs(), 150);
    }

    #[test]
    fn parse_case_insensitive() {
        let d: Duration = "1M30S".parse().unwrap();
        assert_eq!(d.as_secs(), 90);
    }

    #[test]
    fn parse_with_whitespace() {
        let d: Duration = "  30s  ".parse().unwrap();
        assert_eq!(d.as_secs(), 30);
    }

    #[test]
    fn parse_invalid_empty() {
        assert!("".parse::<Duration>().is_err());
    }

    #[test]
    fn parse_invalid_zero() {
        assert!("0s".parse::<Duration>().is_err());
        assert!("0m0s".parse::<Duration>().is_err());
    }

    #[test]
    fn parse_invalid_format() {
        assert!("30".parse::<Duration>().is_err());
        assert!("abc".parse::<Duration>().is_err());
        assert!("30x".parse::<Duration>().is_err());
    }

    #[test]
    fn display_seconds_only() {
        let d = Duration::from_secs(30);
        assert_eq!(d.to_string(), "30s");
    }

    #[test]
    fn display_minutes_only() {
        let d = Duration::from_secs(120);
        assert_eq!(d.to_string(), "2m");
    }

    #[test]
    fn display_minutes_and_seconds() {
        let d = Duration::from_secs(150);
        assert_eq!(d.to_string(), "2m30s");
    }

    #[test]
    fn as_std_duration() {
        let d = Duration::from_secs(30);
        assert_eq!(d.as_std(), StdDuration::from_secs(30));
    }

    #[test]
    fn default_values() {
        assert_eq!(Duration::default_duration().as_secs(), 10);
        assert_eq!(Duration::default_max_duration().as_secs(), 60);
    }
}
