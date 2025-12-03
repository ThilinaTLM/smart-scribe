/**
 * Signal handler for daemon mode.
 * Handles SIGUSR1, SIGUSR2, and SIGINT signals.
 */
export class DaemonSignalHandler {
  private isSetup = false

  constructor(
    private readonly onStart: () => void,
    private readonly onStop: () => void,
    private readonly onCancel: () => void,
  ) {}

  /**
   * Setup signal handlers
   */
  setup(): void {
    if (this.isSetup) return

    process.on("SIGUSR1", () => {
      this.onStart()
    })

    process.on("SIGUSR2", () => {
      this.onStop()
    })

    process.on("SIGINT", () => {
      this.onCancel()
    })

    process.on("SIGTERM", () => {
      this.onCancel()
    })

    this.isSetup = true
  }

  /**
   * Remove all signal handlers
   */
  cleanup(): void {
    process.removeAllListeners("SIGUSR1")
    process.removeAllListeners("SIGUSR2")
    process.removeAllListeners("SIGINT")
    process.removeAllListeners("SIGTERM")
    this.isSetup = false
  }
}
