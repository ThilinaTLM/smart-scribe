import type { Result } from "../../domain/shared/result"

/**
 * Error that occurs during notification operations
 */
export class NotificationError extends Error {
  readonly code = "NOTIFICATION_ERROR"

  constructor(
    message: string,
    public readonly cause?: Error,
  ) {
    super(message)
    this.name = "NotificationError"
  }
}

/**
 * Port interface for desktop notification operations.
 * Infrastructure layer implements this interface.
 */
export interface NotificationPort {
  /**
   * Display a desktop notification
   * @param title The notification title
   * @param message The notification message body
   * @param icon Optional icon name (e.g., "audio-input-microphone")
   * @returns Success or an error
   */
  notify(
    title: string,
    message: string,
    icon?: string,
  ): Promise<Result<void, NotificationError>>
}
