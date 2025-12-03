# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SmartScribe-TS is a Bun-based TypeScript CLI tool for AI-powered audio transcription using Google Gemini. It records from the microphone using FFmpeg and outputs context-aware text to stdout and clipboard.

## Development Commands

```bash
bun install              # Install dependencies
bun run src/index.ts     # Run the CLI directly
smart-scribe -h          # Show help (after linking: bun link)
```

## Environment Setup

Create `.env` in the project root with:
```
GEMINI_API_KEY=your_api_key_here
```

## CLI Usage

```bash
smart-scribe                     # 10s general transcription
smart-scribe -d 60s              # 60 second recording
smart-scribe -d 1m -D dev        # 1 minute, software domain
smart-scribe --domain medical    # Medical terminology mode
```

**Options:**
- `-d, --duration <TIME>` - Recording duration (10s, 1m, 2m30s)
- `-D, --domain <DOMAIN>` - Domain preset (general|dev|medical|legal|finance)
- `-h, --help` / `-v, --version`

## Architecture

**Hexagonal (Ports & Adapters) Architecture:**

```
src/
├── domain/           # Core domain logic (value objects, errors)
├── application/      # Use cases and port interfaces
├── infrastructure/   # Adapter implementations (FFmpeg, Gemini, clipboard)
└── cli/              # CLI entry point and presentation
```

**Data Flow:**
1. CLI parses args → creates `TranscribeRecordingUseCase` with injected adapters
2. Use case orchestrates: Record → Transcribe → Copy to clipboard
3. All operations return `Result<T, E>` for explicit error handling (no exceptions)

**Key Abstractions:**

- `AudioRecorderPort` → `FFmpegRecorderAdapter`: Records via `ffmpeg` subprocess
- `TranscriptionPort` → `GeminiTranscriptionAdapter`: Calls Gemini API with base64 audio
- `ClipboardPort` → `WaylandClipboardAdapter`: Uses `wl-copy` for Wayland clipboard

**Value Objects (immutable):**
- `Duration`: Parses time strings (30s, 1m, 2m30s)
- `DomainPreset`: Domain-specific prompt configuration
- `SystemPrompt`: Builds full transcription prompt from preset
- `AudioData`: Holds base64-encoded audio with MIME type

**Result Type Pattern:**
All fallible operations return `Result<T, E>` instead of throwing. Check with `result.ok`, access `result.value` or `result.error`.
