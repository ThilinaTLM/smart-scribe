import { tmpdir } from "node:os"
import { join } from "node:path"
import {
  type AudioRecorderPort,
  RecordingError,
  type RecordingProgressCallback,
} from "../../application/ports/audio-recorder.port"
import type {
  UnboundedProgressCallback,
  UnboundedRecorderPort,
} from "../../application/ports/unbounded-recorder.port"
import type { Duration } from "../../domain/recording/value-objects/duration.vo"
import { Result } from "../../domain/shared/result"
import { AudioData } from "../../domain/transcription/value-objects/audio-data.vo"

/**
 * FFmpeg-based audio recorder adapter for Pipewire/PulseAudio on Linux.
 * Implements both AudioRecorderPort (bounded) and UnboundedRecorderPort (daemon) interfaces.
 */
export class FFmpegRecorderAdapter
  implements AudioRecorderPort, UnboundedRecorderPort
{
  private currentProcess: ReturnType<typeof Bun.spawn> | null = null
  private outputPath: string = ""
  private shouldStop = false
  private progressInterval: Timer | null = null
  private maxDurationTimeout: Timer | null = null

  /**
   * Record audio from the microphone for the specified duration
   */
  async record(
    duration: Duration,
    onProgress?: RecordingProgressCallback,
  ): Promise<Result<AudioData, RecordingError>> {
    this.shouldStop = false
    const durationSeconds = duration.toSeconds()
    this.outputPath = join(tmpdir(), `smartscribe-${Date.now()}.ogg`)

    try {
      // FFmpeg command for Pipewire/PulseAudio recording
      // -f pulse: Use PulseAudio/Pipewire input
      // -i default: Default input device (microphone)
      // -t: Recording duration
      // -ar 16000: 16kHz sample rate (good for speech)
      // -ac 1: Mono audio
      // -c:a libopus: Opus codec (optimized for speech, efficient at low bitrates)
      // -b:a 16k: 16kbps bitrate (matches Gemini's internal 16kbps resolution)
      // -application voip: Optimize Opus for voice
      // -y: Overwrite output file
      const args = [
        "-f",
        "pulse",
        "-i",
        "default",
        "-t",
        durationSeconds.toString(),
        "-ar",
        "16000",
        "-ac",
        "1",
        "-c:a",
        "libopus",
        "-b:a",
        "16k",
        "-application",
        "voip",
        "-y",
        this.outputPath,
      ]

      this.currentProcess = Bun.spawn(["ffmpeg", ...args], {
        stdout: "pipe",
        stderr: "pipe",
      })

      // Start progress tracking if callback provided
      let progressInterval: Timer | null = null
      if (onProgress) {
        const startTime = Date.now()
        progressInterval = setInterval(() => {
          const elapsed = (Date.now() - startTime) / 1000
          onProgress(Math.min(elapsed, durationSeconds), durationSeconds)
        }, 100)
      }

      // Wait for process to complete
      const exitCode = await this.currentProcess.exited

      // Clean up progress interval
      if (progressInterval) {
        clearInterval(progressInterval)
      }

      // Final progress update
      if (onProgress) {
        onProgress(durationSeconds, durationSeconds)
      }

      if (exitCode !== 0 && !this.shouldStop) {
        const stderr = await new Response(this.currentProcess.stderr).text()
        return Result.err(
          new RecordingError(`FFmpeg recording failed: ${stderr}`),
        )
      }

      // Read the recorded file
      return await this.readRecordedFile()
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : "Unknown error during recording"
      return Result.err(new RecordingError(message))
    } finally {
      this.currentProcess = null
    }
  }

  /**
   * Stop any ongoing recording early
   */
  async stop(): Promise<Result<AudioData | null, RecordingError>> {
    if (!this.currentProcess) {
      return Result.ok(null)
    }

    this.shouldStop = true

    try {
      // Send SIGINT to gracefully stop FFmpeg (it will finalize the file)
      this.currentProcess.kill("SIGINT")

      // Wait for process to finish
      await this.currentProcess.exited

      // Read whatever was recorded
      return await this.readRecordedFile()
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Error stopping recording"
      return Result.err(new RecordingError(message))
    }
  }

  /**
   * Read the recorded audio file and convert to AudioData
   */
  private async readRecordedFile(): Promise<Result<AudioData, RecordingError>> {
    try {
      const file = Bun.file(this.outputPath)
      const exists = await file.exists()

      if (!exists) {
        return Result.err(
          new RecordingError(
            "Recording file not found. FFmpeg may have failed.",
          ),
        )
      }

      const arrayBuffer = await file.arrayBuffer()
      const buffer = Buffer.from(arrayBuffer)

      // Clean up temp file
      await Bun.write(this.outputPath, "") // Empty the file
      // Note: In production, you might want to actually delete the file

      return Result.ok(AudioData.fromBuffer(buffer, "audio/ogg"))
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Error reading recorded file"
      return Result.err(new RecordingError(message))
    }
  }

  // ============================================
  // UnboundedRecorderPort implementation
  // ============================================

  /**
   * Start recording without a fixed duration (daemon mode).
   * Recording continues until stopAndFinalize() or cancel() is called,
   * or maxDuration is reached.
   */
  startRecording(
    maxDuration: Duration,
    onProgress?: UnboundedProgressCallback,
    onMaxDurationReached?: () => void,
  ): Result<void, RecordingError> {
    if (this.currentProcess) {
      return Result.err(new RecordingError("Recording already in progress"))
    }

    this.shouldStop = false
    this.outputPath = join(tmpdir(), `smartscribe-${Date.now()}.ogg`)

    try {
      // FFmpeg command without -t flag (records indefinitely until stopped)
      // Uses Opus codec at 16kbps for optimal Gemini compatibility
      const args = [
        "-f",
        "pulse",
        "-i",
        "default",
        "-ar",
        "16000",
        "-ac",
        "1",
        "-c:a",
        "libopus",
        "-b:a",
        "16k",
        "-application",
        "voip",
        "-y",
        this.outputPath,
      ]

      this.currentProcess = Bun.spawn(["ffmpeg", ...args], {
        stdout: "pipe",
        stderr: "pipe",
      })

      // Start progress tracking
      const startTime = Date.now()
      if (onProgress) {
        this.progressInterval = setInterval(() => {
          const elapsed = (Date.now() - startTime) / 1000
          onProgress(elapsed)
        }, 100)
      }

      // Set max duration timeout
      const maxDurationMs = maxDuration.toSeconds() * 1000
      this.maxDurationTimeout = setTimeout(() => {
        if (this.currentProcess && onMaxDurationReached) {
          onMaxDurationReached()
        }
      }, maxDurationMs)

      return Result.ok(undefined)
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : "Unknown error starting recording"
      return Result.err(new RecordingError(message))
    }
  }

  /**
   * Stop recording gracefully and return the recorded audio.
   */
  async stopAndFinalize(): Promise<Result<AudioData, RecordingError>> {
    this.clearTimers()

    if (!this.currentProcess) {
      return Result.err(new RecordingError("No recording in progress"))
    }

    this.shouldStop = true

    try {
      // Send SIGINT to gracefully stop FFmpeg (it will finalize the file)
      this.currentProcess.kill("SIGINT")

      // Wait for process to finish
      await this.currentProcess.exited

      // Read the recorded audio
      const result = await this.readRecordedFile()

      this.currentProcess = null
      return result
    } catch (error) {
      this.currentProcess = null
      const message =
        error instanceof Error ? error.message : "Error stopping recording"
      return Result.err(new RecordingError(message))
    }
  }

  /**
   * Cancel recording and discard output (no transcription).
   */
  async cancel(): Promise<Result<void, RecordingError>> {
    this.clearTimers()

    if (!this.currentProcess) {
      return Result.ok(undefined)
    }

    this.shouldStop = true

    try {
      // Send SIGKILL for immediate termination
      this.currentProcess.kill("SIGKILL")

      // Wait for process to finish
      await this.currentProcess.exited

      // Clean up temp file without reading
      try {
        const file = Bun.file(this.outputPath)
        if (await file.exists()) {
          await Bun.write(this.outputPath, "")
        }
      } catch {
        // Ignore cleanup errors
      }

      this.currentProcess = null
      return Result.ok(undefined)
    } catch (error) {
      this.currentProcess = null
      const message =
        error instanceof Error ? error.message : "Error cancelling recording"
      return Result.err(new RecordingError(message))
    }
  }

  /**
   * Check if recording is currently in progress
   */
  isRecording(): boolean {
    return this.currentProcess !== null
  }

  /**
   * Clear progress and timeout timers
   */
  private clearTimers(): void {
    if (this.progressInterval) {
      clearInterval(this.progressInterval)
      this.progressInterval = null
    }
    if (this.maxDurationTimeout) {
      clearTimeout(this.maxDurationTimeout)
      this.maxDurationTimeout = null
    }
  }
}
