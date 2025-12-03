import {
  NotificationError,
  type NotificationPort,
} from "../../application/ports/notification.port"
import { Result } from "../../domain/shared/result"

/**
 * Desktop notification adapter using notify-send (libnotify).
 * Implements the NotificationPort interface.
 */
export class NotifySendAdapter implements NotificationPort {
  /**
   * Display a desktop notification using notify-send
   */
  async notify(
    title: string,
    message: string,
    icon?: string,
  ): Promise<Result<void, NotificationError>> {
    try {
      const args = ["notify-send"]

      if (icon) {
        args.push("-i", icon)
      }

      args.push(title, message)

      const proc = Bun.spawn(args, {
        stdout: "pipe",
        stderr: "pipe",
      })

      const exitCode = await proc.exited

      if (exitCode !== 0) {
        const stderr = await new Response(proc.stderr).text()
        return Result.err(
          new NotificationError(
            `notify-send failed (exit code ${exitCode}): ${stderr}. ` +
              "Make sure libnotify is installed: sudo pacman -S libnotify",
          ),
        )
      }

      return Result.ok(undefined)
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Unknown notification error"

      // Provide helpful error message
      if (message.includes("ENOENT") || message.includes("not found")) {
        return Result.err(
          new NotificationError(
            "notify-send not found. Install libnotify: sudo pacman -S libnotify",
          ),
        )
      }

      return Result.err(new NotificationError(`Notification error: ${message}`))
    }
  }
}
