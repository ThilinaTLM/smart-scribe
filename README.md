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
| `bun` | JavaScript runtime | `yay -S bun-bin` |
| `ffmpeg` | Audio recording (PulseAudio/Pipewire) | `pacman -S ffmpeg` |
| `wl-clipboard` | Clipboard support (`wl-copy`) | `pacman -S wl-clipboard` |
| `xdotool` | Keystroke typing | `pacman -S xdotool` |
| `libnotify` | Desktop notifications (`notify-send`) | `pacman -S libnotify` |

### API Key

Get a Google Gemini API key from [Google AI Studio](https://aistudio.google.com/apikey).

## Installation

```bash
# Clone the repository
git clone <repo-url>
cd smart-scribe-ts

# Install dependencies
bun install

# Create environment file
echo "GEMINI_API_KEY=your_api_key_here" > .env

# Link CLI globally (optional)
bun link
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
kill -SIGUSR1 $(cat /tmp/smart-scribe.pid)   # Start recording
kill -SIGUSR2 $(cat /tmp/smart-scribe.pid)   # Stop and transcribe
kill -SIGINT  $(cat /tmp/smart-scribe.pid)   # Cancel or exit
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
| `-v, --version` | Show version | |

### Daemon Mode

| Option | Description | Default |
|--------|-------------|---------|
| `--daemon` | Run in daemon mode | off |
| `--max-duration <TIME>` | Max recording duration (safety limit) | 60s |

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

## License

MIT
