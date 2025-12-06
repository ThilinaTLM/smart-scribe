import type { NotificationPort } from "../application/ports/notification.port"
import { DaemonTranscriptionUseCase } from "../application/use-cases/daemon-transcription"
import { TranscribeRecordingUseCase } from "../application/use-cases/transcribe-recording"
import type { AppConfig } from "../domain/config"
import { Duration } from "../domain/recording/value-objects/duration.vo"
import type { DomainId } from "../domain/transcription/value-objects/domain-preset.vo"
import { WaylandClipboardAdapter } from "../infrastructure/clipboard/clipboard.adapter"
import { ConfigService } from "../infrastructure/config/config.service"
import { XdgConfigAdapter } from "../infrastructure/config/xdg-config.adapter"
import { XdotoolKeystrokeAdapter } from "../infrastructure/keystroke/xdotool.adapter"
import { NotifySendAdapter } from "../infrastructure/notification/notify-send.adapter"
import { FFmpegRecorderAdapter } from "../infrastructure/recording/ffmpeg-recorder.adapter"
import { GeminiTranscriptionAdapter } from "../infrastructure/transcription/gemini-transcription.adapter"
import { ConfigCommand } from "./config-command"
import { DaemonApp } from "./daemon-app"
import {
  type CliOptions,
  type DaemonCliOptions,
  EXIT_CODES,
  getHelpText,
  parseCliArgs,
  type ResolvedCliOptions,
  type ResolvedDaemonCliOptions,
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

    // Handle config subcommand (doesn't need API key)
    if (options.mode === "config") {
      const configAdapter = new XdgConfigAdapter()
      const configCommand = new ConfigCommand(configAdapter)
      return await configCommand.execute(options.configAction)
    }

    // Handle --help and --version (only in oneshot mode)
    if (options.mode === "oneshot") {
      if (options.help) {
        console.log(getHelpText())
        return EXIT_CODES.SUCCESS
      }

      if (options.version) {
        console.log(`smart-scribe v${VERSION}`)
        return EXIT_CODES.SUCCESS
      }
    }

    // Load config with priority merging
    const configAdapter = new XdgConfigAdapter()
    const configService = new ConfigService(configAdapter)
    const configResult = await configService.loadMergedConfig()

    if (!configResult.ok) {
      this.presenter.error(configResult.error.message)
      return EXIT_CODES.ERROR
    }

    const config = configResult.value

    // Branch based on mode
    if (options.mode === "daemon") {
      return await this.runDaemonMode(options, config)
    }

    return await this.runOneshotMode(options, config)
  }

  /**
   * Run in daemon mode
   */
  private async runDaemonMode(
    options: DaemonCliOptions,
    config: AppConfig,
  ): Promise<number> {
    // API key is guaranteed by ConfigService validation
    const apiKey = config.apiKey
    if (!apiKey) {
      this.presenter.error("API key not configured")
      return EXIT_CODES.ERROR
    }

    // Merge CLI options with config defaults (CLI > config > hardcoded default)
    const clipboard = options.clipboard || (config.clipboard ?? false)
    const keystroke = options.keystroke || (config.keystroke ?? false)
    const notify = options.notify || (config.notify ?? false)
    const domainId: DomainId =
      options.domainId ?? (config.domain as DomainId) ?? "general"

    // Parse maxDuration from config if not provided via CLI
    let maxDuration = options.maxDuration
    if (!maxDuration) {
      const configMaxDuration = config.maxDuration ?? "60s"
      const parseResult = Duration.parse(configMaxDuration)
      if (!parseResult.ok) {
        this.presenter.error(
          `Invalid max_duration in config: ${configMaxDuration}`,
        )
        return EXIT_CODES.ERROR
      }
      maxDuration = parseResult.value
    }

    // Create infrastructure adapters
    const recorder = new FFmpegRecorderAdapter()
    const transcriber = new GeminiTranscriptionAdapter(apiKey)
    const clipboardAdapter = clipboard
      ? new WaylandClipboardAdapter()
      : undefined
    const keystrokeAdapter = keystroke
      ? new XdotoolKeystrokeAdapter()
      : undefined
    const notifier = notify ? new NotifySendAdapter() : null

    // Create daemon use case
    const daemonUseCase = new DaemonTranscriptionUseCase(
      recorder,
      transcriber,
      {
        domainId,
        maxDuration,
        enableClipboard: clipboard,
        enableKeystroke: keystroke,
      },
      clipboardAdapter,
      keystrokeAdapter,
    )

    // Create and run daemon app with merged options
    const mergedOptions: ResolvedDaemonCliOptions = {
      mode: "daemon",
      domainId,
      maxDuration,
      clipboard,
      keystroke,
      notify,
    }
    const daemonApp = new DaemonApp(daemonUseCase, notifier, mergedOptions)
    return await daemonApp.run()
  }

  /**
   * Run in one-shot mode
   */
  private async runOneshotMode(
    options: CliOptions,
    config: AppConfig,
  ): Promise<number> {
    // API key is guaranteed by ConfigService validation
    const apiKey = config.apiKey
    if (!apiKey) {
      this.presenter.error("API key not configured")
      return EXIT_CODES.ERROR
    }

    // Merge CLI options with config defaults (CLI > config > hardcoded default)
    const clipboard = options.clipboard || (config.clipboard ?? false)
    const keystroke = options.keystroke || (config.keystroke ?? false)
    const notify = options.notify || (config.notify ?? false)
    const domainId: DomainId =
      options.domainId ?? (config.domain as DomainId) ?? "general"

    // Parse duration from config if not provided via CLI
    let duration = options.duration
    if (!duration) {
      const configDuration = config.duration ?? "10s"
      const parseResult = Duration.parse(configDuration)
      if (!parseResult.ok) {
        this.presenter.error(`Invalid duration in config: ${configDuration}`)
        return EXIT_CODES.ERROR
      }
      duration = parseResult.value
    }

    // Update options with merged values for later use
    const mergedOptions: ResolvedCliOptions = {
      mode: "oneshot",
      duration,
      domainId,
      clipboard,
      keystroke,
      notify,
      help: false,
      version: false,
    }

    // Create infrastructure adapters
    const recorder = new FFmpegRecorderAdapter()
    const transcriber = new GeminiTranscriptionAdapter(apiKey)
    const clipboardAdapter = clipboard
      ? new WaylandClipboardAdapter()
      : undefined
    const keystrokeAdapter = keystroke
      ? new XdotoolKeystrokeAdapter()
      : undefined

    // Create notifier if enabled
    this.notifier = notify ? new NotifySendAdapter() : null

    // Create use case
    this.useCase = new TranscribeRecordingUseCase(
      recorder,
      transcriber,
      clipboardAdapter,
      keystrokeAdapter,
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
    return await this.executeTranscription(mergedOptions)
  }

  /**
   * Execute the transcription workflow
   */
  private async executeTranscription(
    options: ResolvedCliOptions,
  ): Promise<number> {
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

      onRecordingComplete: (audioSize) => {
        this.presenter.spinnerSuccess(`Recording complete (${audioSize})`)
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
