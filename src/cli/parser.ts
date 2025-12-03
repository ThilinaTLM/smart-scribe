import { parseArgs } from 'node:util';
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
 * Parsed CLI options
 */
export interface CliOptions {
  duration: Duration
  domainId: DomainId
  clipboard: boolean
  keystroke: boolean
  help: boolean
  version: boolean
}

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

OPTIONS:
    -d, --duration <TIME>    Recording duration (default: 10s)
                             Formats: 30s, 1m, 2m30s
    -D, --domain <DOMAIN>    Domain preset (default: general)
                             Options: ${domains}
    -c, --clipboard          Copy transcription to clipboard
    -k, --keystroke          Type transcription into focused window
    -h, --help               Show this help message
    -v, --version            Show version

EXAMPLES:
    smart-scribe                     # 10s transcription to stdout only
    smart-scribe -c                  # Copy result to clipboard
    smart-scribe -k                  # Type result into focused window
    smart-scribe -c -k               # Both clipboard and keystroke
    smart-scribe -d 60s -c           # 60 second recording + clipboard
    smart-scribe -d 1m -D dev -k     # 1 minute, dev domain, keystroke

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
): Result<CliOptions, CliParseError> {
  try {
    const { values } = parseArgs({
      args: argv,
      options: {
        duration: { type: "string", short: "d", default: "10s" },
        domain: { type: "string", short: "D", default: "general" },
        clipboard: { type: "boolean", short: "c", default: false },
        keystroke: { type: "boolean", short: "k", default: false },
        help: { type: "boolean", short: "h", default: false },
        version: { type: "boolean", short: "v", default: false },
      },
      strict: true,
      allowPositionals: true,
    })

    // Check for help/version first
    if (values.help) {
      return Result.ok({
        duration: Duration.fromSeconds(10),
        domainId: "general",
        clipboard: false,
        keystroke: false,
        help: true,
        version: false,
      })
    }

    if (values.version) {
      return Result.ok({
        duration: Duration.fromSeconds(10),
        domainId: "general",
        clipboard: false,
        keystroke: false,
        help: false,
        version: true,
      })
    }

    // Parse duration
    const durationResult = Duration.parse(values.duration as string)
    if (!durationResult.ok) {
      return Result.err(new CliParseError(durationResult.error.message))
    }

    // Validate domain
    const domainValue = values.domain as string
    if (!DomainPreset.isValidId(domainValue)) {
      const validDomains = DomainPreset.getAllIds().join(", ")
      return Result.err(
        new CliParseError(
          `Invalid domain "${domainValue}". Valid options: ${validDomains}`,
        ),
      )
    }

    return Result.ok({
      duration: durationResult.value,
      domainId: domainValue,
      clipboard: values.clipboard as boolean,
      keystroke: values.keystroke as boolean,
      help: false,
      version: false,
    })
  } catch (error) {
    const message =
      error instanceof Error ? error.message : "Unknown argument parsing error"
    return Result.err(new CliParseError(message))
  }
}
