# CLAUDE.md

## Project Overview

SmartScribe is a cross-platform Rust CLI tool for AI-powered audio transcription (Gemini and ChatGPT backends). Records from microphone, outputs text to stdout. Two modes: **one-shot** (fixed duration) and **daemon** (background service with socket/pipe control).

## Development Commands

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo test               # Run all tests
cargo clippy             # Lint
cargo fmt                # Format
cargo run -- -h          # Show help
```

## Environment Setup

**Gemini (default):**
```bash
smart-scribe config set api_key YOUR_API_KEY
# or: export GEMINI_API_KEY=your_api_key_here
```

**ChatGPT:**
```bash
smart-scribe config set backend chatgpt
# Place cookies at ~/.config/smart-scribe/chatgpt-cookies.json
# or: smart-scribe config set chatgpt_cookie_file /path/to/cookies.json
# or: export CHATGPT_COOKIE_FILE=/path/to/cookies.json
```

## Configuration

Config file location:
- Linux/macOS: `~/.config/smart-scribe/config.toml`
- Windows: `%APPDATA%\smart-scribe\config.toml`

Config priority: CLI args > environment > config.toml > defaults
