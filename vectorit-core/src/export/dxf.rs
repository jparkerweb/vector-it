use std::io::Write;

use crate::types::{Result, RgbColor, VectorizationResult, VectorItError};

/// Map an RGB color to the nearest DXF ACI (AutoCAD Color Index).
/// ACI has 256 indexed colors; this is inherently lossy.
fn rgb_to_aci(color: &RgbColor) -> u8 {
    // Standard ACI color mapping for the 7 primary colors
    // 1=Red, 2=Yellow, 3=Green, 4=Cyan, 5=Blue, 6=Magenta, 7=White/Black
    let r = color.r as f64 / 255.0;
    let g = color.g as f64 / 255.0;
    let b = color.b as f64 / 255.0;

    // Check near-black
    if r < 0.1 && g < 0.1 && b < 0.1 {
        return 7; // Use white/black (adapts to background)
    }

    // Simple nearest-primary mapping
    let candidates: [(u8, f64, f64, f64); 7] = [
        (1, 1.0, 0.0, 0.0), // Red
        (2, 1.0, 1.0, 0.0), // Yellow
        (3, 0.0, 1.0, 0.0), // Green
        (4, 0.0, 1.0, 1.0), // Cyan
        (5, 0.0, 0.0, 1.0), // Blue
        (6, 1.0, 0.0, 1.0), // Magenta
        (7, 1.0, 1.0, 1.0), // White
    ];

    let mut best_aci = 7u8;
    let mut best_dist = f64::MAX;
    for &(aci, cr, cg, cb) in &candidates {
        let dist = (r - cr).powi(2) + (g - cg).powi(2) + (b - cb).powi(2);
        if dist < best_dist {
            best_dist = dist;
            best_aci = aci;
        }
    }
    best_aci
}

/// Export vectorization result as DXF using SPLINE entities (degree 3).
/// Targets AutoCAD 2018 (AC1032) format.
pub fn export_dxf_spline(result: &VectorizationResult, writer: &mut impl Write) -> Result<()> {
    let mut drawing = dxf::Drawing::new();
    drawing.header.version = dxf::enums::AcadVersion::R2018;

    for path in &result.paths {
        if path.segments.is_empty() {
            continue;
        }

        let aci = rgb_to_aci(&path.fill_color);

        // Build a single spline from all Bézier segments in this path
        let mut control_points: Vec<dxf::Point> = Vec::new();
        let mut knots: Vec<f64> = Vec::new();

        for (seg_idx, seg) in path.segments.iter().enumerate() {
            let c = &seg.curve;
            if seg_idx == 0 {
                control_points.push(dxf::Point::new(c.p0.x, c.p0.y, 0.0));
            }
            control_points.push(dxf::Point::new(c.p1.x, c.p1.y, 0.0));
            control_points.push(dxf::Point::new(c.p2.x, c.p2.y, 0.0));
            control_points.push(dxf::Point::new(c.p3.x, c.p3.y, 0.0));
        }

        // Build clamped cubic B-spline knot vector
        let n_ctrl = control_points.len();
        let degree = 3;
        let n_knots = n_ctrl + degree + 1;
        knots.clear();
        for i in 0..n_knots {
            if i <= degree {
                knots.push(0.0);
            } else if i >= n_knots - degree - 1 {
                knots.push((n_knots - 2 * degree - 1) as f64);
            } else {
                knots.push((i - degree) as f64);
            }
        }

        let mut spline = dxf::entities::Spline::default();
        spline.degree_of_curve = degree as i32;
        spline.control_points = control_points;
        spline.knot_values = knots;

        let mut entity = dxf::entities::Entity::new(dxf::entities::EntityType::Spline(spline));
        entity.common.layer = "0".to_string();
        entity.common.color = dxf::Color::from_index(aci);

        drawing.add_entity(entity);
    }

    let mut buf = Vec::new();
    drawing
        .save(&mut buf)
        .map_err(|e| VectorItError::ExportFailed(format!("DXF write error: {}", e)))?;
    writer
        .write_all(&buf)
        .map_err(|e| VectorItError::ExportFailed(e.to_string()))?;

    Ok(())
}

/// Export vectorization result as DXF using LWPOLYLINE entities.
/// Approximates each Bézier curve by evaluating at `segments_per_curve` evenly-spaced t values.
/// This mode is for CNC/laser cutters that don't support splines.
pub fn export_dxf_polyline(
    result: &VectorizationResult,
    writer: &mut impl Write,
    segments_per_curve: u16,
) -> Result<()> {
    let mut drawing = dxf::Drawing::new();
    drawing.header.version = dxf::enums::AcadVersion::R2018;

    let segs = segments_per_curve.max(1) as usize;

    for path in &result.paths {
        if path.segments.is_empty() {
            continue;
        }

        let aci = rgb_to_aci(&path.fill_color);

        let mut vertices: Vec<dxf::LwPolylineVertex> = Vec::new();

        for (seg_idx, seg) in path.segments.iter().enumerate() {
            let start_t = if seg_idx == 0 { 0 } else { 1 };
            for i in start_t..=segs {
                let t = i as f64 / segs as f64;
                let pt = seg.curve.eval(t);
                vertices.push(dxf::LwPolylineVertex {
                    x: pt.x,
                    y: pt.y,
                    ..Default::default()
                });
            }
        }

        let mut polyline = dxf::entities::LwPolyline::default();
        polyline.vertices = vertices;
        if path.is_closed {
            polyline.flags = 1; // Closed polyline flag
        }

        let mut entity =
            dxf::entities::Entity::new(dxf::entities::EntityType::LwPolyline(polyline));
        entity.common.layer = "0".to_string();
        entity.common.color = dxf::Color::from_index(aci);

        drawing.add_entity(entity);
    }

    let mut buf = Vec::new();
    drawing
        .save(&mut buf)
        .map_err(|e| VectorItError::ExportFailed(format!("DXF write error: {}", e)))?;
    writer
        .write_all(&buf)
        .map_err(|e| VectorItError::ExportFailed(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    fn test_result() -> VectorizationResult {
        VectorizationResult {
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
        }
    }

    #[test]
    fn test_export_dxf_spline_produces_output() {
        let result = test_result();
        let mut output = Vec::new();
        export_dxf_spline(&result, &mut output).unwrap();
        let dxf_str = String::from_utf8_lossy(&output);
        assert!(dxf_str.contains("SPLINE"));
    }

    #[test]
    fn test_export_dxf_polyline_produces_output() {
        let result = test_result();
        let mut output = Vec::new();
        export_dxf_polyline(&result, &mut output, 8).unwrap();
        let dxf_str = String::from_utf8_lossy(&output);
        assert!(dxf_str.contains("LWPOLYLINE"));
    }

    #[test]
    fn test_rgb_to_aci() {
        assert_eq!(rgb_to_aci(&RgbColor { r: 255, g: 0, b: 0 }), 1); // Red
        assert_eq!(rgb_to_aci(&RgbColor { r: 0, g: 255, b: 0 }), 3); // Green
        assert_eq!(rgb_to_aci(&RgbColor { r: 0, g: 0, b: 255 }), 5); // Blue
        assert_eq!(rgb_to_aci(&RgbColor { r: 0, g: 0, b: 0 }), 7); // Black -> 7
    }
}
