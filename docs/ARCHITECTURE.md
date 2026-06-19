# VectorIt Architecture

## Overview

VectorIt is a desktop application for converting raster images to vector graphics. It uses a **pipeline architecture** with a **command pattern** for Tauri IPC.

## System Components

```
┌─────────────────────────────────────────────────────┐
│                   Tauri Shell                        │
│  ┌──────────────┐         ┌──────────────────────┐  │
│  │  React + TS  │◄─IPC──►│  Rust Backend         │  │
│  │  Frontend    │         │  (commands.rs)        │  │
│  └──────────────┘         └──────────┬───────────┘  │
│                                      │              │
│                            ┌─────────▼──────────┐   │
│                            │  vectorit-core      │   │
│                            │  (pure library)     │   │
│                            └────────────────────┘   │
└─────────────────────────────────────────────────────┘
```

## Pipeline Data Flow

```
Input Image (PNG/JPG/BMP/GIF/TIFF)
         │
         ▼
    ┌──────────┐
    │ Decoder  │  image crate → RawImage (RGBA pixels)
    └────┬─────┘
         │
         ▼
    ┌──────────┐
    │ Resizer  │  Box-filter downsample if > 4MP (auto_resize)
    └────┬─────┘
         │
         ▼
    ┌────────────┐
    │ Quantizer  │  K-means++ in CIE Lab → Palette + labels
    └────┬───────┘
         │
         ▼
    ┌────────────┐
    │ Segmenter  │  Connected-component flood fill → Regions
    └────┬───────┘
         │
         ▼ (optional)
    ┌────────────┐
    │  Editor    │  Manual region corrections (paint/split/merge)
    └────┬───────┘
         │
         ▼
    ┌────────────┐
    │  vtracer   │  Vectorization: tracing + curve fitting
    └────┬───────┘
         │
         ▼
    ┌────────────┐
    │  Exporter  │  SVG / EPS / PDF / DXF / Bitmap
    └────────────┘
```

## Module Dependency Graph

```
types.rs ◄──── (all modules depend on types)
    │
    ├── decoder.rs          Image decoding (image crate)
    ├── resizer.rs          Pixel-averaging downsample
    ├── quantizer.rs        K-means++ quantization (palette crate)
    ├── segmenter.rs        Connected-component labeling
    ├── detector.rs         Image type detection
    ├── aa_detector.rs      Anti-aliasing detection
    ├── subpixel.rs         Sub-pixel edge refinement
    ├── editor.rs           Segmentation editing
    ├── batch.rs            Batch processing orchestrator
    │
    ├── pipeline.rs ◄───── Orchestrates all above stages
    │
    └── export/
        ├── svg.rs          SVG 1.1 export
        ├── eps.rs          EPS Level 3 export
        ├── pdf.rs          PDF export (pdf-writer)
        ├── dxf.rs          DXF Spline & Polyline (dxf-rs)
        └── bitmap.rs       PNG/BMP/JPEG rasterization
```

## Tauri IPC Protocol

All communication between frontend and backend uses Tauri's command system:

| Command | Direction | Purpose |
|---------|-----------|---------|
| `load_image` | FE → BE | Decode image, return metadata + thumbnail |
| `detect_type` | FE → BE | Classify image type |
| `vectorize` | FE → BE | Run full pipeline, emit `progress` events |
| `suggest_palette` | FE → BE | Generate palette suggestions |
| `export_svg/eps/pdf/dxf` | FE → BE | Export to file |
| `export_bitmap` | FE → BE | Rasterize vectors to bitmap |
| `batch_vectorize` | FE → BE | Process multiple images |
| `paste_from_clipboard` | FE → BE | Read clipboard image |
| `paste_image` | FE → BE | Save pasted PNG bytes |
| `init_editor` | FE → BE | Initialize segmentation editor |
| `apply_edit` | FE → BE | Apply paint/split/merge edit |
| `undo_edit` | FE → BE | Undo last edit |
| `re_vectorize` | FE → BE | Re-vectorize from edited segmentation |
| `get_documents_dir` | FE → BE | Get user's Documents path |
| `start_drag` | FE → BE | Validate file for drag-and-drop |

### Events (BE → FE)

| Event | Payload | Purpose |
|-------|---------|---------|
| `progress` | `{ stage, percent }` | Pipeline progress updates |
| `batch_item_complete` | `{ index, total, current_file }` | Batch item progress |

## Error Handling

- **Core library**: Uses `VectorItError` enum with `thiserror` for typed errors
- **Tauri boundary**: Errors are converted to `String` via `.map_err(|e| e.to_string())`
- **Frontend**: Errors displayed via the error banner in the toolbar

## Key Design Decisions

1. **vtracer integration**: Uses the `vtracer` crate for the actual vectorization (tracing + curve fitting) rather than a custom implementation. This provides battle-tested vectorization.

2. **Pipeline + vtracer hybrid**: Our pipeline handles pre-processing (decode, resize, quantize, segment) and post-processing (export), while vtracer handles the core vectorization.

3. **Sequential batch processing**: Batch mode processes images one at a time (single-threaded) to maintain predictable memory usage.

4. **Auto-resize over rejection**: Large images are automatically downsampled to 4MP for analysis rather than being rejected, preserving the original dimensions for export.

5. **Persist middleware for settings**: Quick Save settings use Zustand's persist middleware to remember last export format/directory across sessions.
