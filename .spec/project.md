# SmartScribe Project Conventions

## Project Overview

SmartScribe is a CLI tool for AI-powered audio transcription using Google Gemini. Currently implemented in TypeScript/Bun, being rewritten in Rust.

## Technology Stack (Rust Rewrite)

| Component | Technology | Version |
|-----------|------------|---------|
| Language | Rust | 2021 Edition |
| Runtime | Tokio | 1.x |
| HTTP Client | reqwest | 0.12 |
| CLI Parser | clap | 4.x |
| Serialization | serde/serde_json | 1.x |
| Config | toml | 0.8 |
| Error Handling | thiserror | 2.x |
| Terminal UI | colored, indicatif | 2.x, 0.17 |

## Architecture

**Hexagonal (Ports & Adapters)**

```
src/
├── domain/           # Core business logic (value objects, entities, errors)
├── application/      # Use cases and port interfaces (traits)
├── infrastructure/   # Adapter implementations (FFmpeg, Gemini, etc.)
└── cli/              # CLI entry point, parser, presenter, signals
```

## Code Conventions

### Module Organization
- One module per concept (e.g., `domain/recording/duration.rs`)
- Re-export public types from `mod.rs`
- Keep modules focused and cohesive

### Error Handling
- Use `thiserror` for error types
- Return `Result<T, E>` everywhere, no panics in library code
- Domain errors separate from infrastructure errors

### Naming
- Types: PascalCase (`Duration`, `DomainId`)
- Functions/methods: snake_case (`parse_duration`, `as_seconds`)
- Constants: SCREAMING_SNAKE_CASE (`DEFAULT_DURATION`)
- Traits: Named for capability (`AudioRecorder`, `Transcriber`)

### Testing
- Unit tests in same file with `#[cfg(test)] mod tests`
- Integration tests in `tests/` directory
- Use `assert_cmd` for CLI testing
- Use `wiremock` for HTTP mocking

### Documentation
- Rustdoc for public items
- Examples in doc comments for complex APIs

## External Tools

| Tool | Purpose | Install |
|------|---------|---------|
| FFmpeg | Audio recording | `pacman -S ffmpeg` |
| wl-copy | Wayland clipboard | `pacman -S wl-clipboard` |
| xdotool | Keystroke injection | `pacman -S xdotool` |
| notify-send | Desktop notifications | `pacman -S libnotify` |

## Git Workflow

- Feature branches: `feat/rust-{component}`
- Commit convention: `feat(rust): description`
- Squash merge to main when feature complete

## Config File

Location: `~/.config/smart-scribe/config.toml`

```toml
api_key = "your-api-key"
duration = "10s"
max_duration = "60s"
domain = "general"
clipboard = false
keystroke = false
notify = false
```
