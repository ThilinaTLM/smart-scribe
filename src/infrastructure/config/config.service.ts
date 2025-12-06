import type { ConfigPort } from "../../application/ports/config.port"
import {
  AppConfig,
  type AppConfigData,
  type ConfigError,
} from "../../domain/config"
import { Result } from "../../domain/shared/result"
import { EnvironmentError } from "./environment"

/**
 * Service that orchestrates config loading from multiple sources.
 * Handles priority: CLI args > env vars > config file > defaults
 */
export class ConfigService {
  constructor(
    private readonly configAdapter: ConfigPort,
    private readonly cliOptions?: Partial<AppConfigData>,
  ) {}

  /**
   * Load configuration with priority merging.
   * Priority: CLI args > env vars > config file > defaults
   */
  async loadMergedConfig(): Promise<
    Result<AppConfig, ConfigError | EnvironmentError>
  > {
    // 1. Start with defaults
    let config = AppConfig.defaults()

    // 2. Merge config file (if exists)
    if (await this.configAdapter.exists()) {
      const fileResult = await this.configAdapter.load()
      if (fileResult.ok) {
        config = config.merge(fileResult.value)
      }
      // Ignore file errors gracefully (use defaults)
    }

    // 3. Merge environment variables
    const envConfig = this.loadEnvConfig()
    config = config.merge(envConfig)

    // 4. Merge CLI options (highest priority)
    if (this.cliOptions) {
      config = config.merge(AppConfig.fromPartial(this.cliOptions))
    }

    // 5. Validate required fields
    if (!config.apiKey) {
      return Result.err(
        new EnvironmentError(
          "GEMINI_API_KEY not set. Use 'smart-scribe config set api_key <key>' " +
            "or set GEMINI_API_KEY environment variable.",
        ),
      )
    }

    return Result.ok(config)
  }

  /**
   * Load config values from environment variables
   */
  private loadEnvConfig(): AppConfig {
    return AppConfig.fromPartial({
      apiKey: process.env.GEMINI_API_KEY,
    })
  }
}
