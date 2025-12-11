# SmartScribe Requirements Specification

This document provides a language-independent specification of SmartScribe's features and requirements for implementation reference.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Functional Requirements](#2-functional-requirements)
3. [Domain Model](#3-domain-model)
4. [Use Cases](#4-use-cases)
5. [External Interfaces](#5-external-interfaces)
6. [Configuration System](#6-configuration-system)
7. [Error Handling](#7-error-handling)
8. [Non-Functional Requirements](#8-non-functional-requirements)

---

## 1. Overview

### 1.1 Purpose

SmartScribe is a CLI tool for AI-powered audio transcription. It records audio from a microphone, sends it to Google Gemini for transcription, and outputs context-aware, cleaned text.

### 1.2 Operating Modes

| Mode | Description |
|------|-------------|
| **One-Shot** | Record for a fixed duration, transcribe, output, and exit |
| **Daemon** | Run as background process, controlled via Unix signals |
| **Config** | Manage persistent configuration settings |

### 1.3 Core Capabilities

- Audio recording from system default microphone
- AI-powered transcription with domain-specific context
- Clipboard integration (copy result)
- Keystroke injection (type result into focused window)
- Desktop notifications
- Persistent configuration via TOML file

---

## 2. Functional Requirements

### 2.1 Command-Line Interface

#### 2.1.1 One-Shot Mode (Default)

```
smart-scribe [OPTIONS]
```

**Options:**

| Flag | Long | Argument | Default | Description |
|------|------|----------|---------|-------------|
| `-d` | `--duration` | TIME | 10s | Recording duration |
| `-D` | `--domain` | DOMAIN | general | Domain preset for transcription context |
| `-c` | `--clipboard` | - | false | Copy result to clipboard |
| `-k` | `--keystroke` | - | false | Type result into focused window |
| `-n` | `--notify` | - | false | Show desktop notifications |
| `-h` | `--help` | - | - | Show help and exit |
| `-v` | `--version` | - | - | Show version and exit |

**Constraints:**
- `-d` and `--daemon` are mutually exclusive
- Duration format: `Ns`, `Nm`, `NmNs` (e.g., `30s`, `1m`, `2m30s`)

#### 2.1.2 Daemon Mode

```
smart-scribe --daemon [OPTIONS]
```

**Additional Options:**

| Flag | Long | Argument | Default | Description |
|------|------|----------|---------|-------------|
| - | `--daemon` | - | - | Enable daemon mode |
| - | `--max-duration` | TIME | 60s | Maximum recording duration (safety limit) |

**Signal Control:**

| Signal | Action |
|--------|--------|
| `SIGUSR1` | Toggle recording (start if idle, stop+transcribe if recording) |
| `SIGUSR2` | Cancel current recording (discard audio, no transcription) |
| `SIGINT` | Graceful shutdown |
| `SIGTERM` | Graceful shutdown |

#### 2.1.3 Config Subcommand

```
smart-scribe config <ACTION> [ARGS]
```

**Actions:**

| Action | Arguments | Description |
|--------|-----------|-------------|
| `init` | - | Create default config file |
| `set` | KEY VALUE | Set a configuration value |
| `get` | KEY | Get a configuration value |
| `list` | - | List all configuration values |
| `path` | - | Show config file path |

**Valid Keys:** `api_key`, `duration`, `max_duration`, `domain`, `clipboard`, `keystroke`, `notify`

#### 2.1.4 Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error (recording, transcription, config) |
| 2 | Usage error (invalid arguments) |
| 130 | Interrupted by user (SIGINT) |

#### 2.1.5 Output Streams

| Stream | Content |
|--------|---------|
| stdout | Final transcription text only (for piping) |
| stderr | Status messages, progress, errors |

### 2.2 Recording Requirements

#### 2.2.1 Audio Capture

- **Source:** System default microphone (PulseAudio/Pipewire)
- **Sample Rate:** 16 kHz (optimized for speech)
- **Channels:** Mono
- **Codec:** Opus
- **Container:** OGG
- **Bitrate:** 16 kbps (VoIP optimization)

#### 2.2.2 One-Shot Recording

- Record for exact specified duration
- Support early termination via SIGINT
- Report progress during recording
- Report final audio file size

#### 2.2.3 Daemon Recording

- Start/stop controlled by signals
- Enforce maximum duration safety limit
- Auto-stop and transcribe when max duration reached
- Support cancellation (discard without transcription)
- Report elapsed time during recording

### 2.3 Transcription Requirements

#### 2.3.1 AI Service

- **Provider:** Google Gemini
- **Model:** `gemini-2.0-flash-lite`
- **Input:** Base64-encoded audio with MIME type
- **Output:** Cleaned, transcribed text

#### 2.3.2 System Prompt

**Base Instructions:**
```
You are a voice-to-text assistant that transcribes audio into grammatically
correct, context-aware text output.

Instructions:
- Remove filler words (um, ah, like, you know)
- Must have correct grammar and punctuation
- Do NOT transcribe stutters, false starts, or repeated words
- Output ONLY the final cleaned text
- Do NOT include meta-commentary or explanations
```

**Domain Context:** Appended based on selected domain preset.

#### 2.3.3 Domain Presets

| ID | Label | Context Focus |
|----|-------|---------------|
| `general` | General Conversation | Standard grammar correction and clarity |
| `dev` | Software Engineering | Programming terminology, variable naming, technical jargon |
| `medical` | Medical / Healthcare | Medical conditions, medications, procedures, anatomy |
| `legal` | Legal | Legal terminology, formal tone, case citations |
| `finance` | Finance | Financial markets, acronyms (ETF, ROI, CAGR), figures |

### 2.4 Output Actions

#### 2.4.1 Clipboard Copy

- Copy transcription text to system clipboard
- Requires: Wayland clipboard utility (`wl-copy`)
- Non-fatal: Failure shows warning but doesn't prevent text output

#### 2.4.2 Keystroke Injection

- Type transcription text into currently focused window
- Requires: X11 automation tool (`xdotool`)
- Non-fatal: Failure shows warning but doesn't prevent text output

#### 2.4.3 Desktop Notifications

- Show notifications for recording events
- Requires: Desktop notification utility (`notify-send`)
- Non-fatal: Failure is silently ignored

**Notification Events:**

| Event | Title | Icon |
|-------|-------|------|
| Recording started | SmartScribe | `audio-input-microphone` |
| Recording stopped | SmartScribe | `system-run` |
| Transcription complete | SmartScribe | `dialog-ok` |
| Copied to clipboard | SmartScribe | `edit-copy` |
| Error | SmartScribe | `dialog-error` |

---

## 3. Domain Model

### 3.1 Value Objects

#### 3.1.1 Duration

Represents a time duration for recording.

**Properties:**
- Internal storage in milliseconds

**Construction:**
- Parse from string: `"30s"`, `"1m"`, `"2m30s"`, `"90s"`
- Pattern: `^(?:(\d+)m)?(?:(\d+)s)?$` (case-insensitive, trimmed)
- Must be greater than 0

**Operations:**
- Convert to seconds
- Convert to milliseconds
- Format as human-readable string (`"2m30s"`, `"30s"`)

**Validation:**
- Return error if pattern doesn't match
- Return error if total duration ≤ 0

#### 3.1.2 DomainPreset

Represents a transcription domain with specialized prompt.

**Properties:**
- `id`: Domain identifier (enum)
- `label`: Human-readable name
- `prompt`: Domain-specific prompt text

**Valid IDs:** `general`, `dev`, `medical`, `legal`, `finance`

**Operations:**
- Get preset by ID
- Validate ID string
- Get all available IDs
- Get default preset (`general`)

#### 3.1.3 SystemPrompt

Represents the complete system instruction for transcription.

**Properties:**
- `content`: Full prompt string (immutable)

**Construction:**
- Build from base instructions + domain preset
- Format: `{BASE_INSTRUCTIONS}\n\nDomain Context: {label}\n{prompt}`

#### 3.1.4 AudioData

Represents recorded audio for transcription.

**Properties:**
- `base64`: Base64-encoded audio data
- `mimeType`: Audio MIME type

**Supported MIME Types:**
- `audio/ogg`
- `audio/mp3`
- `audio/mpeg`
- `audio/wav`
- `audio/webm`
- `audio/mp4`

**Computed Properties:**
- Size in bytes (from base64 length)
- Human-readable size (`"1.5 KB"`, `"2.3 MB"`)

#### 3.1.5 AppConfig

Represents application configuration.

**Properties (all optional except where noted):**
- `apiKey`: Gemini API key (required at runtime)
- `duration`: Default recording duration string
- `maxDuration`: Maximum daemon recording duration string
- `domain`: Default domain preset ID
- `clipboard`: Enable clipboard by default
- `keystroke`: Enable keystroke by default
- `notify`: Enable notifications by default

**Operations:**
- Merge two configs (later values override)
- Get default config
- Convert to/from raw object

### 3.2 Entities

#### 3.2.1 DaemonSession

State machine managing daemon recording lifecycle.

**States:**

```
┌──────────────────────────────────────────────────────┐
│                                                      │
│    ┌──────┐   start    ┌───────────┐                │
│    │ IDLE │ ─────────> │ RECORDING │                │
│    └──────┘            └───────────┘                │
│       ▲                  │       │                   │
│       │                  │       │                   │
│       │   complete       │stop   │cancel             │
│       │                  │       │                   │
│       │                  ▼       │                   │
│       │              ┌──────────┐│                   │
│       └───────────── │PROCESSING││                   │
│                      └──────────┘│                   │
│       ▲                          │                   │
│       └──────────────────────────┘                   │
│                                                      │
└──────────────────────────────────────────────────────┘
```

**Transitions:**

| From | To | Trigger | Notes |
|------|----|---------|-------|
| IDLE | RECORDING | `startRecording()` | Begin audio capture |
| RECORDING | PROCESSING | `stopRecording()` | Stop capture, begin transcription |
| RECORDING | IDLE | `cancelRecording()` | Discard audio, no transcription |
| PROCESSING | IDLE | `completeProcessing()` | Transcription finished |

**Constraints:**
- Each transition validates current state
- Invalid transitions return error with current state and attempted action

### 3.3 Domain Errors

| Error | Code | When |
|-------|------|------|
| DurationParseError | `DURATION_PARSE_ERROR` | Invalid duration format |
| RecordingError | `RECORDING_ERROR` | Audio capture failure |
| TranscriptionError | `TRANSCRIPTION_ERROR` | API call failure |
| ClipboardError | `CLIPBOARD_ERROR` | Clipboard operation failure |
| KeystrokeError | `KEYSTROKE_ERROR` | Keystroke injection failure |
| NotificationError | `NOTIFICATION_ERROR` | Notification failure |
| InvalidStateTransitionError | `INVALID_STATE_TRANSITION` | Invalid daemon state transition |
| ConfigFileNotFoundError | `CONFIG_FILE_NOT_FOUND` | Config file doesn't exist |
| ConfigParseError | `CONFIG_PARSE_ERROR` | Config file malformed |
| ConfigValidationError | `CONFIG_VALIDATION_ERROR` | Config value invalid |
| ConfigWriteError | `CONFIG_WRITE_ERROR` | Cannot write config file |
| ConfigKeyNotFoundError | `CONFIG_KEY_NOT_FOUND` | Unknown config key |
| EnvironmentError | `ENVIRONMENT_ERROR` | Required env var missing |

---

## 4. Use Cases

### 4.1 One-Shot Transcription

**Input:**
- Duration
- Domain ID
- Enable clipboard (optional)
- Enable keystroke (optional)
- Progress callbacks (optional)

**Output:**
- Transcribed text
- Clipboard copied flag
- Keystroke sent flag

**Flow:**

```
1. Notify: recording started
2. Record audio for specified duration
   - Report progress periodically
   - Support early termination
3. Notify: recording complete (with audio size)
4. Build system prompt from domain
5. Notify: transcription started
6. Send audio to Gemini API
7. Receive transcribed text
8. Notify: transcription complete
9. If clipboard enabled:
   a. Copy text to clipboard
   b. Notify: clipboard result (success/failure)
10. If keystroke enabled:
    a. Type text into focused window
    b. Notify: keystroke result (success/failure)
11. Return result
```

**Error Handling:**
- Recording error: abort, return error with stage="recording"
- Transcription error: abort, return error with stage="transcription"
- Clipboard/keystroke error: continue, mark as failed in result

### 4.2 Daemon Transcription

**Config:**
- Domain ID
- Max duration
- Enable clipboard
- Enable keystroke

**Operations:**

#### Start Recording
1. Validate state is IDLE
2. Transition to RECORDING
3. Begin audio capture (unbounded)
4. Start progress timer
5. Set max duration timeout

#### Stop and Transcribe
1. Validate state is RECORDING
2. Transition to PROCESSING
3. Stop audio capture
4. Get recorded audio data
5. Build system prompt
6. Send to Gemini API
7. Execute output actions (clipboard, keystroke)
8. Transition to IDLE
9. Return transcribed text

#### Cancel Recording
1. Validate state is RECORDING
2. Discard recorded audio
3. Transition to IDLE

**Max Duration Behavior:**
- When max duration reached during recording
- Automatically trigger stop and transcribe
- Notify user that max duration was reached

### 4.3 Configuration Management

#### Initialize Config
1. Check if file already exists
2. If exists: return error (prevent overwrite)
3. Create default config
4. Write to config file path

#### Set Value
1. Validate key is known
2. Validate value format based on key type:
   - Duration keys: parse as Duration
   - Domain key: validate against valid IDs
   - Boolean keys: parse "true"/"false"
   - API key: accept any string
3. Load existing config (or empty)
4. Update value
5. Save config

#### Get Value
1. Validate key is known
2. Load config
3. Return value (or undefined if not set)

#### List Values
1. Load config (use defaults if file missing)
2. Output all key-value pairs

---

## 5. External Interfaces

### 5.1 Audio Recording (FFmpeg)

**Purpose:** Capture audio from microphone

**Command (bounded):**
```bash
ffmpeg -f pulse -i default -t <SECONDS> \
  -ar 16000 -ac 1 -c:a libopus -b:a 16k \
  -application voip -y <OUTPUT_PATH>
```

**Command (unbounded):**
```bash
ffmpeg -f pulse -i default \
  -ar 16000 -ac 1 -c:a libopus -b:a 16k \
  -application voip -y <OUTPUT_PATH>
```

**Parameters:**
| Flag | Value | Purpose |
|------|-------|---------|
| `-f pulse` | - | PulseAudio input |
| `-i default` | - | Default microphone |
| `-t` | seconds | Duration (bounded only) |
| `-ar` | 16000 | Sample rate |
| `-ac` | 1 | Mono |
| `-c:a libopus` | - | Opus codec |
| `-b:a` | 16k | Bitrate |
| `-application voip` | - | VoIP optimization |
| `-y` | - | Overwrite output |

**Output:** OGG file with Opus audio

**Signals:**
- `SIGINT`: Graceful stop (finalize file)
- `SIGKILL`: Force stop (cancel mode)

**Temp File:** `/tmp/smartscribe-{timestamp}.ogg`

### 5.2 Transcription (Google Gemini)

**Purpose:** Convert audio to text

**Model:** `gemini-2.0-flash-lite`

**Request:**
```
Model: gemini-2.0-flash-lite
Contents:
  - Role: user
    Parts:
      - InlineData:
          mimeType: <audio MIME type>
          data: <base64 audio>
Config:
  systemInstruction: <prompt>
  thinkingConfig:
    thinkingBudget: 0  (disable thinking for speed)
```

**Response:** Text content from model response

**Authentication:** API key via `GEMINI_API_KEY` env var or config

### 5.3 Clipboard (wl-copy)

**Purpose:** Copy text to Wayland clipboard

**Command:**
```bash
wl-copy -- <TEXT>
```

**Input:** Text via command argument

**Requirements:** `wl-clipboard` package, Wayland session

### 5.4 Keystroke Injection (xdotool)

**Purpose:** Type text into focused window

**Command:**
```bash
xdotool type -- <TEXT>
```

**Input:** Text via command argument

**Requirements:** `xdotool` package, X11 session

### 5.5 Desktop Notifications (notify-send)

**Purpose:** Show desktop notifications

**Command:**
```bash
notify-send [-i <ICON>] <TITLE> <MESSAGE>
```

**Parameters:**
- `-i`: Icon name (freedesktop standard)
- Title: Notification title
- Message: Notification body

**Requirements:** `libnotify` package, notification daemon

### 5.6 Icons Used

| Name | Usage |
|------|-------|
| `audio-input-microphone` | Recording started |
| `system-run` | Processing |
| `edit-copy` | Clipboard |
| `input-keyboard` | Keystroke |
| `dialog-ok` | Success |
| `dialog-error` | Error |
| `dialog-information` | Info |
| `dialog-warning` | Warning |

---

## 6. Configuration System

### 6.1 File Location

**Primary:** `$XDG_CONFIG_HOME/smart-scribe/config.toml`
**Fallback:** `~/.config/smart-scribe/config.toml`

### 6.2 File Format

```toml
api_key = "your-api-key"
duration = "10s"
max_duration = "60s"
domain = "general"
clipboard = false
keystroke = false
notify = false
```

### 6.3 Configuration Priority

From highest to lowest:
1. CLI arguments
2. Environment variables (`GEMINI_API_KEY`)
3. Config file
4. Hardcoded defaults

### 6.4 Default Values

| Key | Default |
|-----|---------|
| `duration` | `"10s"` |
| `max_duration` | `"60s"` |
| `domain` | `"general"` |
| `clipboard` | `false` |
| `keystroke` | `false` |
| `notify` | `false` |
| `api_key` | (none, required) |

### 6.5 Validation Rules

| Key | Type | Validation |
|-----|------|------------|
| `api_key` | string | Any non-empty string |
| `duration` | string | Must parse as Duration |
| `max_duration` | string | Must parse as Duration |
| `domain` | string | Must be valid domain ID |
| `clipboard` | boolean | "true" or "false" |
| `keystroke` | boolean | "true" or "false" |
| `notify` | boolean | "true" or "false" |

### 6.6 Legacy Format Support

Support for older nested TOML format:

```toml
[gemini]
api_key = "..."

[recording]
duration = 10        # numeric seconds
max_duration = 60

[domain]
default = "general"

[output]
clipboard = true
notification = true
```

**Conversion:** Detect by presence of section headers, convert numeric durations to string format.

---

## 7. Error Handling

### 7.1 Error Classification

| Category | Severity | Behavior |
|----------|----------|----------|
| Recording errors | Fatal | Abort operation, report error |
| Transcription errors | Fatal | Abort operation, report error |
| Clipboard/keystroke errors | Non-fatal | Warn user, continue with output |
| Notification errors | Silent | Ignore, continue normally |
| Config errors | Context-dependent | Fatal for required values, warn otherwise |
| State transition errors | Contextual | Warn user in daemon mode |

### 7.2 Error Messages

All errors should include:
- Error code for programmatic handling
- Human-readable message
- Context (what operation failed)
- Suggestion for resolution (when applicable)

### 7.3 Missing External Tools

When external command not found:
- Include tool name in error
- Suggest installation command (package manager specific)
- Example: `"wl-copy not found. Install with: sudo pacman -S wl-clipboard"`

### 7.4 Recovery Strategies

| Scenario | Strategy |
|----------|----------|
| FFmpeg fails | Report error, exit with code 1 |
| Gemini API fails | Report error, exit with code 1 |
| Clipboard fails | Warn, output text anyway |
| Config file missing | Use environment + defaults |
| Invalid config value | Report validation error |
| Daemon already running | Report error with existing PID |
| Signal during processing | Queue action, process when able |

---

## 8. Non-Functional Requirements

### 8.1 Performance

- Audio recording should have minimal latency
- Progress updates every 100ms during recording
- Transcription latency depends on API (typically 1-5 seconds)
- Daemon mode should have minimal memory footprint while idle

### 8.2 Reliability

- Clean temp file cleanup on exit (normal or error)
- Graceful handling of SIGINT during all operations
- Daemon should recover to idle state on errors
- PID file prevents multiple daemon instances

### 8.3 Security

- API keys stored in config file with user-only permissions
- API keys not logged or displayed in output
- Temp audio files created in /tmp with unique names
- No network connections except to Gemini API

### 8.4 Platform Requirements

- Linux (primary target)
- PulseAudio or Pipewire (audio)
- Wayland (clipboard) or X11 (keystroke)
- FFmpeg installed
- Internet connection for transcription

### 8.5 User Experience

- Progress feedback during long operations
- Colored output for different message types
- Spinner animation during waiting periods
- Clear error messages with actionable suggestions
- Output suitable for piping (text to stdout, status to stderr)

---

## Appendix A: Message Formatting

### A.1 Progress Bar

```
Recording: [████████░░░░░░░░░░░░] 4.0s remaining
```

- Width: 20 characters
- Filled: `█` (green)
- Empty: `░` (gray)

### A.2 Status Messages

| Type | Prefix | Color |
|------|--------|-------|
| Info | `ℹ` | Blue |
| Success | `✓` | Green |
| Warning | `⚠` | Yellow |
| Error | `✗` | Red |

### A.3 Duration Formatting

| Seconds | Output |
|---------|--------|
| 30 | `30s` |
| 60 | `1m` |
| 90 | `1m30s` |
| 120 | `2m` |

### A.4 File Size Formatting

| Bytes | Output |
|-------|--------|
| 500 | `500 B` |
| 1500 | `1.5 KB` |
| 1500000 | `1.5 MB` |

---

## Appendix B: Signal Handling Summary

### B.1 One-Shot Mode

| Signal | Handler | Action |
|--------|---------|--------|
| SIGINT | SignalHandler | Stop recording, cleanup, exit 130 |
| SIGTERM | SignalHandler | Stop recording, cleanup, exit 130 |

### B.2 Daemon Mode

| Signal | Handler | Action |
|--------|---------|--------|
| SIGUSR1 | DaemonSignalHandler | Toggle recording |
| SIGUSR2 | DaemonSignalHandler | Cancel recording |
| SIGINT | DaemonSignalHandler | Graceful shutdown |
| SIGTERM | DaemonSignalHandler | Graceful shutdown |

---

## Appendix C: PID File

**Location:** `/tmp/smart-scribe.pid`

**Contents:** Process ID (decimal, newline terminated)

**Lifecycle:**
1. Acquire: Check for stale PID, write current PID
2. Release: Delete file on daemon exit

**Stale Detection:** Send signal 0 to PID, if process doesn't exist, file is stale

---

## Appendix D: Complete CLI Help Text

```
SmartScribe - AI-powered audio transcription

Usage: smart-scribe [OPTIONS]
       smart-scribe --daemon [OPTIONS]
       smart-scribe config <ACTION> [ARGS]

Options:
  -d, --duration <TIME>      Recording duration (default: 10s)
  -D, --domain <DOMAIN>      Domain preset (default: general)
                             Values: general, dev, medical, legal, finance
  -c, --clipboard            Copy result to clipboard
  -k, --keystroke            Type result into focused window
  -n, --notify               Show desktop notifications
  -h, --help                 Show this help message
  -v, --version              Show version

Daemon mode:
  --daemon                   Run as background daemon
  --max-duration <TIME>      Max recording duration (default: 60s)

Config commands:
  config init                Create default config file
  config set <KEY> <VALUE>   Set a config value
  config get <KEY>           Get a config value
  config list                List all config values
  config path                Show config file path

Daemon signals:
  SIGUSR1                    Toggle recording (start/stop)
  SIGUSR2                    Cancel recording
  SIGINT/SIGTERM             Exit daemon

Examples:
  smart-scribe                     Record 10s, transcribe
  smart-scribe -d 30s -c          Record 30s, copy to clipboard
  smart-scribe --daemon -D dev    Daemon mode, dev domain
```
