# Code Style & Conventions
> Part of [AGENTS.md](../AGENTS.md) — project guidance for AI coding agents.

## Rust

- Format with `cargo fmt` (default rustfmt settings)
- Lint with `cargo clippy` — no warnings allowed
- Use `thiserror` for error types in `vectorit-core` (typed `VectorItError` enum)
- Use `anyhow` at application boundaries (Tauri commands in `vectorit-app`)
- Errors cross the IPC boundary via `.map_err(|e| e.to_string())`
- All shared types live in `vectorit-core/src/types.rs`
- Keep `vectorit-core` as a pure library — no Tauri/UI dependencies

## TypeScript / Frontend

- TypeScript strict mode enabled
- Tailwind CSS v4 utilities for styling (no custom CSS unless necessary)
- Zustand for state management (stores in `src/stores/`)
- Component files in `src/components/`

## Crate Boundaries

- `vectorit-core` must never depend on Tauri or any UI framework
- `vectorit-app/src-tauri` is a thin wrapper: deserializes IPC args, calls core, serializes results
- New vectorization logic belongs in `vectorit-core`, not in the app crate

## Dependencies

- Workspace dependencies defined in root `Cargo.toml` — use `{ workspace = true }` in member crates
- Frontend deps managed via `vectorit-app/package.json`
