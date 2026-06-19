use std::io::Write;

use pdf_writer::{Content, Pdf, Rect, Ref, TextStr};

use crate::types::{Result, VectorizationResult, VectorItError};

/// Export vectorization result as a single-page PDF.
/// Page size matches image dimensions in points (1px = 1pt).
pub fn export_pdf(result: &VectorizationResult, writer: &mut impl Write) -> Result<()> {
    let (width, height) = result.dimensions;
    let w = width as f32;
    let h = height as f32;

    let mut pdf = Pdf::new();

    // Allocate object references
    let catalog_id = Ref::new(1);
    let page_tree_id = Ref::new(2);
    let page_id = Ref::new(3);
    let content_id = Ref::new(4);

    // Catalog
    pdf.catalog(catalog_id).pages(page_tree_id);

    // Page tree
    pdf.pages(page_tree_id).kids([page_id]).count(1);

    // Page
    pdf.page(page_id)
        .parent(page_tree_id)
        .media_box(Rect::new(0.0, 0.0, w, h))
        .contents(content_id);

    // Build content stream
    let mut content = Content::new();

    for path in &result.paths {
        if path.segments.is_empty() {
            continue;
        }

        // Set fill color (RGB normalized to 0.0-1.0)
        let r = path.fill_color.r as f32 / 255.0;
        let g = path.fill_color.g as f32 / 255.0;
        let b = path.fill_color.b as f32 / 255.0;
        content.set_fill_rgb(r, g, b);

        // Move to first point (flip Y: PDF origin is bottom-left)
        let first = &path.segments[0].curve;
        content.move_to(first.p0.x as f32, h - first.p0.y as f32);

        // Cubic Bézier curves
        for seg in &path.segments {
            content.cubic_to(
                seg.curve.p1.x as f32,
                h - seg.curve.p1.y as f32,
                seg.curve.p2.x as f32,
                h - seg.curve.p2.y as f32,
                seg.curve.p3.x as f32,
                h - seg.curve.p3.y as f32,
            );
        }

        if path.is_closed {
            content.close_path();
        }

        // Fill and optionally stroke
        if let (Some(stroke_color), Some(stroke_width)) = (&path.stroke_color, path.stroke_width) {
            let sr = stroke_color.r as f32 / 255.0;
            let sg = stroke_color.g as f32 / 255.0;
            let sb = stroke_color.b as f32 / 255.0;
            content.set_stroke_rgb(sr, sg, sb);
            content.set_line_width(stroke_width as f32);
            content.fill_nonzero_and_stroke();
        } else {
            content.fill_nonzero();
        }
    }

    let content_data = content.finish();
    pdf.stream(content_id, &content_data);

    // Set document metadata
    let info_id = Ref::new(5);
    pdf.document_info(info_id)
        .title(TextStr("VectorIt Export"))
        .creator(TextStr("VectorIt"));

    let pdf_bytes = pdf.finish();
    writer
        .write_all(&pdf_bytes)
        .map_err(|e| VectorItError::ExportFailed(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    #[test]
    fn test_export_pdf_produces_valid_output() {
        let result = VectorizationResult {
            paths: vec![VectorPath {
                segments: vec![BezierSegment {
                    curve: CubicBezier {
                        p0: Point::new(0.0, 0.0),
                        p1: Point::new(50.0, 0.0),
                        p2: Point::new(50.0, 50.0),
                        p3: Point::new(0.0, 50.0),
                    },
                    is_corner_start: false,
                }],
                fill_color: RgbColor { r: 0, g: 128, b: 255 },
                is_closed: true,
                stroke_color: None,
                stroke_width: None,
            }],
            palette: Palette { colors: vec![] },
            dimensions: (100, 100),
            segmentation: Segmentation {
                regions: vec![],
                label_map: vec![],
                width: 100,
                height: 100,
            },
        };
        let mut output = Vec::new();
        export_pdf(&result, &mut output).unwrap();
        // PDF should start with %PDF
        assert!(output.starts_with(b"%PDF"));
        assert!(!output.is_empty());
    }

    #[test]
    fn test_export_empty_pdf() {
        let result = VectorizationResult {
            paths: vec![],
            palette: Palette { colors: vec![] },
            dimensions: (200, 300),
            segmentation: Segmentation {
                regions: vec![],
                label_map: vec![],
                width: 200,
                height: 300,
            },
        };
        let mut output = Vec::new();
        export_pdf(&result, &mut output).unwrap();
        assert!(output.starts_with(b"%PDF"));
    }
}
