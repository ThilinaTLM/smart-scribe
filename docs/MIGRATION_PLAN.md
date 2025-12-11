# TypeScript to Rust Migration Plan

This document outlines the strategy for migrating SmartScribe from TypeScript to Rust within the same repository.

---

## Table of Contents

1. [Migration Strategy Options](#1-migration-strategy-options)
2. [Recommended Approach](#2-recommended-approach)
3. [Repository Structure](#3-repository-structure)
4. [Migration Phases](#4-migration-phases)
5. [Git Workflow](#5-git-workflow)
6. [Version Strategy](#6-version-strategy)
7. [Transition Timeline](#7-transition-timeline)

---

## 1. Migration Strategy Options

### Option A: Parallel Development (Recommended)

```
smart-scribe/
├── src/                    # TypeScript (current)
├── rust/                   # Rust (new) - temporary location
├── package.json            # TypeScript deps
├── Cargo.toml              # Rust deps
└── ...
```

**Pros:**
- Both versions coexist and can be tested
- Easy to compare behavior
- No disruption to current users
- Can gradually validate Rust implementation

**Cons:**
- Temporary complexity in repo structure
- Need to maintain both during transition

### Option B: Branch-Based Development

```
main     → TypeScript (stable)
rust-dev → Rust development
```

**Pros:**
- Clean separation
- Main branch stays stable

**Cons:**
- Hard to compare implementations side-by-side
- Large merge at the end
- Divergence issues if main gets updates

### Option C: New Repository

**Pros:**
- Clean slate
- No legacy baggage

**Cons:**
- Lose git history
- Need to manage two repos
- Users need to find new repo

### Option D: Direct Replacement

Just replace TypeScript with Rust in-place.

**Pros:**
- Simple

**Cons:**
- High risk - no fallback
- Can't compare implementations
- Breaking change for users mid-development

---

## 2. Recommended Approach

**Option A: Parallel Development** is recommended because:

1. **Risk Mitigation** - TypeScript version remains functional throughout
2. **Validation** - Can test both versions against same inputs
3. **Gradual Transition** - Users can try Rust version before it becomes default
4. **Single Repo** - Keeps history, issues, and documentation together

### High-Level Plan

```
Phase 1: Setup Rust alongside TypeScript
Phase 2: Implement Rust version feature-by-feature
Phase 3: Validation and testing
Phase 4: Swap (Rust becomes primary)
Phase 5: Cleanup (remove TypeScript)
```

---

## 3. Repository Structure

### During Migration (Phase 1-3)

```
smart-scribe/
├── .github/
│   └── workflows/
│       ├── ci.yml              # Both TS and Rust CI
│       └── release.yml         # Both releases
│
├── src/                        # TypeScript source (unchanged)
│   ├── domain/
│   ├── application/
│   ├── infrastructure/
│   └── cli/
│
├── rust/                       # Rust source (new)
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── domain/
│   │   ├── application/
│   │   ├── infrastructure/
│   │   └── cli/
│   ├── Cargo.toml
│   └── Cargo.lock
│
├── tests/                      # Shared integration tests
│   ├── fixtures/
│   │   └── test_audio.ogg
│   └── e2e/                    # End-to-end test scripts
│       └── test_transcription.sh
│
├── docs/
│   ├── REQUIREMENTS.md
│   ├── GEMINI_RUST_INTEGRATION.md
│   ├── RUST_ARCHITECTURE.md
│   └── MIGRATION_PLAN.md
│
├── scripts/
│   ├── signal-toggle.sh
│   ├── signal-cancel.sh
│   ├── build-ts.sh             # Build TypeScript
│   ├── build-rust.sh           # Build Rust
│   └── compare-versions.sh     # Compare outputs
│
├── package.json                # TypeScript
├── bun.lockb
├── tsconfig.json
├── biome.json
│
├── Cargo.toml                  # Workspace root (points to rust/)
├── .gitignore                  # Updated for both
├── CLAUDE.md
└── README.md                   # Updated with both versions
```

### Workspace Cargo.toml (Root)

```toml
[workspace]
members = ["rust"]
resolver = "2"
```

### After Migration (Phase 5)

```
smart-scribe/
├── .github/workflows/
├── src/                        # Rust source (moved from rust/src)
│   ├── main.rs
│   ├── lib.rs
│   ├── domain/
│   ├── application/
│   ├── infrastructure/
│   └── cli/
├── tests/
├── docs/
├── scripts/
├── Cargo.toml                  # Direct (not workspace)
├── Cargo.lock
├── .gitignore
├── CLAUDE.md
└── README.md
```

---

## 4. Migration Phases

### Phase 1: Setup (1-2 days)

**Goal:** Establish Rust project structure alongside TypeScript

**Tasks:**
- [ ] Create `rust/` directory with Cargo.toml
- [ ] Setup workspace Cargo.toml at root
- [ ] Add Rust to `.gitignore`
- [ ] Setup CI for both languages
- [ ] Create basic `main.rs` that compiles

**Commit:** `feat: add Rust project structure for migration`

```bash
# Commands
mkdir -p rust/src
cd rust && cargo init --name smart-scribe
# Edit root Cargo.toml for workspace
```

### Phase 2: Domain Layer (2-3 days)

**Goal:** Implement all domain types in Rust

**Tasks:**
- [ ] Duration value object with tests
- [ ] DomainId enum with tests
- [ ] SystemPrompt builder with tests
- [ ] AudioData value object with tests
- [ ] AppConfig with merge logic and tests
- [ ] DaemonSession state machine with tests
- [ ] Domain error types

**Commits:** One per value object/entity
- `feat(rust): add Duration value object`
- `feat(rust): add DomainPreset enum`
- `feat(rust): add SystemPrompt builder`
- etc.

### Phase 3: Application Layer (2-3 days)

**Goal:** Implement ports and use cases

**Tasks:**
- [ ] Define port traits (AudioRecorder, Transcriber, etc.)
- [ ] Implement TranscribeRecordingUseCase
- [ ] Implement DaemonTranscriptionUseCase
- [ ] Unit tests with mock adapters

**Commits:**
- `feat(rust): add port trait definitions`
- `feat(rust): add TranscribeRecordingUseCase`
- `feat(rust): add DaemonTranscriptionUseCase`

### Phase 4: Infrastructure Layer (3-4 days)

**Goal:** Implement all adapters

**Tasks:**
- [ ] FFmpegRecorder adapter
- [ ] GeminiTranscriber adapter
- [ ] WaylandClipboard adapter
- [ ] XdotoolKeystroke adapter
- [ ] NotifySendNotifier adapter
- [ ] XdgConfigStore adapter
- [ ] Integration tests

**Commits:** One per adapter
- `feat(rust): add FFmpeg recorder adapter`
- `feat(rust): add Gemini transcription adapter`
- etc.

### Phase 5: CLI Layer (2-3 days)

**Goal:** Complete CLI implementation

**Tasks:**
- [ ] Clap argument definitions
- [ ] Main app runner
- [ ] Daemon app runner
- [ ] Config subcommand handler
- [ ] Presenter (output formatting)
- [ ] Signal handlers
- [ ] PID file management

**Commits:**
- `feat(rust): add CLI argument parsing`
- `feat(rust): add one-shot mode`
- `feat(rust): add daemon mode`
- `feat(rust): add config commands`

### Phase 6: Validation (2-3 days)

**Goal:** Ensure Rust version matches TypeScript behavior

**Tasks:**
- [ ] Create comparison test script
- [ ] Test all CLI options
- [ ] Test daemon mode signals
- [ ] Test config commands
- [ ] Test error scenarios
- [ ] Performance comparison
- [ ] Fix any discrepancies

**Validation Script:**
```bash
#!/bin/bash
# scripts/compare-versions.sh

echo "Testing one-shot mode..."
TS_OUTPUT=$(bun run src/index.ts -d 5s 2>/dev/null)
RS_OUTPUT=$(./rust/target/release/smart-scribe -d 5s 2>/dev/null)

# Compare outputs, timing, etc.
```

### Phase 7: Swap (1 day)

**Goal:** Make Rust the primary version

**Tasks:**
- [ ] Move `rust/src/*` to `src/`
- [ ] Move `rust/Cargo.toml` to root
- [ ] Update CI/CD for Rust only
- [ ] Update README
- [ ] Update CLAUDE.md
- [ ] Tag release v2.0.0

**Commits:**
- `refactor: make Rust implementation primary`
- `docs: update documentation for Rust version`
- `chore: update CI for Rust`

### Phase 8: Cleanup (1 day)

**Goal:** Remove TypeScript code

**Tasks:**
- [ ] Remove TypeScript source files
- [ ] Remove package.json, bun.lockb, tsconfig.json, biome.json
- [ ] Remove TypeScript from .gitignore
- [ ] Remove `rust/` directory (now empty)
- [ ] Final README cleanup
- [ ] Archive TypeScript version as git tag

**Commits:**
- `chore: archive TypeScript version as v1.x-typescript`
- `chore: remove TypeScript implementation`

---

## 5. Git Workflow

### Branch Strategy

```
main
  │
  ├── feat/rust-setup          # Phase 1
  ├── feat/rust-domain         # Phase 2
  ├── feat/rust-application    # Phase 3
  ├── feat/rust-infrastructure # Phase 4
  ├── feat/rust-cli            # Phase 5
  ├── feat/rust-validation     # Phase 6
  └── feat/rust-swap           # Phase 7-8
```

### Commit Convention

```
feat(rust): description     # New Rust features
fix(rust): description      # Rust bug fixes
test(rust): description     # Rust tests
refactor: description       # Migration refactoring
chore: description          # Build, CI changes
docs: description           # Documentation
```

### Tags

```
v1.0.0                      # Current TypeScript release
v1.1.0                      # Any TS fixes during migration
v1.x-typescript-final       # Archive tag before removal
v2.0.0                      # First Rust release
```

---

## 6. Version Strategy

### Semantic Versioning

| Version | Description |
|---------|-------------|
| v1.x.x | TypeScript implementation |
| v2.0.0 | Rust implementation (breaking: different binary) |
| v2.x.x | Rust improvements |

### Why Major Version Bump?

1. **Different binary** - Compiled Rust vs Bun script
2. **Installation changes** - No Node.js/Bun required
3. **Potential subtle behavior differences**
4. **Clear signal to users**

### Changelog Entry (v2.0.0)

```markdown
## [2.0.0] - 2025-XX-XX

### Changed
- **BREAKING**: Rewritten in Rust for improved performance and standalone binary
- No longer requires Bun/Node.js runtime
- Single static binary distribution

### Added
- Native signal handling
- Faster startup time
- Reduced memory footprint

### Migration
- Binary name unchanged: `smart-scribe`
- All CLI options unchanged
- Config file format unchanged
- Config file location unchanged
```

---

## 7. Transition Timeline

### Estimated Duration: 2-3 weeks

```
Week 1:
├── Day 1-2: Phase 1 (Setup)
├── Day 3-4: Phase 2 (Domain)
└── Day 5-7: Phase 3 (Application)

Week 2:
├── Day 1-3: Phase 4 (Infrastructure)
├── Day 4-5: Phase 5 (CLI)
└── Day 6-7: Phase 6 (Validation)

Week 3:
├── Day 1: Phase 7 (Swap)
├── Day 2: Phase 8 (Cleanup)
└── Day 3: Release v2.0.0
```

### Milestones

| Milestone | Deliverable |
|-----------|-------------|
| M1: Rust Compiles | Basic Rust project that builds |
| M2: Domain Complete | All value objects and entities |
| M3: Use Cases Work | Can transcribe with mock adapters |
| M4: Full Integration | All adapters implemented |
| M5: CLI Complete | Full CLI parity with TypeScript |
| M6: Validated | Passes all comparison tests |
| M7: Released | v2.0.0 published |

---

## Appendix A: Quick Start Commands

### Initial Setup

```bash
# Create Rust project
mkdir -p rust/src
cat > rust/Cargo.toml << 'EOF'
[package]
name = "smart-scribe"
version = "2.0.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full", "signal"] }
reqwest = { version = "0.12", features = ["json"] }
clap = { version = "4", features = ["derive", "env"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
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

[profile.release]
lto = true
codegen-units = 1
strip = true
EOF

# Create workspace Cargo.toml at root
cat > Cargo.toml << 'EOF'
[workspace]
members = ["rust"]
resolver = "2"
EOF

# Create initial main.rs
cat > rust/src/main.rs << 'EOF'
fn main() {
    println!("SmartScribe Rust - Coming Soon!");
}
EOF

# Build to verify setup
cargo build -p smart-scribe
```

### Update .gitignore

```bash
cat >> .gitignore << 'EOF'

# Rust
/target/
/rust/target/
Cargo.lock
EOF
```

### Dual Build Script

```bash
# scripts/build-all.sh
#!/bin/bash
set -e

echo "Building TypeScript..."
bun run build

echo "Building Rust..."
cargo build --release -p smart-scribe

echo "Done!"
echo "TypeScript: dist/smart-scribe"
echo "Rust: rust/target/release/smart-scribe"
```

---

## Appendix B: CI Configuration

### GitHub Actions (Both Languages)

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  typescript:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v1
      - run: bun install
      - run: bun run check
      - run: bun run build

  rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: "rust -> target"
      - run: cargo fmt --check -p smart-scribe
      - run: cargo clippy -p smart-scribe -- -D warnings
      - run: cargo test -p smart-scribe
      - run: cargo build --release -p smart-scribe
```

---

## Appendix C: Decision Log

| Decision | Rationale |
|----------|-----------|
| Same repo | Preserve history, easier comparison |
| Parallel development | Risk mitigation, validation |
| Workspace setup | Clean separation during migration |
| Major version bump | Clear signal of rewrite |
| Rust in `rust/` subdir | Avoids conflicts with `src/` |
| Move to `src/` at end | Standard Rust project layout |
