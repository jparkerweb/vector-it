use vectorit_core::decoder;
use vectorit_core::export::{bitmap, dxf, eps, pdf, svg};
use vectorit_core::pipeline;
use vectorit_core::types::*;

fn fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("logo.png")
}

fn vectorize_fixture() -> VectorizationResult {
    let raw = decoder::decode_image(&fixture_path()).expect("Failed to decode fixture");
    let config = VectorizationConfig {
        color_count: 6,
        auto_resize: true,
        ..VectorizationConfig::default()
    };
    pipeline::vectorize(raw, &config, None).expect("Failed to vectorize fixture")
}

// --- SVG Tests ---

#[test]
fn test_svg_export_valid_structure() {
    let result = vectorize_fixture();
    let mut output = Vec::new();
    svg::export_svg(&result, &mut output).expect("SVG export failed");
    let svg_str = String::from_utf8(output).expect("SVG is not valid UTF-8");

    // Verify valid SVG structure
    assert!(svg_str.contains("<svg"));
    assert!(svg_str.contains("xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(svg_str.contains("viewBox=\"0 0"));
    assert!(svg_str.contains("</svg>"));

    // Verify path elements with valid d attributes
    assert!(svg_str.contains("<path"));
    assert!(svg_str.contains(" d=\"M"));
    assert!(svg_str.contains("fill=\"#"));
}

// --- EPS Tests ---

#[test]
fn test_eps_export_valid_header() {
    let result = vectorize_fixture();
    let mut output = Vec::new();
    eps::export_eps(&result, &mut output).expect("EPS export failed");
    let eps_str = String::from_utf8(output).expect("EPS is not valid UTF-8");

    assert!(eps_str.starts_with("%!PS-Adobe-3.0 EPSF-3.0"));
    assert!(eps_str.contains("%%BoundingBox"));
    assert!(eps_str.contains("%%EOF"));
}

// --- PDF Tests ---

#[test]
fn test_pdf_export_valid_header() {
    let result = vectorize_fixture();
    let mut output = Vec::new();
    pdf::export_pdf(&result, &mut output).expect("PDF export failed");

    assert!(output.starts_with(b"%PDF-"));
    assert!(!output.is_empty());
}

// --- DXF Spline Tests ---

#[test]
fn test_dxf_spline_export() {
    let result = vectorize_fixture();
    let mut output = Vec::new();
    dxf::export_dxf_spline(&result, &mut output).expect("DXF Spline export failed");
    let dxf_str = String::from_utf8_lossy(&output);

    // Verify SPLINE entities exist in the output
    assert!(
        dxf_str.contains("SPLINE"),
        "DXF output should contain SPLINE entities"
    );
}

// --- DXF Line-only Tests ---

#[test]
fn test_dxf_polyline_export() {
    let result = vectorize_fixture();
    let mut output = Vec::new();
    dxf::export_dxf_polyline(&result, &mut output, 8).expect("DXF Polyline export failed");
    let dxf_str = String::from_utf8_lossy(&output);

    assert!(
        dxf_str.contains("LWPOLYLINE"),
        "DXF output should contain LWPOLYLINE entities"
    );
}

// --- Large Image / Resize Tests ---

#[test]
fn test_large_image_auto_resizes() {
    // Create a 5MP synthetic image (wider than 4MP limit)
    let width = 2500u32;
    let height = 2000u32; // 5MP
    let pixels: Vec<[u8; 4]> = (0..(width * height) as usize)
        .map(|i| {
            let x = (i % width as usize) as u8;
            let y = (i / width as usize) as u8;
            [x, y, 128, 255]
        })
        .collect();

    let image = RawImage {
        width,
        height,
        pixels,
        has_alpha: false,
    };

    let config = VectorizationConfig {
        color_count: 4,
        auto_resize: true,
        ..VectorizationConfig::default()
    };

    let result = pipeline::vectorize(image, &config, None);
    assert!(result.is_ok(), "5MP image with auto_resize should succeed");

    let result = result.unwrap();
    // Output dimensions should reflect original (not resized)
    assert_eq!(result.dimensions, (width, height));
}

// --- Bitmap Export Tests ---

#[test]
fn test_bitmap_export_png() {
    let result = vectorize_fixture();
    let data = bitmap::export_bitmap(&result, 100, 100, BitmapFormat::Png)
        .expect("Bitmap PNG export failed");

    // PNG magic bytes
    assert_eq!(&data[0..4], &[0x89, 0x50, 0x4E, 0x47]);
}

#[test]
fn test_bitmap_export_bmp() {
    let result = vectorize_fixture();
    let data = bitmap::export_bitmap(&result, 50, 50, BitmapFormat::Bmp)
        .expect("Bitmap BMP export failed");

    // BMP magic bytes
    assert_eq!(&data[0..2], b"BM");
}

#[test]
fn test_bitmap_export_jpg() {
    let result = vectorize_fixture();
    let data = bitmap::export_bitmap(&result, 50, 50, BitmapFormat::Jpg(85))
        .expect("Bitmap JPEG export failed");

    // JPEG magic bytes (SOI marker)
    assert_eq!(&data[0..2], &[0xFF, 0xD8]);
}
