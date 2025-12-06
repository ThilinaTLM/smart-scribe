import type { DomainId } from "../../transcription/value-objects/domain-preset.vo"

/**
 * Raw config data structure
 */
export interface AppConfigData {
  apiKey?: string
  duration?: string
  maxDuration?: string
  domain?: DomainId
  clipboard?: boolean
  keystroke?: boolean
  notify?: boolean
}

/**
 * Value object representing application configuration.
 * Immutable and supports merging with priority.
 */
export class AppConfig {
  private constructor(private readonly data: AppConfigData) {}

  /**
   * Create config with default values
   */
  static defaults(): AppConfig {
    return new AppConfig({
      duration: "10s",
      maxDuration: "60s",
      domain: "general",
      clipboard: false,
      keystroke: false,
      notify: false,
    })
  }

  /**
   * Create config from partial data
   */
  static fromPartial(data: Partial<AppConfigData>): AppConfig {
    return new AppConfig(data)
  }

  /**
   * Merge this config with another, where other takes precedence.
   * Only non-undefined values from other will override this.
   */
  merge(other: AppConfig): AppConfig {
    const merged: AppConfigData = { ...this.data }

    for (const [key, value] of Object.entries(other.data)) {
      if (value !== undefined) {
        merged[key as keyof AppConfigData] = value as never
      }
    }

    return new AppConfig(merged)
  }

  get apiKey(): string | undefined {
    return this.data.apiKey
  }
  get duration(): string | undefined {
    return this.data.duration
  }
  get maxDuration(): string | undefined {
    return this.data.maxDuration
  }
  get domain(): DomainId | undefined {
    return this.data.domain
  }
  get clipboard(): boolean | undefined {
    return this.data.clipboard
  }
  get keystroke(): boolean | undefined {
    return this.data.keystroke
  }
  get notify(): boolean | undefined {
    return this.data.notify
  }

  /**
   * Get raw config data object
   */
  toObject(): AppConfigData {
    return { ...this.data }
  }
}
