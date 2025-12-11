# Implementation Plan: SmartScribe Rust Rewrite

## Technical Approach

Parallel development approach: Rust code lives in `src/` alongside TypeScript (which will be archived). Maintains hexagonal architecture with ports (traits) and adapters (implementations).

**Key Architectural Decisions:**
1. **Hexagonal Architecture** - Clean separation of domain, application, and infrastructure
2. **Async-first** - Tokio runtime for all I/O operations
3. **Explicit error handling** - `Result<T, E>` everywhere, no panics
4. **Trait-based ports** - Easy testing with mock implementations
5. **Runtime state machine** - For daemon flexibility (vs compile-time type-state)

## Stack/Dependencies

```toml
[dependencies]
# Async runtime
tokio = { version = "1", features = ["full", "signal"] }

# HTTP
reqwest = { version = "0.12", features = ["json"] }

# CLI
clap = { version = "4", features = ["derive", "env"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Utilities
base64 = "0.22"
thiserror = "2"
colored = "2"
indicatif = "0.17"
dirs = "5"
dotenvy = "0.15"

[dev-dependencies]
wiremock = "0.6"
tempfile = "3"
assert_cmd = "2"
predicates = "3"
```

## Project Structure

```
src/
├── main.rs                 # Entry point
├── lib.rs                  # Library root (re-exports)
│
├── domain/                 # Core business logic
│   ├── mod.rs
│   ├── error.rs            # Domain errors
│   ├── recording/
│   │   ├── mod.rs
│   │   └── duration.rs     # Duration value object
│   ├── transcription/
│   │   ├── mod.rs
│   │   ├── audio_data.rs   # AudioData value object
│   │   ├── domain_preset.rs # DomainId enum
│   │   └── system_prompt.rs # SystemPrompt builder
│   ├── config/
│   │   ├── mod.rs
│   │   └── app_config.rs   # AppConfig value object
│   └── daemon/
│       ├── mod.rs
│       └── session.rs      # DaemonSession state machine
│
├── application/            # Use cases and ports
│   ├── mod.rs
│   ├── ports/
│   │   ├── mod.rs
│   │   ├── recorder.rs     # AudioRecorder trait
│   │   ├── transcriber.rs  # Transcriber trait
│   │   ├── clipboard.rs    # Clipboard trait
│   │   ├── keystroke.rs    # Keystroke trait
│   │   ├── notifier.rs     # Notifier trait
│   │   └── config.rs       # ConfigStore trait
│   ├── transcribe.rs       # TranscribeRecordingUseCase
│   └── daemon.rs           # DaemonTranscriptionUseCase
│
├── infrastructure/         # External adapters
│   ├── mod.rs
│   ├── recording/
│   │   ├── mod.rs
│   │   └── ffmpeg.rs       # FFmpegRecorder
│   ├── transcription/
│   │   ├── mod.rs
│   │   └── gemini.rs       # GeminiTranscriber
│   ├── clipboard/
│   │   ├── mod.rs
│   │   └── wayland.rs      # WaylandClipboard
│   ├── keystroke/
│   │   ├── mod.rs
│   │   └── xdotool.rs      # XdotoolKeystroke
│   ├── notification/
│   │   ├── mod.rs
│   │   └── notify_send.rs  # NotifySendNotifier
│   └── config/
│       ├── mod.rs
│       └── xdg.rs          # XdgConfigStore
│
└── cli/                    # CLI interface
    ├── mod.rs
    ├── args.rs             # Clap argument definitions
    ├── app.rs              # Main application runner
    ├── daemon_app.rs       # Daemon mode runner
    ├── config_cmd.rs       # Config subcommand handler
    ├── presenter.rs        # Output formatting
    ├── signals.rs          # Signal handlers
    └── pid_file.rs         # PID file management
```

## Implementation Phases

### Phase 1: Project Setup & Domain Layer
**Goal:** Establish Rust project and implement core domain types.

**Components:**
- Cargo.toml with all dependencies
- `Duration` value object with parsing
- `DomainId` enum with presets
- `SystemPrompt` builder
- `AudioData` value object
- `AppConfig` with merge logic
- `DaemonSession` state machine
- Domain error types

**Validation:** All domain unit tests pass.

### Phase 2: Application Layer (Ports & Use Cases)
**Goal:** Define port traits and implement use cases.

**Components:**
- `AudioRecorder` trait (bounded + unbounded)
- `Transcriber` trait
- `Clipboard` trait
- `Keystroke` trait
- `Notifier` trait
- `ConfigStore` trait
- `TranscribeRecordingUseCase`
- `DaemonTranscriptionUseCase`

**Validation:** Use cases work with mock adapters.

### Phase 3: Infrastructure Layer
**Goal:** Implement all external adapters.

**Components:**
- `FFmpegRecorder` adapter
- `GeminiTranscriber` adapter
- `WaylandClipboard` adapter
- `XdotoolKeystroke` adapter
- `NotifySendNotifier` adapter
- `XdgConfigStore` adapter

**Validation:** Each adapter works with real external tools.

### Phase 4: CLI Layer
**Goal:** Complete CLI implementation with all modes.

**Components:**
- Clap argument definitions
- Main app runner (one-shot mode)
- Daemon app runner
- Config subcommand handler
- Presenter (output formatting)
- Signal handlers
- PID file management

**Validation:** CLI has feature parity with TypeScript version.

### Phase 5: Integration & Testing
**Goal:** End-to-end testing and validation.

**Components:**
- CLI integration tests
- Comparison tests with TypeScript version
- Error scenario testing
- Signal handling tests
- Performance benchmarks

**Validation:** All acceptance criteria pass.

### Phase 6: Cleanup & Release
**Goal:** Finalize migration and release.

**Components:**
- Archive TypeScript source
- Update README
- Update CLAUDE.md
- Release v2.0.0

**Validation:** Clean repo with Rust-only implementation.

## Data Flow

### One-Shot Mode
```
CLI Args → Config Merge → TranscribeUseCase
    → FFmpegRecorder.record(duration)
    → GeminiTranscriber.transcribe(audio, prompt)
    → [Clipboard.copy(text)]
    → [Keystroke.type(text)]
    → stdout: text
```

### Daemon Mode
```
--daemon → DaemonUseCase.start()
    → PidFile.acquire()
    → Loop:
        SIGUSR1 (idle) → FFmpegRecorder.start_unbounded()
        SIGUSR1 (recording) → stop → transcribe → output
        SIGUSR2 → cancel recording
        SIGINT → cleanup → exit
```

## API Contracts

### Gemini API Request
```json
{
  "contents": [{
    "role": "user",
    "parts": [{
      "inlineData": {
        "mimeType": "audio/ogg",
        "data": "<base64>"
      }
    }]
  }],
  "systemInstruction": {
    "parts": [{"text": "<prompt>"}]
  },
  "generationConfig": {
    "thinkingConfig": {"thinkingBudget": 0}
  }
}
```

### FFmpeg Command (Bounded)
```bash
ffmpeg -f pulse -i default -t <SECONDS> \
  -ar 16000 -ac 1 -c:a libopus -b:a 16k \
  -application voip -y /tmp/smartscribe-{ts}.ogg
```

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| FFmpeg signal handling differs | M | Test extensively, use tokio kill_on_drop |
| Async trait complexity | L | Use async-trait crate, well-tested pattern |
| Gemini API changes | M | Isolate in adapter, easy to update |
| Cross-platform signals | L | Unix-only target, use tokio::signal |
| Binary size too large | L | LTO + strip in release profile |
| Different behavior from TS | M | Comparison testing before release |
