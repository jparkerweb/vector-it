use std::path::Path;
use vectorit_core::{decoder, quantizer, segmenter, tracer, simplifier, corner, fitter, optimizer};
use vectorit_core::types::{VectorizationConfig, Quality};
use vectorit_core::export::svg::export_svg;
use vectorit_core::pipeline;

fn main() {
    let input_path = std::env::args().nth(1).expect("Usage: trace_stages <path>");
    let raw = decoder::decode_image(Path::new(&input_path)).expect("Failed to decode");
    println!("Image: {}x{}", raw.width, raw.height);

    let config = VectorizationConfig {
        color_count: 12,
        quality: Quality::High,
        ..Default::default()
    };

    // Stage 1: Quantize
    let quantized = quantizer::quantize(&raw, config.color_count).unwrap();
    println!("\n=== QUANTIZATION ===");
    println!("Palette: {} colors", quantized.palette.colors.len());
    let rgb_palette = quantized.palette.to_rgb();
    for (i, c) in rgb_palette.iter().enumerate() {
        let count = quantized.labels.iter().filter(|&&l| l == i as u16).count();
        println!("  Color {}: #{:02x}{:02x}{:02x} ({} pixels)", i, c.r, c.g, c.b, count);
    }

    // Stage 2: Segment
    let segmentation = segmenter::segment(&quantized);
    println!("\n=== SEGMENTATION ===");
    println!("Regions: {}", segmentation.regions.len());
    let mut sorted_regions: Vec<_> = segmentation.regions.iter().collect();
    sorted_regions.sort_by(|a, b| b.pixel_count.cmp(&a.pixel_count));
    for r in sorted_regions.iter().take(15) {
        let color = if (r.color_index as usize) < rgb_palette.len() {
            let c = &rgb_palette[r.color_index as usize];
            format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
        } else {
            "???".to_string()
        };
        println!("  Region {}: {} pixels, color={}", r.id, r.pixel_count, color);
    }

    // Stage 3: Trace boundaries
    let boundaries = tracer::trace_boundaries(&segmentation, raw.width, raw.height);
    println!("\n=== TRACING ===");
    println!("Total boundaries: {}", boundaries.len());

    // Focus on the largest regions (the two buttons + background)
    let large_boundaries: Vec<_> = boundaries.iter()
        .filter(|b| {
            segmentation.regions.iter()
                .find(|r| r.id == b.region_id)
                .map(|r| r.pixel_count >= 100)
                .unwrap_or(false)
        })
        .collect();

    println!("Large boundaries (region >= 100px): {}", large_boundaries.len());

    let smoothness = config.effective_smoothness();
    let corner_threshold = config.effective_corner_threshold();
    let simplify_tolerance = config.effective_simplify_tolerance();
    let fit_tolerance = 0.2 + smoothness * 1.8;

    println!("\n=== PIPELINE PARAMS (High quality) ===");
    println!("smoothness={}, corner_threshold={}, simplify_tolerance={}, fit_tolerance={}",
        smoothness, corner_threshold, simplify_tolerance, fit_tolerance);

    for b in &large_boundaries {
        let region = segmentation.regions.iter().find(|r| r.id == b.region_id).unwrap();
        let color = if (region.color_index as usize) < rgb_palette.len() {
            let c = &rgb_palette[region.color_index as usize];
            format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
        } else {
            "???".to_string()
        };

        // Compute bbox
        let (mut min_x, mut min_y, mut max_x, mut max_y) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for p in &b.points {
            if p.x < min_x { min_x = p.x; }
            if p.y < min_y { min_y = p.y; }
            if p.x > max_x { max_x = p.x; }
            if p.y > max_y { max_y = p.y; }
        }
        let bbox_size = (max_x - min_x).min(max_y - min_y).max(1.0);

        println!("\n--- Region {} ({}, {} pixels) ---", b.region_id, color, region.pixel_count);
        println!("  Traced: {} points, bbox=[{:.1},{:.1}]-[{:.1},{:.1}] (bbox_size={:.1})",
            b.points.len(), min_x, min_y, max_x, max_y, bbox_size);

        // Print first 30 traced points to see the shape
        if b.points.len() <= 60 {
            println!("  All traced points:");
            for (i, p) in b.points.iter().enumerate() {
                print!("    ({:.1},{:.1})", p.x, p.y);
                if (i + 1) % 8 == 0 { println!(); }
            }
            println!();
        }

        // Simplify
        let size_scale = (bbox_size / 20.0).min(1.0).max(0.05);
        let effective_simplify = simplify_tolerance * size_scale;
        let effective_fit = fit_tolerance * size_scale;
        let simplified = simplifier::simplify(&b.points, effective_simplify);
        println!("  Simplified: {} -> {} points (tol={:.3})",
            b.points.len(), simplified.len(), effective_simplify);

        // Print simplified points
        if simplified.len() <= 40 {
            println!("  Simplified points:");
            for (i, p) in simplified.iter().enumerate() {
                print!("    ({:.1},{:.1})", p.x, p.y);
                if (i + 1) % 8 == 0 { println!(); }
            }
            println!();
        }

        // Corner detection
        let effective_corner = corner_threshold;
        let corners = corner::detect_corners(&simplified, effective_corner);
        let corner_count = corners.iter().filter(|&&c| c).count();
        println!("  Corners: {}/{} marked (threshold={:.1}°)", corner_count, simplified.len(), effective_corner);

        let corner_indices: Vec<_> = corners.iter().enumerate()
            .filter(|&(_, &c)| c)
            .map(|(i, _)| format!("{}({:.1},{:.1})", i, simplified[i].x, simplified[i].y))
            .collect();
        println!("  Corner points: {}", corner_indices.join(", "));

        // Fit curves
        let curves = fitter::fit_curves(&simplified, &corners, effective_fit);
        println!("  Fitted: {} curves (tol={:.3})", curves.len(), effective_fit);

        // Optimize
        let optimized = optimizer::optimize(&curves, effective_fit);
        println!("  Optimized: {} -> {} curves", curves.len(), optimized.len());

        // Check control point bounds
        for (i, c) in optimized.iter().enumerate() {
            let cp_ok = c.p1.x >= -5.0 && c.p1.x <= raw.width as f64 + 5.0
                && c.p1.y >= -5.0 && c.p1.y <= raw.height as f64 + 5.0
                && c.p2.x >= -5.0 && c.p2.x <= raw.width as f64 + 5.0
                && c.p2.y >= -5.0 && c.p2.y <= raw.height as f64 + 5.0;
            if !cp_ok {
                println!("  ⚠ Curve {} has out-of-bounds CP: p1=({:.1},{:.1}) p2=({:.1},{:.1})",
                    i, c.p1.x, c.p1.y, c.p2.x, c.p2.y);
            }
        }
    }

    // Also output the full pipeline result for comparison
    let result = pipeline::vectorize(raw, &config, None).unwrap();
    let mut svg_buf = Vec::new();
    export_svg(&result, &mut svg_buf).unwrap();
    let svg_path = format!("{}.stages.svg", input_path);
    std::fs::write(&svg_path, &svg_buf).unwrap();
    println!("\n\nSVG written to: {}", svg_path);
}
