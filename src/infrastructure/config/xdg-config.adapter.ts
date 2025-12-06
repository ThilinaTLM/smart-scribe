import { mkdir } from "node:fs/promises"
import { parse, stringify } from "@iarna/toml"
import type { ConfigPort } from "../../application/ports/config.port"
import {
  AppConfig,
  type AppConfigData,
  type ConfigError,
  ConfigFileNotFoundError,
  ConfigKeyNotFoundError,
  ConfigParseError,
  ConfigPath,
  ConfigValidationError,
  ConfigWriteError,
} from "../../domain/config"
import { Duration } from "../../domain/recording/value-objects/duration.vo"
import { Result } from "../../domain/shared/result"
import { DomainPreset } from "../../domain/transcription/value-objects/domain-preset.vo"

/**
 * TOML config file structure (flat, snake_case for file format)
 */
interface TomlConfig {
  api_key?: string
  duration?: string
  max_duration?: string
  domain?: string
  clipboard?: boolean
  keystroke?: boolean
  notify?: boolean
}

/**
 * Legacy nested TOML config structure (for backwards compatibility)
 */
interface LegacyTomlConfig {
  gemini?: { api_key?: string }
  recording?: { max_duration?: number; duration?: number }
  domain?: { default?: string }
  output?: { clipboard?: boolean; notification?: boolean }
}

/**
 * Valid config keys for user-facing commands
 */
const VALID_KEYS = [
  "api_key",
  "duration",
  "max_duration",
  "domain",
  "clipboard",
  "keystroke",
  "notify",
] as const

type ConfigKey = (typeof VALID_KEYS)[number]

/**
 * Adapter for XDG-compliant TOML config file operations.
 */
export class XdgConfigAdapter implements ConfigPort {
  private readonly configPath: string

  constructor() {
    this.configPath = ConfigPath.getConfigFilePath()
  }

  getPath(): string {
    return this.configPath
  }

  async exists(): Promise<boolean> {
    const file = Bun.file(this.configPath)
    return file.exists()
  }

  async load(): Promise<Result<AppConfig, ConfigError>> {
    if (!(await this.exists())) {
      return Result.err(new ConfigFileNotFoundError(this.configPath))
    }

    try {
      const file = Bun.file(this.configPath)
      const content = await file.text()
      const raw = parse(content) as Record<string, unknown>

      // Detect format: if we have nested sections like [gemini], use legacy parser
      const isLegacy =
        "gemini" in raw || ("domain" in raw && typeof raw.domain === "object")

      const data: AppConfigData = isLegacy
        ? this.parseLegacyConfig(raw as LegacyTomlConfig)
        : this.parseFlatConfig(raw as TomlConfig)

      return Result.ok(AppConfig.fromPartial(data))
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error"
      return Result.err(new ConfigParseError(message))
    }
  }

  /**
   * Parse flat config format (new style)
   */
  private parseFlatConfig(toml: TomlConfig): AppConfigData {
    return {
      apiKey: toml.api_key,
      duration: toml.duration,
      maxDuration: toml.max_duration,
      domain: toml.domain as AppConfigData["domain"],
      clipboard: toml.clipboard,
      keystroke: toml.keystroke,
      notify: toml.notify,
    }
  }

  /**
   * Parse legacy nested config format (for backwards compatibility)
   */
  private parseLegacyConfig(toml: LegacyTomlConfig): AppConfigData {
    // Convert numeric duration to string format
    const durationToString = (seconds?: number): string | undefined => {
      if (seconds === undefined) return undefined
      if (seconds >= 60 && seconds % 60 === 0) return `${seconds / 60}m`
      return `${seconds}s`
    }

    return {
      apiKey: toml.gemini?.api_key,
      duration: durationToString(toml.recording?.duration),
      maxDuration: durationToString(toml.recording?.max_duration),
      domain: toml.domain?.default as AppConfigData["domain"],
      clipboard: toml.output?.clipboard,
      notify: toml.output?.notification,
    }
  }

  async save(config: AppConfig): Promise<Result<void, ConfigError>> {
    try {
      // Ensure directory exists
      const dir = ConfigPath.getConfigDir()
      await mkdir(dir, { recursive: true })

      const data = config.toObject()

      // Convert camelCase to snake_case for TOML
      const toml: TomlConfig = {}
      if (data.apiKey !== undefined) toml.api_key = data.apiKey
      if (data.duration !== undefined) toml.duration = data.duration
      if (data.maxDuration !== undefined) toml.max_duration = data.maxDuration
      if (data.domain !== undefined) toml.domain = data.domain
      if (data.clipboard !== undefined) toml.clipboard = data.clipboard
      if (data.keystroke !== undefined) toml.keystroke = data.keystroke
      if (data.notify !== undefined) toml.notify = data.notify

      const content = stringify(toml as Record<string, unknown>)
      await Bun.write(this.configPath, content)

      return Result.ok(undefined)
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error"
      return Result.err(new ConfigWriteError(message))
    }
  }

  async createDefault(): Promise<Result<void, ConfigError>> {
    const defaultConfig = AppConfig.defaults()
    return this.save(defaultConfig)
  }

  async getValue(
    key: string,
  ): Promise<Result<string | boolean | undefined, ConfigError>> {
    if (!this.isValidKey(key)) {
      return Result.err(new ConfigKeyNotFoundError(key))
    }

    const loadResult = await this.load()
    if (!loadResult.ok) {
      // If file doesn't exist, return undefined (not an error)
      if (loadResult.error.code === "CONFIG_FILE_NOT_FOUND") {
        return Result.ok(undefined)
      }
      return loadResult
    }

    const config = loadResult.value
    const value = this.getValueByKey(config, key)
    return Result.ok(value)
  }

  async setValue(
    key: string,
    value: string,
  ): Promise<Result<void, ConfigError>> {
    if (!this.isValidKey(key)) {
      return Result.err(new ConfigKeyNotFoundError(key))
    }

    // Validate the value based on key type
    const validationResult = this.validateValue(key, value)
    if (!validationResult.ok) {
      return validationResult
    }

    // Load existing config or start with empty
    let config: AppConfig
    const loadResult = await this.load()
    if (loadResult.ok) {
      config = loadResult.value
    } else if (loadResult.error.code === "CONFIG_FILE_NOT_FOUND") {
      config = AppConfig.fromPartial({})
    } else {
      return Result.err(loadResult.error)
    }

    // Create new config with updated value
    const updatedData = this.setValueByKey(config.toObject(), key, value)
    const newConfig = AppConfig.fromPartial(updatedData)

    return this.save(newConfig)
  }

  private isValidKey(key: string): key is ConfigKey {
    return VALID_KEYS.includes(key as ConfigKey)
  }

  private getValueByKey(
    config: AppConfig,
    key: ConfigKey,
  ): string | boolean | undefined {
    switch (key) {
      case "api_key":
        return config.apiKey
      case "duration":
        return config.duration
      case "max_duration":
        return config.maxDuration
      case "domain":
        return config.domain
      case "clipboard":
        return config.clipboard
      case "keystroke":
        return config.keystroke
      case "notify":
        return config.notify
    }
  }

  private setValueByKey(
    data: AppConfigData,
    key: ConfigKey,
    value: string,
  ): AppConfigData {
    const updated = { ...data }

    switch (key) {
      case "api_key":
        updated.apiKey = value
        break
      case "duration":
        updated.duration = value
        break
      case "max_duration":
        updated.maxDuration = value
        break
      case "domain":
        updated.domain = value as AppConfigData["domain"]
        break
      case "clipboard":
        updated.clipboard = value.toLowerCase() === "true"
        break
      case "keystroke":
        updated.keystroke = value.toLowerCase() === "true"
        break
      case "notify":
        updated.notify = value.toLowerCase() === "true"
        break
    }

    return updated
  }

  private validateValue(
    key: ConfigKey,
    value: string,
  ): Result<void, ConfigError> {
    switch (key) {
      case "duration":
      case "max_duration": {
        const result = Duration.parse(value)
        if (!result.ok) {
          return Result.err(
            new ConfigValidationError(key, result.error.message),
          )
        }
        break
      }
      case "domain": {
        if (!DomainPreset.isValidId(value)) {
          const validDomains = DomainPreset.getAllIds().join(", ")
          return Result.err(
            new ConfigValidationError(key, `must be one of: ${validDomains}`),
          )
        }
        break
      }
      case "clipboard":
      case "keystroke":
      case "notify": {
        const lower = value.toLowerCase()
        if (lower !== "true" && lower !== "false") {
          return Result.err(
            new ConfigValidationError(key, "must be 'true' or 'false'"),
          )
        }
        break
      }
      // api_key: no validation needed
    }

    return Result.ok(undefined)
  }
}
