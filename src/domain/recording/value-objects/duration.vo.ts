import { DomainError, Result } from "../../shared/result"

export class DurationParseError extends DomainError {
  readonly code = "DURATION_PARSE_ERROR"

  constructor(input: string) {
    super(
      `Invalid duration format: "${input}". ` +
        `Expected format: <number>s, <number>m, or <number>m<number>s (e.g., 30s, 1m, 2m30s)`,
    )
  }
}

/**
 * Value object representing a time duration.
 * Immutable and validated on creation.
 */
export class Duration {
  private constructor(private readonly milliseconds: number) {}

  /**
   * Parse a duration string into a Duration value object.
   * Supported formats: "30s", "1m", "2m30s", "90s"
   */
  static parse(input: string): Result<Duration, DurationParseError> {
    const trimmed = input.trim().toLowerCase()

    // Match patterns like "30s", "1m", "2m30s"
    const match = trimmed.match(/^(?:(\d+)m)?(?:(\d+)s)?$/)

    if (!match || (!match[1] && !match[2])) {
      return Result.err(new DurationParseError(input))
    }

    const minutes = match[1] ? parseInt(match[1], 10) : 0
    const seconds = match[2] ? parseInt(match[2], 10) : 0

    const totalMilliseconds = (minutes * 60 + seconds) * 1000

    if (totalMilliseconds <= 0) {
      return Result.err(new DurationParseError(input))
    }

    return Result.ok(new Duration(totalMilliseconds))
  }

  /**
   * Create a Duration from seconds
   */
  static fromSeconds(seconds: number): Duration {
    return new Duration(seconds * 1000)
  }

  /**
   * Get duration in seconds
   */
  toSeconds(): number {
    return this.milliseconds / 1000
  }

  /**
   * Get duration in milliseconds
   */
  toMilliseconds(): number {
    return this.milliseconds
  }

  /**
   * Get human-readable string representation
   */
  toString(): string {
    const totalSeconds = this.toSeconds()
    const minutes = Math.floor(totalSeconds / 60)
    const seconds = totalSeconds % 60

    if (minutes === 0) {
      return `${seconds}s`
    }
    if (seconds === 0) {
      return `${minutes}m`
    }
    return `${minutes}m${seconds}s`
  }
}
