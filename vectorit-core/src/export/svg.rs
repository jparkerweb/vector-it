use std::io::Write;

use crate::types::{CubicBezier, Result, VectorizationResult, VectorItError};

/// Check if a cubic Bézier is nearly a straight line by measuring
/// control point deviation from the chord.
fn is_nearly_linear(b: &CubicBezier, threshold: f64) -> bool {
    let dx = b.p3.x - b.p0.x;
    let dy = b.p3.y - b.p0.y;
    let len_sq = dx * dx + dy * dy;

    if len_sq < 1e-12 {
        // Degenerate — treat as linear
        return true;
    }

    let len = len_sq.sqrt();
    let d1 = ((b.p1.x - b.p0.x) * dy - (b.p1.y - b.p0.y) * dx).abs() / len;
    let d2 = ((b.p2.x - b.p0.x) * dy - (b.p2.y - b.p0.y) * dx).abs() / len;
    d1 < threshold && d2 < threshold
}

/// Export vectorization result as SVG 1.1.
pub fn export_svg(result: &VectorizationResult, writer: &mut impl Write) -> Result<()> {
    let (width, height) = result.dimensions;

    writeln!(
        writer,
        r#"<?xml version="1.0" encoding="UTF-8"?>"#
    )
    .map_err(|e| VectorItError::ExportFailed(e.to_string()))?;

    writeln!(
        writer,
        r#"<svg xmlns="http://www.w3.org/2000/svg" version="1.1" viewBox="0 0 {} {}" width="{}" height="{}">"#,
        width, height, width, height
    )
    .map_err(|e| VectorItError::ExportFailed(e.to_string()))?;

    for path in &result.paths {
        if path.segments.is_empty() {
            continue;
        }

        let mut d = String::new();
        let first_seg = &path.segments[0];
        d.push_str(&format!("M{:.3},{:.3}", first_seg.curve.p0.x, first_seg.curve.p0.y));

        for seg in &path.segments {
            if is_nearly_linear(&seg.curve, 0.15) {
                // Emit straight line command for near-linear curves
                d.push_str(&format!(
                    " L{:.3},{:.3}",
                    seg.curve.p3.x, seg.curve.p3.y
                ));
            } else {
                d.push_str(&format!(
                    " C{:.3},{:.3} {:.3},{:.3} {:.3},{:.3}",
                    seg.curve.p1.x, seg.curve.p1.y,
                    seg.curve.p2.x, seg.curve.p2.y,
                    seg.curve.p3.x, seg.curve.p3.y
                ));
            }
        }

        if path.is_closed {
            d.push('Z');
        }

        let fill_hex = path.fill_color.to_hex();

        if let (Some(stroke_color), Some(stroke_width)) = (&path.stroke_color, path.stroke_width) {
            // User-specified stroke
            let stroke_hex = stroke_color.to_hex();
            writeln!(
                writer,
                r#"  <path d="{}" fill="{}" stroke="{}" stroke-width="{:.3}" stroke-linejoin="round" stroke-linecap="round"/>"#,
                d, fill_hex, stroke_hex, stroke_width
            )
            .map_err(|e| VectorItError::ExportFailed(e.to_string()))?;
        } else {
            // Default hairline stroke matching fill to prevent seams between adjacent regions
            writeln!(
                writer,
                r#"  <path d="{}" fill="{}" stroke="{}" stroke-width="0.09375"/>"#,
                d, fill_hex, fill_hex
            )
            .map_err(|e| VectorItError::ExportFailed(e.to_string()))?;
        }
    }

    writeln!(writer, "</svg>")
        .map_err(|e| VectorItError::ExportFailed(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    #[test]
    fn test_export_empty_result() {
        let result = VectorizationResult {
            paths: vec![],
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
        export_svg(&result, &mut output).unwrap();
        let svg = String::from_utf8(output).unwrap();
        assert!(svg.contains("viewBox=\"0 0 100 100\""));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn test_export_single_path() {
        let result = VectorizationResult {
            paths: vec![VectorPath {
                segments: vec![BezierSegment {
                    curve: CubicBezier {
                        p0: Point::new(0.0, 0.0),
                        p1: Point::new(10.0, 0.0),
                        p2: Point::new(10.0, 10.0),
                        p3: Point::new(0.0, 10.0),
                    },
                    is_corner_start: false,
                }],
                fill_color: RgbColor { r: 255, g: 0, b: 0 },
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
        export_svg(&result, &mut output).unwrap();
        let svg = String::from_utf8(output).unwrap();
        assert!(svg.contains("fill=\"#ff0000\""));
        assert!(svg.contains("stroke=\"#ff0000\""));
        assert!(svg.contains("stroke-width=\"0.09375\""));
    }
}
