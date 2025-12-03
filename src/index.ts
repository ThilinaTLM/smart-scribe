#!/usr/bin/env bun
/**
 * SmartScribe - AI-powered voice to text transcription CLI
 *
 * Uses Google Gemini for transcription with domain-specific prompts.
 * Records from microphone using FFmpeg and outputs to stdout + clipboard.
 */

import { App } from "./cli/app"

const app = new App()
const exitCode = await app.run(Bun.argv.slice(2))
process.exit(exitCode)
