# Tasks: SmartScribe Rust Rewrite

## Legend
- [ ] Pending
- [x] Complete
- [P] Can run in parallel
- [B] Blocked
- [S] Skipped

## Phase 1: Project Setup & Domain Layer

### 1.1 Initialize Rust Project
- Files: `Cargo.toml`, `src/main.rs`, `src/lib.rs`
- Depends: None
- [x] Create Cargo.toml with all dependencies
- [x] Create basic main.rs entry point
- [x] Create lib.rs with module structure
- [x] Verify build compiles

### 1.2 Duration Value Object [P]
- Files: `src/domain/mod.rs`, `src/domain/recording/mod.rs`, `src/domain/recording/duration.rs`
- Depends: 1.1
- [x] Implement Duration struct with milliseconds storage
- [x] Implement FromStr for parsing ("30s", "1m", "2m30s")
- [x] Implement Display for formatting
- [x] Add DEFAULT and DEFAULT_MAX constants
- [x] Add as_seconds(), as_milliseconds(), as_std() methods
- [x] Write unit tests

### 1.3 DomainId Enum [P]
- Files: `src/domain/transcription/mod.rs`, `src/domain/transcription/domain_preset.rs`
- Depends: 1.1
- [x] Implement DomainId enum (General, Dev, Medical, Legal, Finance)
- [x] Implement FromStr with validation
- [x] Implement Display
- [x] Add label() and prompt() methods
- [x] Add ALL constant slice
- [x] Write unit tests

### 1.4 SystemPrompt Builder [P]
- Files: `src/domain/transcription/system_prompt.rs`
- Depends: 1.3
- [x] Define BASE_INSTRUCTION constant
- [x] Implement SystemPrompt struct
- [x] Implement build(domain: DomainId) constructor
- [x] Add content() accessor
- [x] Write unit tests

### 1.5 AudioData Value Object [P]
- Files: `src/domain/transcription/audio_data.rs`
- Depends: 1.1
- [x] Implement AudioMimeType enum (Ogg, Mp3, Wav, etc.)
- [x] Implement AudioData struct with data: Vec<u8> and mime_type
- [x] Add size_bytes() and human_readable_size() methods
- [x] Write unit tests

### 1.6 AppConfig Value Object
- Files: `src/domain/config/mod.rs`, `src/domain/config/app_config.rs`
- Depends: 1.2, 1.3
- [x] Implement AppConfig struct with all optional fields
- [x] Implement defaults() constructor
- [x] Implement merge(other: Self) method
- [x] Add duration_or_default(), max_duration_or_default(), domain_or_default()
- [x] Write unit tests for merge logic

### 1.7 DaemonSession State Machine
- Files: `src/domain/daemon/mod.rs`, `src/domain/daemon/session.rs`
- Depends: 1.1
- [x] Implement DaemonState enum (Idle, Recording, Processing)
- [x] Implement InvalidStateTransition error
- [x] Implement RuntimeDaemonSession struct (named DaemonSession)
- [x] Add state transitions: start_recording(), stop_recording(), cancel_recording(), complete_processing()
- [x] Add state query methods: is_idle(), is_recording(), is_processing()
- [x] Write unit tests for all transitions

### 1.8 Domain Error Types
- Files: `src/domain/error.rs`
- Depends: 1.2, 1.3, 1.6, 1.7
- [x] Implement DurationParseError
- [x] Implement InvalidDomainError
- [x] Implement InvalidStateTransition (in session.rs)
- [x] Implement ConfigError variants
- [x] Export from domain/mod.rs

---
**Checkpoint 1:** All domain types compile and unit tests pass.
```bash
cargo test --lib domain
```

## Phase 2: Application Layer

### 2.1 Port Traits - Recording
- Files: `src/application/mod.rs`, `src/application/ports/mod.rs`, `src/application/ports/recorder.rs`
- Depends: Phase 1
- [x] Define RecordingError enum
- [x] Define ProgressCallback type alias
- [x] Define AudioRecorder trait (bounded recording)
- [x] Define UnboundedRecorder trait (daemon recording)

### 2.2 Port Traits - Transcription [P]
- Files: `src/application/ports/transcriber.rs`
- Depends: Phase 1
- [x] Define TranscriptionError enum
- [x] Define Transcriber trait with transcribe() method

### 2.3 Port Traits - Output Actions [P]
- Files: `src/application/ports/clipboard.rs`, `src/application/ports/keystroke.rs`, `src/application/ports/notifier.rs`
- Depends: Phase 1
- [x] Define ClipboardError and Clipboard trait
- [x] Define KeystrokeError and Keystroke trait
- [x] Define NotificationError, NotificationIcon, and Notifier trait

### 2.4 Port Traits - Config [P]
- Files: `src/application/ports/config.rs`
- Depends: Phase 1
- [x] Define ConfigStore trait with load(), save(), path() methods

### 2.5 TranscribeRecordingUseCase
- Files: `src/application/transcribe.rs`
- Depends: 2.1, 2.2, 2.3
- [x] Define TranscribeError enum
- [x] Define TranscribeInput struct
- [x] Define TranscribeOutput struct
- [x] Define TranscribeCallbacks struct
- [x] Implement TranscribeRecordingUseCase with execute() method
- [x] Add stop_early() method for SIGINT handling
- [x] Write unit tests with mock adapters

### 2.6 DaemonTranscriptionUseCase
- Files: `src/application/daemon.rs`
- Depends: 2.1, 2.2, 2.3, 1.7
- [x] Define DaemonConfig struct
- [x] Define DaemonUseCase with start_recording(), stop_and_transcribe(), cancel() methods
- [x] Integrate RuntimeDaemonSession for state management
- [x] Handle max duration timeout
- [x] Write unit tests with mock adapters

---
**Checkpoint 2:** Application layer compiles, use cases work with mock adapters.
```bash
cargo test --lib application
```

## Phase 3: Infrastructure Layer

### 3.1 FFmpeg Recorder Adapter
- Files: `src/infrastructure/mod.rs`, `src/infrastructure/recording/mod.rs`, `src/infrastructure/recording/ffmpeg.rs`
- Depends: 2.1
- [x] Implement FfmpegRecorder struct with process management
- [x] Implement temp file generation (/tmp/smartscribe-{ts}.ogg)
- [x] Implement spawn_ffmpeg() helper
- [x] Implement AudioRecorder trait for bounded recording
- [x] Implement UnboundedRecorder trait for daemon recording
- [x] Add SIGINT signal sending for graceful stop
- [x] Add SIGKILL for cancel
- [x] Implement temp file cleanup
- [ ] Write integration tests (manual testing required)

### 3.2 Gemini Transcriber Adapter
- Files: `src/infrastructure/transcription/mod.rs`, `src/infrastructure/transcription/gemini.rs`
- Depends: 2.2
- [x] Define request types (GenerateContentRequest, Content, Part, InlineData, etc.)
- [x] Define response types (GenerateContentResponse, Candidate, etc.)
- [x] Implement GeminiTranscriber struct with api_key
- [x] Implement Transcriber trait
- [x] Handle error responses (401, 429, etc.)
- [x] Extract text from response
- [x] Write unit tests

### 3.3 Wayland Clipboard Adapter
- Files: `src/infrastructure/clipboard/mod.rs`, `src/infrastructure/clipboard/wayland.rs`
- Depends: 2.3
- [x] Implement WaylandClipboard struct
- [x] Implement Clipboard trait using wl-copy
- [x] Handle command not found error
- [ ] Write integration tests (manual testing required)

### 3.4 Xdotool Keystroke Adapter [P]
- Files: `src/infrastructure/keystroke/mod.rs`, `src/infrastructure/keystroke/xdotool.rs`
- Depends: 2.3
- [x] Implement XdotoolKeystroke struct
- [x] Implement Keystroke trait using xdotool type
- [x] Handle command not found error
- [ ] Write integration tests (manual testing required)

### 3.5 NotifySend Notifier Adapter [P]
- Files: `src/infrastructure/notification/mod.rs`, `src/infrastructure/notification/notify_send.rs`
- Depends: 2.3
- [x] Implement NotifySendNotifier struct
- [x] Implement icon mapping to freedesktop names
- [x] Implement Notifier trait using notify-send
- [x] Handle command not found silently
- [ ] Write integration tests (manual testing required)

### 3.6 XDG Config Store Adapter
- Files: `src/infrastructure/config/mod.rs`, `src/infrastructure/config/xdg.rs`
- Depends: 2.4, 1.6
- [x] Implement XdgConfigStore struct with path
- [x] Implement config file path resolution
- [x] Implement TOML parsing
- [x] Implement TOML serialization
- [S] Support legacy nested format detection (not needed for v2)
- [x] Implement ConfigStore trait
- [x] Write unit tests

---
**Checkpoint 3:** All adapters work with real external tools.
```bash
cargo test --lib infrastructure
# Manual: test with actual ffmpeg, wl-copy, etc.
```

## Phase 4: CLI Layer

### 4.1 Clap Argument Definitions
- Files: `src/cli/mod.rs`, `src/cli/args.rs`
- Depends: 1.2, 1.3
- [x] Define Cli struct with Parser derive
- [x] Define Commands enum (Config subcommand)
- [x] Define ConfigAction enum (Init, Set, Get, List, Path)
- [x] Define TranscribeArgs struct with all flags
- [x] Define DomainArg enum for clap ValueEnum
- [x] Add conflicts_with for daemon + duration
- [x] Write argument parsing tests

### 4.2 Presenter (Output Formatting)
- Files: `src/cli/presenter.rs`
- Depends: None
- [x] Implement Presenter struct with spinner state
- [x] Add start_spinner(), update_spinner(), spinner_success(), spinner_fail(), stop_spinner()
- [x] Add info(), success(), warn(), error() status methods
- [x] Add output() for stdout text
- [x] Implement format_progress() for recording progress bar
- [x] Use colored crate for terminal colors
- [x] Use indicatif for spinners

### 4.3 Config Command Handler
- Files: `src/cli/config_cmd.rs`
- Depends: 3.6, 4.2
- [x] Implement handle_config_command(action: ConfigAction)
- [x] Implement init command
- [x] Implement set command with key validation
- [x] Implement get command
- [x] Implement list command
- [x] Implement path command
- [x] Write unit tests

### 4.4 Signal Handlers
- Files: `src/cli/signals.rs`
- Depends: None
- [x] Implement SignalHandler for one-shot mode (SIGINT/SIGTERM)
- [x] Implement DaemonSignal enum (Toggle, Cancel, Shutdown)
- [x] Implement DaemonSignalHandler with mpsc channel
- [x] Setup signal listeners with tokio::signal
- [x] Write unit tests

### 4.5 PID File Management
- Files: `src/cli/pid_file.rs`
- Depends: None
- [x] Implement PidFile struct
- [x] Implement acquire() with stale detection
- [x] Implement release() with file deletion
- [x] Implement Drop for cleanup
- [x] Handle existing process check (signal 0)
- [x] Write unit tests

### 4.6 Main App Runner (One-Shot Mode)
- Files: `src/cli/app.rs`
- Depends: 2.5, 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 4.1, 4.2, 4.4
- [x] Implement run_oneshot() function
- [x] Implement config loading and merging
- [x] Wire up adapters to use case
- [x] Setup signal handler for early stop
- [x] Implement presenter callbacks
- [x] Return exit codes

### 4.7 Daemon App Runner
- Files: `src/cli/daemon_app.rs`
- Depends: 2.6, 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 4.1, 4.2, 4.4, 4.5
- [x] Implement run_daemon() function
- [x] Setup PID file
- [x] Setup daemon signal handler
- [x] Implement main signal loop
- [x] Handle toggle (start/stop recording)
- [x] Handle cancel
- [x] Handle shutdown
- [x] Implement max duration timeout
- [x] Return exit codes

### 4.8 Main Entry Point
- Files: `src/main.rs`
- Depends: 4.3, 4.6, 4.7
- [x] Setup tokio runtime
- [x] Parse CLI args
- [x] Route to config command handler
- [x] Route to daemon app
- [x] Route to one-shot app
- [x] Handle exit codes

---
**Checkpoint 4:** CLI has full feature parity.
```bash
cargo build --release
./target/release/smart-scribe --help
./target/release/smart-scribe config path
```

## Phase 5: Integration & Testing

### 5.1 CLI Integration Tests
- Files: `tests/cli_tests.rs`
- Depends: Phase 4
- [x] Test --help output
- [x] Test --version output
- [x] Test config path command
- [x] Test invalid duration error
- [x] Test daemon + duration conflict
- [x] Test invalid domain error

### 5.2 Recording Integration Tests [P]
- Files: `tests/recording_tests.rs`
- Depends: 3.1
- [ ] Test bounded recording (manual, requires mic)
- [ ] Test unbounded recording (manual)
- [ ] Test early stop with SIGINT
- [ ] Test file cleanup

### 5.3 Transcription Integration Tests [P]
- Files: `tests/transcription_tests.rs`
- Depends: 3.2
- [x] Test successful transcription (requires API key)
- [x] Test invalid API key error
- [x] Mark as #[ignore] for CI

### 5.4 Comparison Testing
- Files: `scripts/compare-versions.sh`
- Depends: Phase 4
- [x] Create comparison script
- [x] Test CLI option compatibility
- [x] Test output format consistency
- [x] Test exit codes
- [x] Document any differences

### 5.5 Error Scenario Testing
- Files: `tests/error_tests.rs`
- Depends: Phase 4
- [x] Test missing API key
- [S] Test missing FFmpeg (requires uninstalling FFmpeg)
- [S] Test missing wl-copy (requires Wayland-specific test)
- [S] Test missing xdotool (requires X11-specific test)
- [x] Test config file not found (uses defaults)
- [x] Test invalid config values

---
**Checkpoint 5:** All tests pass, feature parity confirmed.
```bash
cargo test
./scripts/compare-versions.sh
```

## Phase 6: Cleanup & Release

### 6.1 Archive TypeScript
- Files: Various
- Depends: Phase 5
- [ ] Tag TypeScript version as v1.x-typescript-final
- [ ] Remove TypeScript source files (src/ TS files)
- [ ] Remove package.json, bun.lockb, tsconfig.json, biome.json
- [ ] Update .gitignore

### 6.2 Update Documentation
- Files: `README.md`, `CLAUDE.md`
- Depends: 6.1
- [ ] Update README for Rust version
- [ ] Update installation instructions
- [ ] Update build instructions
- [ ] Update CLAUDE.md with Rust commands

### 6.3 Update CI/CD
- Files: `.github/workflows/ci.yml`, `.github/workflows/release.yml`
- Depends: 6.1
- [ ] Remove TypeScript CI jobs
- [ ] Update release workflow for Rust binary
- [ ] Add cargo fmt check
- [ ] Add cargo clippy check

### 6.4 Release v2.0.0
- Files: `CHANGELOG.md`
- Depends: 6.2, 6.3
- [ ] Write CHANGELOG entry
- [ ] Create git tag v2.0.0
- [ ] Push release
- [ ] Verify binary release artifacts

---
**Checkpoint 6:** Clean Rust-only repo, v2.0.0 released.
```bash
git status  # No TypeScript files
./target/release/smart-scribe --version  # v2.0.0
```

## Progress Summary

| Phase | Status | Tasks |
|-------|--------|-------|
| 1. Domain | Complete | 8/8 |
| 2. Application | Complete | 6/6 |
| 3. Infrastructure | Complete | 6/6 |
| 4. CLI | Complete | 8/8 |
| 5. Testing | In Progress | 4/5 |
| 6. Cleanup | Pending | 0/4 |
| **Total** | **In Progress** | **32/37** |
