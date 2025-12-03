import type { Result } from "../../domain/shared/result"

/**
 * Error that occurs during keystroke operations
 */
export class KeystrokeError extends Error {
  readonly code = "KEYSTROKE_ERROR"

  constructor(
    message: string,
    public readonly cause?: Error,
  ) {
    super(message)
    this.name = "KeystrokeError"
  }
}

/**
 * Port interface for keystroke injection operations.
 * Infrastructure layer implements this interface.
 */
export interface KeystrokePort {
  /**
   * Type text into the focused application
   * @param text The text to type
   * @returns Success or an error
   */
  type(text: string): Promise<Result<void, KeystrokeError>>
}
