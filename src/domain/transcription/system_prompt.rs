//! System prompt value object

use super::domain_preset::DomainId;

/// Base system instruction for all transcriptions
const BASE_INSTRUCTION: &str = r#"You are a voice-to-text assistant that transcribes audio into grammatically correct, context-aware text output.

Instructions:
- Remove filler words (um, ah, like, you know)
- Must have correct grammar and punctuation
- Do NOT transcribe stutters, false starts, or repeated words
- Output ONLY the final cleaned text
- Do NOT include meta-commentary or explanations"#;

/// Value object representing the complete system prompt for transcription.
/// Combines base instructions with domain-specific context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemPrompt {
    content: String,
}

impl SystemPrompt {
    /// Build a system prompt with domain-specific instructions
    pub fn build(domain: DomainId) -> Self {
        let content = format!(
            "{}\n\nDomain Context: {}\n{}",
            BASE_INSTRUCTION,
            domain.label(),
            domain.prompt()
        );
        Self { content }
    }

    /// Build a system prompt with default (general) domain
    pub fn default_prompt() -> Self {
        Self::build(DomainId::default())
    }

    /// Get the prompt content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Consume and return the content
    pub fn into_content(self) -> String {
        self.content
    }
}

impl Default for SystemPrompt {
    fn default() -> Self {
        Self::default_prompt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_contains_base_instruction() {
        let prompt = SystemPrompt::build(DomainId::General);
        assert!(prompt.content().contains("voice-to-text assistant"));
        assert!(prompt.content().contains("Remove filler words"));
    }

    #[test]
    fn build_contains_domain_context() {
        let prompt = SystemPrompt::build(DomainId::Dev);
        assert!(prompt
            .content()
            .contains("Domain Context: Software Engineering"));
        assert!(prompt.content().contains("programming terminology"));
    }

    #[test]
    fn different_domains_different_prompts() {
        let general = SystemPrompt::build(DomainId::General);
        let dev = SystemPrompt::build(DomainId::Dev);
        assert_ne!(general.content(), dev.content());
    }

    #[test]
    fn default_is_general() {
        let default_prompt = SystemPrompt::default();
        let general_prompt = SystemPrompt::build(DomainId::General);
        assert_eq!(default_prompt.content(), general_prompt.content());
    }

    #[test]
    fn into_content_consumes() {
        let prompt = SystemPrompt::build(DomainId::General);
        let content = prompt.into_content();
        assert!(content.contains("voice-to-text assistant"));
    }
}
