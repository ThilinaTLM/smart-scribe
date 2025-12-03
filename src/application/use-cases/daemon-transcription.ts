import { DaemonSession } from "../../domain/daemon/entities/daemon-session.entity"
import type { InvalidStateTransitionError } from "../../domain/daemon/errors/daemon.errors"
import type { DaemonState } from "../../domain/daemon/value-objects/daemon-state.vo"
import type { Duration } from "../../domain/recording/value-objects/duration.vo"
import { Result } from "../../domain/shared/result"
import type { DomainId } from "../../domain/transcription/value-objects/domain-preset.vo"
import { DomainPreset } from "../../domain/transcription/value-objects/domain-preset.vo"
import { SystemPrompt } from "../../domain/transcription/value-objects/system-prompt.vo"
import type { ClipboardPort } from "../ports/clipboard.port"
import type { KeystrokePort } from "../ports/keystroke.port"
import type { TranscriptionPort } from "../ports/transcription.port"
import type { UnboundedRecorderPort } from "../ports/unbounded-recorder.port"

/**
 * Configuration for the daemon transcription use case
 */
export interface DaemonConfig {
  domainId: DomainId
  maxDuration: Duration
  enableClipboard: boolean
  enableKeystroke: boolean
}

/**
 * Event callbacks for daemon lifecycle events
 */
export interface DaemonEventCallbacks {
  onRecordingStart?: () => void
  onRecordingProgress?: (elapsedSeconds: number) => void
  onMaxDurationReached?: () => void
  onRecordingStop?: () => void
  onRecordingCancel?: () => void
  onTranscriptionStart?: () => void
  onTranscriptionComplete?: (text: string) => void
  onClipboardCopy?: (success: boolean) => void
  onKeystrokeSend?: (success: boolean) => void
  onError?: (error: Error) => void
  onStateChange?: (state: DaemonState) => void
}

/**
 * Error from the daemon use case
 */
export class DaemonTranscriptionError extends Error {
  constructor(
    message: string,
    public readonly stage:
      | "recording"
      | "transcription"
      | "clipboard"
      | "keystroke"
      | "state",
  ) {
    super(message)
    this.name = "DaemonTranscriptionError"
  }
}

/**
 * Daemon transcription use case.
 * Manages the daemon lifecycle: waiting for signals, recording, transcribing.
 * Owns the DaemonSession entity for state management.
 */
export class DaemonTranscriptionUseCase {
  private readonly session: DaemonSession
  private callbacks: DaemonEventCallbacks = {}

  constructor(
    private readonly recorder: UnboundedRecorderPort,
    private readonly transcriber: TranscriptionPort,
    private readonly config: DaemonConfig,
    private readonly clipboard?: ClipboardPort,
    private readonly keystroke?: KeystrokePort,
  ) {
    this.session = new DaemonSession()
  }

  /**
   * Set event callbacks
   */
  setCallbacks(callbacks: DaemonEventCallbacks): void {
    this.callbacks = callbacks
  }

  /**
   * Get current state
   */
  get state(): DaemonState {
    return this.session.state
  }

  /**
   * Check if currently idle
   */
  get isIdle(): boolean {
    return this.session.isIdle
  }

  /**
   * Check if currently recording
   */
  get isRecording(): boolean {
    return this.session.isRecording
  }

  /**
   * Check if currently processing
   */
  get isProcessing(): boolean {
    return this.session.isProcessing
  }

  /**
   * Start recording (SIGUSR1 handler)
   */
  startRecording(): Result<
    void,
    InvalidStateTransitionError | DaemonTranscriptionError
  > {
    const transitionResult = this.session.startRecording()
    if (!transitionResult.ok) {
      return transitionResult
    }

    this.callbacks.onStateChange?.(this.session.state)

    const recordResult = this.recorder.startRecording(
      this.config.maxDuration,
      (elapsed) => this.callbacks.onRecordingProgress?.(elapsed),
      () => this.handleMaxDurationReached(),
    )

    if (!recordResult.ok) {
      // Rollback state on error
      this.session.cancelRecording()
      this.callbacks.onStateChange?.(this.session.state)
      return Result.err(
        new DaemonTranscriptionError(recordResult.error.message, "recording"),
      )
    }

    this.callbacks.onRecordingStart?.()
    return Result.ok(undefined)
  }

  /**
   * Stop recording and transcribe (SIGUSR2 handler)
   */
  async stopAndTranscribe(): Promise<
    Result<string, InvalidStateTransitionError | DaemonTranscriptionError>
  > {
    const transitionResult = this.session.stopRecording()
    if (!transitionResult.ok) {
      return transitionResult
    }

    this.callbacks.onStateChange?.(this.session.state)
    this.callbacks.onRecordingStop?.()

    // Stop recording and get audio
    const audioResult = await this.recorder.stopAndFinalize()
    if (!audioResult.ok) {
      this.session.completeProcessing()
      this.callbacks.onStateChange?.(this.session.state)
      return Result.err(
        new DaemonTranscriptionError(audioResult.error.message, "recording"),
      )
    }

    // Transcribe
    this.callbacks.onTranscriptionStart?.()
    const domainPreset = DomainPreset.fromId(this.config.domainId)
    const systemPrompt = SystemPrompt.build(domainPreset)

    const transcriptionResult = await this.transcriber.transcribe(
      audioResult.value,
      systemPrompt,
    )

    if (!transcriptionResult.ok) {
      this.session.completeProcessing()
      this.callbacks.onStateChange?.(this.session.state)
      return Result.err(
        new DaemonTranscriptionError(
          transcriptionResult.error.message,
          "transcription",
        ),
      )
    }

    const text = transcriptionResult.value
    this.callbacks.onTranscriptionComplete?.(text)

    // Copy to clipboard if enabled
    if (this.config.enableClipboard && this.clipboard) {
      const clipboardResult = await this.clipboard.copy(text)
      this.callbacks.onClipboardCopy?.(clipboardResult.ok)
    }

    // Type into focused window if enabled
    if (this.config.enableKeystroke && this.keystroke) {
      const keystrokeResult = await this.keystroke.type(text)
      this.callbacks.onKeystrokeSend?.(keystrokeResult.ok)
    }

    // Complete processing
    this.session.completeProcessing()
    this.callbacks.onStateChange?.(this.session.state)

    return Result.ok(text)
  }

  /**
   * Cancel recording without transcription (SIGINT during recording)
   */
  async cancel(): Promise<
    Result<void, InvalidStateTransitionError | DaemonTranscriptionError>
  > {
    if (!this.session.isRecording) {
      return Result.err(
        new DaemonTranscriptionError(
          "Cannot cancel: not currently recording",
          "state",
        ),
      )
    }

    const cancelResult = await this.recorder.cancel()
    if (!cancelResult.ok) {
      return Result.err(
        new DaemonTranscriptionError(cancelResult.error.message, "recording"),
      )
    }

    const transitionResult = this.session.cancelRecording()
    if (!transitionResult.ok) {
      return transitionResult
    }

    this.callbacks.onStateChange?.(this.session.state)
    this.callbacks.onRecordingCancel?.()

    return Result.ok(undefined)
  }

  /**
   * Handle max duration reached - auto-trigger transcription
   */
  private handleMaxDurationReached(): void {
    this.callbacks.onMaxDurationReached?.()
    // Trigger stop and transcribe asynchronously
    this.stopAndTranscribe().catch((error) => {
      this.callbacks.onError?.(error)
    })
  }
}
