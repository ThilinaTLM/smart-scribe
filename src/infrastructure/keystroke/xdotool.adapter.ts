import {
  KeystrokeError,
  type KeystrokePort,
} from "../../application/ports/keystroke.port"
import { Result } from "../../domain/shared/result"

/**
 * X11 keystroke adapter using xdotool.
 * Implements the KeystrokePort interface.
 */
export class XdotoolKeystrokeAdapter implements KeystrokePort {
  /**
   * Type text into the focused application using xdotool
   */
  async type(text: string): Promise<Result<void, KeystrokeError>> {
    try {
      // Use xdotool for X11 keystroke injection
      const proc = Bun.spawn(["xdotool", "type", "--", text], {
        stdout: "pipe",
        stderr: "pipe",
      })

      const exitCode = await proc.exited

      if (exitCode !== 0) {
        const stderr = await new Response(proc.stderr).text()
        return Result.err(
          new KeystrokeError(
            `xdotool failed (exit code ${exitCode}): ${stderr}. ` +
              "Make sure xdotool is installed: sudo pacman -S xdotool",
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
            "xdotool not found. Install xdotool: sudo pacman -S xdotool",
          ),
        )
      }

      return Result.err(new KeystrokeError(`Keystroke error: ${message}`))
    }
  }
}
