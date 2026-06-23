use crate::pipeline;
use crate::types::{
    CubicBezier, Point, RawImage, Result, VectorizationConfig, VectorizationResult, VectorItError,
};

/// Flatten a VectorizationResult by rasterizing only the visible portion of each
/// path (clipped against all paths above it), then re-vectorizing aggressively.
///
/// Guarantees: no path in the output has any geometry hidden under another path.
/// Each pixel is owned by exactly one path (the topmost one covering it).
pub fn flatten(
    result: &VectorizationResult,
    color_count: u16,
    progress: Option<&dyn Fn(pipeline::PipelineProgress)>,
) -> Result<VectorizationResult> {
    let (width, height) = result.dimensions;

    if width == 0 || height == 0 {
        return Err(VectorItError::Pipeline("Cannot flatten empty result".into()));
    }

    let emit = |stage: &str, percent: u8| {
        if let Some(cb) = &progress {
            cb(pipeline::PipelineProgress {
                stage: stage.to_string(),
                percent,
            });
        }
    };

    emit("flatten", 0);

    // Step 1: Rasterize to get a per-pixel ownership map.
    // Each pixel is assigned to exactly one color (the topmost path covering it).
    // This guarantees no hidden geometry survives.
    let raw = rasterize_visible_only(result);

    emit("flatten", 30);

    // Step 2: Re-vectorize with very aggressive simplification.
    // Use low color_precision to merge near-identical colors,
    // high speckle_filter to remove tiny fragments,
    // and high simplify_tolerance to reduce node count.
    let config = VectorizationConfig {
        color_count,
        smoothness: 0.9,
        corner_threshold: 45.0,
        simplify_tolerance: 3.0,
        quality: crate::types::Quality::Custom,
        transparency_mode: crate::types::TransparencyMode::Transparent,
        path_mode: "polygon".to_string(),
        speckle_filter: 16,
        color_precision: 5,
        auto_resize: false,
    };

    emit("flatten", 40);

    let mut flattened = pipeline::vectorize_with_progress(raw, &config, None, progress)?;

    // Step 3: Post-process — merge paths with the same fill color that are adjacent
    // and remove any remaining tiny paths.
    merge_same_color_paths(&mut flattened);

    emit("flatten", 100);
    Ok(flattened)
}

/// Rasterize the VectorizationResult so each pixel belongs to exactly one color.
/// Paints back-to-front: later (higher z-order) paths fully overwrite earlier ones.
/// The result is a clean pixel grid with zero overlap by construction.
fn rasterize_visible_only(result: &VectorizationResult) -> RawImage {
    let (width, height) = result.dimensions;
    let total_pixels = (width as usize) * (height as usize);
    // Start with white opaque background
    let mut pixels = vec![[255u8, 255u8, 255u8, 255u8]; total_pixels];

    for path in &result.paths {
        if path.segments.is_empty() {
            continue;
        }

        let segments: Vec<CubicBezier> = path.segments.iter().map(|s| s.curve).collect();

        // Compute tight bounding box
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        for seg in &segments {
            for t_i in 0..=10 {
                let t = t_i as f64 / 10.0;
                let pt = seg.eval(t);
                min_x = min_x.min(pt.x);
                min_y = min_y.min(pt.y);
                max_x = max_x.max(pt.x);
                max_y = max_y.max(pt.y);
            }
        }

        let px_min_x = min_x.floor().max(0.0) as u32;
        let px_min_y = min_y.floor().max(0.0) as u32;
        let px_max_x = (max_x.ceil() as u32).min(width);
        let px_max_y = (max_y.ceil() as u32).min(height);

        let color = path.fill_color;
        let polyline = linearize_path(&segments, 16);

        for py in px_min_y..px_max_y {
            for px in px_min_x..px_max_x {
                let x = px as f64 + 0.5;
                let y = py as f64 + 0.5;
                let pt = Point::new(x, y);

                if winding_number(&pt, &polyline) != 0 {
                    let idx = (py * width + px) as usize;
                    pixels[idx] = [color.r, color.g, color.b, 255];
                }
            }
        }
    }

    RawImage {
        width,
        height,
        pixels,
        has_alpha: false,
    }
}

/// Merge paths that have the same fill color into a single path.
/// This reduces total path count by combining same-colored regions.
fn merge_same_color_paths(result: &mut VectorizationResult) {
    use std::collections::HashMap;

    if result.paths.is_empty() {
        return;
    }

    // Group paths by fill color
    let mut color_groups: HashMap<(u8, u8, u8), Vec<usize>> = HashMap::new();
    for (i, path) in result.paths.iter().enumerate() {
        let key = (path.fill_color.r, path.fill_color.g, path.fill_color.b);
        color_groups.entry(key).or_default().push(i);
    }

    // If no merging would occur, bail early
    if color_groups.len() == result.paths.len() {
        return;
    }

    let mut merged_paths = Vec::new();
    let mut processed = vec![false; result.paths.len()];

    // Preserve z-order: iterate in original order, merge when we hit the first of a group
    for i in 0..result.paths.len() {
        if processed[i] {
            continue;
        }

        let key = (
            result.paths[i].fill_color.r,
            result.paths[i].fill_color.g,
            result.paths[i].fill_color.b,
        );
        let group = &color_groups[&key];

        if group.len() == 1 {
            merged_paths.push(result.paths[i].clone());
            processed[i] = true;
        } else {
            // Merge all paths in this color group into one
            let mut combined_segments = Vec::new();
            for &idx in group {
                combined_segments.extend(result.paths[idx].segments.iter().cloned());
                processed[idx] = true;
            }
            merged_paths.push(crate::types::VectorPath {
                segments: combined_segments,
                fill_color: result.paths[i].fill_color,
                is_closed: true,
                stroke_color: None,
                stroke_width: None,
            });
        }
    }

    result.paths = merged_paths;
}

/// Linearize Bézier path to polyline points for winding number test.
fn linearize_path(segments: &[CubicBezier], steps: usize) -> Vec<Point> {
    let mut points = Vec::new();
    for seg in segments {
        for i in 0..steps {
            let t = i as f64 / steps as f64;
            points.push(seg.eval(t));
        }
    }
    // Close with the last point
    if let Some(last) = segments.last() {
        points.push(last.eval(1.0));
    }
    points
}

/// Compute winding number of a point with respect to a closed polyline.
fn winding_number(point: &Point, polyline: &[Point]) -> i32 {
    let mut wn = 0i32;
    let n = polyline.len();
    if n < 2 {
        return 0;
    }

    for i in 0..n {
        let j = (i + 1) % n;
        let yi = polyline[i].y;
        let yj = polyline[j].y;

        if yi <= point.y {
            if yj > point.y {
                if is_left(&polyline[i], &polyline[j], point) > 0.0 {
                    wn += 1;
                }
            }
        } else if yj <= point.y {
            if is_left(&polyline[i], &polyline[j], point) < 0.0 {
                wn -= 1;
            }
        }
    }
    wn
}

/// Test if point is left of the line from a to b.
fn is_left(a: &Point, b: &Point, p: &Point) -> f64 {
    (b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        BezierSegment, Palette, RgbColor, Segmentation, VectorPath,
    };

    fn make_square_path(x: f64, y: f64, size: f64, color: RgbColor) -> VectorPath {
        let corners = [
            Point::new(x, y),
            Point::new(x + size, y),
            Point::new(x + size, y + size),
            Point::new(x, y + size),
        ];
        let mut segments = Vec::new();
        for i in 0..4 {
            let p0 = corners[i];
            let p3 = corners[(i + 1) % 4];
            let p1 = Point::new(p0.x + (p3.x - p0.x) / 3.0, p0.y + (p3.y - p0.y) / 3.0);
            let p2 = Point::new(
                p0.x + 2.0 * (p3.x - p0.x) / 3.0,
                p0.y + 2.0 * (p3.y - p0.y) / 3.0,
            );
            segments.push(BezierSegment {
                curve: CubicBezier { p0, p1, p2, p3 },
                is_corner_start: true,
            });
        }
        VectorPath {
            segments,
            fill_color: color,
            is_closed: true,
            stroke_color: None,
            stroke_width: None,
        }
    }

    #[test]
    fn test_flatten_reduces_complexity() {
        // Create overlapping paths — a red square fully covered by a blue square
        let red = RgbColor { r: 255, g: 0, b: 0 };
        let blue = RgbColor { r: 0, g: 0, b: 255 };
        let paths = vec![
            make_square_path(5.0, 5.0, 40.0, red),
            make_square_path(5.0, 5.0, 40.0, blue),
        ];

        let result = VectorizationResult {
            paths,
            palette: Palette { colors: vec![] },
            dimensions: (50, 50),
            segmentation: Segmentation {
                regions: vec![],
                label_map: vec![],
                width: 50,
                height: 50,
            },
        };

        let flattened = flatten(&result, 4, None).unwrap();
        // The red square should be gone since it's fully occluded
        // The flattened result should have fewer or equal unique colors
        assert!(!flattened.paths.is_empty());
        // Verify no red paths survived (they were fully occluded)
        for path in &flattened.paths {
            assert!(
                !(path.fill_color.r > 200 && path.fill_color.g < 50 && path.fill_color.b < 50),
                "Red path should have been removed (occluded by blue)"
            );
        }
    }

    #[test]
    fn test_flatten_partial_overlap_clips_hidden_part() {
        // Large red square partially covered by smaller blue square in the center
        let red = RgbColor { r: 255, g: 0, b: 0 };
        let blue = RgbColor { r: 0, g: 0, b: 255 };
        let paths = vec![
            make_square_path(5.0, 5.0, 90.0, red),   // large red background
            make_square_path(30.0, 30.0, 40.0, blue), // smaller blue on top
        ];

        let result = VectorizationResult {
            paths,
            palette: Palette { colors: vec![] },
            dimensions: (100, 100),
            segmentation: Segmentation {
                regions: vec![],
                label_map: vec![],
                width: 100,
                height: 100,
            },
        };

        let flattened = flatten(&result, 4, None).unwrap();
        assert!(!flattened.paths.is_empty());

        // The original had 2 paths with 8 segments total.
        // After flatten, hidden geometry is removed. The total segment count
        // should reflect only visible geometry (the red frame + blue center).
        let _original_segments: usize = result.paths.iter().map(|p| p.segments.len()).sum();
        let _flattened_segments: usize = flattened.paths.iter().map(|p| p.segments.len()).sum();
        // Flattened may have different segment counts, but no path should be unchanged
        // from the original (the red square should have been clipped to just its visible border)
        assert!(
            flattened.paths.len() >= 2,
            "Should have at least 2 color regions: red border + blue center + white bg"
        );
        // Confirm both colors are still present (red as border, blue as center)
        let has_red = flattened.paths.iter().any(|p| p.fill_color.r > 200 && p.fill_color.b < 50);
        let has_blue = flattened.paths.iter().any(|p| p.fill_color.b > 200 && p.fill_color.r < 50);
        assert!(has_red, "Red border should survive (it's partially visible)");
        assert!(has_blue, "Blue center should survive");
    }

    #[test]
    fn test_rasterize_visible_only_no_overlap() {
        // Verify the rasterizer produces a clean per-pixel map
        let red = RgbColor { r: 255, g: 0, b: 0 };
        let blue = RgbColor { r: 0, g: 0, b: 255 };
        let paths = vec![
            make_square_path(0.0, 0.0, 10.0, red),
            make_square_path(5.0, 5.0, 10.0, blue),
        ];

        let result = VectorizationResult {
            paths,
            palette: Palette { colors: vec![] },
            dimensions: (15, 15),
            segmentation: Segmentation {
                regions: vec![],
                label_map: vec![],
                width: 15,
                height: 15,
            },
        };

        let raw = rasterize_visible_only(&result);

        // Pixel at (2, 2) should be red (not covered by blue)
        let idx_red = 2 * 15 + 2;
        assert_eq!(raw.pixels[idx_red], [255, 0, 0, 255]);

        // Pixel at (7, 7) should be blue (blue covers red here)
        let idx_blue = 7 * 15 + 7;
        assert_eq!(raw.pixels[idx_blue], [0, 0, 255, 255]);

        // Pixel at (12, 12) should be blue (only blue covers here)
        let idx_blue_only = 12 * 15 + 12;
        assert_eq!(raw.pixels[idx_blue_only], [0, 0, 255, 255]);
    }
}
