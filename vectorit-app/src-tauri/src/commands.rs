use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;
use tauri::Emitter;
use vectorit_core::{decoder, detector, editor, quantizer, export::svg::export_svg as write_svg, export::eps::export_eps as write_eps, export::pdf::export_pdf as write_pdf, export::dxf::export_dxf_spline as write_dxf_spline, export::dxf::export_dxf_polyline as write_dxf_polyline, pipeline};
use vectorit_core::editor::SegmentationEditor;
use vectorit_core::pipeline::PipelineProgress;
use vectorit_core::types::{SegEdit, Segmentation, VectorizationConfig, VectorizationResult};

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub has_alpha: bool,
    pub file_size_bytes: u64,
    pub thumbnail_base64: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaletteSuggestion {
    pub count: u16,
    pub colors: Vec<String>,
    pub quality_score: f64,
}

#[tauri::command]
pub fn load_image(path: String) -> std::result::Result<ImageInfo, String> {
    let file_path = Path::new(&path);
    let metadata = std::fs::metadata(file_path).map_err(|e| e.to_string())?;
    let raw = decoder::decode_image(file_path).map_err(|e| e.to_string())?;

    // Build thumbnail from already-decoded pixels (avoid re-reading the file)
    let flat_pixels: Vec<u8> = raw.pixels.iter().flat_map(|p| p.iter().copied()).collect();
    let buffer = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(
        raw.width, raw.height, flat_pixels,
    ).ok_or("Failed to create image buffer")?;
    let img = image::DynamicImage::from(buffer);
    let thumbnail = if raw.width > 200 {
        img.thumbnail(200, 200)
    } else {
        img
    };

    let mut png_bytes = Vec::new();
    thumbnail
        .write_to(
            &mut std::io::Cursor::new(&mut png_bytes),
            image::ImageFormat::Png,
        )
        .map_err(|e| e.to_string())?;

    use base64::Engine;
    let thumbnail_base64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);

    Ok(ImageInfo {
        width: raw.width,
        height: raw.height,
        has_alpha: raw.has_alpha,
        file_size_bytes: metadata.len(),
        thumbnail_base64,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SvgLoadResult {
    pub svg_content: String,
    pub vector_result: VectorizationResult,
}

/// Load an SVG file directly without rasterizing. Returns the raw SVG content and a parsed VectorizationResult.
#[tauri::command]
pub fn load_svg_file(path: String) -> std::result::Result<SvgLoadResult, String> {
    let svg_content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let vector_result = parse_svg_to_vector_result(&svg_content)?;
    Ok(SvgLoadResult { svg_content, vector_result })
}

/// Parse raw SVG content (e.g. from clipboard) into a VectorizationResult.
#[tauri::command]
pub fn parse_svg_content(svg_content: String) -> std::result::Result<SvgLoadResult, String> {
    let vector_result = parse_svg_to_vector_result(&svg_content)?;
    Ok(SvgLoadResult { svg_content, vector_result })
}

/// Parse an SVG string into a VectorizationResult by extracting paths via usvg.
fn parse_svg_to_vector_result(svg_content: &str) -> std::result::Result<VectorizationResult, String> {
    use vectorit_core::types::{VectorPath, RgbColor, Palette, LabColor, Segmentation};
    use resvg::usvg::{self, tiny_skia_path::PathSegment};

    let tree = usvg::Tree::from_data(
        svg_content.as_bytes(),
        &usvg::Options::default(),
    ).map_err(|e| format!("Failed to parse SVG: {e}"))?;

    let size = tree.size();
    let width = size.width().ceil() as u32;
    let height = size.height().ceil() as u32;

    let mut paths = Vec::new();

    fn extract_paths(
        node: &usvg::Node,
        paths: &mut Vec<VectorPath>,
    ) {
        use vectorit_core::types::{Point, CubicBezier, BezierSegment, VectorPath, RgbColor};

        match node {
            usvg::Node::Path(p) => {
                let transform = p.abs_transform();

                // Extract fill color
                let fill_color = if let Some(ref fill) = p.fill() {
                    match fill.paint() {
                        usvg::Paint::Color(c) => RgbColor { r: c.red, g: c.green, b: c.blue },
                        _ => RgbColor { r: 0, g: 0, b: 0 },
                    }
                } else {
                    // No fill — check stroke
                    if let Some(ref stroke) = p.stroke() {
                        match stroke.paint() {
                            usvg::Paint::Color(c) => RgbColor { r: c.red, g: c.green, b: c.blue },
                            _ => RgbColor { r: 0, g: 0, b: 0 },
                        }
                    } else {
                        return; // No visible paint
                    }
                };

                // Convert path segments to our bezier format
                let mut segments = Vec::new();
                let mut current = Point::new(0.0, 0.0);
                let mut first_point = Point::new(0.0, 0.0);
                let mut is_closed = false;

                for seg in p.data().segments() {
                    match seg {
                        PathSegment::MoveTo(pt) => {
                            let (tx, ty) = transform_point(pt.x as f64, pt.y as f64, &transform);
                            current = Point::new(tx, ty);
                            first_point = current;
                        }
                        PathSegment::LineTo(pt) => {
                            let (tx, ty) = transform_point(pt.x as f64, pt.y as f64, &transform);
                            let end = Point::new(tx, ty);
                            // Line as a degenerate cubic bezier
                            let p1 = Point::new(
                                current.x + (end.x - current.x) / 3.0,
                                current.y + (end.y - current.y) / 3.0,
                            );
                            let p2 = Point::new(
                                current.x + 2.0 * (end.x - current.x) / 3.0,
                                current.y + 2.0 * (end.y - current.y) / 3.0,
                            );
                            segments.push(BezierSegment {
                                curve: CubicBezier { p0: current, p1, p2, p3: end },
                                is_corner_start: true,
                            });
                            current = end;
                        }
                        PathSegment::QuadTo(p1, pt) => {
                            let (t1x, t1y) = transform_point(p1.x as f64, p1.y as f64, &transform);
                            let (tx, ty) = transform_point(pt.x as f64, pt.y as f64, &transform);
                            let end = Point::new(tx, ty);
                            let qp = Point::new(t1x, t1y);
                            // Convert quadratic to cubic
                            let cp1 = Point::new(
                                current.x + 2.0 / 3.0 * (qp.x - current.x),
                                current.y + 2.0 / 3.0 * (qp.y - current.y),
                            );
                            let cp2 = Point::new(
                                end.x + 2.0 / 3.0 * (qp.x - end.x),
                                end.y + 2.0 / 3.0 * (qp.y - end.y),
                            );
                            segments.push(BezierSegment {
                                curve: CubicBezier { p0: current, p1: cp1, p2: cp2, p3: end },
                                is_corner_start: false,
                            });
                            current = end;
                        }
                        PathSegment::CubicTo(c1, c2, pt) => {
                            let (t1x, t1y) = transform_point(c1.x as f64, c1.y as f64, &transform);
                            let (t2x, t2y) = transform_point(c2.x as f64, c2.y as f64, &transform);
                            let (tx, ty) = transform_point(pt.x as f64, pt.y as f64, &transform);
                            let end = Point::new(tx, ty);
                            segments.push(BezierSegment {
                                curve: CubicBezier {
                                    p0: current,
                                    p1: Point::new(t1x, t1y),
                                    p2: Point::new(t2x, t2y),
                                    p3: end,
                                },
                                is_corner_start: false,
                            });
                            current = end;
                        }
                        PathSegment::Close => {
                            if current.x != first_point.x || current.y != first_point.y {
                                let p1 = Point::new(
                                    current.x + (first_point.x - current.x) / 3.0,
                                    current.y + (first_point.y - current.y) / 3.0,
                                );
                                let p2 = Point::new(
                                    current.x + 2.0 * (first_point.x - current.x) / 3.0,
                                    current.y + 2.0 * (first_point.y - current.y) / 3.0,
                                );
                                segments.push(BezierSegment {
                                    curve: CubicBezier { p0: current, p1, p2, p3: first_point },
                                    is_corner_start: true,
                                });
                            }
                            is_closed = true;
                        }
                    }
                }

                if !segments.is_empty() {
                    paths.push(VectorPath {
                        segments,
                        fill_color,
                        is_closed,
                        stroke_color: None,
                        stroke_width: None,
                    });
                }
            }
            usvg::Node::Group(g) => {
                for child in g.children() {
                    extract_paths(child, paths);
                }
            }
            _ => {}
        }
    }

    fn transform_point(x: f64, y: f64, t: &usvg::Transform) -> (f64, f64) {
        let (sx, ky, kx, sy, tx, ty) = (
            t.sx as f64, t.ky as f64, t.kx as f64,
            t.sy as f64, t.tx as f64, t.ty as f64,
        );
        (sx * x + kx * y + tx, ky * x + sy * y + ty)
    }

    let root = tree.root();
    for child in root.children() {
        extract_paths(child, &mut paths);
    }

    // Build a minimal palette from unique colors
    let mut unique_colors: Vec<RgbColor> = Vec::new();
    for p in &paths {
        if !unique_colors.iter().any(|c| c.r == p.fill_color.r && c.g == p.fill_color.g && c.b == p.fill_color.b) {
            unique_colors.push(p.fill_color);
        }
    }
    let palette_colors: Vec<LabColor> = unique_colors.iter().map(|rgb| {
        use palette::{FromColor, Lab, Srgb};
        let srgb = Srgb::new(rgb.r as f32 / 255.0, rgb.g as f32 / 255.0, rgb.b as f32 / 255.0);
        let lab: Lab = Lab::from_color(srgb);
        LabColor { l: lab.l, a: lab.a, b: lab.b }
    }).collect();

    // Build minimal segmentation (empty — not applicable for SVG imports)
    let segmentation = Segmentation {
        regions: Vec::new(),
        label_map: Vec::new(),
        width,
        height,
    };

    Ok(VectorizationResult {
        paths,
        palette: Palette { colors: palette_colors },
        dimensions: (width, height),
        segmentation,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PasteResult {
    pub path: String,
    pub info: ImageInfo,
}

/// Accept raw PNG bytes from a clipboard paste, save to a temp file, and return path + info.
#[tauri::command]
pub fn paste_image(png_data: Vec<u8>) -> std::result::Result<PasteResult, String> {
    // Save to a temp file
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join("vectorit_paste.png");
    std::fs::write(&temp_path, &png_data).map_err(|e| format!("Failed to write temp file: {}", e))?;

    let path_str = temp_path.to_string_lossy().to_string();
    let info = load_image(path_str.clone())?;

    Ok(PasteResult { path: path_str, info })
}

#[tauri::command]
pub fn detect_type(path: String) -> std::result::Result<detector::DetectionResult, String> {
    let raw = decoder::decode_image(Path::new(&path)).map_err(|e| e.to_string())?;
    Ok(detector::detect_image_type(&raw))
}

#[tauri::command]
pub fn vectorize(
    path: String,
    config: VectorizationConfig,
    app_handle: tauri::AppHandle,
) -> std::result::Result<VectorizationResult, String> {
    let raw = decoder::decode_image(Path::new(&path)).map_err(|e| e.to_string())?;

    let progress_callback = move |progress: PipelineProgress| {
        let _ = app_handle.emit("progress", &progress);
    };

    pipeline::vectorize_with_progress(raw, &config, None, Some(&progress_callback))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn suggest_palette(
    path: String,
    max_colors: u16,
) -> std::result::Result<Vec<PaletteSuggestion>, String> {
    let raw = decoder::decode_image(Path::new(&path)).map_err(|e| e.to_string())?;

    let counts = [2, 4, 6, 8, 12, 16]
        .iter()
        .copied()
        .filter(|&c| c <= max_colors)
        .collect::<Vec<_>>();

    let mut suggestions = Vec::new();

    for count in counts {
        let quantized = quantizer::quantize(&raw, count).map_err(|e| e.to_string())?;
        let rgb_colors = quantized.palette.to_rgb();

        // Compute quality score: ratio of explained color variance
        let quality_score = compute_palette_quality_score(&raw, &quantized);

        suggestions.push(PaletteSuggestion {
            count,
            colors: rgb_colors.iter().map(|c| c.to_hex()).collect(),
            quality_score,
        });
    }

    Ok(suggestions)
}

fn compute_palette_quality_score(
    image: &vectorit_core::types::RawImage,
    quantized: &vectorit_core::types::QuantizedImage,
) -> f64 {
    use palette::{FromColor, Lab, Srgb};

    // Compute total variance of original colors
    let mut total_error = 0.0f64;
    let mut total_variance = 0.0f64;

    let palette_lab: Vec<Lab> = quantized
        .palette
        .colors
        .iter()
        .map(|c| Lab::new(c.l, c.a, c.b))
        .collect();

    // Compute mean color
    let mut mean_l = 0.0f64;
    let mut mean_a = 0.0f64;
    let mut mean_b = 0.0f64;
    let n = image.pixels.len() as f64;

    for pixel in &image.pixels {
        let srgb = Srgb::new(
            pixel[0] as f32 / 255.0,
            pixel[1] as f32 / 255.0,
            pixel[2] as f32 / 255.0,
        );
        let lab: Lab = Lab::from_color(srgb);
        mean_l += lab.l as f64;
        mean_a += lab.a as f64;
        mean_b += lab.b as f64;
    }
    mean_l /= n;
    mean_a /= n;
    mean_b /= n;

    // Compute total variance and quantization error
    for (i, pixel) in image.pixels.iter().enumerate() {
        let srgb = Srgb::new(
            pixel[0] as f32 / 255.0,
            pixel[1] as f32 / 255.0,
            pixel[2] as f32 / 255.0,
        );
        let lab: Lab = Lab::from_color(srgb);

        // Variance from mean
        let dl = lab.l as f64 - mean_l;
        let da = lab.a as f64 - mean_a;
        let db = lab.b as f64 - mean_b;
        total_variance += dl * dl + da * da + db * db;

        // Error from quantized palette
        let label = quantized.labels[i] as usize;
        if label < palette_lab.len() {
            let pal = &palette_lab[label];
            let el = lab.l - pal.l;
            let ea = lab.a - pal.a;
            let eb = lab.b - pal.b;
            total_error += (el * el + ea * ea + eb * eb) as f64;
        }
    }

    if total_variance < 1e-6 {
        return 1.0;
    }

    // Quality = 1 - (error / variance), clamped to [0, 1]
    (1.0 - total_error / total_variance).clamp(0.0, 1.0)
}

#[tauri::command]
pub fn export_svg(result: VectorizationResult, output_path: String) -> std::result::Result<String, String> {
    let mut file = std::fs::File::create(&output_path).map_err(|e| e.to_string())?;
    write_svg(&result, &mut file).map_err(|e| e.to_string())?;
    Ok(output_path)
}

#[tauri::command]
pub fn render_svg_string(result: VectorizationResult) -> std::result::Result<String, String> {
    let mut buf = Vec::new();
    write_svg(&result, &mut buf).map_err(|e| e.to_string())?;
    String::from_utf8(buf).map_err(|e| e.to_string())
}

/// Write a string to a file (used for saving raw SVG source).
#[tauri::command]
pub fn write_file(path: String, content: String) -> std::result::Result<String, String> {
    std::fs::write(&path, &content).map_err(|e| e.to_string())?;
    Ok(path)
}

#[tauri::command]
pub fn export_eps(result: VectorizationResult, output_path: String) -> std::result::Result<String, String> {
    let mut file = std::fs::File::create(&output_path).map_err(|e| e.to_string())?;
    write_eps(&result, &mut file).map_err(|e| e.to_string())?;
    Ok(output_path)
}

#[tauri::command]
pub fn export_pdf(result: VectorizationResult, output_path: String) -> std::result::Result<String, String> {
    let mut file = std::fs::File::create(&output_path).map_err(|e| e.to_string())?;
    write_pdf(&result, &mut file).map_err(|e| e.to_string())?;
    Ok(output_path)
}

#[tauri::command]
pub fn export_dxf(result: VectorizationResult, output_path: String, line_only: bool, segments_per_curve: Option<u16>) -> std::result::Result<String, String> {
    let mut file = std::fs::File::create(&output_path).map_err(|e| e.to_string())?;
    if line_only {
        write_dxf_polyline(&result, &mut file, segments_per_curve.unwrap_or(8)).map_err(|e| e.to_string())?;
    } else {
        write_dxf_spline(&result, &mut file).map_err(|e| e.to_string())?;
    }
    Ok(output_path)
}

// --- Segmentation Editor State ---

static EDITOR_STATE: std::sync::LazyLock<Mutex<Option<SegmentationEditor>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));

/// Initialize the editor with a segmentation from a vectorization result.
#[tauri::command]
pub fn init_editor(segmentation: Segmentation) -> std::result::Result<(), String> {
    let mut state = EDITOR_STATE.lock().map_err(|e| e.to_string())?;
    *state = Some(SegmentationEditor::new(segmentation));
    Ok(())
}

/// Apply an edit to the current segmentation.
#[tauri::command]
pub fn apply_edit(edit: SegEdit) -> std::result::Result<Segmentation, String> {
    let mut state = EDITOR_STATE.lock().map_err(|e| e.to_string())?;
    let editor = state.as_mut().ok_or("Editor not initialized")?;
    editor.apply_edit(edit).map_err(|e| e.to_string())?;
    Ok(editor.get_current().clone())
}

/// Undo the last edit.
#[tauri::command]
pub fn undo_edit() -> std::result::Result<bool, String> {
    let mut state = EDITOR_STATE.lock().map_err(|e| e.to_string())?;
    let editor = state.as_mut().ok_or("Editor not initialized")?;
    Ok(editor.undo())
}

/// Reset all edits to the original segmentation.
#[tauri::command]
pub fn reset_edits() -> std::result::Result<(), String> {
    let mut state = EDITOR_STATE.lock().map_err(|e| e.to_string())?;
    let editor = state.as_mut().ok_or("Editor not initialized")?;
    editor.reset();
    Ok(())
}

/// Get the current segmentation state.
#[tauri::command]
pub fn get_segmentation() -> std::result::Result<Segmentation, String> {
    let state = EDITOR_STATE.lock().map_err(|e| e.to_string())?;
    let editor = state.as_ref().ok_or("Editor not initialized")?;
    Ok(editor.get_current().clone())
}

/// Find articulation point artifacts in the current segmentation.
#[tauri::command]
pub fn find_artifacts() -> std::result::Result<Vec<(u32, u32)>, String> {
    let state = EDITOR_STATE.lock().map_err(|e| e.to_string())?;
    let editor = state.as_ref().ok_or("Editor not initialized")?;
    Ok(editor::find_artifacts(editor.get_current()))
}

/// Fix an artifact pixel by merging the smaller side into a neighbor.
#[tauri::command]
pub fn fix_artifact(x: u32, y: u32) -> std::result::Result<Segmentation, String> {
    let mut state = EDITOR_STATE.lock().map_err(|e| e.to_string())?;
    let editor = state.as_mut().ok_or("Editor not initialized")?;

    let seg = editor.get_current();
    let width = seg.width as usize;
    let idx = y as usize * width + x as usize;
    let region_id = seg.label_map[idx];

    // Find the most common neighboring region
    let offsets: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
    let mut neighbor_counts: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();

    for (dx, dy) in &offsets {
        let nx = x as i32 + dx;
        let ny = y as i32 + dy;
        if nx < 0 || ny < 0 || nx >= seg.width as i32 || ny >= seg.height as i32 {
            continue;
        }
        let nidx = ny as usize * width + nx as usize;
        let nlabel = seg.label_map[nidx];
        if nlabel != region_id {
            *neighbor_counts.entry(nlabel).or_insert(0) += 1;
        }
    }

    if let Some((&target, _)) = neighbor_counts.iter().max_by_key(|&(_, &count)| count) {
        // Paint this single pixel to the neighboring region
        editor
            .apply_edit(SegEdit::PaintPixels {
                pixels: vec![(x, y)],
                target_region: target,
            })
            .map_err(|e| e.to_string())?;
    }

    Ok(editor.get_current().clone())
}

/// Sample color at a pixel from current or original segmentation.
#[tauri::command]
pub fn sample_color(x: u32, y: u32, _from_original: bool) -> std::result::Result<SampleResult, String> {
    let state = EDITOR_STATE.lock().map_err(|e| e.to_string())?;
    let editor = state.as_ref().ok_or("Editor not initialized")?;
    let seg = editor.get_current();
    let width = seg.width as usize;

    if x >= seg.width || y >= seg.height {
        return Err("Coordinates out of bounds".to_string());
    }

    let idx = y as usize * width + x as usize;
    let region_id = seg.label_map[idx];

    let color_index = seg
        .regions
        .iter()
        .find(|r| r.id == region_id)
        .map(|r| r.color_index)
        .unwrap_or(0);

    // Return a placeholder hex — the actual color mapping happens via the palette on the frontend
    let color_hex = format!("#{:02x}{:02x}{:02x}", color_index * 37 % 256, color_index * 73 % 256, color_index * 111 % 256);

    Ok(SampleResult { region_id, color_hex })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SampleResult {
    pub region_id: u32,
    pub color_hex: String,
}

/// Zap: split a region at the clicked boundary and merge the smaller piece.
#[tauri::command]
pub fn zap_region(x: u32, y: u32, min_region_size: u32) -> std::result::Result<Segmentation, String> {
    let mut state = EDITOR_STATE.lock().map_err(|e| e.to_string())?;
    let editor = state.as_mut().ok_or("Editor not initialized")?;

    let seg = editor.get_current();
    let width = seg.width as usize;

    if x >= seg.width || y >= seg.height {
        return Err("Coordinates out of bounds".to_string());
    }

    let idx = y as usize * width + x as usize;
    let region_id = seg.label_map[idx];

    // Create a split line perpendicular to the boundary at click point
    let split_line = (
        vectorit_core::types::Point::new(x as f64, y as f64 - 20.0),
        vectorit_core::types::Point::new(x as f64, y as f64 + 20.0),
    );

    // Apply split
    editor
        .apply_edit(SegEdit::SplitRegion {
            region_id,
            split_line,
        })
        .map_err(|e| e.to_string())?;

    // Find the smaller region and merge it with the most common neighbor
    let seg_after = editor.get_current();
    let new_region = seg_after
        .regions
        .iter()
        .filter(|r| r.color_index == seg_after.regions.iter().find(|rr| rr.id == region_id).map(|rr| rr.color_index).unwrap_or(0))
        .min_by_key(|r| r.pixel_count);

    if let Some(small) = new_region {
        if small.pixel_count < min_region_size || small.id != region_id {
            // Find the smaller of the two pieces
            let smaller_id = seg_after
                .regions
                .iter()
                .filter(|r| {
                    r.color_index
                        == seg_after
                            .regions
                            .iter()
                            .find(|rr| rr.id == region_id)
                            .map(|rr| rr.color_index)
                            .unwrap_or(u16::MAX)
                })
                .min_by_key(|r| r.pixel_count)
                .map(|r| r.id);

            if let Some(smaller) = smaller_id {
                // Find neighbor to merge into
                let mut neighbor_counts: std::collections::HashMap<u32, u32> =
                    std::collections::HashMap::new();
                for (i, &label) in seg_after.label_map.iter().enumerate() {
                    if label != smaller {
                        continue;
                    }
                    let px = i % width;
                    let py = i / width;
                    let offsets: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
                    for (dx, dy) in &offsets {
                        let nx = px as i32 + dx;
                        let ny = py as i32 + dy;
                        if nx < 0 || ny < 0 || nx >= seg_after.width as i32 || ny >= seg_after.height as i32 {
                            continue;
                        }
                        let nidx = ny as usize * width + nx as usize;
                        let nlabel = seg_after.label_map[nidx];
                        if nlabel != smaller {
                            *neighbor_counts.entry(nlabel).or_insert(0) += 1;
                        }
                    }
                }

                if let Some((&target, _)) = neighbor_counts.iter().max_by_key(|&(_, &c)| c) {
                    editor
                        .apply_edit(SegEdit::MergeRegions {
                            source: smaller,
                            target,
                        })
                        .map_err(|e| e.to_string())?;
                }
            }
        }
    }

    Ok(editor.get_current().clone())
}

/// Re-vectorize from an edited segmentation (skips decode/quantize/segment stages).
#[tauri::command]
pub fn re_vectorize(
    config: VectorizationConfig,
    app_handle: tauri::AppHandle,
) -> std::result::Result<VectorizationResult, String> {
    let state = EDITOR_STATE.lock().map_err(|e| e.to_string())?;
    let editor = state.as_ref().ok_or("Editor not initialized")?;
    let segmentation = editor.get_current().clone();
    drop(state); // Release lock before long operation

    let progress_callback = move |progress: PipelineProgress| {
        let _ = app_handle.emit("progress", &progress);
    };

    pipeline::vectorize_from_segmentation(segmentation, &config, Some(&progress_callback))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_documents_dir() -> std::result::Result<String, String> {
    dirs::document_dir()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "Could not determine Documents directory".to_string())
}

#[tauri::command]
pub fn paste_from_clipboard() -> std::result::Result<ImageInfo, String> {
    let path = crate::clipboard::read_clipboard_image()?;
    load_image(path)
}

#[tauri::command]
pub fn start_drag(file_path: String) -> std::result::Result<(), String> {
    crate::dragdrop::start_file_drag(&file_path)
}

#[tauri::command]
pub fn export_bitmap(
    result: VectorizationResult,
    width: u32,
    height: u32,
    format: String,
    path: String,
) -> std::result::Result<String, String> {
    let bitmap_format = match format.to_lowercase().as_str() {
        "png" => vectorit_core::types::BitmapFormat::Png,
        "bmp" => vectorit_core::types::BitmapFormat::Bmp,
        "jpg" | "jpeg" => vectorit_core::types::BitmapFormat::Jpg(90),
        _ => return Err(format!("Unsupported bitmap format: {}", format)),
    };

    let data = vectorit_core::export::bitmap::export_bitmap(&result, width, height, bitmap_format)
        .map_err(|e| e.to_string())?;

    std::fs::write(&path, &data).map_err(|e| e.to_string())?;
    Ok(path)
}
