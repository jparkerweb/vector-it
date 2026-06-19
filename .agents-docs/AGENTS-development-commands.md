# Development Commands
> Part of [AGENTS.md](../AGENTS.md) — project guidance for AI coding agents.

## Prerequisites

- Rust 1.78+ (rustup)
- Node.js 20+ with npm
- Tauri CLI v2: `cargo install tauri-cli`
- Windows: Visual Studio Build Tools with C++ workload

## Running the App (Dev Mode)

```powershell
cd vectorit-app
npm install          # first time only
cargo tauri dev      # starts Vite dev server + compiles Rust + opens window
```

First build takes 2–5 min (compiles ~200+ crates). Subsequent incremental builds: 5–15s.

**Important:** `cargo tauri dev` must run from `vectorit-app/`, NOT the workspace root.

## Building for Production

```powershell
cd vectorit-app
npm install
cargo tauri build
```

Output: `vectorit-app/src-tauri/target/release/bundle/nsis/VectorIt_*_x64-setup.exe`

## Versioning

The app version appears in three files that must stay in sync:

- `CHANGELOG.md` — source of truth for release history
- `vectorit-app/src-tauri/tauri.conf.json` — `"version"` field (drives installer name, e.g., `VectorIt_1.0.0_x64-setup.exe`)
- `vectorit-app/package.json` — `"version"` field

When adding a new version to `CHANGELOG.md`, update both `tauri.conf.json` and `package.json` to match.

## Core Library Only (No UI)

```powershell
# From workspace root
cargo test --workspace          # all tests
cargo test -p vectorit-core     # core engine only
cargo clippy --workspace        # lint all
cargo fmt --all -- --check      # format check
```

## Frontend Only

```powershell
cd vectorit-app
npm run build       # TypeScript type-check + Vite production build
npm run dev         # Vite dev server only (no Rust backend)
```

## Single Test

```powershell
cargo test test_name_here               # run a specific test by name
cargo test --test format_tests          # run a specific test file
```

## Benchmarks

```powershell
cd vectorit-core
cargo bench
```

## Clean Build Cache

```powershell
cargo clean
```
