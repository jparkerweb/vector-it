# Architecture
> Part of [AGENTS.md](../AGENTS.md) — project guidance for AI coding agents.

## Workspace Structure

Rust workspace with two crates:

| Crate | Path | Role |
|-------|------|------|
| `vectorit-core` | `vectorit-core/` | Pure Rust library — vectorization engine, no UI dependencies |
| `vectorit-app` | `vectorit-app/src-tauri/` | Tauri v2 desktop shell — thin wrapper calling `vectorit-core` |

Frontend (React + TS) lives in `vectorit-app/src/`.

## Pipeline Data Flow

```
Input Image → Decoder → Resizer → Quantizer → Segmenter → [Editor] → Tracer → Exporter
```

- **Decoder** (`decoder.rs`): image crate → `RawImage` (RGBA pixels)
- **Resizer** (`resizer.rs`): Box-filter downsample if > 4MP
- **Quantizer** (`quantizer.rs`): K-means++ in CIE Lab color space → palette + labels
- **Segmenter** (`segmenter.rs`): Connected-component flood fill → regions
- **Editor** (`editor.rs`): Optional manual region corrections
- **Tracer/vtracer**: Boundary tracing + Bézier curve fitting → vector paths
- **Exporters** (`export/`): SVG, EPS, PDF, DXF, Bitmap

## Core Module Map

```
types.rs        ← shared types (all modules depend on this)
pipeline.rs     ← orchestrates all pipeline stages
decoder.rs      ← image format decoding
resizer.rs      ← pixel-averaging downsample
quantizer.rs    ← K-means++ color quantization
segmenter.rs    ← connected-component labeling
detector.rs     ← image type classification
aa_detector.rs  ← anti-aliasing detection
subpixel.rs     ← sub-pixel edge refinement
editor.rs       ← segmentation editing operations
tracer.rs       ← path tracing
fitter.rs       ← curve fitting
simplifier.rs   ← path simplification
optimizer.rs    ← path optimization
corner.rs       ← corner detection
export/         ← format-specific exporters (svg, eps, pdf, dxf, bitmap)
```

## Tauri IPC Commands

Frontend ↔ Backend communication via Tauri commands in `vectorit-app/src-tauri/src/commands.rs`:

| Command | Purpose |
|---------|---------|
| `load_image` | Decode image, return metadata + thumbnail |
| `vectorize` | Run full pipeline, emit `progress` events |
| `export_svg/eps/pdf/dxf` | Export to file |
| `export_bitmap` | Rasterize vectors to bitmap |
| `batch_vectorize` | Process multiple images |
| `paste_from_clipboard` | Read clipboard image |
| `init_editor` / `apply_edit` / `undo_edit` | Segmentation editor |

Backend → Frontend events: `progress` (stage + percent), `batch_item_complete`.

## Frontend Architecture

- **Framework**: React 18 + TypeScript (strict mode)
- **State**: Zustand stores in `vectorit-app/src/stores/`
- **Styling**: Tailwind CSS v4 (utility-first)
- **Bundler**: Vite with hot module reload
- **Entry**: `App.tsx` is the main component

## Key Design Decisions

1. **vtracer for vectorization**: Uses the `vtracer` crate rather than custom tracing
2. **Pipeline + vtracer hybrid**: Our pipeline handles pre/post-processing, vtracer handles core vectorization
3. **Sequential batch processing**: One image at a time for predictable memory usage
4. **Auto-resize over rejection**: Large images downsampled to 4MP rather than rejected
5. **Zustand persist middleware**: Quick Save settings remembered across sessions
