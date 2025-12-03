import chalk from "chalk"
import ora, { type Ora } from "ora"

/**
 * CLI presenter for formatted output using ora and chalk.
 * Handles spinners, colors, and POSIX-compliant output streams.
 */
export class Presenter {
  private spinner: Ora | null = null

  /**
   * Start a spinner with a message (writes to stderr)
   */
  startSpinner(message: string): void {
    this.spinner = ora({
      text: message,
      stream: process.stderr,
    }).start()
  }

  /**
   * Update the spinner text
   */
  updateSpinner(message: string): void {
    if (this.spinner) {
      this.spinner.text = message
    }
  }

  /**
   * Stop the spinner with a success message
   */
  spinnerSuccess(message: string): void {
    if (this.spinner) {
      this.spinner.succeed(message)
      this.spinner = null
    }
  }

  /**
   * Stop the spinner with a failure message
   */
  spinnerFail(message: string): void {
    if (this.spinner) {
      this.spinner.fail(message)
      this.spinner = null
    }
  }

  /**
   * Stop the spinner with an info message
   */
  spinnerInfo(message: string): void {
    if (this.spinner) {
      this.spinner.info(message)
      this.spinner = null
    }
  }

  /**
   * Stop any running spinner
   */
  stopSpinner(): void {
    if (this.spinner) {
      this.spinner.stop()
      this.spinner = null
    }
  }

  /**
   * Write info message to stderr
   */
  info(message: string): void {
    console.error(chalk.blue("ℹ"), message)
  }

  /**
   * Write success message to stderr
   */
  success(message: string): void {
    console.error(chalk.green("✓"), message)
  }

  /**
   * Write warning message to stderr
   */
  warn(message: string): void {
    console.error(chalk.yellow("⚠"), message)
  }

  /**
   * Write error message to stderr
   */
  error(message: string): void {
    console.error(chalk.red("✗"), message)
  }

  /**
   * Write the final transcription result to stdout (for piping)
   */
  output(text: string): void {
    console.log(text)
  }

  /**
   * Format recording progress message
   */
  formatRecordingProgress(elapsed: number, total: number): string {
    const remaining = Math.max(0, total - elapsed)
    const bar = this.createProgressBar(elapsed, total, 20)
    return `Recording ${bar} ${remaining.toFixed(1)}s remaining`
  }

  /**
   * Create a simple progress bar
   */
  private createProgressBar(
    current: number,
    total: number,
    width: number,
  ): string {
    const progress = Math.min(current / total, 1)
    const filled = Math.round(progress * width)
    const empty = width - filled
    return chalk.green("█".repeat(filled)) + chalk.gray("░".repeat(empty))
  }

  /**
   * Format duration for display
   */
  formatDuration(seconds: number): string {
    if (seconds < 60) {
      return `${seconds}s`
    }
    const mins = Math.floor(seconds / 60)
    const secs = seconds % 60
    return secs > 0 ? `${mins}m${secs}s` : `${mins}m`
  }
}
