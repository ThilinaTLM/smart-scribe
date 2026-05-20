# CLAUDE.md

## Project Overview

SmartScribe is a cross-platform Rust CLI tool for AI-powered audio transcription. It records from the microphone and outputs cleaned text via stdout. Two transcription paths are supported:

- **ChatGPT OAuth** (`auth = "oauth"`, default) — Bearer token issued by the public OpenAI Codex CLI OAuth client, used against `chatgpt.com/backend-api/transcribe`. Counts against the user's ChatGPT subscription.
- **OpenAI API key** (`auth = "api_key"`) — standard `Authorization: Bearer sk-...` against `api.openai.com/v1/audio/transcriptions`. Metered API usage.

Run modes: **one-shot** (fixed duration) and **daemon** (background service with socket/pipe control).

## Development Commands

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo test               # Run all tests
cargo clippy --all-targets -- -D warnings
cargo fmt
cargo run -- -h          # Show help
```

## Environment Setup

**OAuth (recommended):**

```bash
smart-scribe login                  # opens browser, persists token
# or, if codex CLI is already installed:
smart-scribe login --from-codex     # imports + rotates refresh token
smart-scribe auth status            # check state
```

**API key:**

```bash
smart-scribe config set auth api_key
export OPENAI_API_KEY=sk-...        # or: smart-scribe config set openai_api_key sk-...
```

## Configuration

Config file locations (`<config_dir>` resolved via `dirs::config_dir()`):

- Linux: `~/.config/smart-scribe/config.toml`
- macOS: `~/Library/Application Support/smart-scribe/config.toml`
- Windows: `%APPDATA%\smart-scribe\config.toml`

OAuth tokens live next to the config in `oauth.json` (mode 0600 on Unix). Never edit by hand; use `smart-scribe login` / `logout`.

Config priority: CLI args > environment > `config.toml` > defaults.

### Recognised keys

| Key                         | Notes                                         |
| --------------------------- | --------------------------------------------- |
| `auth`                      | `oauth` (default) or `api_key`                |
| `openai_api_key`            | Used when `auth = "api_key"`                  |
| `openai_transcribe_model`   | Default `gpt-4o-transcribe`. Applies to **both** auth modes: the OAuth `/backend-api/transcribe` endpoint accepts `model` as a multipart field (verified with `whisper-1`, `gpt-4o-transcribe`, `gpt-4o-mini-transcribe`; unknown values silently fall back to the server default). |
| `transcribe_prompt`         | Optional `prompt` form field. Per OpenAI docs, the single biggest accuracy lever (corrects acronyms, brand names). Sent on both paths.       |
| `transcribe_language`       | Optional ISO 639-1 hint (`en`, `es`, ...). Reduces hallucination on short audio. Sent on both paths.                                          |
| `duration`, `max_duration`  | e.g. `30s`, `1m`, `2m30s`                     |
| `clipboard`, `keystroke`, `notify`, `audio_cue` | booleans                  |
| `linux.*`, `windows.*`      | Platform-specific subtables (portable schema) |

Legacy keys (`api_key`, `backend`, `chatgpt_cookie_file`, `domain`) are no longer recognised. The config loader prints a one-time warning if it sees them in a TOML file so the user knows to clean up.
