import { DomainError } from "../../domain/shared/result"

/**
 * Error for environment/configuration issues.
 * Used by ConfigService when required configuration is missing.
 */
export class EnvironmentError extends DomainError {
  readonly code = "ENVIRONMENT_ERROR"
}
