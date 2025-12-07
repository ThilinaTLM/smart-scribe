# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SmartScribe-TS is a Bun-based TypeScript CLI tool for AI-powered audio transcription using Google Gemini. It records from the microphone using FFmpeg and outputs context-aware text to stdout. Supports both one-shot mode (fixed duration) and daemon mode (signal-controlled recording).

## Development Commands

```bash
bun install              # Install dependencies
bun run src/index.ts     # Run the CLI directly
smart-scribe -h          # Show help (after linking: bun link)

bun run check            # Lint + format check (Biome)
bun run check:fix        # Auto-fix lint + format issues
bun run build            # Build standalone binary (dist/smart-scribe)
```

## Environment Setup

Create `.env` in the project root with:
```
GEMINI_API_KEY=your_api_key_here
```

Or use XDG config file at `~/.config/smart-scribe/config.toml`:
```toml
api_key = "your_api_key_here"
```

Config priority: CLI args > .env > config.toml > defaults

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
./scripts/signal-toggle.sh       # SIGUSR1: toggle recording
./scripts/signal-cancel.sh       # SIGUSR2: cancel recording
```

**Config commands:**
```bash
smart-scribe config init         # Create default config file
smart-scribe config set domain dev
smart-scribe config get domain
smart-scribe config list
```

**Options:**
- `-d, --duration <TIME>` - Recording duration (10s, 1m, 2m30s)
- `-D, --domain <DOMAIN>` - Domain preset (general|dev|medical|legal|finance)
- `-c, --clipboard` - Copy result to clipboard (wl-copy)
- `-k, --keystroke` - Type result into focused window (xdotool)
- `-n, --notify` - Show desktop notifications
- `--daemon` - Run as daemon (signal-controlled)
- `--max-duration <TIME>` - Safety limit for daemon mode (default: 60s)

## Architecture

**Hexagonal (Ports & Adapters) Architecture:**

```
src/
├── domain/           # Core domain logic (value objects, entities, errors)
├── application/      # Use cases and port interfaces
├── infrastructure/   # Adapter implementations (FFmpeg, Gemini, clipboard, etc.)
└── cli/              # CLI entry point, parser, presenter, signal handling
```

**Data Flow (One-shot):**
1. CLI parses args, merges config → creates `TranscribeRecordingUseCase`
2. Use case orchestrates: Record → Transcribe → Clipboard/Keystroke
3. All operations return `Result<T, E>` for explicit error handling

**Data Flow (Daemon):**
1. `DaemonTranscriptionUseCase` manages `DaemonSession` state machine
2. Signal handlers: SIGUSR1 (toggle), SIGUSR2 (cancel), SIGINT (exit)
3. States: IDLE → RECORDING → PROCESSING → IDLE

**Key Abstractions (Ports → Adapters):**
- `AudioRecorderPort` / `UnboundedRecorderPort` → `FFmpegRecorderAdapter`
- `TranscriptionPort` → `GeminiTranscriptionAdapter`
- `ClipboardPort` → `WaylandClipboardAdapter`
- `KeystrokePort` → `XdotoolKeystrokeAdapter`
- `NotificationPort` → `NotifySendAdapter`
- `ConfigPort` → `XdgConfigAdapter`

**Value Objects (immutable):**
- `Duration`: Parses time strings (30s, 1m, 2m30s)
- `DomainPreset`: Domain-specific prompt configuration
- `SystemPrompt`: Builds full transcription prompt from preset
- `AudioData`: Holds base64-encoded audio with MIME type
- `AppConfig`: Configuration with merge support

**Entities:**
- `DaemonSession`: State machine for daemon mode recording lifecycle

**Result Type Pattern:**
All fallible operations return `Result<T, E>` instead of throwing. Check with `result.ok`, access `result.value` or `result.error`.
