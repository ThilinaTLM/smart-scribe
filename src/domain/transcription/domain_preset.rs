//! Domain preset value object

use std::fmt;
use std::str::FromStr;

use crate::domain::error::InvalidDomainError;

/// All available domain IDs
pub const ALL_DOMAINS: &[DomainId] = &[
    DomainId::General,
    DomainId::Dev,
    DomainId::Medical,
    DomainId::Legal,
    DomainId::Finance,
];

/// Domain identifiers for transcription presets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DomainId {
    #[default]
    General,
    Dev,
    Medical,
    Legal,
    Finance,
}

impl DomainId {
    /// Get the human-readable label for this domain
    pub const fn label(&self) -> &'static str {
        match self {
            Self::General => "General Conversation",
            Self::Dev => "Software Engineering",
            Self::Medical => "Medical / Healthcare",
            Self::Legal => "Legal",
            Self::Finance => "Finance",
        }
    }

    /// Get the domain-specific prompt instructions
    pub const fn prompt(&self) -> &'static str {
        match self {
            Self::General => "Standard grammar correction and clarity.",
            Self::Dev => "Focus on programming terminology, variable naming conventions where appropriate, and tech stack names.",
            Self::Medical => "Ensure accurate spelling of medical conditions, medications, and anatomical terms.",
            Self::Legal => "Maintain formal tone, ensure accurate legal terminology and citation formats if applicable.",
            Self::Finance => "Focus on financial markets, acronyms (ETF, ROI, CAGR), and numerical accuracy.",
        }
    }

    /// Get the string identifier for this domain
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Dev => "dev",
            Self::Medical => "medical",
            Self::Legal => "legal",
            Self::Finance => "finance",
        }
    }
}

impl FromStr for DomainId {
    type Err = InvalidDomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "general" => Ok(Self::General),
            "dev" => Ok(Self::Dev),
            "medical" => Ok(Self::Medical),
            "legal" => Ok(Self::Legal),
            "finance" => Ok(Self::Finance),
            _ => Err(InvalidDomainError { input: s.to_string() }),
        }
    }
}

impl fmt::Display for DomainId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_all_domains() {
        assert_eq!("general".parse::<DomainId>().unwrap(), DomainId::General);
        assert_eq!("dev".parse::<DomainId>().unwrap(), DomainId::Dev);
        assert_eq!("medical".parse::<DomainId>().unwrap(), DomainId::Medical);
        assert_eq!("legal".parse::<DomainId>().unwrap(), DomainId::Legal);
        assert_eq!("finance".parse::<DomainId>().unwrap(), DomainId::Finance);
    }

    #[test]
    fn parse_case_insensitive() {
        assert_eq!("DEV".parse::<DomainId>().unwrap(), DomainId::Dev);
        assert_eq!("Dev".parse::<DomainId>().unwrap(), DomainId::Dev);
        assert_eq!("GENERAL".parse::<DomainId>().unwrap(), DomainId::General);
    }

    #[test]
    fn parse_with_whitespace() {
        assert_eq!("  dev  ".parse::<DomainId>().unwrap(), DomainId::Dev);
    }

    #[test]
    fn parse_invalid() {
        assert!("invalid".parse::<DomainId>().is_err());
        assert!("".parse::<DomainId>().is_err());
    }

    #[test]
    fn display() {
        assert_eq!(DomainId::General.to_string(), "general");
        assert_eq!(DomainId::Dev.to_string(), "dev");
    }

    #[test]
    fn labels() {
        assert_eq!(DomainId::General.label(), "General Conversation");
        assert_eq!(DomainId::Dev.label(), "Software Engineering");
    }

    #[test]
    fn prompts_not_empty() {
        for domain in ALL_DOMAINS {
            assert!(!domain.prompt().is_empty());
        }
    }

    #[test]
    fn all_domains_constant() {
        assert_eq!(ALL_DOMAINS.len(), 5);
    }

    #[test]
    fn default_is_general() {
        assert_eq!(DomainId::default(), DomainId::General);
    }
}
