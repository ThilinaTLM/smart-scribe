# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SmartScribe is a Rust CLI tool for AI-powered audio transcription using Google Gemini. It records from the microphone using FFmpeg and outputs context-aware text to stdout. Supports both one-shot mode (fixed duration) and daemon mode (socket-controlled recording).

## Development Commands

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo test               # Run all tests
cargo clippy             # Lint
cargo fmt                # Format code

# Run directly
cargo run -- -h          # Show help
cargo run -- -d 10s      # 10 second recording
cargo run -- --daemon    # Daemon mode
```

## Environment Setup

Set API key via config file or environment:

```bash
# Option 1: Config command
smart-scribe config set api_key YOUR_API_KEY

# Option 2: Environment variable
export GEMINI_API_KEY=your_api_key_here

# Option 3: Config file at ~/.config/smart-scribe/config.toml
api_key = "your_api_key_here"
```

Config priority: CLI args > environment > config.toml > defaults

## CLI Usage

**One-shot mode:**
```bash
smart-scribe                     # 10s general transcription
smart-scribe -d 60s              # 60 second recording
smart-scribe -d 1m -D dev        # 1 minute, software domain
smart-scribe -c -k               # Copy to clipboard + type into window
```

**Daemon mode:**
```bash
smart-scribe --daemon            # Run as background daemon
smart-scribe daemon toggle       # Toggle recording (start/stop)
smart-scribe daemon cancel       # Cancel current recording
smart-scribe daemon status       # Show daemon state (idle/recording/processing)
```

**Config commands:**
```bash
smart-scribe config init         # Create default config file
smart-scribe config set domain dev
smart-scribe config get domain
smart-scribe config list
smart-scribe config path
```

**Options:**
- `-d, --duration <TIME>` - Recording duration (10s, 1m, 2m30s)
- `-D, --domain <DOMAIN>` - Domain preset (general|dev|medical|legal|finance)
- `-c, --clipboard` - Copy result to clipboard (wl-copy)
- `-k, --keystroke` - Type result into focused window (xdotool)
- `-n, --notify` - Show desktop notifications
- `--daemon` - Run as daemon (socket-controlled)
- `--max-duration <TIME>` - Safety limit for daemon mode (default: 60s)

## Architecture

**Hexagonal (Ports & Adapters) Architecture:**

```
src/
├── domain/           # Core domain logic (value objects, entities, errors)
├── application/      # Use cases and port traits
├── infrastructure/   # Adapter implementations (FFmpeg, Gemini, clipboard, etc.)
├── cli/              # CLI entry point, args, presenter, signal handling
├── lib.rs            # Library crate root
└── main.rs           # Binary entry point
```

**Data Flow (One-shot):**
1. CLI parses args (clap), merges config → creates `TranscribeRecordingUseCase`
2. Use case orchestrates: Record → Transcribe → Clipboard/Keystroke
3. All operations return `Result<T, E>` for explicit error handling

**Data Flow (Daemon):**
1. `DaemonTranscriptionUseCase` manages `DaemonSession` state machine
2. Unix Domain Socket server receives commands: toggle, cancel, status
3. SIGINT/SIGTERM for graceful shutdown
4. States: IDLE → RECORDING → PROCESSING → IDLE

**Key Abstractions (Ports → Adapters):**
- `AudioRecorder` / `UnboundedRecorder` traits → `FfmpegRecorder`
- `Transcriber` trait → `GeminiTranscriber`
- `Clipboard` trait → `WaylandClipboard`
- `Keystroke` trait → `XdotoolKeystroke`
- `Notifier` trait → `NotifySendNotifier`
- `ConfigStore` trait → `XdgConfigStore`

**Value Objects (immutable):**
- `Duration`: Parses time strings (30s, 1m, 2m30s)
- `DomainId`: Domain-specific prompt configuration
- `SystemPrompt`: Builds full transcription prompt from domain
- `AudioData`: Holds audio bytes with MIME type
- `AppConfig`: Configuration with merge support

**Entities:**
- `DaemonSession`: State machine for daemon mode recording lifecycle

## Key Dependencies

- `clap` - CLI argument parsing
- `tokio` - Async runtime
- `reqwest` - HTTP client for Gemini API
- `serde` / `toml` - Config file parsing
- `colored` / `indicatif` - Terminal output formatting
- `nix` - Unix signal handling

## Testing

```bash
cargo test                           # All tests
cargo test --lib                     # Unit tests only
cargo test --test cli_tests          # CLI integration tests
cargo test --test transcription_tests -- --ignored  # API tests (needs key)
./scripts/compare-versions.sh        # Version comparison tests
```
