# SmartScribe

AI-powered voice-to-text transcription CLI using Google Gemini.

## Features

- Record audio from microphone via FFmpeg
- Transcribe using Google Gemini with domain-specific context
- Copy transcription to clipboard
- Type transcription directly into focused window
- Desktop notifications
- Daemon mode for signal-based control (hotkey integration)

## Requirements

### System Dependencies

| Package | Purpose | Install (Arch) |
|---------|---------|----------------|
| `ffmpeg` | Audio recording (PulseAudio/Pipewire) | `pacman -S ffmpeg` |
| `wl-clipboard` | Clipboard support (`wl-copy`) | `pacman -S wl-clipboard` |
| `xdotool` | Keystroke typing | `pacman -S xdotool` |
| `libnotify` | Desktop notifications (`notify-send`) | `pacman -S libnotify` |

### API Key

Get a Google Gemini API key from [Google AI Studio](https://aistudio.google.com/apikey).

## Installation

### Download Binary

Download the latest release from [GitHub Releases](../../releases):

```bash
# Download and make executable
chmod +x smart-scribe-linux-x86_64
sudo mv smart-scribe-linux-x86_64 /usr/local/bin/smart-scribe
```

### From Source

```bash
# Clone the repository
git clone <repo-url>
cd smart-scribe

# Build release binary
cargo build --release

# Install to PATH
sudo cp target/release/smart-scribe /usr/local/bin/
```

### Configuration

Configure the API key via config file or environment:

```bash
# Option 1: Use config command
smart-scribe config init
smart-scribe config set api_key YOUR_API_KEY

# Option 2: Environment variable
export GEMINI_API_KEY=your_api_key_here

# Option 3: Create config file manually
mkdir -p ~/.config/smart-scribe
echo 'api_key = "your_api_key_here"' > ~/.config/smart-scribe/config.toml
```

## Usage

### One-Shot Mode

Record for a fixed duration, transcribe, and exit:

```bash
smart-scribe                     # 10s recording, output to stdout
smart-scribe -d 30s              # 30 second recording
smart-scribe -d 1m -c            # 1 minute, copy to clipboard
smart-scribe -d 2m -D dev -k     # 2 minutes, dev domain, type into window
smart-scribe -c -k -n            # Clipboard + keystroke + notifications
```

### Daemon Mode

Run as a background process, controlled by signals:

```bash
smart-scribe --daemon -c -n      # Start daemon with clipboard + notifications
smart-scribe --daemon -D dev     # Daemon with dev domain
smart-scribe --daemon --max-duration 5m  # 5 minute max recording
```

Control the daemon with signals:

```bash
kill -SIGUSR1 $(cat /tmp/smart-scribe.pid)   # Toggle (start or stop+transcribe)
kill -SIGUSR2 $(cat /tmp/smart-scribe.pid)   # Cancel recording
kill -SIGINT  $(cat /tmp/smart-scribe.pid)   # Exit daemon
```

Or use the helper scripts (useful for binding to global hotkeys):

```bash
./scripts/signal-toggle.sh   # Toggle recording
./scripts/signal-cancel.sh   # Cancel recording
```

## CLI Options

### One-Shot Mode

| Option | Description | Default |
|--------|-------------|---------|
| `-d, --duration <TIME>` | Recording duration (10s, 1m, 2m30s) | 10s |
| `-D, --domain <DOMAIN>` | Domain preset | general |
| `-c, --clipboard` | Copy transcription to clipboard | off |
| `-k, --keystroke` | Type transcription into focused window | off |
| `-n, --notify` | Show desktop notifications | off |
| `-h, --help` | Show help | |
| `-V, --version` | Show version | |

### Daemon Mode

| Option | Description | Default |
|--------|-------------|---------|
| `--daemon` | Run in daemon mode | off |
| `--max-duration <TIME>` | Max recording duration (safety limit) | 60s |

### Config Commands

```bash
smart-scribe config init              # Create config file with defaults
smart-scribe config set <key> <value> # Set a config value
smart-scribe config get <key>         # Get a config value
smart-scribe config list              # List all config values
smart-scribe config path              # Show config file path
```

Config keys: `api_key`, `duration`, `max_duration`, `domain`, `clipboard`, `keystroke`, `notify`

## Domain Presets

| Domain | Description |
|--------|-------------|
| `general` | General conversation (default) |
| `dev` | Software engineering terminology |
| `medical` | Medical/healthcare terms |
| `legal` | Legal terminology |
| `finance` | Financial terms and acronyms |

## Output

- Transcribed text is written to **stdout**
- Status messages are written to **stderr**
- Use `-c` to copy to clipboard, `-k` to type into focused window

## Development

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo test               # Run tests
cargo clippy             # Lint
cargo fmt                # Format
```

## Releasing

To create a new release:

```bash
git tag v2.0.0
git push origin v2.0.0
```

GitHub Actions will automatically build the binary and create a release.

## License

MIT
