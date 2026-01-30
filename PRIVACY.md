# Privacy Policy

## Overview

SmartScribe is free and open-source software licensed under the MIT License. This document explains how SmartScribe handles your data.

## Data Collection

**SmartScribe itself does not collect any data.** The application:

- Has no backend servers
- Contains no analytics or telemetry
- Does not transmit data to the SmartScribe project or its contributors
- Stores configuration locally on your machine only

## Third-Party Services

SmartScribe uses the **Google Gemini API** for audio transcription. When you use SmartScribe:

- Audio recordings are sent directly from your machine to Google's API servers
- You provide and manage your own Google API key
- Google may collect and process data according to their terms of service

### Your Responsibilities

Before using SmartScribe, you should:

1. Review [Google's Privacy Policy](https://policies.google.com/privacy)
2. Review the [Google Cloud Terms of Service](https://cloud.google.com/terms)
3. Understand your API key settings and any data retention policies that apply to your Google account

### Disclaimer

The SmartScribe project and its contributors are not responsible for how Google handles data sent through their API. Your use of the Gemini API is governed by your agreement with Google, not with SmartScribe.

## Local Data

SmartScribe stores the following data locally on your machine:

- **Configuration file**: `~/.config/smart-scribe/config.toml` containing your settings and API key
- **Temporary audio files**: Created during recording and deleted after transcription

## Questions

If you have questions about this privacy policy, please open an issue on the [GitHub repository](https://github.com/ThilinaTLM/smart-scribe).
