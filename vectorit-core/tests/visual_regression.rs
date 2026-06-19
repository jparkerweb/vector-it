use std::path::Path;
use vectorit_core::export::svg::export_svg;
use vectorit_core::pipeline;
use vectorit_core::types::{RawImage, VectorizationConfig};

/// Create a simple 2-color logo test image (red square on white background).
fn make_two_color_logo(width: u32, height: u32) -> RawImage {
    let mut pixels = vec![[255u8, 255, 255, 255]; (width * height) as usize];
    // Draw a red square in the center
    let margin = width / 4;
    for y in margin..(height - margin) {
        for x in margin..(width - margin) {
            pixels[(y * width + x) as usize] = [255, 0, 0, 255];
        }
    }
    RawImage {
        width,
        height,
        pixels,
        has_alpha: false,
    }
}

/// Create a 4-color icon test image (quadrants of different colors).
fn make_four_color_icon(width: u32, height: u32) -> RawImage {
    let mut pixels = Vec::with_capacity((width * height) as usize);
    let half_w = width / 2;
    let half_h = height / 2;
    for y in 0..height {
        for x in 0..width {
            let color = match (x < half_w, y < half_h) {
                (true, true) => [255, 0, 0, 255],     // red
                (false, true) => [0, 255, 0, 255],     // green
                (true, false) => [0, 0, 255, 255],     // blue
                (false, false) => [255, 255, 0, 255],  // yellow
            };
            pixels.push(color);
        }
    }
    RawImage {
        width,
        height,
        pixels,
        has_alpha: false,
    }
}

/// Create an anti-aliased text-like image (gradient edges).
fn make_aa_text_image(width: u32, height: u32) -> RawImage {
    let mut pixels = vec![[255u8, 255, 255, 255]; (width * height) as usize];
    // Draw a circle with anti-aliased edges
    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;
    let radius = width.min(height) as f64 / 3.0;

    for y in 0..height {
        for x in 0..width {
            let dx = x as f64 - cx;
            let dy = y as f64 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let diff = dist - radius;

            if diff < -1.0 {
                pixels[(y * width + x) as usize] = [0, 0, 0, 255]; // inside
            } else if diff < 1.0 {
                // Anti-aliased edge
                let alpha = ((1.0 - diff) / 2.0 * 255.0).clamp(0.0, 255.0) as u8;
                let inv = 255 - alpha;
                pixels[(y * width + x) as usize] = [inv, inv, inv, 255];
            }
            // else: white background
        }
    }
    RawImage {
        width,
        height,
        pixels,
        has_alpha: false,
    }
}

/// Run vectorization pipeline and produce SVG, then render to bitmap using resvg.
fn pipeline_to_bitmap(image: RawImage, color_count: u16) -> (Vec<u8>, u32, u32) {
    let config = VectorizationConfig {
        color_count,
        ..Default::default()
    };

    let result = pipeline::vectorize(image, &config, None).expect("Pipeline failed");

    // Export to SVG
    let mut svg_buf = Vec::new();
    export_svg(&result, &mut svg_buf).expect("SVG export failed");
    let svg_str = String::from_utf8(svg_buf).expect("SVG not valid UTF-8");

    // Render SVG to bitmap using resvg
    let tree = resvg::usvg::Tree::from_str(
        &svg_str,
        &resvg::usvg::Options::default(),
    )
    .expect("Failed to parse SVG");

    let size = tree.size();
    let px_w = size.width().ceil() as u32;
    let px_h = size.height().ceil() as u32;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(px_w, px_h).expect("Failed to create pixmap");
    pixmap.fill(resvg::tiny_skia::Color::WHITE);
    resvg::render(&tree, resvg::tiny_skia::Transform::default(), &mut pixmap.as_mut());

    (pixmap.data().to_vec(), px_w, px_h)
}

/// Compute RMSE between two bitmaps (RGBA).
fn compute_rmse(a: &[u8], b: &[u8]) -> f64 {
    assert_eq!(a.len(), b.len(), "Bitmap sizes must match");
    let n = a.len() as f64;
    let sum_sq: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(&x, &y)| {
            let diff = x as f64 - y as f64;
            diff * diff
        })
        .sum();
    (sum_sq / n).sqrt()
}

#[test]
fn visual_regression_two_color_logo() {
    let image = make_two_color_logo(32, 32);
    let (bitmap, w, h) = pipeline_to_bitmap(image, 2);
    assert!(w > 0 && h > 0, "Rendered bitmap has zero dimensions");
    assert!(!bitmap.is_empty(), "Rendered bitmap is empty");

    // Save as reference or compare with existing
    let ref_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/references/two_color_logo.bin");

    if ref_path.exists() {
        let reference = std::fs::read(&ref_path).expect("Failed to read reference");
        if reference.len() == bitmap.len() {
            let rmse = compute_rmse(&reference, &bitmap);
            assert!(
                rmse < 2.0,
                "Visual regression: RMSE {:.3} exceeds threshold 2.0",
                rmse
            );
        }
    } else {
        // First run: save reference
        std::fs::write(&ref_path, &bitmap).expect("Failed to write reference");
    }
}

#[test]
fn visual_regression_four_color_icon() {
    let image = make_four_color_icon(32, 32);
    let (bitmap, w, h) = pipeline_to_bitmap(image, 4);
    assert!(w > 0 && h > 0);
    assert!(!bitmap.is_empty());

    let ref_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/references/four_color_icon.bin");

    if ref_path.exists() {
        let reference = std::fs::read(&ref_path).expect("Failed to read reference");
        if reference.len() == bitmap.len() {
            let rmse = compute_rmse(&reference, &bitmap);
            assert!(
                rmse < 2.0,
                "Visual regression: RMSE {:.3} exceeds threshold 2.0",
                rmse
            );
        }
    } else {
        std::fs::write(&ref_path, &bitmap).expect("Failed to write reference");
    }
}

#[test]
fn visual_regression_aa_text() {
    let image = make_aa_text_image(32, 32);
    let (bitmap, w, h) = pipeline_to_bitmap(image, 4);
    assert!(w > 0 && h > 0);
    assert!(!bitmap.is_empty());

    let ref_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/references/aa_text.bin");

    if ref_path.exists() {
        let reference = std::fs::read(&ref_path).expect("Failed to read reference");
        if reference.len() == bitmap.len() {
            let rmse = compute_rmse(&reference, &bitmap);
            assert!(
                rmse < 2.0,
                "Visual regression: RMSE {:.3} exceeds threshold 2.0",
                rmse
            );
        }
    } else {
        std::fs::write(&ref_path, &bitmap).expect("Failed to write reference");
    }
}
