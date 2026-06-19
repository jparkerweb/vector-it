# Changelog

All notable changes to VectorIt will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-06-18

### Added

- **Core vectorization pipeline:** decode → quantize → segment → trace → simplify → corners → fit → optimize → export
- **Input formats:** PNG, JPG, BMP, GIF (first frame), TIFF
- **Output formats:** SVG, EPS, PDF, DXF (Spline & Line-only), Bitmap (PNG/BMP/JPEG)
- **Color quantization:** K-means++ in CIE Lab color space with configurable color count (2–256)
- **Anti-aliasing detection:** Sub-pixel edge placement using AA gradient data for smoother curves
- **Image type auto-detection:** Photo, AntiAliased, and Aliased classification via Sobel + entropy analysis
- **Quality presets:** Low, Medium, High with mapped pipeline parameters
- **Wizard UI:** 5-step guided workflow (Image Type → Quality → Color Mode → Review → Export)
- **Palette editor:** Add, remove, reorder colors; quick palette suggestions at multiple color counts
- **Segmentation editor:** Pencil, Eyedropper, Finder, and Zap tools with undo/reset (20-level stack)
- **Three-way view toggle:** Original, Segmentation, and Vector views with keyboard shortcuts (1/2/3)
- **Advanced parameter panel:** Smoothness, corner threshold, color count, AA sensitivity sliders
- **Transparency handling:** Transparent mode (omit alpha regions) or flatten-to-color mode
- **Zoom/pan canvas:** Mouse wheel zoom (0.25×–16×), middle-click/spacebar pan, fit-to-window, 1:1 buttons
- **Progress reporting:** Real-time pipeline stage progress via Tauri events
- **Clipboard paste:** Ctrl+V to paste and vectorize images from clipboard
- **Quick Save:** Ctrl+S for instant re-export with last-used settings
- **Drag-and-drop output:** Drag exported files to other applications via OLE
- **Image resizer:** Auto-downsample large images (>4MP) with pixel-averaging box filter
- **Re-vectorize:** Run pipeline from trace stage onward after segmentation edits
- **Performance benchmarks:** Criterion benchmarks for pipeline at multiple resolutions
- **Memory optimization:** Chunked Lab conversion, streaming boundary tracing, debug-level allocation logging
- **Visual regression tests:** resvg-based pixel comparison with RMSE threshold
- **Property tests:** proptest-based randomized input validation (no panics, valid output)
- **Format integration tests:** Structural validation for SVG, EPS, PDF, DXF exports
- **GitHub Actions CI:** Build workflow triggered on tag push (`v*`)
- **Documentation:** README, CONTRIBUTING, docs/ARCHITECTURE

### Known Limitations

- DXF color mapping limited to 256 ACI indexed colors
- GIF animation not supported (first frame only)
