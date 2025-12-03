import {
  ClipboardError,
  type ClipboardPort,
} from "../../application/ports/clipboard.port"
import { Result } from "../../domain/shared/result"

/**
 * Wayland clipboard adapter using wl-copy.
 * Implements the ClipboardPort interface.
 */
export class WaylandClipboardAdapter implements ClipboardPort {
  /**
   * Copy text to the Wayland clipboard using wl-copy
   */
  async copy(text: string): Promise<Result<void, ClipboardError>> {
    try {
      // Use wl-copy for Wayland clipboard
      const proc = Bun.spawn(["wl-copy", "--"], {
        stdin: new TextEncoder().encode(text),
        stdout: "pipe",
        stderr: "pipe",
      })

      const exitCode = await proc.exited

      if (exitCode !== 0) {
        const stderr = await new Response(proc.stderr).text()
        return Result.err(
          new ClipboardError(
            `wl-copy failed (exit code ${exitCode}): ${stderr}. ` +
              "Make sure wl-clipboard is installed: sudo pacman -S wl-clipboard",
          ),
        )
      }

      return Result.ok(undefined)
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Unknown clipboard error"

      // Provide helpful error message
      if (message.includes("ENOENT") || message.includes("not found")) {
        return Result.err(
          new ClipboardError(
            "wl-copy not found. Install wl-clipboard: sudo pacman -S wl-clipboard",
          ),
        )
      }

      return Result.err(new ClipboardError(`Clipboard error: ${message}`))
    }
  }
}
