import type { ConfigPort } from "../application/ports/config.port"
import { AppConfig } from "../domain/config"
import type { ConfigAction } from "./config-parser"
import { EXIT_CODES } from "./parser"
import { Presenter } from "./presenter"

/**
 * Handler for config subcommands (init, set, get, list, path)
 */
export class ConfigCommand {
  private presenter: Presenter

  constructor(private readonly configAdapter: ConfigPort) {
    this.presenter = new Presenter()
  }

  /**
   * Execute a config action and return exit code
   */
  async execute(action: ConfigAction): Promise<number> {
    switch (action.action) {
      case "init":
        return this.init()
      case "set":
        return this.set(action.key, action.value)
      case "get":
        return this.get(action.key)
      case "list":
        return this.list()
      case "path":
        return this.path()
    }
  }

  private async init(): Promise<number> {
    if (await this.configAdapter.exists()) {
      this.presenter.warn(
        `Config file already exists: ${this.configAdapter.getPath()}`,
      )
      return EXIT_CODES.ERROR
    }

    const result = await this.configAdapter.createDefault()
    if (!result.ok) {
      this.presenter.error(result.error.message)
      return EXIT_CODES.ERROR
    }

    this.presenter.success(
      `Created config file: ${this.configAdapter.getPath()}`,
    )
    return EXIT_CODES.SUCCESS
  }

  private async set(key: string, value: string): Promise<number> {
    const result = await this.configAdapter.setValue(key, value)
    if (!result.ok) {
      this.presenter.error(result.error.message)
      return EXIT_CODES.ERROR
    }

    this.presenter.success(`Set ${key} = ${value}`)
    return EXIT_CODES.SUCCESS
  }

  private async get(key: string): Promise<number> {
    const result = await this.configAdapter.getValue(key)
    if (!result.ok) {
      this.presenter.error(result.error.message)
      return EXIT_CODES.ERROR
    }

    const value = result.value
    if (value === undefined) {
      this.presenter.info(`${key}: (not set)`)
    } else {
      console.log(value)
    }
    return EXIT_CODES.SUCCESS
  }

  private async list(): Promise<number> {
    const result = await this.configAdapter.load()

    let config: AppConfig
    if (!result.ok) {
      // If file doesn't exist, show defaults
      if (result.error.code === "CONFIG_FILE_NOT_FOUND") {
        this.presenter.info("No config file found. Showing defaults:")
        config = AppConfig.defaults()
      } else {
        this.presenter.error(result.error.message)
        return EXIT_CODES.ERROR
      }
    } else {
      config = result.value
    }

    const data = config.toObject()

    // Convert camelCase to snake_case for display
    const keyMap: Record<string, string> = {
      apiKey: "api_key",
      duration: "duration",
      maxDuration: "max_duration",
      domain: "domain",
      clipboard: "clipboard",
      keystroke: "keystroke",
      notify: "notify",
    }

    for (const [camelKey, snakeKey] of Object.entries(keyMap)) {
      const value = data[camelKey as keyof typeof data]
      const displayValue = value === undefined ? "(not set)" : String(value)
      console.log(`${snakeKey} = ${displayValue}`)
    }

    return EXIT_CODES.SUCCESS
  }

  private path(): number {
    console.log(this.configAdapter.getPath())
    return EXIT_CODES.SUCCESS
  }
}
