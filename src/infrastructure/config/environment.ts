import { DomainError, Result } from "../../domain/shared/result"

export class EnvironmentError extends DomainError {
  readonly code = "ENVIRONMENT_ERROR"
}

export interface Environment {
  geminiApiKey: string
}

export function loadEnvironment(): Result<Environment, EnvironmentError> {
  const geminiApiKey = process.env.GEMINI_API_KEY

  if (!geminiApiKey) {
    return Result.err(
      new EnvironmentError(
        "GEMINI_API_KEY environment variable is not set. " +
          "Please create a .env file with GEMINI_API_KEY=your_api_key",
      ),
    )
  }

  return Result.ok({ geminiApiKey })
}
