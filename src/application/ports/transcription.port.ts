import type { Result } from "../../domain/shared/result"
import type { AudioData } from "../../domain/transcription/value-objects/audio-data.vo"
import type { SystemPrompt } from "../../domain/transcription/value-objects/system-prompt.vo"

/**
 * Error that occurs during transcription
 */
export class TranscriptionError extends Error {
  readonly code = "TRANSCRIPTION_ERROR"

  constructor(
    message: string,
    public readonly cause?: Error,
  ) {
    super(message)
    this.name = "TranscriptionError"
  }
}

/**
 * Port interface for audio transcription.
 * Infrastructure layer implements this interface.
 */
export interface TranscriptionPort {
  /**
   * Transcribe audio data to text using the provided system prompt
   * @param audioData The audio to transcribe
   * @param systemPrompt The system prompt with domain-specific instructions
   * @returns The transcribed text or an error
   */
  transcribe(
    audioData: AudioData,
    systemPrompt: SystemPrompt,
  ): Promise<Result<string, TranscriptionError>>
}
