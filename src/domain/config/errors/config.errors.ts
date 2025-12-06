import { DomainError } from "../../shared/result"

export class ConfigFileNotFoundError extends DomainError {
  readonly code = "CONFIG_FILE_NOT_FOUND"
  constructor(path: string) {
    super(`Config file not found: ${path}`)
  }
}

export class ConfigParseError extends DomainError {
  readonly code = "CONFIG_PARSE_ERROR"
  constructor(message: string) {
    super(`Failed to parse config file: ${message}`)
  }
}

export class ConfigValidationError extends DomainError {
  readonly code = "CONFIG_VALIDATION_ERROR"
  constructor(key: string, message: string) {
    super(`Invalid config value for '${key}': ${message}`)
  }
}

export class ConfigWriteError extends DomainError {
  readonly code = "CONFIG_WRITE_ERROR"
  constructor(message: string) {
    super(`Failed to write config file: ${message}`)
  }
}

export class ConfigKeyNotFoundError extends DomainError {
  readonly code = "CONFIG_KEY_NOT_FOUND"
  constructor(key: string) {
    super(
      `Unknown config key: ${key}. Valid keys: api_key, duration, max_duration, domain, clipboard, keystroke, notify`,
    )
  }
}

export type ConfigError =
  | ConfigFileNotFoundError
  | ConfigParseError
  | ConfigValidationError
  | ConfigWriteError
  | ConfigKeyNotFoundError
