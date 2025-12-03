/**
 * Signal handler for daemon mode.
 * Handles SIGUSR1 (toggle), SIGUSR2 (cancel), and SIGINT/SIGTERM (exit).
 */
export class DaemonSignalHandler {
  private isSetup = false

  constructor(
    private readonly onToggle: () => void,
    private readonly onCancel: () => void,
    private readonly onExit: () => void,
  ) {}

  /**
   * Setup signal handlers
   */
  setup(): void {
    if (this.isSetup) return

    process.on("SIGUSR1", () => {
      this.onToggle()
    })

    process.on("SIGUSR2", () => {
      this.onCancel()
    })

    process.on("SIGINT", () => {
      this.onExit()
    })

    process.on("SIGTERM", () => {
      this.onExit()
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
