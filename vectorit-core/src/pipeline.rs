use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tracing::debug;

use crate::types::*;
use crate::{quantizer, resizer, segmenter};

/// Pipeline progress information.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PipelineProgress {
    pub stage: String,
    pub percent: u8,
}

/// Run the full vectorization pipeline.
pub fn vectorize(
    image: RawImage,
    config: &VectorizationConfig,
    cancel: Option<Arc<AtomicBool>>,
) -> Result<VectorizationResult> {
    vectorize_with_progress(image, config, cancel, None)
}

/// Run the full vectorization pipeline with optional progress callback.
pub fn vectorize_with_progress(
    image: RawImage,
    config: &VectorizationConfig,
    cancel: Option<Arc<AtomicBool>>,
    progress: Option<&dyn Fn(PipelineProgress)>,
) -> Result<VectorizationResult> {
    let check_cancel = |cancel: &Option<Arc<AtomicBool>>| -> Result<()> {
        if let Some(flag) = cancel {
            if flag.load(Ordering::Relaxed) {
                return Err(VectorItError::Cancelled);
            }
        }
        Ok(())
    };

    let emit = |stage: &str, percent: u8| {
        if let Some(cb) = &progress {
            cb(PipelineProgress {
                stage: stage.to_string(),
                percent,
            });
        }
    };

    // Auto-resize large images instead of rejecting them
    let (processed_for_pipeline, original_dimensions) = if config.auto_resize {
        let resized = resizer::resize_for_analysis(&image, 4.0);
        let orig_dims = (image.width, image.height);
        debug!(
            original_mp = (image.width as f64 * image.height as f64 / 1_000_000.0),
            resized_mp = (resized.width as f64 * resized.height as f64 / 1_000_000.0),
            "resize stage complete"
        );
        (resized, orig_dims)
    } else {
        let dims = (image.width, image.height);
        (image, dims)
    };

    check_cancel(&cancel)?;

    // Handle transparency
    let (processed_image, transparent_pixels) = match config.transparency_mode {
        TransparencyMode::Transparent => {
            let transparent: Vec<bool> = processed_for_pipeline.pixels.iter().map(|p| p[3] < 128).collect();
            (processed_for_pipeline, transparent)
        }
        TransparencyMode::FlattenToColor(bg) => {
            let mut flattened = processed_for_pipeline.clone();
            for pixel in flattened.pixels.iter_mut() {
                let alpha = pixel[3] as f32 / 255.0;
                pixel[0] = (pixel[0] as f32 * alpha + bg.r as f32 * (1.0 - alpha)) as u8;
                pixel[1] = (pixel[1] as f32 * alpha + bg.g as f32 * (1.0 - alpha)) as u8;
                pixel[2] = (pixel[2] as f32 * alpha + bg.b as f32 * (1.0 - alpha)) as u8;
                pixel[3] = 255;
            }
            flattened.has_alpha = false;
            let transparent = vec![false; flattened.pixels.len()];
            (flattened, transparent)
        }
    };

    // Stage 1: Quantize (still needed for segmentation/editor)
    emit("quantize", 0);
    let quantized = quantizer::quantize(&processed_image, config.color_count)?;
    debug!(
        palette_size = quantized.palette.colors.len(),
        labels_bytes = quantized.labels.len() * 2,
        "quantize stage complete"
    );
    check_cancel(&cancel)?;
    emit("quantize", 15);

    // Stage 2: Segment (still needed for editor)
    emit("segment", 15);
    let mut segmentation = segmenter::segment(&quantized);
    debug!(
        region_count = segmentation.regions.len(),
        label_map_bytes = segmentation.label_map.len() * 4,
        "segment stage complete"
    );
    check_cancel(&cancel)?;
    emit("segment", 25);

    // Handle transparent pixels
    let has_transparency = transparent_pixels.iter().any(|&t| t);
    if has_transparency {
        for label in segmentation.label_map.iter_mut() {
            *label += 1;
        }
        for region in segmentation.regions.iter_mut() {
            region.id += 1;
        }
        let mut transparent_count = 0u32;
        for (i, &is_transparent) in transparent_pixels.iter().enumerate() {
            if is_transparent {
                segmentation.label_map[i] = 0;
                transparent_count += 1;
            }
        }
        if transparent_count > 0 {
            segmentation.regions.insert(
                0,
                Region {
                    id: 0,
                    color_index: u16::MAX,
                    pixel_count: transparent_count,
                },
            );
        }
    }

    // Stage 3: Vectorize with vtracer
    emit("vectorize", 25);
    check_cancel(&cancel)?;

    // Build RGBA pixel buffer for vtracer
    let width = processed_image.width as usize;
    let height = processed_image.height as usize;
    let mut rgba_pixels = Vec::with_capacity(width * height * 4);
    for pixel in &processed_image.pixels {
        rgba_pixels.push(pixel[0]); // R
        rgba_pixels.push(pixel[1]); // G
        rgba_pixels.push(pixel[2]); // B
        rgba_pixels.push(pixel[3]); // A
    }

    let color_image = vtracer::ColorImage {
        pixels: rgba_pixels,
        width,
        height,
    };

    // Map quality settings to vtracer config
    let smoothness = config.effective_smoothness();
    let vtracer_config = build_vtracer_config(config, smoothness);

    emit("vectorize", 40);
    let svg_file = vtracer::convert(color_image, vtracer_config)
        .map_err(|e| VectorItError::Pipeline(format!("vtracer error: {}", e)))?;

    emit("vectorize", 80);
    check_cancel(&cancel)?;

    // Convert vtracer paths to our VectorPath format
    let paths = convert_vtracer_paths(&svg_file);

    emit("export", 95);
    emit("export", 100);

    Ok(VectorizationResult {
        paths,
        palette: quantized.palette,
        dimensions: original_dimensions,
        segmentation,
    })
}

/// Run the vectorization pipeline starting from an already-edited segmentation.
pub fn vectorize_from_segmentation(
    segmentation: Segmentation,
    config: &VectorizationConfig,
    progress: Option<&dyn Fn(PipelineProgress)>,
) -> Result<VectorizationResult> {
    let emit = |stage: &str, percent: u8| {
        if let Some(cb) = &progress {
            cb(PipelineProgress {
                stage: stage.to_string(),
                percent,
            });
        }
    };

    emit("vectorize", 0);

    // Reconstruct an RGBA image from the segmentation label map and region colors
    let width = segmentation.width as usize;
    let height = segmentation.height as usize;

    // Build a color lookup from region IDs
    let max_color_index = segmentation
        .regions
        .iter()
        .map(|r| r.color_index)
        .max()
        .unwrap_or(0) as usize;

    let rgb_palette: Vec<RgbColor> = (0..=max_color_index)
        .map(|i| RgbColor {
            r: ((i * 37) % 256) as u8,
            g: ((i * 73) % 256) as u8,
            b: ((i * 111) % 256) as u8,
        })
        .collect();

    // Build region ID → color map
    let region_colors: std::collections::HashMap<u32, RgbColor> = segmentation
        .regions
        .iter()
        .map(|r| {
            let color = if (r.color_index as usize) < rgb_palette.len() {
                rgb_palette[r.color_index as usize]
            } else {
                RgbColor { r: 128, g: 128, b: 128 }
            };
            (r.id, color)
        })
        .collect();

    // Build RGBA pixel buffer
    let mut rgba_pixels = Vec::with_capacity(width * height * 4);
    for &label in &segmentation.label_map {
        let color = region_colors.get(&label).copied().unwrap_or(RgbColor { r: 0, g: 0, b: 0 });
        rgba_pixels.push(color.r);
        rgba_pixels.push(color.g);
        rgba_pixels.push(color.b);
        rgba_pixels.push(255u8);
    }

    let color_image = vtracer::ColorImage {
        pixels: rgba_pixels,
        width,
        height,
    };

    let smoothness = config.effective_smoothness();
    let vtracer_config = build_vtracer_config(config, smoothness);

    emit("vectorize", 30);
    let svg_file = vtracer::convert(color_image, vtracer_config)
        .map_err(|e| VectorItError::Pipeline(format!("vtracer error: {}", e)))?;

    emit("vectorize", 80);

    let paths = convert_vtracer_paths(&svg_file);

    emit("export", 100);

    let palette_colors: Vec<crate::types::LabColor> = (0..=max_color_index)
        .map(|_| crate::types::LabColor { l: 50.0, a: 0.0, b: 0.0 })
        .collect();

    Ok(VectorizationResult {
        paths,
        palette: crate::types::Palette { colors: palette_colors },
        dimensions: (segmentation.width, segmentation.height),
        segmentation,
    })
}

/// Build vtracer config from our VectorizationConfig.
fn build_vtracer_config(config: &VectorizationConfig, _smoothness: f64) -> vtracer::Config {
    use visioncortex::PathSimplifyMode;

    let mode = match config.path_mode.as_str() {
        "spline" => PathSimplifyMode::Spline,
        _ => PathSimplifyMode::Polygon,
    };

    vtracer::Config {
        color_mode: vtracer::ColorMode::Color,
        hierarchical: vtracer::Hierarchical::Stacked,
        mode,
        filter_speckle: config.speckle_filter as usize,
        color_precision: config.color_precision as i32,
        layer_difference: 16,
        corner_threshold: config.effective_corner_threshold() as i32,
        length_threshold: 4.0,
        splice_threshold: 91,
        max_iterations: 10,
        path_precision: Some(8),
    }
}

/// Convert vtracer's SvgFile paths into our VectorPath format.
fn convert_vtracer_paths(svg_file: &vtracer::SvgFile) -> Vec<VectorPath> {
    let mut paths = Vec::new();

    for svg_path in &svg_file.paths {
        let color = &svg_path.color;
        let fill_color = RgbColor {
            r: color.r,
            g: color.g,
            b: color.b,
        };

        // Get SVG path string from vtracer
        let (path_string, offset) = svg_path
            .path
            .to_svg_string(true, visioncortex::PointF64::default(), Some(3));

        // Parse the SVG path string into CubicBezier segments
        let segments = parse_svg_path(&path_string, offset.x, offset.y);
        if segments.is_empty() {
            continue;
        }

        paths.push(VectorPath {
            segments,
            fill_color,
            is_closed: true,
            stroke_color: None,
            stroke_width: None,
        });
    }

    paths
}

/// Parse an SVG path `d` attribute string into CubicBezier segments.
/// Handles M, L, C, Q, H, V, Z commands (absolute only — vtracer uses absolute).
fn parse_svg_path(d: &str, offset_x: f64, offset_y: f64) -> Vec<BezierSegment> {
    let mut segments = Vec::new();
    let mut current = Point::new(0.0, 0.0);
    let mut start = Point::new(0.0, 0.0);

    let tokens = tokenize_svg_path(d);
    let mut i = 0;

    while i < tokens.len() {
        match tokens[i].as_str() {
            "M" | "m" => {
                let is_rel = tokens[i] == "m";
                i += 1;
                if i + 1 < tokens.len() {
                    let (x, y) = parse_coords(&tokens, &mut i, is_rel, &current);
                    current = Point::new(x + offset_x, y + offset_y);
                    start = current;
                    // Subsequent coordinate pairs after M are implicit L
                    while i < tokens.len() && is_number(&tokens[i]) {
                        let (x, y) = parse_coords(&tokens, &mut i, is_rel, &current);
                        let next = Point::new(x + offset_x, y + offset_y);
                        segments.push(line_to_bezier(current, next));
                        current = next;
                    }
                }
            }
            "L" | "l" => {
                let is_rel = tokens[i] == "l";
                i += 1;
                while i < tokens.len() && is_number(&tokens[i]) {
                    let (x, y) = parse_coords(&tokens, &mut i, is_rel, &current);
                    let next = Point::new(x + offset_x, y + offset_y);
                    segments.push(line_to_bezier(current, next));
                    current = next;
                }
            }
            "H" | "h" => {
                let is_rel = tokens[i] == "h";
                i += 1;
                while i < tokens.len() && is_number(&tokens[i]) {
                    let val: f64 = tokens[i].parse().unwrap_or(0.0);
                    i += 1;
                    let x = if is_rel { current.x + val } else { val + offset_x };
                    let next = Point::new(x, current.y);
                    segments.push(line_to_bezier(current, next));
                    current = next;
                }
            }
            "V" | "v" => {
                let is_rel = tokens[i] == "v";
                i += 1;
                while i < tokens.len() && is_number(&tokens[i]) {
                    let val: f64 = tokens[i].parse().unwrap_or(0.0);
                    i += 1;
                    let y = if is_rel { current.y + val } else { val + offset_y };
                    let next = Point::new(current.x, y);
                    segments.push(line_to_bezier(current, next));
                    current = next;
                }
            }
            "C" | "c" => {
                let is_rel = tokens[i] == "c";
                i += 1;
                while i + 5 < tokens.len() && is_number(&tokens[i]) {
                    let (x1, y1) = parse_coords(&tokens, &mut i, is_rel, &current);
                    let (x2, y2) = parse_coords(&tokens, &mut i, is_rel, &current);
                    let (x3, y3) = parse_coords(&tokens, &mut i, is_rel, &current);
                    let p1 = Point::new(x1 + offset_x, y1 + offset_y);
                    let p2 = Point::new(x2 + offset_x, y2 + offset_y);
                    let p3 = Point::new(x3 + offset_x, y3 + offset_y);
                    segments.push(BezierSegment {
                        curve: CubicBezier {
                            p0: current,
                            p1,
                            p2,
                            p3,
                        },
                        is_corner_start: false,
                    });
                    current = p3;
                }
            }
            "Q" | "q" => {
                // Convert quadratic to cubic Bézier
                let is_rel = tokens[i] == "q";
                i += 1;
                while i + 3 < tokens.len() && is_number(&tokens[i]) {
                    let (qx, qy) = parse_coords(&tokens, &mut i, is_rel, &current);
                    let (ex, ey) = parse_coords(&tokens, &mut i, is_rel, &current);
                    let q = Point::new(qx + offset_x, qy + offset_y);
                    let end = Point::new(ex + offset_x, ey + offset_y);
                    // Quadratic to cubic: CP1 = P0 + 2/3*(Q-P0), CP2 = P3 + 2/3*(Q-P3)
                    let cp1 = Point::new(
                        current.x + 2.0 / 3.0 * (q.x - current.x),
                        current.y + 2.0 / 3.0 * (q.y - current.y),
                    );
                    let cp2 = Point::new(
                        end.x + 2.0 / 3.0 * (q.x - end.x),
                        end.y + 2.0 / 3.0 * (q.y - end.y),
                    );
                    segments.push(BezierSegment {
                        curve: CubicBezier {
                            p0: current,
                            p1: cp1,
                            p2: cp2,
                            p3: end,
                        },
                        is_corner_start: false,
                    });
                    current = end;
                }
            }
            "Z" | "z" => {
                i += 1;
                if current.distance_to(&start) > 0.01 {
                    segments.push(line_to_bezier(current, start));
                }
                current = start;
            }
            _ => {
                i += 1; // skip unknown
            }
        }
    }

    segments
}

/// Create a degenerate CubicBezier representing a straight line.
fn line_to_bezier(from: Point, to: Point) -> BezierSegment {
    let third = 1.0 / 3.0;
    BezierSegment {
        curve: CubicBezier {
            p0: from,
            p1: Point::new(
                from.x + (to.x - from.x) * third,
                from.y + (to.y - from.y) * third,
            ),
            p2: Point::new(
                from.x + (to.x - from.x) * (2.0 * third),
                from.y + (to.y - from.y) * (2.0 * third),
            ),
            p3: to,
        },
        is_corner_start: true,
    }
}

/// Tokenize an SVG path string into commands and numbers.
fn tokenize_svg_path(d: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut chars = d.chars().peekable();

    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() || ch == ',' {
            chars.next();
        } else if ch.is_ascii_alphabetic() {
            tokens.push(ch.to_string());
            chars.next();
        } else if ch == '-' || ch == '+' || ch == '.' || ch.is_ascii_digit() {
            let mut num = String::new();
            if ch == '-' || ch == '+' {
                num.push(ch);
                chars.next();
            }
            let mut has_dot = false;
            while let Some(&c) = chars.peek() {
                if c.is_ascii_digit() {
                    num.push(c);
                    chars.next();
                } else if c == '.' && !has_dot {
                    has_dot = true;
                    num.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            if !num.is_empty() && num != "-" && num != "+" && num != "." {
                tokens.push(num);
            }
        } else {
            chars.next();
        }
    }

    tokens
}

fn is_number(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_digit() || c == '-' || c == '+' || c == '.')
}

fn parse_coords(tokens: &[String], i: &mut usize, is_rel: bool, current: &Point) -> (f64, f64) {
    let x: f64 = tokens[*i].parse().unwrap_or(0.0);
    *i += 1;
    let y: f64 = tokens[*i].parse().unwrap_or(0.0);
    *i += 1;
    if is_rel {
        (current.x + x, current.y + y)
    } else {
        (x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_large_image_auto_resizes() {
        let image = RawImage {
            width: 3000,
            height: 2000, // 6 MP > 4 MP
            pixels: vec![[0, 0, 0, 255]; 6_000_000],
            has_alpha: false,
        };
        let config = VectorizationConfig::default(); // auto_resize: true
        let result = vectorize(image, &config, None);
        assert!(result.is_ok(), "Large image should auto-resize, not fail");
    }

    #[test]
    fn test_small_solid_image() {
        let image = RawImage {
            width: 4,
            height: 4,
            pixels: vec![[255, 0, 0, 255]; 16],
            has_alpha: false,
        };
        let config = VectorizationConfig::default();
        let result = vectorize(image, &config, None);
        assert!(result.is_ok());
    }
}
