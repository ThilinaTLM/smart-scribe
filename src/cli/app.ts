import type { NotificationPort } from "../application/ports/notification.port"
import { TranscribeRecordingUseCase } from "../application/use-cases/transcribe-recording"
import { WaylandClipboardAdapter } from "../infrastructure/clipboard/clipboard.adapter"
import { loadEnvironment } from "../infrastructure/config/environment"
import { XdotoolKeystrokeAdapter } from "../infrastructure/keystroke/xdotool.adapter"
import { NotifySendAdapter } from "../infrastructure/notification/notify-send.adapter"
import { FFmpegRecorderAdapter } from "../infrastructure/recording/ffmpeg-recorder.adapter"
import { GeminiTranscriptionAdapter } from "../infrastructure/transcription/gemini-transcription.adapter"
import {
  type CliOptions,
  EXIT_CODES,
  getHelpText,
  parseCliArgs,
  VERSION,
} from "./parser"
import { Presenter } from "./presenter"
import { SignalHandler } from "./signals"

/**
 * Main CLI application.
 * Wires dependencies and orchestrates the CLI flow.
 */
export class App {
  private presenter: Presenter
  private signalHandler: SignalHandler
  private useCase: TranscribeRecordingUseCase | null = null
  private notifier: NotificationPort | null = null

  constructor() {
    this.presenter = new Presenter()
    this.signalHandler = new SignalHandler()
  }

  /**
   * Run the CLI application
   */
  async run(argv: string[]): Promise<number> {
    // Parse CLI arguments
    const parseResult = parseCliArgs(argv)

    if (!parseResult.ok) {
      this.presenter.error(parseResult.error.message)
      this.presenter.info("Run 'smart-scribe --help' for usage information.")
      return EXIT_CODES.USAGE_ERROR
    }

    const options = parseResult.value

    // Handle --help
    if (options.help) {
      console.log(getHelpText())
      return EXIT_CODES.SUCCESS
    }

    // Handle --version
    if (options.version) {
      console.log(`smart-scribe v${VERSION}`)
      return EXIT_CODES.SUCCESS
    }

    // Load environment
    const envResult = loadEnvironment()
    if (!envResult.ok) {
      this.presenter.error(envResult.error.message)
      return EXIT_CODES.ERROR
    }

    const env = envResult.value

    // Create infrastructure adapters
    const recorder = new FFmpegRecorderAdapter()
    const transcriber = new GeminiTranscriptionAdapter(env.geminiApiKey)
    const clipboard = options.clipboard
      ? new WaylandClipboardAdapter()
      : undefined
    const keystroke = options.keystroke
      ? new XdotoolKeystrokeAdapter()
      : undefined

    // Create notifier if enabled
    this.notifier = options.notify ? new NotifySendAdapter() : null

    // Create use case
    this.useCase = new TranscribeRecordingUseCase(
      recorder,
      transcriber,
      clipboard,
      keystroke,
    )

    // Setup signal handler to stop recording on Ctrl+C
    this.signalHandler.onCleanup(async () => {
      this.presenter.stopSpinner()
      this.presenter.info("Stopping...")
      this.notifier?.notify(
        "SmartScribe",
        "Recording cancelled",
        "dialog-warning",
      )
      if (this.useCase) {
        await this.useCase.stopEarly()
      }
    })

    // Execute the use case
    return await this.executeTranscription(options)
  }

  /**
   * Execute the transcription workflow
   */
  private async executeTranscription(options: CliOptions): Promise<number> {
    if (!this.useCase) {
      this.presenter.error("Application not initialized")
      return EXIT_CODES.ERROR
    }

    const durationStr = this.presenter.formatDuration(
      options.duration.toSeconds(),
    )

    const result = await this.useCase.execute({
      duration: options.duration,
      domainId: options.domainId,
      enableClipboard: options.clipboard,
      enableKeystroke: options.keystroke,

      onRecordingStart: () => {
        this.presenter.startSpinner(
          `Recording for ${durationStr} (domain: ${options.domainId})...`,
        )
        this.notifier?.notify(
          "SmartScribe",
          `Recording started (${durationStr})`,
          "audio-input-microphone",
        )
      },

      onRecordingProgress: (elapsed, total) => {
        if (!this.signalHandler.shuttingDown) {
          this.presenter.updateSpinner(
            this.presenter.formatRecordingProgress(elapsed, total),
          )
        }
      },

      onRecordingComplete: () => {
        this.presenter.spinnerSuccess("Recording complete")
      },

      onTranscriptionStart: () => {
        this.presenter.startSpinner("Transcribing with Gemini...")
        this.notifier?.notify(
          "SmartScribe",
          "Transcribing audio...",
          "system-run",
        )
      },

      onTranscriptionComplete: () => {
        this.presenter.spinnerSuccess("Transcription complete")
      },

      onClipboardCopy: (success) => {
        if (success) {
          this.presenter.success("Copied to clipboard")
          this.notifier?.notify(
            "SmartScribe",
            "Copied to clipboard",
            "edit-copy",
          )
        } else {
          this.presenter.warn("Could not copy to clipboard")
        }
      },

      onKeystrokeSend: (success) => {
        if (success) {
          this.presenter.success("Typed into focused window")
          this.notifier?.notify(
            "SmartScribe",
            "Typed into focused window",
            "input-keyboard",
          )
        } else {
          this.presenter.warn("Could not type into window")
        }
      },
    })

    if (!result.ok) {
      this.presenter.error(result.error.message)
      return EXIT_CODES.ERROR
    }

    // Output the transcription to stdout (for piping)
    this.presenter.output(result.value.text)

    return EXIT_CODES.SUCCESS
  }
}
