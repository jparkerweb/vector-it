use proptest::prelude::*;
use vectorit_core::export::svg::export_svg;
use vectorit_core::pipeline;
use vectorit_core::types::{RawImage, VectorizationConfig};

/// Generate a random RGBA image of given dimensions.
fn arb_rgba_image(
    min_dim: u32,
    max_dim: u32,
) -> impl Strategy<Value = RawImage> {
    (min_dim..=max_dim, min_dim..=max_dim).prop_flat_map(|(w, h)| {
        let pixel_count = (w * h) as usize;
        proptest::collection::vec(
            proptest::array::uniform4(0u8..=255u8),
            pixel_count..=pixel_count,
        )
        .prop_map(move |pixels| RawImage {
            width: w,
            height: h,
            pixels,
            has_alpha: false,
        })
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Pipeline must not panic on random input.
    #[test]
    fn pipeline_does_not_panic(image in arb_rgba_image(4, 32)) {
        let config = VectorizationConfig {
            color_count: 4,
            ..Default::default()
        };
        let _ = pipeline::vectorize(image, &config, None);
    }

    /// All output paths must be closed.
    #[test]
    fn all_paths_are_closed(image in arb_rgba_image(4, 32)) {
        let config = VectorizationConfig {
            color_count: 4,
            ..Default::default()
        };
        if let Ok(result) = pipeline::vectorize(image, &config, None) {
            for path in &result.paths {
                prop_assert!(path.is_closed, "Found unclosed path");
            }
        }
    }

    /// All Bézier control points must be finite (no NaN, no infinity).
    #[test]
    fn all_control_points_finite(image in arb_rgba_image(4, 32)) {
        let config = VectorizationConfig {
            color_count: 4,
            ..Default::default()
        };
        if let Ok(result) = pipeline::vectorize(image, &config, None) {
            for path in &result.paths {
                for seg in &path.segments {
                    let pts = [seg.curve.p0, seg.curve.p1, seg.curve.p2, seg.curve.p3];
                    for pt in &pts {
                        prop_assert!(pt.x.is_finite(), "Non-finite x: {}", pt.x);
                        prop_assert!(pt.y.is_finite(), "Non-finite y: {}", pt.y);
                    }
                }
            }
        }
    }

    /// Palette output color count must be ≤ requested color_count.
    #[test]
    fn palette_count_within_requested(
        image in arb_rgba_image(4, 16),
        color_count in 2u16..=8u16,
    ) {
        let config = VectorizationConfig {
            color_count,
            ..Default::default()
        };
        if let Ok(result) = pipeline::vectorize(image, &config, None) {
            prop_assert!(
                result.palette.colors.len() <= color_count as usize,
                "Palette has {} colors, requested {}",
                result.palette.colors.len(),
                color_count
            );
        }
    }

    /// SVG output must be valid XML.
    #[test]
    fn svg_output_is_valid_xml(image in arb_rgba_image(4, 16)) {
        let config = VectorizationConfig {
            color_count: 4,
            ..Default::default()
        };
        if let Ok(result) = pipeline::vectorize(image, &config, None) {
            let mut svg_buf = Vec::new();
            export_svg(&result, &mut svg_buf).expect("SVG export failed");
            let svg_str = String::from_utf8(svg_buf).expect("SVG is not valid UTF-8");

            // Parse with quick-xml to validate
            let mut reader = quick_xml::Reader::from_str(&svg_str);
            loop {
                match reader.read_event() {
                    Ok(quick_xml::events::Event::Eof) => break,
                    Err(e) => prop_assert!(false, "Invalid XML: {:?}", e),
                    _ => {}
                }
            }
        }
    }
}
