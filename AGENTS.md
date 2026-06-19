# AGENTS.md

This file provides guidance to AI coding agents like Claude Code (claude.ai/code), Cursor AI, Codex, Gemini CLI, GitHub Copilot, Devin, Zed, and other AI coding assistants when working with code in this repository.

## Project Overview

VectorIt is an open-source desktop application that converts raster images (PNG, JPG, BMP, GIF, TIFF) into high-quality vector graphics (SVG, EPS, PDF, DXF). Built as a Rust workspace with two crates: `vectorit-core` (pure vectorization engine) and `vectorit-app` (Tauri v2 desktop shell with React + TypeScript frontend). The pipeline: Decode → Resize → Quantize → Segment → [Edit] → Trace → Export.

## Git Commit Messages

All commits must follow this format:

```
<description>

<JIRA-Ticket-ID>
AI Assisted
```

Derive the JIRA ticket ID from the current branch name — the format is `<prefix>/<TICKET-ID>-description` where the ticket ID is an uppercase project key, hyphen, and integer (e.g., `PCWEB-10968`). The ticket ID and `AI Assisted` go on consecutive lines after a blank line.

## How to Use This File

The sections below contain brief summaries. Follow the markdown links to `.agents-docs/` for full details — only read what's relevant to your current task. This keeps context windows small and focused.

## Development Commands

Build, test, lint, and run the application in development mode.

Details: [Development Commands](./.agents-docs/AGENTS-development-commands.md)

## Architecture

Rust workspace with a pipeline-based vectorization engine and Tauri IPC command layer connecting to a React frontend.

Details: [Architecture](./.agents-docs/AGENTS-architecture.md)

## Code Style & Conventions

Rust formatting/clippy conventions, TypeScript strict mode, error handling patterns, and crate boundaries.

Details: [Code Style & Conventions](./.agents-docs/AGENTS-code-style.md)

## Testing

Unit tests, integration tests, visual regression (insta snapshots), property tests, and benchmarks.

Details: [Testing](./.agents-docs/AGENTS-testing.md)
