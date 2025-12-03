import { parseArgs } from "node:util"
import { Duration } from "../domain/recording/value-objects/duration.vo"
import { Result } from "../domain/shared/result"
import {
  type DomainId,
  DomainPreset,
} from "../domain/transcription/value-objects/domain-preset.vo"

/**
 * POSIX exit codes
 */
export const EXIT_CODES = {
  SUCCESS: 0,
  ERROR: 1,
  USAGE_ERROR: 2,
} as const

/**
 * Parsed CLI options for one-shot mode
 */
export interface CliOptions {
  mode: "oneshot"
  duration: Duration
  domainId: DomainId
  clipboard: boolean
  keystroke: boolean
  notify: boolean
  help: boolean
  version: boolean
}

/**
 * Parsed CLI options for daemon mode
 */
export interface DaemonCliOptions {
  mode: "daemon"
  maxDuration: Duration
  domainId: DomainId
  clipboard: boolean
  keystroke: boolean
  notify: boolean
}

/**
 * Union of all CLI option types
 */
export type ParsedCliOptions = CliOptions | DaemonCliOptions

/**
 * CLI parsing error
 */
export class CliParseError extends Error {
  constructor(message: string) {
    super(message)
    this.name = "CliParseError"
  }
}

/**
 * Package version (read from package.json would be ideal, but hardcode for simplicity)
 */
export const VERSION = "1.0.0"

/**
 * Help text
 */
export function getHelpText(): string {
  const domains = DomainPreset.getAllIds().join("|")

  return `
smart-scribe - AI-powered voice to text transcription

USAGE:
    smart-scribe [OPTIONS]
    smart-scribe --daemon [OPTIONS]

ONE-SHOT MODE OPTIONS:
    -d, --duration <TIME>    Recording duration (default: 10s)
                             Formats: 30s, 1m, 2m30s

DAEMON MODE OPTIONS:
    --daemon                 Run as daemon, controlled by signals
    --max-duration <TIME>    Max recording duration (default: 60s)
                             Auto-stops and transcribes when reached

COMMON OPTIONS:
    -D, --domain <DOMAIN>    Domain preset (default: general)
                             Options: ${domains}
    -c, --clipboard          Copy transcription to clipboard
    -k, --keystroke          Type transcription into focused window
    -n, --notify             Show desktop notifications
    -h, --help               Show this help message
    -v, --version            Show version

ONE-SHOT EXAMPLES:
    smart-scribe                     # 10s transcription to stdout only
    smart-scribe -c                  # Copy result to clipboard
    smart-scribe -d 60s -c           # 60 second recording + clipboard
    smart-scribe -d 1m -D dev -k     # 1 minute, dev domain, keystroke

DAEMON EXAMPLES:
    smart-scribe --daemon -c -n      # Daemon with clipboard + notifications
    smart-scribe --daemon -D dev     # Daemon with dev domain
    smart-scribe --daemon --max-duration 5m  # 5 minute max recording

DAEMON SIGNALS:
    SIGUSR1  Start recording
    SIGUSR2  Stop recording and transcribe
    SIGINT   Cancel recording (or exit if idle)

DOMAINS:
    general   - General conversation (default)
    dev       - Software engineering terminology
    medical   - Medical/healthcare terms
    legal     - Legal terminology
    finance   - Financial terms and acronyms

OUTPUT:
    Transcribed text is always written to stdout.
    Use -c to copy to clipboard, -k to type into focused window.
    Status messages are written to stderr.
`.trim()
}

/**
 * Parse command line arguments
 */
export function parseCliArgs(
  argv: string[],
): Result<ParsedCliOptions, CliParseError> {
  try {
    const { values } = parseArgs({
      args: argv,
      options: {
        duration: { type: "string", short: "d", default: "10s" },
        domain: { type: "string", short: "D", default: "general" },
        clipboard: { type: "boolean", short: "c", default: false },
        keystroke: { type: "boolean", short: "k", default: false },
        notify: { type: "boolean", short: "n", default: false },
        help: { type: "boolean", short: "h", default: false },
        version: { type: "boolean", short: "v", default: false },
        daemon: { type: "boolean", default: false },
        "max-duration": { type: "string", default: "60s" },
      },
      strict: true,
      allowPositionals: true,
    })

    // Check for help/version first
    if (values.help) {
      return Result.ok({
        mode: "oneshot",
        duration: Duration.fromSeconds(10),
        domainId: "general",
        clipboard: false,
        keystroke: false,
        notify: false,
        help: true,
        version: false,
      })
    }

    if (values.version) {
      return Result.ok({
        mode: "oneshot",
        duration: Duration.fromSeconds(10),
        domainId: "general",
        clipboard: false,
        keystroke: false,
        notify: false,
        help: false,
        version: true,
      })
    }

    // Validate domain (common to both modes)
    const domainValue = values.domain as string
    if (!DomainPreset.isValidId(domainValue)) {
      const validDomains = DomainPreset.getAllIds().join(", ")
      return Result.err(
        new CliParseError(
          `Invalid domain "${domainValue}". Valid options: ${validDomains}`,
        ),
      )
    }

    // Check for daemon mode
    if (values.daemon) {
      // Validate mutual exclusivity: daemon + custom duration is an error
      const durationProvided = argv.some(
        (arg) => arg === "-d" || arg.startsWith("--duration"),
      )
      if (durationProvided) {
        return Result.err(
          new CliParseError(
            "--daemon and --duration are mutually exclusive. Use --max-duration with daemon mode.",
          ),
        )
      }

      // Parse max duration
      const maxDurationResult = Duration.parse(values["max-duration"] as string)
      if (!maxDurationResult.ok) {
        return Result.err(new CliParseError(maxDurationResult.error.message))
      }

      return Result.ok({
        mode: "daemon",
        maxDuration: maxDurationResult.value,
        domainId: domainValue,
        clipboard: values.clipboard as boolean,
        keystroke: values.keystroke as boolean,
        notify: values.notify as boolean,
      })
    }

    // One-shot mode: Parse duration
    const durationResult = Duration.parse(values.duration as string)
    if (!durationResult.ok) {
      return Result.err(new CliParseError(durationResult.error.message))
    }

    return Result.ok({
      mode: "oneshot",
      duration: durationResult.value,
      domainId: domainValue,
      clipboard: values.clipboard as boolean,
      keystroke: values.keystroke as boolean,
      notify: values.notify as boolean,
      help: false,
      version: false,
    })
  } catch (error) {
    const message =
      error instanceof Error ? error.message : "Unknown argument parsing error"
    return Result.err(new CliParseError(message))
  }
}
