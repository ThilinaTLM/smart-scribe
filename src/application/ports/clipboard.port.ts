import type { Result } from "../../domain/shared/result"

/**
 * Error that occurs during clipboard operations
 */
export class ClipboardError extends Error {
  readonly code = "CLIPBOARD_ERROR"

  constructor(
    message: string,
    public readonly cause?: Error,
  ) {
    super(message)
    this.name = "ClipboardError"
  }
}

/**
 * Port interface for clipboard operations.
 * Infrastructure layer implements this interface.
 */
export interface ClipboardPort {
  /**
   * Copy text to the system clipboard
   * @param text The text to copy
   * @returns Success or an error
   */
  copy(text: string): Promise<Result<void, ClipboardError>>
}
