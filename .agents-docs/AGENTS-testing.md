# Testing
> Part of [AGENTS.md](../AGENTS.md) — project guidance for AI coding agents.

## Test Types

| Type | Command | Location |
|------|---------|----------|
| Unit tests | `cargo test -p vectorit-core` | Inline `#[cfg(test)]` modules |
| Integration tests | `cargo test --test format_tests` | `vectorit-core/tests/` |
| Visual regression | `cargo test --test visual_regression` | `vectorit-core/tests/` |
| Property tests | `cargo test --test property_tests` | `vectorit-core/tests/` |
| Benchmarks | `cargo bench` | `vectorit-core/benches/` |
| Frontend type-check | `npm run build` (from `vectorit-app/`) | TypeScript compilation |

## Visual Regression (insta)

- Golden SVG files stored in `vectorit-core/tests/snapshots/`
- After updating golden files, run `cargo insta review` to accept/reject changes
- Uses the `insta` crate for snapshot testing

## Property Tests

- Uses `proptest` crate for property-based testing
- Tests in `vectorit-core/tests/property_tests.rs`

## Benchmarks

- Uses `criterion` crate with HTML reports
- Benchmark file: `vectorit-core/benches/pipeline_bench.rs`
- Run with: `cargo bench`

## Adding New Tests

- Unit tests: add `#[test]` functions in the relevant module's `#[cfg(test)]` block
- Integration tests: add new `.rs` files in `vectorit-core/tests/`
- Keep test images small (< 100KB) to avoid bloating the repo
