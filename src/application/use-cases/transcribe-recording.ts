import type { Duration } from "../../domain/recording/value-objects/duration.vo"
import { Result } from "../../domain/shared/result"
import type { DomainId } from "../../domain/transcription/value-objects/domain-preset.vo"
import { DomainPreset } from "../../domain/transcription/value-objects/domain-preset.vo"
import { SystemPrompt } from "../../domain/transcription/value-objects/system-prompt.vo"
import type {
  AudioRecorderPort,
  RecordingProgressCallback,
} from "../ports/audio-recorder.port"
import type { ClipboardPort } from "../ports/clipboard.port"
import type { TranscriptionPort } from "../ports/transcription.port"

/**
 * Input for the transcribe recording use case
 */
export interface TranscribeRecordingInput {
  duration: Duration
  domainId: DomainId
  onRecordingProgress?: RecordingProgressCallback
  onRecordingStart?: () => void
  onRecordingComplete?: () => void
  onTranscriptionStart?: () => void
  onTranscriptionComplete?: (text: string) => void
  onClipboardCopy?: (success: boolean) => void
}

/**
 * Output from the transcribe recording use case
 */
export interface TranscribeRecordingOutput {
  text: string
  clipboardCopied: boolean
}

/**
 * Error from the use case
 */
export class TranscribeRecordingError extends Error {
  constructor(
    message: string,
    public readonly stage: "recording" | "transcription" | "clipboard",
  ) {
    super(message)
    this.name = "TranscribeRecordingError"
  }
}

/**
 * Main use case: Record audio and transcribe it to text.
 * Orchestrates the recording, transcription, and clipboard operations.
 */
export class TranscribeRecordingUseCase {
  constructor(
    private readonly recorder: AudioRecorderPort,
    private readonly transcriber: TranscriptionPort,
    private readonly clipboard: ClipboardPort,
  ) {}

  /**
   * Execute the use case
   */
  async execute(
    input: TranscribeRecordingInput,
  ): Promise<Result<TranscribeRecordingOutput, TranscribeRecordingError>> {
    // 1. Start recording
    input.onRecordingStart?.()

    const recordingResult = await this.recorder.record(
      input.duration,
      input.onRecordingProgress,
    )

    if (!recordingResult.ok) {
      return Result.err(
        new TranscribeRecordingError(
          recordingResult.error.message,
          "recording",
        ),
      )
    }

    input.onRecordingComplete?.()

    // 2. Build system prompt with domain context
    const domainPreset = DomainPreset.fromId(input.domainId)
    const systemPrompt = SystemPrompt.build(domainPreset)

    // 3. Transcribe the audio
    input.onTranscriptionStart?.()

    const transcriptionResult = await this.transcriber.transcribe(
      recordingResult.value,
      systemPrompt,
    )

    if (!transcriptionResult.ok) {
      return Result.err(
        new TranscribeRecordingError(
          transcriptionResult.error.message,
          "transcription",
        ),
      )
    }

    const text = transcriptionResult.value
    input.onTranscriptionComplete?.(text)

    // 4. Copy to clipboard
    const clipboardResult = await this.clipboard.copy(text)
    const clipboardCopied = clipboardResult.ok

    input.onClipboardCopy?.(clipboardCopied)

    // Return success even if clipboard fails (it's secondary)
    return Result.ok({
      text,
      clipboardCopied,
    })
  }

  /**
   * Stop an ongoing recording early
   */
  async stopEarly(): Promise<void> {
    await this.recorder.stop()
  }
}
