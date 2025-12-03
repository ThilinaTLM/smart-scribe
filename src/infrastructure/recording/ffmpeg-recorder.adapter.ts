import { tmpdir } from "node:os"
import { join } from "node:path"
import {
  type AudioRecorderPort,
  RecordingError,
  type RecordingProgressCallback,
} from "../../application/ports/audio-recorder.port"
import type { Duration } from "../../domain/recording/value-objects/duration.vo"
import { Result } from "../../domain/shared/result"
import { AudioData } from "../../domain/transcription/value-objects/audio-data.vo"

/**
 * FFmpeg-based audio recorder adapter for Pipewire/PulseAudio on Linux.
 * Implements the AudioRecorderPort interface.
 */
export class FFmpegRecorderAdapter implements AudioRecorderPort {
  private currentProcess: ReturnType<typeof Bun.spawn> | null = null
  private outputPath: string = ""
  private shouldStop = false

  /**
   * Record audio from the microphone for the specified duration
   */
  async record(
    duration: Duration,
    onProgress?: RecordingProgressCallback,
  ): Promise<Result<AudioData, RecordingError>> {
    this.shouldStop = false
    const durationSeconds = duration.toSeconds()
    this.outputPath = join(tmpdir(), `smartscribe-${Date.now()}.mp3`)

    try {
      // FFmpeg command for Pipewire/PulseAudio recording
      // -f pulse: Use PulseAudio/Pipewire input
      // -i default: Default input device (microphone)
      // -t: Recording duration
      // -ar 16000: 16kHz sample rate (good for speech)
      // -ac 1: Mono audio
      // -c:a libmp3lame: MP3 encoding
      // -q:a 2: Good quality
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
        "libmp3lame",
        "-q:a",
        "2",
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

      return Result.ok(AudioData.fromBuffer(buffer, "audio/mp3"))
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Error reading recorded file"
      return Result.err(new RecordingError(message))
    }
  }
}
