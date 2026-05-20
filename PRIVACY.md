# Privacy Policy

## Overview

SmartScribe is free and open-source software licensed under the MIT License. This document explains how SmartScribe handles your data.

## Data Collection

**SmartScribe itself does not collect any data.** The application:

- Has no backend servers
- Contains no analytics or telemetry
- Does not transmit data to the SmartScribe project or its contributors
- Stores configuration locally on your machine only

## Third-Party Services

SmartScribe transcribes audio by sending it to OpenAI. Two auth paths are available; both target OpenAI services.

### ChatGPT subscription (OAuth, default)

When `auth = "oauth"`:

- Audio is sent from your machine to `chatgpt.com/backend-api/transcribe`
- Authentication uses an OAuth Bearer token obtained from `auth.openai.com` via the public Codex CLI OAuth client (the same client used by OpenAI's official Codex CLI)
- The OAuth token is cached locally; SmartScribe refreshes it automatically and never transmits it anywhere other than to OpenAI

### OpenAI API key

When `auth = "api_key"`:

- Audio is sent from your machine to `api.openai.com/v1/audio/transcriptions`
- You provide and manage your own OpenAI API key

### Your Responsibilities

Before using SmartScribe, you should:

1. Review [OpenAI's Privacy Policy](https://openai.com/policies/privacy-policy)
2. Review the [OpenAI Terms of Use](https://openai.com/policies/terms-of-use)
3. Understand the data-retention settings on your OpenAI / ChatGPT account

### Disclaimer

The SmartScribe project and its contributors are not responsible for how OpenAI handles data sent through their services. Your use of OpenAI services is governed by your agreement with OpenAI, not with SmartScribe.

SmartScribe is an unofficial, community-maintained project and is not affiliated with, endorsed by, or sponsored by OpenAI.

## Local Data

SmartScribe stores the following data locally on your machine:

- **Configuration file**: TOML configuration with your settings (and, optionally, your OpenAI API key)
- **OAuth token file** (`oauth.json`): cached when `auth = "oauth"`, written with mode `0600` on Unix. Managed via `smart-scribe login` / `smart-scribe logout`.
- **Temporary audio buffers**: held in memory during a recording session; not written to disk.

Both files live under your platform's per-user config directory (`~/.config/smart-scribe/` on Linux, `~/Library/Application Support/smart-scribe/` on macOS, `%APPDATA%\smart-scribe\` on Windows).

## Questions

If you have questions about this privacy policy, please open an issue on the [GitHub repository](https://github.com/ThilinaTLM/smart-scribe).
