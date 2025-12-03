/**
 * Supported audio MIME types
 */
export type AudioMimeType =
  | "audio/mp3"
  | "audio/mpeg"
  | "audio/wav"
  | "audio/webm"
  | "audio/ogg"
  | "audio/mp4"

/**
 * Value object representing audio data ready for transcription.
 * Contains base64-encoded audio and its MIME type.
 */
export class AudioData {
  private constructor(
    readonly base64: string,
    readonly mimeType: AudioMimeType,
  ) {}

  /**
   * Create AudioData from a Buffer
   */
  static fromBuffer(buffer: Buffer, mimeType: AudioMimeType): AudioData {
    const base64 = buffer.toString("base64")
    return new AudioData(base64, mimeType)
  }

  /**
   * Create AudioData from a base64 string
   */
  static fromBase64(base64: string, mimeType: AudioMimeType): AudioData {
    return new AudioData(base64, mimeType)
  }

  /**
   * Get the size of the audio data in bytes (approximate)
   */
  get sizeInBytes(): number {
    // Base64 encoding increases size by ~33%
    return Math.ceil((this.base64.length * 3) / 4)
  }

  /**
   * Get human-readable size
   */
  get humanReadableSize(): string {
    const bytes = this.sizeInBytes
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  }
}
