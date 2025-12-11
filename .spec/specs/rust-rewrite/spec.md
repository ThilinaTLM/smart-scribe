# SmartScribe Rust Rewrite

## Purpose

Complete rewrite of SmartScribe from TypeScript/Bun to Rust for improved performance, standalone binary distribution, and native signal handling.

## User Stories

### US-1: One-Shot Transcription
AS A user I WANT to record audio for a fixed duration and get AI transcription SO THAT I can quickly convert speech to text.

#### Acceptance Criteria
- [ ] GIVEN default config WHEN running `smart-scribe` THEN records for 10s and outputs transcription
- [ ] GIVEN `-d 30s` flag WHEN running THEN records for exactly 30 seconds
- [ ] GIVEN `-D dev` flag WHEN running THEN uses software development domain context
- [ ] GIVEN `-c` flag WHEN transcription complete THEN copies result to clipboard
- [ ] GIVEN `-k` flag WHEN transcription complete THEN types result into focused window
- [ ] GIVEN `-n` flag WHEN recording events occur THEN shows desktop notifications
- [ ] GIVEN SIGINT during recording WHEN received THEN stops early and transcribes captured audio
- [ ] GIVEN successful transcription WHEN complete THEN outputs text to stdout only

### US-2: Daemon Mode
AS A user I WANT to run SmartScribe as a daemon and control recording via signals SO THAT I can integrate it with keyboard shortcuts.

#### Acceptance Criteria
- [ ] GIVEN `--daemon` flag WHEN running THEN starts daemon and waits for signals
- [ ] GIVEN daemon running WHEN SIGUSR1 sent THEN toggles recording (start if idle, stop+transcribe if recording)
- [ ] GIVEN daemon recording WHEN SIGUSR2 sent THEN cancels recording without transcription
- [ ] GIVEN daemon running WHEN SIGINT/SIGTERM sent THEN shuts down gracefully
- [ ] GIVEN `--max-duration 30s` WHEN recording reaches limit THEN auto-stops and transcribes
- [ ] GIVEN daemon running WHEN started THEN writes PID to `/tmp/smart-scribe.pid`
- [ ] GIVEN daemon running WHEN another instance started THEN fails with existing PID error

### US-3: Configuration Management
AS A user I WANT to manage persistent configuration SO THAT I don't need to specify options every time.

#### Acceptance Criteria
- [ ] GIVEN `config init` WHEN no config exists THEN creates default config file
- [ ] GIVEN `config init` WHEN config exists THEN fails with error (prevent overwrite)
- [ ] GIVEN `config set domain dev` WHEN run THEN saves domain=dev to config
- [ ] GIVEN `config get domain` WHEN run THEN outputs current domain value
- [ ] GIVEN `config list` WHEN run THEN outputs all config key-value pairs
- [ ] GIVEN `config path` WHEN run THEN outputs config file path
- [ ] GIVEN invalid key WHEN `config set/get` THEN outputs error with valid keys
- [ ] GIVEN CLI args + env vars + config file WHEN running THEN merges with correct priority

### US-4: Audio Recording
AS A user I WANT reliable audio recording from my microphone SO THAT my speech is captured accurately.

#### Acceptance Criteria
- [ ] GIVEN PulseAudio/Pipewire WHEN recording THEN captures from default microphone
- [ ] GIVEN recording WHEN complete THEN outputs OGG/Opus format at 16kHz mono
- [ ] GIVEN bounded recording WHEN duration elapsed THEN stops automatically
- [ ] GIVEN unbounded recording WHEN stopped THEN finalizes audio file cleanly
- [ ] GIVEN recording WHEN progress callback provided THEN reports progress every 100ms
- [ ] GIVEN recording complete WHEN called THEN reports file size

### US-5: AI Transcription
AS A user I WANT accurate AI-powered transcription SO THAT my speech becomes usable text.

#### Acceptance Criteria
- [ ] GIVEN audio data WHEN transcribing THEN sends to Gemini 2.0 Flash Lite
- [ ] GIVEN domain preset WHEN transcribing THEN includes domain-specific context in prompt
- [ ] GIVEN transcription WHEN complete THEN returns cleaned text without filler words
- [ ] GIVEN invalid API key WHEN transcribing THEN returns clear error message
- [ ] GIVEN rate limit WHEN hit THEN returns appropriate error
- [ ] GIVEN empty audio WHEN transcribing THEN returns empty response error

### US-6: Output Actions
AS A user I WANT optional output actions (clipboard, keystroke) SO THAT I can use transcription immediately.

#### Acceptance Criteria
- [ ] GIVEN clipboard enabled WHEN transcription complete THEN copies via wl-copy
- [ ] GIVEN keystroke enabled WHEN transcription complete THEN types via xdotool
- [ ] GIVEN clipboard fails WHEN copying THEN warns but still outputs text
- [ ] GIVEN keystroke fails WHEN typing THEN warns but still outputs text
- [ ] GIVEN notifications enabled WHEN events occur THEN shows via notify-send

## Requirements

### Recording Requirements
- REQ-R1: The system SHALL record audio using FFmpeg with PulseAudio input
- REQ-R2: The system SHALL encode audio as OGG/Opus at 16kHz mono, 16kbps
- REQ-R3: The system SHALL support bounded (fixed duration) recording
- REQ-R4: The system SHALL support unbounded (signal-controlled) recording
- REQ-R5: The system SHALL clean up temp files on exit (normal or error)
- REQ-R6: The system MUST handle SIGINT gracefully during recording

### Transcription Requirements
- REQ-T1: The system SHALL use Google Gemini API for transcription
- REQ-T2: The system SHALL use `gemini-2.0-flash-lite` model
- REQ-T3: The system SHALL disable thinking mode (`thinkingBudget: 0`)
- REQ-T4: The system SHALL include domain-specific context in system prompt
- REQ-T5: The system MUST NOT expose API key in logs or output

### Configuration Requirements
- REQ-C1: The system SHALL use XDG config path (`~/.config/smart-scribe/config.toml`)
- REQ-C2: The system SHALL support TOML format for config file
- REQ-C3: The system SHALL merge configs with priority: CLI > ENV > file > defaults
- REQ-C4: The system SHALL validate config values on load
- REQ-C5: The system SHALL support legacy nested TOML format

### CLI Requirements
- REQ-CLI1: The system SHALL output transcription to stdout only
- REQ-CLI2: The system SHALL output status messages to stderr
- REQ-CLI3: The system SHALL exit with code 0 on success, 1 on error, 2 on usage error
- REQ-CLI4: The system SHALL show progress during recording
- REQ-CLI5: The system MUST NOT allow `-d` with `--daemon` simultaneously

### Daemon Requirements
- REQ-D1: The system SHALL handle SIGUSR1 (toggle), SIGUSR2 (cancel), SIGINT/SIGTERM (exit)
- REQ-D2: The system SHALL maintain state machine (IDLE → RECORDING → PROCESSING → IDLE)
- REQ-D3: The system SHALL enforce max duration safety limit
- REQ-D4: The system SHALL create PID file at `/tmp/smart-scribe.pid`
- REQ-D5: The system SHALL prevent multiple daemon instances

## Domain Model

### Value Objects
| Name | Description |
|------|-------------|
| Duration | Time duration (parses `30s`, `1m`, `2m30s`) |
| DomainId | Transcription domain enum (general, dev, medical, legal, finance) |
| SystemPrompt | Complete prompt with base instructions + domain context |
| AudioData | Raw audio bytes with MIME type |
| AppConfig | Application configuration with merge support |

### Entities
| Name | Description |
|------|-------------|
| DaemonSession | State machine for daemon recording lifecycle |

### Domain Errors
| Error | When |
|-------|------|
| DurationParseError | Invalid duration format |
| InvalidDomainError | Unknown domain ID |
| InvalidStateTransition | Invalid daemon state transition |
| ConfigError | Config file issues (parse, write, validation) |

## Out of Scope

- Windows/macOS support (Linux only)
- Non-PulseAudio audio backends
- File upload to Gemini (inline only)
- Streaming transcription
- Multiple language support
- Audio preprocessing/noise reduction

## Open Questions

- [x] Should we use `async-trait` or nightly async traits? → Use `async-trait`
- [x] Manual REST vs `gemini-rust` crate? → Manual REST with reqwest
- [x] Type-state vs runtime state machine for daemon? → Runtime for flexibility
