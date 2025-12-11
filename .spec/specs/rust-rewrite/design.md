# Architecture Design: SmartScribe Rust Rewrite

## Overview

This document captures key architectural decisions and design patterns for the Rust implementation.

## Decision Log

### ADR-1: Hexagonal Architecture

**Context:** Need clean separation between domain logic and external systems (FFmpeg, Gemini, clipboard tools).

**Decision:** Use hexagonal (ports & adapters) architecture with traits as ports and struct implementations as adapters.

**Rationale:**
- Testability: Can mock all external dependencies
- Flexibility: Easy to swap implementations
- Separation: Domain logic stays pure
- Follows TypeScript version's architecture

**Consequences:**
- More boilerplate (trait definitions)
- Clear boundaries between layers
- Easy unit testing with mock adapters

### ADR-2: Async Runtime

**Context:** Need async for HTTP calls (Gemini API), process management (FFmpeg), and signal handling.

**Decision:** Use Tokio as the async runtime.

**Rationale:**
- Industry standard for Rust async
- Required by reqwest (HTTP client)
- Excellent signal handling support
- Multi-threaded runtime for parallel operations

**Configuration:**
```rust
#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() { ... }
```

### ADR-3: Error Handling Strategy

**Context:** Need consistent error handling across all layers without panics.

**Decision:**
- Use `thiserror` for error type derivation
- Layer-specific error types that compose
- `Result<T, E>` everywhere
- No `unwrap()` in library code

**Error Hierarchy:**
```
AppError (top-level)
├── RecordingError (from infrastructure)
├── TranscriptionError (from infrastructure)
├── ConfigError (from domain/infrastructure)
└── DomainError (from domain)
```

### ADR-4: Runtime vs Type-State for Daemon

**Context:** Daemon session needs state machine (IDLE → RECORDING → PROCESSING → IDLE).

**Decision:** Use runtime state machine with enum, not compile-time type-state pattern.

**Rationale:**
- Signal handlers need dynamic state transitions
- Type-state would complicate async code
- Runtime validation is sufficient
- Matches TypeScript implementation behavior

**Implementation:**
```rust
pub struct RuntimeDaemonSession {
    state: DaemonState,
}

impl RuntimeDaemonSession {
    pub fn start_recording(&mut self) -> Result<(), InvalidStateTransition> {
        if self.state != DaemonState::Idle {
            return Err(InvalidStateTransition { ... });
        }
        self.state = DaemonState::Recording;
        Ok(())
    }
}
```

### ADR-5: Signal Handling Approach

**Context:** Need to handle SIGINT (one-shot), SIGUSR1/SIGUSR2 (daemon), SIGTERM.

**Decision:** Use tokio::signal with channels for daemon, atomic bool for one-shot.

**Rationale:**
- Tokio provides cross-platform signal handling
- Channels allow clean async integration
- Atomic bool sufficient for simple shutdown flag

**Patterns:**
```rust
// One-shot: atomic shutdown flag
let shutdown = Arc::new(AtomicBool::new(false));

// Daemon: channel-based signal routing
let (tx, rx) = mpsc::channel(10);
tokio::spawn(async move {
    let mut sig = signal(SignalKind::user_defined1()).unwrap();
    loop {
        sig.recv().await;
        tx.send(DaemonSignal::Toggle).await;
    }
});
```

### ADR-6: FFmpeg Process Management

**Context:** Need to spawn FFmpeg, monitor progress, and gracefully stop.

**Decision:** Use tokio::process with SIGINT for graceful stop, SIGKILL for cancel.

**Rationale:**
- SIGINT tells FFmpeg to finalize file properly
- SIGKILL for cancel when we don't want the file
- Progress via elapsed time tracking (FFmpeg stderr is complex)

**Implementation Notes:**
- Use `kill_on_drop(true)` for cleanup
- Send SIGINT via nix crate for graceful stop
- Track progress with separate timer task

### ADR-7: Configuration Merge Strategy

**Context:** Multiple config sources (CLI, env, file, defaults) need merging.

**Decision:** Explicit merge function with "later wins" semantics.

**Priority (highest to lowest):**
1. CLI arguments
2. Environment variables (`GEMINI_API_KEY`)
3. Config file
4. Hardcoded defaults

**Implementation:**
```rust
impl AppConfig {
    pub fn merge(self, other: Self) -> Self {
        Self {
            api_key: other.api_key.or(self.api_key),
            duration: other.duration.or(self.duration),
            // ... etc
        }
    }
}

// Usage
let config = AppConfig::defaults()
    .merge(file_config)
    .merge(env_config)
    .merge(cli_config);
```

## Patterns

### Pattern: Callback-Based Progress Reporting

Used for recording progress updates.

```rust
pub type ProgressCallback = Box<dyn Fn(u64, u64) + Send + Sync>;

pub async fn record(
    &self,
    duration: Duration,
    on_progress: Option<ProgressCallback>,
) -> Result<AudioData, RecordingError> {
    // Spawn progress reporter
    if let Some(cb) = on_progress {
        tokio::spawn(async move {
            loop {
                cb(elapsed, total);
                sleep(Duration::from_millis(100)).await;
            }
        });
    }
    // ...
}
```

### Pattern: Non-Fatal Actions

Clipboard and keystroke failures shouldn't stop the main flow.

```rust
pub struct TranscribeOutput {
    pub text: String,
    pub clipboard_copied: bool,  // true if succeeded
    pub keystroke_sent: bool,    // true if succeeded
}

// In use case
let clipboard_copied = if input.enable_clipboard {
    clipboard.copy(&text).await.is_ok()
} else {
    false
};
```

### Pattern: Temp File Cleanup with Drop

Ensure temp files are cleaned up even on panic.

```rust
pub struct TempAudioFile {
    path: PathBuf,
}

impl Drop for TempAudioFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
```

### Pattern: Builder for Complex Types

For types with many optional fields.

```rust
impl GeminiTranscriber {
    pub fn builder() -> GeminiTranscriberBuilder {
        GeminiTranscriberBuilder::default()
    }
}

pub struct GeminiTranscriberBuilder {
    api_key: Option<String>,
    model: Option<String>,
    // ...
}

impl GeminiTranscriberBuilder {
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    pub fn build(self) -> Result<GeminiTranscriber, BuilderError> {
        Ok(GeminiTranscriber {
            api_key: self.api_key.ok_or(BuilderError::MissingApiKey)?,
            model: self.model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            // ...
        })
    }
}
```

## Module Dependencies

```
main.rs
    └── cli/app.rs
        ├── cli/args.rs
        ├── cli/presenter.rs
        ├── cli/signals.rs
        └── application/transcribe.rs
            ├── application/ports/recorder.rs
            ├── application/ports/transcriber.rs
            └── domain/*
                └── infrastructure/ffmpeg.rs (impl)
                └── infrastructure/gemini.rs (impl)
```

## Testing Strategy

### Unit Tests
- Domain layer: Pure function tests
- Application layer: Use mock adapters
- Infrastructure layer: Use wiremock for HTTP

### Integration Tests
- CLI tests with assert_cmd
- Real FFmpeg tests (manual, require mic)
- Real Gemini tests (manual, require API key)

### Test Utilities
```rust
// Mock adapter for testing
pub struct MockRecorder {
    pub response: Result<AudioData, RecordingError>,
}

#[async_trait]
impl AudioRecorder for MockRecorder {
    async fn record(&self, _: Duration, _: Option<ProgressCallback>)
        -> Result<AudioData, RecordingError>
    {
        self.response.clone()
    }
}
```

## Performance Considerations

### Binary Size
```toml
[profile.release]
lto = true           # Link-time optimization
codegen-units = 1    # Single codegen unit
strip = true         # Strip symbols
panic = "abort"      # No unwinding
```

Expected binary size: ~5-10 MB (vs ~100+ MB for Bun bundle)

### Startup Time
- No runtime initialization (unlike Node/Bun)
- Expected: <10ms vs ~200ms for TypeScript

### Memory Usage
- No GC overhead
- Predictable memory patterns
- Expected: ~5-10 MB idle (daemon mode)

## Security Considerations

1. **API Key Handling**
   - Never log API keys
   - Read from env/file only
   - Use `secrecy` crate if needed later

2. **Temp Files**
   - Create in /tmp with unique names
   - Clean up on exit (Drop trait)
   - User-only permissions (0600)

3. **Process Execution**
   - No shell interpolation
   - Direct exec of known commands
   - Validate paths/arguments
