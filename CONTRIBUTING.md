# Contributing to VectorIt

Thank you for your interest in contributing to VectorIt!

## Development Setup

### Prerequisites

- Rust 1.78+ (`rustup update`)
- Node.js 18+ with npm
- Tauri CLI v2 (`cargo install tauri-cli`)

### Getting Started

```bash
git clone https://github.com/your-username/vector-it.git
cd vector-it

# Install frontend dependencies
cd vectorit-app && npm install && cd ..

# Run in development mode
cd vectorit-app && cargo tauri dev
```

## Architecture

VectorIt is a Rust + Tauri application with two main crates:

- **`vectorit-core`** — Pure Rust library containing the vectorization pipeline, exporters, and batch processing
- **`vectorit-app`** — Tauri desktop shell with React + TypeScript frontend

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the full architecture document.

### Pipeline Data Flow

```
RawImage → Resizer → Quantizer → Segmenter → [Editor] → vtracer → VectorPath[] → Exporter
```

## Testing

```bash
# Run all tests
cd vectorit-core && cargo test

# Run format-specific integration tests
cargo test --test format_tests

# Run visual regression tests
cargo test --test visual_regression

# Run property tests
cargo test --test property_tests

# Run benchmarks
cargo bench
```

### Visual Regression

Golden SVG files are stored in `vectorit-core/tests/snapshots/`. Run `cargo insta review` after updating golden files.

## Code Style

- Follow existing Rust conventions (rustfmt + clippy)
- Use `thiserror` for error types in the core library
- Use `anyhow` at application boundaries (Tauri commands)
- Frontend: TypeScript strict mode, Tailwind CSS utilities

## Pull Request Checklist

- [ ] All existing tests pass (`cargo test`)
- [ ] New functionality includes tests
- [ ] No new clippy warnings (`cargo clippy`)
- [ ] Code formatted with `cargo fmt`
- [ ] Frontend code compiles (`cd vectorit-app && npm run build`)
- [ ] PR description explains the change
- [ ] Screenshots for UI changes
