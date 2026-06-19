use std::io::Write;

use crate::types::{Result, VectorizationResult, VectorItError};

/// Export vectorization result as EPS (Encapsulated PostScript Level 3).
pub fn export_eps(result: &VectorizationResult, writer: &mut impl Write) -> Result<()> {
    let (width, height) = result.dimensions;

    // EPS header
    writeln!(writer, "%!PS-Adobe-3.0 EPSF-3.0")
        .map_err(|e| VectorItError::ExportFailed(e.to_string()))?;
    writeln!(writer, "%%BoundingBox: 0 0 {} {}", width, height)
        .map_err(|e| VectorItError::ExportFailed(e.to_string()))?;
    writeln!(writer, "%%HiResBoundingBox: 0.000000 0.000000 {}.000000 {}.000000", width, height)
        .map_err(|e| VectorItError::ExportFailed(e.to_string()))?;
    writeln!(writer, "%%Creator: VectorIt")
        .map_err(|e| VectorItError::ExportFailed(e.to_string()))?;
    writeln!(writer, "%%Title: VectorIt Export")
        .map_err(|e| VectorItError::ExportFailed(e.to_string()))?;
    writeln!(writer, "%%Pages: 1")
        .map_err(|e| VectorItError::ExportFailed(e.to_string()))?;
    writeln!(writer, "%%EndComments")
        .map_err(|e| VectorItError::ExportFailed(e.to_string()))?;
    writeln!(writer)
        .map_err(|e| VectorItError::ExportFailed(e.to_string()))?;

    for path in &result.paths {
        if path.segments.is_empty() {
            continue;
        }

        // Set fill color (RGB normalized to 0.0-1.0)
        let r = path.fill_color.r as f64 / 255.0;
        let g = path.fill_color.g as f64 / 255.0;
        let b = path.fill_color.b as f64 / 255.0;
        writeln!(writer, "{:.4} {:.4} {:.4} setrgbcolor", r, g, b)
            .map_err(|e| VectorItError::ExportFailed(e.to_string()))?;

        writeln!(writer, "newpath")
            .map_err(|e| VectorItError::ExportFailed(e.to_string()))?;

        // Move to first point (flip Y: EPS origin is bottom-left)
        let first = &path.segments[0].curve;
        writeln!(
            writer,
            "{:.3} {:.3} moveto",
            first.p0.x,
            height as f64 - first.p0.y
        )
        .map_err(|e| VectorItError::ExportFailed(e.to_string()))?;

        // Emit curveto for each segment
        for seg in &path.segments {
            writeln!(
                writer,
                "{:.3} {:.3} {:.3} {:.3} {:.3} {:.3} curveto",
                seg.curve.p1.x,
                height as f64 - seg.curve.p1.y,
                seg.curve.p2.x,
                height as f64 - seg.curve.p2.y,
                seg.curve.p3.x,
                height as f64 - seg.curve.p3.y
            )
            .map_err(|e| VectorItError::ExportFailed(e.to_string()))?;
        }

        if path.is_closed {
            writeln!(writer, "closepath")
                .map_err(|e| VectorItError::ExportFailed(e.to_string()))?;
        }

        writeln!(writer, "gsave fill grestore")
            .map_err(|e| VectorItError::ExportFailed(e.to_string()))?;

        // Stroke if user-specified
        if let (Some(stroke_color), Some(stroke_width)) = (&path.stroke_color, path.stroke_width) {
            let sr = stroke_color.r as f64 / 255.0;
            let sg = stroke_color.g as f64 / 255.0;
            let sb = stroke_color.b as f64 / 255.0;
            writeln!(writer, "{:.4} {:.4} {:.4} setrgbcolor", sr, sg, sb)
                .map_err(|e| VectorItError::ExportFailed(e.to_string()))?;
            writeln!(writer, "{:.3} setlinewidth", stroke_width)
                .map_err(|e| VectorItError::ExportFailed(e.to_string()))?;
            writeln!(writer, "1 setlinejoin 1 setlinecap stroke")
                .map_err(|e| VectorItError::ExportFailed(e.to_string()))?;
        }
    }

    writeln!(writer, "%%EOF")
        .map_err(|e| VectorItError::ExportFailed(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    #[test]
    fn test_export_eps_header() {
        let result = VectorizationResult {
            paths: vec![],
            palette: Palette { colors: vec![] },
            dimensions: (100, 200),
            segmentation: Segmentation {
                regions: vec![],
                label_map: vec![],
                width: 100,
                height: 200,
            },
        };
        let mut output = Vec::new();
        export_eps(&result, &mut output).unwrap();
        let eps = String::from_utf8(output).unwrap();
        assert!(eps.contains("%!PS-Adobe-3.0 EPSF-3.0"));
        assert!(eps.contains("%%BoundingBox: 0 0 100 200"));
        assert!(eps.contains("%%EOF"));
    }

    #[test]
    fn test_export_eps_with_path() {
        let result = VectorizationResult {
            paths: vec![VectorPath {
                segments: vec![BezierSegment {
                    curve: CubicBezier {
                        p0: Point::new(10.0, 20.0),
                        p1: Point::new(30.0, 20.0),
                        p2: Point::new(30.0, 40.0),
                        p3: Point::new(10.0, 40.0),
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
        export_eps(&result, &mut output).unwrap();
        let eps = String::from_utf8(output).unwrap();
        assert!(eps.contains("setrgbcolor"));
        assert!(eps.contains("newpath"));
        assert!(eps.contains("moveto"));
        assert!(eps.contains("curveto"));
        assert!(eps.contains("closepath"));
        assert!(eps.contains("fill"));
        // Y should be flipped (100 - 20 = 80)
        assert!(eps.contains("10.000 80.000 moveto"));
    }
}
