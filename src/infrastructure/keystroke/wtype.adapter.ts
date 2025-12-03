import {
  KeystrokeError,
  type KeystrokePort,
} from "../../application/ports/keystroke.port"
import { Result } from "../../domain/shared/result"

/**
 * Wayland keystroke adapter using wtype.
 * Implements the KeystrokePort interface.
 */
export class WtypeKeystrokeAdapter implements KeystrokePort {
  /**
   * Type text into the focused application using wtype
   */
  async type(text: string): Promise<Result<void, KeystrokeError>> {
    try {
      // Use wtype for Wayland keystroke injection
      const proc = Bun.spawn(["wtype", "--", text], {
        stdout: "pipe",
        stderr: "pipe",
      })

      const exitCode = await proc.exited

      if (exitCode !== 0) {
        const stderr = await new Response(proc.stderr).text()
        return Result.err(
          new KeystrokeError(
            `wtype failed (exit code ${exitCode}): ${stderr}. ` +
              "Make sure wtype is installed: sudo pacman -S wtype",
          ),
        )
      }

      return Result.ok(undefined)
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Unknown keystroke error"

      // Provide helpful error message
      if (message.includes("ENOENT") || message.includes("not found")) {
        return Result.err(
          new KeystrokeError(
            "wtype not found. Install wtype: sudo pacman -S wtype",
          ),
        )
      }

      return Result.err(new KeystrokeError(`Keystroke error: ${message}`))
    }
  }
}
