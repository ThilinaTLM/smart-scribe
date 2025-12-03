import { DomainPreset } from "./domain-preset.vo"

/**
 * Base system instruction for all transcriptions
 */
const BASE_SYSTEM_INSTRUCTION = `You are a voice-to-text assistant that transcribes audio into grammatically correct, context-aware text output.

Instructions:
- Remove filler words (um, ah, like, you know)
- Must have correct grammar and punctuation
- Do NOT transcribe stutters, false starts, or repeated words
- Output ONLY the final cleaned text
- Do NOT include meta-commentary or explanations`

/**
 * Value object representing the complete system prompt for transcription.
 * Combines base instructions with domain-specific context.
 */
export class SystemPrompt {
  private constructor(readonly content: string) {}

  /**
   * Build a system prompt with domain-specific instructions
   */
  static build(domainPreset: DomainPreset): SystemPrompt {
    const content = `${BASE_SYSTEM_INSTRUCTION}

Domain Context: ${domainPreset.label}
${domainPreset.prompt}`

    return new SystemPrompt(content)
  }

  /**
   * Build a system prompt with default (general) domain
   */
  static default(): SystemPrompt {
    return SystemPrompt.build(DomainPreset.default())
  }
}
