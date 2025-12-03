import type { Duration } from "../../domain/recording/value-objects/duration.vo"
import type { Result } from "../../domain/shared/result"
import type { AudioData } from "../../domain/transcription/value-objects/audio-data.vo"
import type { RecordingError } from "./audio-recorder.port"

/**
 * Progress callback for unbounded recording
 */
export type UnboundedProgressCallback = (elapsedSeconds: number) => void

/**
 * Port interface for unbounded (daemon-style) audio recording.
 * Unlike AudioRecorderPort, this supports starting recording without a fixed duration,
 * and provides separate stop/cancel operations.
 */
export interface UnboundedRecorderPort {
  /**
   * Start recording audio from the microphone.
   * Recording continues until stopAndFinalize() or cancel() is called,
   * or maxDuration is reached.
   *
   * @param maxDuration Maximum recording duration (safety limit)
   * @param onProgress Optional callback for elapsed time updates
   * @param onMaxDurationReached Optional callback when max duration is hit
   * @returns Success or error if recording couldn't start
   */
  startRecording(
    maxDuration: Duration,
    onProgress?: UnboundedProgressCallback,
    onMaxDurationReached?: () => void,
  ): Result<void, RecordingError>

  /**
   * Stop recording gracefully and return the recorded audio.
   * Sends SIGINT to FFmpeg which finalizes the audio file.
   *
   * @returns The recorded audio data or an error
   */
  stopAndFinalize(): Promise<Result<AudioData, RecordingError>>

  /**
   * Cancel recording and discard the output.
   * Used when user wants to abort without transcription.
   *
   * @returns Success or error
   */
  cancel(): Promise<Result<void, RecordingError>>

  /**
   * Check if recording is currently in progress
   */
  isRecording(): boolean
}
