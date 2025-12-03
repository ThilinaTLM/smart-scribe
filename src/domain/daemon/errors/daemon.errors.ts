import type { DaemonState } from "../value-objects/daemon-state.vo"

/**
 * Error thrown when an invalid state transition is attempted
 */
export class InvalidStateTransitionError extends Error {
  readonly code = "INVALID_STATE_TRANSITION"

  constructor(
    public readonly currentState: DaemonState,
    public readonly attemptedAction: string,
  ) {
    super(`Cannot ${attemptedAction} while in ${currentState} state`)
    this.name = "InvalidStateTransitionError"
  }
}

/**
 * Error thrown when PID file operations fail
 */
export class PidFileError extends Error {
  readonly code = "PID_FILE_ERROR"

  constructor(
    message: string,
    public readonly existingPid?: number,
  ) {
    super(message)
    this.name = "PidFileError"
  }
}
