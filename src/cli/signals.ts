/**
 * Signal handler for graceful shutdown.
 * Handles SIGINT (Ctrl+C) and SIGTERM.
 */
export class SignalHandler {
  private cleanupCallbacks: Array<() => Promise<void> | void> = []
  private isShuttingDown = false

  constructor() {
    this.setupHandlers()
  }

  /**
   * Register a cleanup callback to run on shutdown
   */
  onCleanup(callback: () => Promise<void> | void): void {
    this.cleanupCallbacks.push(callback)
  }

  /**
   * Check if shutdown is in progress
   */
  get shuttingDown(): boolean {
    return this.isShuttingDown
  }

  /**
   * Setup signal handlers
   */
  private setupHandlers(): void {
    const handler = async (_signal: string) => {
      if (this.isShuttingDown) {
        // Force exit on second signal
        process.exit(130)
      }

      this.isShuttingDown = true

      // Run all cleanup callbacks
      for (const callback of this.cleanupCallbacks) {
        try {
          await callback()
        } catch {
          // Ignore cleanup errors during shutdown
        }
      }

      // Exit after cleanup (exit code 130 = 128 + SIGINT(2))
      process.exit(130)
    }

    process.on("SIGINT", () => handler("SIGINT"))
    process.on("SIGTERM", () => handler("SIGTERM"))
  }

  /**
   * Remove signal handlers (for testing)
   */
  removeHandlers(): void {
    process.removeAllListeners("SIGINT")
    process.removeAllListeners("SIGTERM")
  }
}
