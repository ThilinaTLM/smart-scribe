/**
 * Result type for explicit error handling without exceptions.
 * Inspired by Rust's Result<T, E> type.
 */
export type Result<T, E> = { ok: true; value: T } | { ok: false; error: E }

export const Result = {
  ok: <T>(value: T): Result<T, never> => ({ ok: true, value }),
  err: <E>(error: E): Result<never, E> => ({ ok: false, error }),

  isOk: <T, E>(result: Result<T, E>): result is { ok: true; value: T } =>
    result.ok,

  isErr: <T, E>(result: Result<T, E>): result is { ok: false; error: E } =>
    !result.ok,

  map: <T, U, E>(result: Result<T, E>, fn: (value: T) => U): Result<U, E> =>
    result.ok ? Result.ok(fn(result.value)) : result,

  mapErr: <T, E, F>(result: Result<T, E>, fn: (error: E) => F): Result<T, F> =>
    result.ok ? result : Result.err(fn(result.error)),

  flatMap: <T, U, E>(
    result: Result<T, E>,
    fn: (value: T) => Result<U, E>,
  ): Result<U, E> => (result.ok ? fn(result.value) : result),

  unwrap: <T, E>(result: Result<T, E>): T => {
    if (result.ok) return result.value
    throw new Error(`Unwrap called on Err: ${result.error}`)
  },

  unwrapOr: <T, E>(result: Result<T, E>, defaultValue: T): T =>
    result.ok ? result.value : defaultValue,

  unwrapOrElse: <T, E>(result: Result<T, E>, fn: (error: E) => T): T =>
    result.ok ? result.value : fn(result.error),
}

/**
 * Base class for domain errors
 */
export abstract class DomainError extends Error {
  abstract readonly code: string

  constructor(message: string) {
    super(message)
    this.name = this.constructor.name
  }
}
