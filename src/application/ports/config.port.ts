import type { AppConfig, ConfigError } from "../../domain/config"
import type { Result } from "../../domain/shared/result"

/**
 * Port for configuration file operations.
 * Handles reading/writing config files using XDG conventions.
 */
export interface ConfigPort {
  /**
   * Load configuration from file
   */
  load(): Promise<Result<AppConfig, ConfigError>>

  /**
   * Save configuration to file
   */
  save(config: AppConfig): Promise<Result<void, ConfigError>>

  /**
   * Get the config file path
   */
  getPath(): string

  /**
   * Check if config file exists
   */
  exists(): Promise<boolean>

  /**
   * Create config file with default values
   */
  createDefault(): Promise<Result<void, ConfigError>>

  /**
   * Get a single config value by key
   */
  getValue(
    key: string,
  ): Promise<Result<string | boolean | undefined, ConfigError>>

  /**
   * Set a single config value by key
   */
  setValue(key: string, value: string): Promise<Result<void, ConfigError>>
}
