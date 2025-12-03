/**
 * Daemon state value object.
 * Represents the current state of the daemon session.
 */
export type DaemonState = "idle" | "recording" | "processing"

export const DaemonStates = {
  IDLE: "idle" as const,
  RECORDING: "recording" as const,
  PROCESSING: "processing" as const,
}
