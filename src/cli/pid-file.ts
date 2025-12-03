import { existsSync, readFileSync, unlinkSync, writeFileSync } from "node:fs"
import { PidFileError } from "../domain/daemon/errors/daemon.errors"
import { Result } from "../domain/shared/result"

const PID_FILE_PATH = "/tmp/smart-scribe.pid"

/**
 * PID file management for daemon mode.
 * Ensures only one daemon instance runs at a time.
 */
export class PidFile {
  private readonly path: string

  constructor(path: string = PID_FILE_PATH) {
    this.path = path
  }

  /**
   * Get the PID file path
   */
  getPath(): string {
    return this.path
  }

  /**
   * Acquire the PID file (write current PID).
   * Checks for stale PID files and handles them appropriately.
   */
  acquire(): Result<void, PidFileError> {
    // Check if PID file already exists
    if (existsSync(this.path)) {
      try {
        const existingPid = Number.parseInt(
          readFileSync(this.path, "utf-8").trim(),
          10,
        )

        if (Number.isNaN(existingPid)) {
          // Invalid PID file, overwrite
        } else if (PidFile.isProcessRunning(existingPid)) {
          return Result.err(
            new PidFileError(
              `Another daemon instance is running (PID: ${existingPid})`,
              existingPid,
            ),
          )
        }
        // Stale PID file, will be overwritten
      } catch {
        // Error reading file, will be overwritten
      }
    }

    // Write current PID
    try {
      writeFileSync(this.path, process.pid.toString(), "utf-8")
      return Result.ok(undefined)
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to write PID file"
      return Result.err(new PidFileError(message))
    }
  }

  /**
   * Release the PID file (delete it)
   */
  release(): void {
    try {
      if (existsSync(this.path)) {
        unlinkSync(this.path)
      }
    } catch {
      // Ignore cleanup errors
    }
  }

  /**
   * Check if a process with the given PID is running
   */
  static isProcessRunning(pid: number): boolean {
    try {
      // Sending signal 0 checks if process exists without actually sending a signal
      process.kill(pid, 0)
      return true
    } catch {
      return false
    }
  }
}
