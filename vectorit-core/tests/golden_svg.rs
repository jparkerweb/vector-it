use std::path::Path;
use vectorit_core::{decoder, pipeline};
use vectorit_core::export::svg::export_svg;
use vectorit_core::types::VectorizationConfig;

#[test]
fn test_golden_svg_pipeline() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("logo.png");

    assert!(fixture_path.exists(), "Test fixture not found: {:?}", fixture_path);

    // Decode the test image
    let raw = decoder::decode_image(&fixture_path).expect("Failed to decode test image");
    assert_eq!(raw.width, 64);
    assert_eq!(raw.height, 64);

    // Run pipeline with default config
    let config = VectorizationConfig {
        color_count: 4,
        ..Default::default()
    };
    let result = pipeline::vectorize(raw, &config, None).expect("Pipeline failed");

    // Verify basic structure
    assert!(!result.paths.is_empty(), "Expected at least one vector path");
    assert_eq!(result.dimensions, (64, 64));

    // Export to SVG string
    let mut svg_output = Vec::new();
    export_svg(&result, &mut svg_output).expect("SVG export failed");
    let svg_str = String::from_utf8(svg_output).expect("SVG is not valid UTF-8");

    // Verify SVG structure
    assert!(svg_str.contains("<?xml"), "Missing XML declaration");
    assert!(svg_str.contains("<svg"), "Missing SVG element");
    assert!(svg_str.contains("viewBox=\"0 0 64 64\""), "Incorrect viewBox");
    assert!(svg_str.contains("<path"), "Missing path elements");
    assert!(svg_str.contains("</svg>"), "Missing closing SVG tag");

    // Snapshot test with insta
    insta::assert_snapshot!("golden_svg", svg_str);
}
