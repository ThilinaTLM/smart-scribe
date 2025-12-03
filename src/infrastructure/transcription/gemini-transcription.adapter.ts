import { GoogleGenAI } from "@google/genai"
import {
  TranscriptionError,
  type TranscriptionPort,
} from "../../application/ports/transcription.port"
import { Result } from "../../domain/shared/result"
import type { AudioData } from "../../domain/transcription/value-objects/audio-data.vo"
import type { SystemPrompt } from "../../domain/transcription/value-objects/system-prompt.vo"

/**
 * Gemini API-based transcription adapter.
 * Implements the TranscriptionPort interface.
 */
export class GeminiTranscriptionAdapter implements TranscriptionPort {
  private readonly ai: GoogleGenAI
  private readonly model = "gemini-2.0-flash-lite"

  constructor(apiKey: string) {
    this.ai = new GoogleGenAI({ apiKey })
  }

  /**
   * Transcribe audio data to text using Gemini
   */
  async transcribe(
    audioData: AudioData,
    systemPrompt: SystemPrompt,
  ): Promise<Result<string, TranscriptionError>> {
    try {
      const contents = [
        {
          role: "user" as const,
          parts: [
            {
              inlineData: {
                mimeType: audioData.mimeType,
                data: audioData.base64,
              },
            },
          ],
        },
      ]

      const config = {
        thinkingConfig: {
          thinkingBudget: 0, // Disable thinking for faster response
        },
        systemInstruction: systemPrompt.content,
      }

      const response = await this.ai.models.generateContent({
        model: this.model,
        config,
        contents,
      })

      const text = response.text

      if (!text) {
        return Result.err(
          new TranscriptionError("Gemini returned empty response"),
        )
      }

      return Result.ok(text.trim())
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : "Unknown error during transcription"
      return Result.err(
        new TranscriptionError(`Transcription failed: ${message}`),
      )
    }
  }
}
