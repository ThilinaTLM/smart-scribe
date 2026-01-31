# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SmartScribe is a **cross-platform** Rust CLI tool for AI-powered audio transcription using Google Gemini. It records from the microphone and outputs context-aware text to stdout. Supports Windows, macOS, and Linux.

**Modes:**
- **One-shot** - Fixed duration recording, immediate transcription
- **Daemon** - Background service with socket/pipe-controlled recording

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

```bash
# Option 1: Config command
smart-scribe config set api_key YOUR_API_KEY

# Option 2: Environment variable
export GEMINI_API_KEY=your_api_key_here

# Option 3: Config file
# Linux/macOS: ~/.config/smart-scribe/config.toml
# Windows: %APPDATA%\smart-scribe\config.toml
```

Config priority: CLI args > environment > config.toml > defaults

## CLI Usage

**One-shot mode:**
```bash
smart-scribe                     # 10s general transcription
smart-scribe -d 60s              # 60 second recording
smart-scribe -d 1m -D dev        # 1 minute, software domain
smart-scribe -c -k               # Copy to clipboard + type into window
smart-scribe -c -k -n            # Also show notification
```

**Daemon mode:**
```bash
smart-scribe --daemon                    # Run daemon
smart-scribe --daemon --indicator        # With recording indicator (Linux)
smart-scribe daemon toggle               # Toggle recording (start/stop)
smart-scribe daemon cancel               # Cancel without transcribing
smart-scribe daemon status               # Show state (idle/recording/processing)
```

**Config commands:**
```bash
smart-scribe config init         # Create default config
smart-scribe config set domain dev
smart-scribe config get domain
smart-scribe config list
smart-scribe config path
```

**Options:**
- `-d, --duration <TIME>` - Recording duration (10s, 1m, 2m30s)
- `-D, --domain <DOMAIN>` - Domain preset (general|dev|medical|legal|finance)
- `-c, --clipboard` - Copy result to clipboard
- `-k, --keystroke` - Type result into focused window
- `-n, --notify` - Show desktop notification
- `-a, --audio-cue` - Play audio cues on recording events
- `--daemon` - Run as daemon
- `--max-duration <TIME>` - Safety limit for daemon mode (default: 60s)
- `--keystroke-tool <TOOL>` - Linux only: enigo|auto|ydotool|xdotool|wtype
- `--indicator` - Linux only: show recording indicator (Wayland)
- `--indicator-position <POS>` - Position: top-left|top-center|top-right|bottom-*

## Architecture

**Hexagonal (Ports & Adapters) Architecture:**

```
src/
├── domain/           # Core logic (value objects, entities, errors)
├── application/      # Use cases and port traits
├── infrastructure/   # Adapter implementations
├── cli/              # CLI, args, presenter, IPC, signals
│   └── ipc/          # Unix sockets (Linux/macOS) + Named pipes (Windows)
├── gui/              # Recording indicator (Linux only)
├── lib.rs            # Library crate root
└── main.rs           # Binary entry point
```

**Key Abstractions (Ports → Adapters):**

| Port | Cross-Platform Adapter | Platform-Specific |
|------|----------------------|-------------------|
| `AudioRecorder` | `CpalRecorder` (cpal) | `FfmpegRecorder` (fallback) |
| `Transcriber` | `GeminiTranscriber` | - |
| `Clipboard` | `ArboardClipboard` (arboard) | `WaylandClipboard` (wl-copy) |
| `Keystroke` | `EnigoKeystroke` (enigo) | `Xdotool`, `Wtype`, `Ydotool` (Linux) |
| `Notifier` | `NotifyRustNotifier` (notify-rust) | `NotifySendNotifier` |
| `AudioCue` | `RodioAudioCue` (rodio) | `NoOpAudioCue` (disabled) |
| `ConfigStore` | `XdgConfigStore` (dirs) | - |
| IPC | - | Unix sockets / Named pipes |

**Value Objects (immutable):**
- `Duration` - Parses time strings (30s, 1m, 2m30s)
- `DomainId` - Domain-specific prompt configuration
- `SystemPrompt` - Builds full transcription prompt from domain
- `AudioData` - Holds audio bytes with MIME type
- `AppConfig` - Configuration with merge support

**Entities:**
- `DaemonSession` - State machine: IDLE ↔ RECORDING ↔ PROCESSING

## Cross-Platform Implementation

**Audio Recording (cpal):**
- Captures at device sample rate
- Resamples to 16kHz via rubato
- Encodes to Opus OGG format

**Clipboard (arboard):**
- Works on Windows, macOS, Linux (X11/Wayland)

**Keystroke (enigo + Linux tools):**
- Windows/macOS: Always uses enigo
- Linux with `--keystroke-tool auto`: Detects ydotool → wtype → xdotool → enigo
- Linux with explicit tool: Uses specified tool

**Notifications (notify-rust):**
- Cross-platform via native APIs

**IPC (Daemon mode):**
- Linux/macOS: Unix Domain Sockets (`$XDG_RUNTIME_DIR/smart-scribe.sock`)
- Windows: Named Pipes (`\\.\pipe\smart-scribe`)

## Linux-Specific Features

- **Recording Indicator** - Wayland layer-shell visual indicator (`--indicator`)
- **Keystroke Tool Selection** - Choose between ydotool, wtype, xdotool, or enigo
- **Auto-detection** - `--keystroke-tool auto` probes available tools

## Key Dependencies

**Cross-Platform:**
- `cpal` - Audio capture
- `rodio` - Audio playback for cues
- `arboard` - Clipboard
- `enigo` - Keystroke injection
- `notify-rust` - Desktop notifications
- `dirs` - Platform-aware directories
- `tokio` - Async runtime
- `reqwest` - HTTP client for Gemini API
- `clap` - CLI parsing
- `serde` / `toml` - Config

**Platform-Specific:**
- `nix` - Unix signal handling (Linux/macOS)
- `smithay-client-toolkit` - Wayland layer-shell (Linux)
- `windows-sys` - Windows API bindings

## Config File Format

```toml
api_key = "your-api-key"
duration = "10s"
max_duration = "60s"
domain = "general"
clipboard = false
keystroke = false
notify = false
audio_cue = false

[linux]
keystroke_tool = "enigo"
indicator = false
indicator_position = "top-right"
```

## Testing

```bash
cargo test                           # All tests
cargo test --lib                     # Unit tests only
cargo test --test cli_tests          # CLI integration tests
cargo test --test transcription_tests -- --ignored  # API tests (needs key)
```

## Gemini API

- Model: `gemini-2.0-flash-lite`
- Audio sent as base64 Opus OGG
- Domain-specific system prompts for context-aware transcription
