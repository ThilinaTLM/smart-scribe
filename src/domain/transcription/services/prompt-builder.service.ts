import { type DomainId, DomainPreset } from "../value-objects/domain-preset.vo"
import { SystemPrompt } from "../value-objects/system-prompt.vo"

/**
 * Domain service for building transcription prompts
 */
export class PromptBuilderService {
  /**
   * Build a system prompt for the given domain
   */
  buildPrompt(domainId: DomainId): SystemPrompt {
    const preset = DomainPreset.fromId(domainId)
    return SystemPrompt.build(preset)
  }

  /**
   * Build a system prompt with custom instructions appended
   */
  buildPromptWithCustomInstructions(
    domainId: DomainId,
    customInstructions: string,
  ): SystemPrompt {
    const basePrompt = this.buildPrompt(domainId)
    const content = `${basePrompt.content}

Additional Instructions:
${customInstructions}`

    return new SystemPrompt(content)
  }
}

// Re-export SystemPrompt constructor for custom prompts
export { SystemPrompt }
