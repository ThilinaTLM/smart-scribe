import { Result } from "../../shared/result"
import { InvalidStateTransitionError } from "../errors/daemon.errors"
import {
  type DaemonState,
  DaemonStates,
} from "../value-objects/daemon-state.vo"

/**
 * Daemon session entity.
 * Manages state transitions for the daemon lifecycle.
 *
 * State machine:
 *   IDLE -> RECORDING (startRecording)
 *   RECORDING -> PROCESSING (stopRecording)
 *   RECORDING -> IDLE (cancelRecording)
 *   PROCESSING -> IDLE (completeProcessing)
 */
export class DaemonSession {
  private _state: DaemonState = DaemonStates.IDLE

  /**
   * Get the current state
   */
  get state(): DaemonState {
    return this._state
  }

  /**
   * Check if currently idle
   */
  get isIdle(): boolean {
    return this._state === DaemonStates.IDLE
  }

  /**
   * Check if currently recording
   */
  get isRecording(): boolean {
    return this._state === DaemonStates.RECORDING
  }

  /**
   * Check if currently processing
   */
  get isProcessing(): boolean {
    return this._state === DaemonStates.PROCESSING
  }

  /**
   * Transition from IDLE to RECORDING
   */
  startRecording(): Result<void, InvalidStateTransitionError> {
    if (this._state !== DaemonStates.IDLE) {
      return Result.err(
        new InvalidStateTransitionError(this._state, "start recording"),
      )
    }
    this._state = DaemonStates.RECORDING
    return Result.ok(undefined)
  }

  /**
   * Transition from RECORDING to PROCESSING
   */
  stopRecording(): Result<void, InvalidStateTransitionError> {
    if (this._state !== DaemonStates.RECORDING) {
      return Result.err(
        new InvalidStateTransitionError(this._state, "stop recording"),
      )
    }
    this._state = DaemonStates.PROCESSING
    return Result.ok(undefined)
  }

  /**
   * Transition from RECORDING to IDLE (cancel without transcription)
   */
  cancelRecording(): Result<void, InvalidStateTransitionError> {
    if (this._state !== DaemonStates.RECORDING) {
      return Result.err(
        new InvalidStateTransitionError(this._state, "cancel recording"),
      )
    }
    this._state = DaemonStates.IDLE
    return Result.ok(undefined)
  }

  /**
   * Transition from PROCESSING to IDLE
   */
  completeProcessing(): Result<void, InvalidStateTransitionError> {
    if (this._state !== DaemonStates.PROCESSING) {
      return Result.err(
        new InvalidStateTransitionError(this._state, "complete processing"),
      )
    }
    this._state = DaemonStates.IDLE
    return Result.ok(undefined)
  }
}
