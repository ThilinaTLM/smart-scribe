import type { Duration } from "../../domain/recording/value-objects/duration.vo"
import type { Result } from "../../domain/shared/result"
import type { AudioData } from "../../domain/transcription/value-objects/audio-data.vo"

/**
 * Error that occurs during audio recording
 */
export class RecordingError extends Error {
  readonly code = "RECORDING_ERROR"

  constructor(
    message: string,
    public readonly cause?: Error,
  ) {
    super(message)
    this.name = "RecordingError"
  }
}

/**
 * Progress callback for recording status
 */
export type RecordingProgressCallback = (
  elapsedSeconds: number,
  totalSeconds: number,
) => void

/**
 * Port interface for audio recording.
 * Infrastructure layer implements this interface.
 */
export interface AudioRecorderPort {
  /**
   * Record audio from the microphone for the specified duration
   * @param duration The duration to record
   * @param onProgress Optional callback for progress updates
   * @returns The recorded audio data or an error
   */
  record(
    duration: Duration,
    onProgress?: RecordingProgressCallback,
  ): Promise<Result<AudioData, RecordingError>>

  /**
   * Stop any ongoing recording early
   * @returns The audio recorded so far
   */
  stop(): Promise<Result<AudioData | null, RecordingError>>
}
