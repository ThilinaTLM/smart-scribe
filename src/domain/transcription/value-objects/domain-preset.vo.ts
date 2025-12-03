/**
 * Domain IDs for transcription presets
 */
export type DomainId = "general" | "dev" | "medical" | "legal" | "finance"

/**
 * Domain preset configuration
 */
interface DomainPresetConfig {
  readonly id: DomainId
  readonly label: string
  readonly prompt: string
}

/**
 * All available domain presets with their specialized prompts
 */
const DOMAIN_PRESETS: Record<DomainId, DomainPresetConfig> = {
  general: {
    id: "general",
    label: "General Conversation",
    prompt: "Standard grammar correction and clarity.",
  },
  dev: {
    id: "dev",
    label: "Software Engineering",
    prompt:
      "Focus on programming terminology, variable naming conventions where appropriate, and tech stack names.",
  },
  medical: {
    id: "medical",
    label: "Medical / Healthcare",
    prompt:
      "Ensure accurate spelling of medical conditions, medications, and anatomical terms.",
  },
  legal: {
    id: "legal",
    label: "Legal",
    prompt:
      "Maintain formal tone, ensure accurate legal terminology and citation formats if applicable.",
  },
  finance: {
    id: "finance",
    label: "Finance",
    prompt:
      "Focus on financial markets, acronyms (ETF, ROI, CAGR), and numerical accuracy.",
  },
}

/**
 * Value object representing a domain-specific transcription preset.
 * Immutable and contains domain-specific prompt configuration.
 */
export class DomainPreset {
  private constructor(
    readonly id: DomainId,
    readonly label: string,
    readonly prompt: string,
  ) {}

  /**
   * Create a DomainPreset from a domain ID
   */
  static fromId(id: DomainId): DomainPreset {
    const config = DOMAIN_PRESETS[id]
    return new DomainPreset(config.id, config.label, config.prompt)
  }

  /**
   * Check if a string is a valid domain ID
   */
  static isValidId(id: string): id is DomainId {
    return id in DOMAIN_PRESETS
  }

  /**
   * Get all available domain IDs
   */
  static getAllIds(): readonly DomainId[] {
    return Object.keys(DOMAIN_PRESETS) as DomainId[]
  }

  /**
   * Get all available presets
   */
  static getAll(): readonly DomainPreset[] {
    return Object.values(DOMAIN_PRESETS).map(
      (config) => new DomainPreset(config.id, config.label, config.prompt),
    )
  }

  /**
   * Get the default preset (general)
   */
  static default(): DomainPreset {
    return DomainPreset.fromId("general")
  }
}
