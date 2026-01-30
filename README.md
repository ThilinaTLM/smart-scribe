```
 ___                      _  ___           _  _
/ __|_ __  __ _ _ _ ___ _| |/ __| __  _ _ |_|| |__   ___
\__ \ '  \/ _` | '_|  _|  _|\__ \/ _|| '_|| || '_ \ / -_)
|___/_|_|_\__,_|_|  \__|_|  |___/\__||_|  |_||_.__/ \___|
```

[![CI](https://github.com/ThilinaTLM/smart-scribe/actions/workflows/ci.yml/badge.svg)](https://github.com/ThilinaTLM/smart-scribe/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/ThilinaTLM/smart-scribe)](https://github.com/ThilinaTLM/smart-scribe/releases)
[![Platforms](https://img.shields.io/badge/platforms-Linux%20|%20macOS%20|%20Windows-blue)]()
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

AI-powered voice-to-text for Linux, macOS, and Windows. Record from your microphone and get accurate, context-aware transcriptions using Google Gemini.

## Install

**Linux / macOS:**

```bash
curl -sSL https://raw.githubusercontent.com/ThilinaTLM/smart-scribe/main/scripts/install.sh | bash
```

**Windows (PowerShell):**

```powershell
irm https://raw.githubusercontent.com/ThilinaTLM/smart-scribe/main/scripts/install.ps1 | iex
```

## Quick Start

1. **Get an API key** from [Google AI Studio](https://aistudio.google.com/apikey)

2. **Configure:**

   ```bash
   smart-scribe config set api_key YOUR_API_KEY
   ```

3. **Record and transcribe:**
   ```bash
   smart-scribe              # 10 second recording
   smart-scribe -d 30s       # 30 second recording
   smart-scribe -d 1m -c     # 1 minute, copy to clipboard
   ```

## Features

- **Voice-to-text** - Record audio and transcribe with Google Gemini
- **Domain presets** - Optimized for dev, medical, legal, finance contexts
- **Clipboard integration** - Copy transcriptions directly (`-c`)
- **Keystroke output** - Type into focused window (`-k`)
- **Desktop notifications** - Get notified when done (`-n`)
- **Daemon mode** - Background service for hotkey integration

### Platform Support

| Feature         |    Linux     |    macOS    |  Windows   |
| --------------- | :----------: | :---------: | :--------: |
| Audio Recording |     cpal     |    cpal     |    cpal    |
| Clipboard       |   arboard    |   arboard   |  arboard   |
| Keystroke       | configurable |   native    |   native   |
| Notifications   | notify-rust  |   native    |   native   |
| Daemon Mode     | Unix socket  | Unix socket | Named pipe |

Linux keystroke: `enigo` (default) or native tools via `--keystroke-tool`

## Usage

### One-Shot Mode

Record for a fixed duration:

```bash
smart-scribe                     # 10s recording, output to stdout
smart-scribe -d 30s              # 30 second recording
smart-scribe -d 1m -c            # 1 minute, copy to clipboard
smart-scribe -d 2m -D dev -k     # 2 minutes, dev domain, type result
smart-scribe -c -k -n            # All outputs: clipboard + keystroke + notify
```

### Daemon Mode

Run as background service (ideal for hotkey integration):

```bash
# Start daemon
smart-scribe --daemon -c -n      # With clipboard + notifications
smart-scribe --daemon -D dev     # With dev domain preset

# Control daemon
smart-scribe daemon toggle       # Start/stop recording
smart-scribe daemon cancel       # Cancel current recording
smart-scribe daemon status       # Show state (idle/recording/processing)
```

Bind `smart-scribe daemon toggle` to a hotkey for push-to-talk.

### Domain Presets

| Domain    | Use Case                                           |
| --------- | -------------------------------------------------- |
| `general` | General conversation (default)                     |
| `dev`     | Software development - code, APIs, technical terms |
| `medical` | Medical/healthcare terminology                     |
| `legal`   | Legal terminology and phrases                      |
| `finance` | Financial terms and acronyms                       |

```bash
smart-scribe -D dev        # Software development context
smart-scribe -D medical    # Medical terminology
```

## Configuration

```bash
smart-scribe config init              # Create config with defaults
smart-scribe config set api_key KEY   # Set API key
smart-scribe config set domain dev    # Set default domain
smart-scribe config list              # Show all settings
smart-scribe config path              # Show config file location
```

**Config file:** `~/.config/smart-scribe/config.toml`

**Priority:** CLI args > environment variables > config file > defaults

### CLI Options

| Option                    | Description                         | Default |
| ------------------------- | ----------------------------------- | ------- |
| `-d, --duration <TIME>`   | Recording duration (10s, 1m, 2m30s) | 10s     |
| `-D, --domain <DOMAIN>`   | Domain preset                       | general |
| `-c, --clipboard`         | Copy to clipboard                   | off     |
| `-k, --keystroke`         | Type into focused window            | off     |
| `--keystroke-tool <TOOL>` | Keystroke tool (Linux only)         | enigo   |
| `-n, --notify`            | Desktop notifications               | off     |
| `--daemon`                | Run in daemon mode                  | off     |
| `--max-duration <TIME>`   | Max recording (daemon safety limit) | 60s     |

<details>
<summary><strong>Platform Notes</strong></summary>

### Linux

Optional dependencies for specific features:

| Feature          | Options (any one)                       |
| ---------------- | --------------------------------------- |
| Keystroke (`-k`) | ydotool, wtype (Wayland), xdotool (X11) |

**Keystroke tool selection:**

By default, SmartScribe uses `enigo` (cross-platform library). On Linux, you can choose a specific tool:

| Tool      | Description                                    |
| --------- | ---------------------------------------------- |
| `enigo`   | Cross-platform library (default)               |
| `auto`    | Auto-detect: ydotool > wtype > xdotool > enigo |
| `ydotool` | Works on both Wayland and X11 (needs daemon)   |
| `wtype`   | Wayland-native                                 |
| `xdotool` | X11-only                                       |

```bash
# Via CLI flag
smart-scribe -k --keystroke-tool auto
smart-scribe -k --keystroke-tool xdotool

# Via config (persistent)
smart-scribe config set linux.keystroke_tool auto
```

**Install keystroke tools:**

```bash
# Arch Linux
sudo pacman -S xdotool   # or ydotool for Wayland

# Ubuntu/Debian
sudo apt install xdotool

# Fedora
sudo dnf install xdotool
```

### macOS

No additional dependencies required. All features use native APIs.

### Windows

No additional dependencies required. All features use native APIs.

</details>

## Building from Source

Requires [Rust](https://rustup.rs/) 1.70+

```bash
git clone https://github.com/ThilinaTLM/smart-scribe.git
cd smart-scribe
cargo build --release
sudo cp target/release/smart-scribe /usr/local/bin/
```

<details>
<summary><strong>Build Dependencies</strong></summary>

| Platform | Dependencies                               |
| -------- | ------------------------------------------ |
| Linux    | `libasound2-dev`, `libxdo-dev`             |
| macOS    | `opus` (via Homebrew: `brew install opus`) |
| Windows  | None                                       |

</details>

## License

MIT
