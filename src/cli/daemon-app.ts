import type { NotificationPort } from "../application/ports/notification.port"
import type { DaemonTranscriptionUseCase } from "../application/use-cases/daemon-transcription"
import { DaemonSignalHandler } from "./daemon-signals"
import type { DaemonCliOptions } from "./parser"
import { EXIT_CODES } from "./parser"
import { PidFile } from "./pid-file"
import { Presenter } from "./presenter"

/**
 * Daemon application orchestrator.
 * Manages the daemon lifecycle and signal handling.
 */
export class DaemonApp {
  private readonly presenter: Presenter
  private readonly pidFile: PidFile
  private signalHandler: DaemonSignalHandler | null = null
  private shouldExit = false
  private exitResolve: (() => void) | null = null

  constructor(
    private readonly useCase: DaemonTranscriptionUseCase,
    private readonly notifier: NotificationPort | null,
    private readonly options: DaemonCliOptions,
  ) {
    this.presenter = new Presenter()
    this.pidFile = new PidFile()
  }

  /**
   * Run the daemon
   */
  async run(): Promise<number> {
    // Acquire PID file
    const pidResult = this.pidFile.acquire()
    if (!pidResult.ok) {
      this.presenter.error(pidResult.error.message)
      return EXIT_CODES.ERROR
    }

    try {
      // Setup signal handlers
      this.setupSignalHandlers()

      // Print startup message
      this.printStartupMessage()

      // Setup use case callbacks
      this.setupUseCaseCallbacks()

      // Wait indefinitely until exit signal
      await this.waitForExit()

      return EXIT_CODES.SUCCESS
    } finally {
      // Cleanup
      this.signalHandler?.cleanup()
      this.pidFile.release()
    }
  }

  /**
   * Setup signal handlers
   */
  private setupSignalHandlers(): void {
    this.signalHandler = new DaemonSignalHandler(
      () => this.handleToggle(),
      () => this.handleCancel(),
      () => this.handleExit(),
    )
    this.signalHandler.setup()
  }

  /**
   * Setup use case event callbacks
   */
  private setupUseCaseCallbacks(): void {
    const maxDurationStr = this.presenter.formatDuration(
      this.options.maxDuration.toSeconds(),
    )

    this.useCase.setCallbacks({
      onRecordingStart: () => {
        this.presenter.info(`Recording started (max ${maxDurationStr})...`)
        this.notifier?.notify(
          "SmartScribe",
          `Recording started (max ${maxDurationStr})`,
          "audio-input-microphone",
        )
      },

      onRecordingProgress: (_elapsed) => {
        // Could update status here if needed
      },

      onMaxDurationReached: () => {
        this.presenter.info("Max duration reached, transcribing...")
        this.notifier?.notify(
          "SmartScribe",
          "Max duration reached, transcribing...",
          "dialog-information",
        )
      },

      onRecordingStop: (audioSize) => {
        this.presenter.info(`Recording stopped (${audioSize}), transcribing...`)
      },

      onRecordingCancel: () => {
        this.presenter.info("Recording cancelled")
        this.notifier?.notify(
          "SmartScribe",
          "Recording cancelled",
          "dialog-warning",
        )
        this.presenter.info("Waiting for signals...")
      },

      onTranscriptionStart: () => {
        this.presenter.startSpinner("Transcribing with Gemini...")
        this.notifier?.notify(
          "SmartScribe",
          "Transcribing audio...",
          "system-run",
        )
      },

      onTranscriptionComplete: (text) => {
        this.presenter.spinnerSuccess("Transcription complete")
        // Output to stdout
        this.presenter.output(text)
        this.notifier?.notify(
          "SmartScribe",
          "Transcription complete",
          "dialog-ok",
        )
        this.presenter.info("Waiting for signals...")
      },

      onClipboardCopy: (success) => {
        if (success) {
          this.presenter.success("Copied to clipboard")
        } else {
          this.presenter.warn("Could not copy to clipboard")
        }
      },

      onKeystrokeSend: (success) => {
        if (success) {
          this.presenter.success("Typed into focused window")
        } else {
          this.presenter.warn("Could not type into window")
        }
      },

      onError: (error) => {
        this.presenter.error(error.message)
        this.notifier?.notify(
          "SmartScribe",
          `Error: ${error.message}`,
          "dialog-error",
        )
        this.presenter.info("Waiting for signals...")
      },

      onStateChange: (_state) => {
        // Could log state changes for debugging
      },
    })
  }

  /**
   * Print the startup message with control commands
   */
  private printStartupMessage(): void {
    const pid = process.pid
    const maxDurationStr = this.presenter.formatDuration(
      this.options.maxDuration.toSeconds(),
    )

    const clipboard = this.options.clipboard ? "yes" : "no"
    const keystroke = this.options.keystroke ? "yes" : "no"
    const notify = this.options.notify ? "yes" : "no"

    this.presenter.info(`SmartScribe daemon started (PID: ${pid})`)
    this.presenter.info(`PID file: ${this.pidFile.getPath()}`)
    this.presenter.info(
      `Domain: ${this.options.domainId} | Clipboard: ${clipboard} | Keystroke: ${keystroke} | Notify: ${notify}`,
    )
    this.presenter.info(`Max recording: ${maxDurationStr}`)
    this.presenter.info("")
    this.presenter.info("Control commands:")
    this.presenter.info(
      `  kill -SIGUSR1 ${pid}   # Toggle recording (start/stop+transcribe)`,
    )
    this.presenter.info(`  kill -SIGUSR2 ${pid}   # Cancel recording`)
    this.presenter.info(`  kill -SIGINT  ${pid}   # Exit daemon`)
    this.presenter.info("")
    this.presenter.info("Waiting for signals...")

    this.notifier?.notify(
      "SmartScribe",
      "Daemon started, waiting for signals...",
      "audio-input-microphone",
    )
  }

  /**
   * Handle SIGUSR1 - Toggle recording (start if idle, stop+transcribe if recording)
   */
  private handleToggle(): void {
    if (this.useCase.isIdle) {
      // Start recording
      const result = this.useCase.startRecording()
      if (!result.ok) {
        this.presenter.error(result.error.message)
      }
    } else if (this.useCase.isRecording) {
      // Stop and transcribe
      this.useCase.stopAndTranscribe().catch((error) => {
        this.presenter.error(`Transcription failed: ${error.message}`)
      })
    } else {
      // Processing - warn user
      this.presenter.warn("Already transcribing, please wait...")
    }
  }

  /**
   * Handle SIGUSR2 - Cancel recording without transcribing
   */
  private handleCancel(): void {
    if (this.useCase.isRecording) {
      this.useCase.cancel().catch((error) => {
        this.presenter.error(`Cancel failed: ${error.message}`)
      })
    } else {
      this.presenter.warn("Nothing to cancel")
    }
  }

  /**
   * Handle SIGINT/SIGTERM - Exit daemon
   */
  private handleExit(): void {
    if (this.useCase.isRecording) {
      // Cancel recording first, then exit
      this.presenter.info("Cancelling recording and shutting down...")
      this.shouldExit = true
      this.useCase.cancel().catch((error) => {
        this.presenter.error(`Cancel failed: ${error.message}`)
      })
    } else if (this.useCase.isProcessing) {
      // Wait for processing to complete, then exit
      this.presenter.info(
        "Waiting for transcription to complete before exit...",
      )
      this.shouldExit = true
    } else {
      // Idle - exit immediately
      this.presenter.info("Shutting down...")
      this.notifier?.notify(
        "SmartScribe",
        "Daemon stopped",
        "dialog-information",
      )
      this.shouldExit = true
      this.exitResolve?.()
    }
  }

  /**
   * Wait for exit signal
   */
  private waitForExit(): Promise<void> {
    return new Promise((resolve) => {
      this.exitResolve = resolve

      // Check periodically if we should exit (for post-processing exit)
      const checkInterval = setInterval(() => {
        if (this.shouldExit && this.useCase.isIdle) {
          clearInterval(checkInterval)
          resolve()
        }
      }, 100)
    })
  }
}
