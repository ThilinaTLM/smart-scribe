import { Result } from "../domain/shared/result"
import { CliParseError } from "./parser"

/**
 * Config subcommand actions
 */
export type ConfigAction =
  | { action: "init" }
  | { action: "set"; key: string; value: string }
  | { action: "get"; key: string }
  | { action: "list" }
  | { action: "path" }

/**
 * Parsed CLI options for config mode
 */
export interface ConfigCliOptions {
  mode: "config"
  configAction: ConfigAction
}

/**
 * Parse config subcommand arguments.
 * Expected: config <action> [args...]
 *
 * @param argv Arguments after "config" (e.g., ["set", "api_key", "xyz"])
 */
export function parseConfigArgs(
  argv: string[],
): Result<ConfigCliOptions, CliParseError> {
  if (argv.length === 0) {
    return Result.err(
      new CliParseError(
        "Missing config action. Usage: smart-scribe config <init|set|get|list|path>",
      ),
    )
  }

  const action = argv[0]

  switch (action) {
    case "init":
      return Result.ok({ mode: "config", configAction: { action: "init" } })

    case "set":
      if (argv.length < 3) {
        return Result.err(
          new CliParseError("Usage: smart-scribe config set <key> <value>"),
        )
      }
      return Result.ok({
        mode: "config",
        configAction: { action: "set", key: argv[1], value: argv[2] },
      })

    case "get":
      if (argv.length < 2) {
        return Result.err(
          new CliParseError("Usage: smart-scribe config get <key>"),
        )
      }
      return Result.ok({
        mode: "config",
        configAction: { action: "get", key: argv[1] },
      })

    case "list":
      return Result.ok({ mode: "config", configAction: { action: "list" } })

    case "path":
      return Result.ok({ mode: "config", configAction: { action: "path" } })

    default:
      return Result.err(
        new CliParseError(
          `Unknown config action: ${action}. Valid actions: init, set, get, list, path`,
        ),
      )
  }
}
