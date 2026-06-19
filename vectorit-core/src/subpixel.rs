use crate::types::{AaPixelInfo, Boundary, Segmentation};

/// Refine boundary points using anti-aliasing pixel data for sub-pixel edge placement.
///
/// For each boundary point that corresponds to an AA pixel location, shifts the
/// point position by interpolating between pixel center and neighbor pixel center
/// using the blend_ratio.
///
/// Shift formula: `new_pos = pixel_center + (neighbor_center - pixel_center) * blend_ratio`
pub fn refine_boundaries(
    boundaries: &mut [Boundary],
    aa_pixels: &[AaPixelInfo],
    _seg: &Segmentation,
) {
    if aa_pixels.is_empty() {
        return;
    }

    // Build a lookup map from (x, y) -> AaPixelInfo for O(1) access
    let mut aa_map = std::collections::HashMap::new();
    for aa in aa_pixels {
        aa_map.entry((aa.x, aa.y)).or_insert_with(Vec::new).push(aa);
    }

    for boundary in boundaries.iter_mut() {
        for point in boundary.points.iter_mut() {
            // Boundary points are in continuous coordinates — find the nearest pixel
            let px = point.x.round() as i64;
            let py = point.y.round() as i64;

            if px < 0 || py < 0 {
                continue;
            }

            let px = px as u32;
            let py = py as u32;

            // Check this pixel and immediate neighbors for AA data
            let search_coords = [
                (px, py),
                (px.wrapping_sub(1), py),
                (px + 1, py),
                (px, py.wrapping_sub(1)),
                (px, py + 1),
            ];

            let mut best_aa: Option<&AaPixelInfo> = None;
            let mut best_dist = f64::MAX;

            for &(sx, sy) in &search_coords {
                if let Some(aa_list) = aa_map.get(&(sx, sy)) {
                    for aa in aa_list {
                        // Check if this AA pixel is relevant to this boundary's region
                        if aa.region_a == boundary.region_id || aa.region_b == boundary.region_id {
                            let dist = ((point.x - aa.x as f64).powi(2)
                                + (point.y - aa.y as f64).powi(2))
                            .sqrt();
                            if dist < best_dist {
                                best_dist = dist;
                                best_aa = Some(aa);
                            }
                        }
                    }
                }
            }

            if let Some(aa) = best_aa {
                if best_dist > 1.5 {
                    continue; // Too far from AA pixel
                }

                // Determine the direction: pixel center → neighbor center
                let pixel_center_x = aa.x as f64 + 0.5;
                let pixel_center_y = aa.y as f64 + 0.5;

                // The "neighbor" direction is from region_a toward region_b
                // Use the boundary point's relative position to determine shift direction
                let dx = point.x - pixel_center_x;
                let dy = point.y - pixel_center_y;
                let len = (dx * dx + dy * dy).sqrt();

                if len < 1e-6 {
                    continue;
                }

                // Normalize direction
                let nx = dx / len;
                let ny = dy / len;

                // Shift amount based on blend ratio
                // blend_ratio 0 = on region_a, 1 = on region_b
                let shift = aa.blend_ratio - 0.5; // -0.5 to +0.5

                point.x += nx * shift;
                point.y += ny * shift;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AaPixelInfo, Boundary, Point, Region, Segmentation};

    fn make_test_segmentation() -> Segmentation {
        Segmentation {
            regions: vec![
                Region { id: 0, color_index: 0, pixel_count: 4 },
                Region { id: 1, color_index: 1, pixel_count: 4 },
            ],
            label_map: vec![0, 0, 1, 1, 0, 0, 1, 1],
            width: 4,
            height: 2,
        }
    }

    #[test]
    fn test_no_refinement_without_aa() {
        let seg = make_test_segmentation();
        let mut boundaries = vec![Boundary {
            region_id: 0,
            points: vec![Point::new(2.0, 0.5), Point::new(2.0, 1.5)],
            is_closed: false,
        }];
        let original = boundaries[0].points.clone();
        refine_boundaries(&mut boundaries, &[], &seg);
        assert_eq!(boundaries[0].points, original);
    }

    #[test]
    fn test_refinement_shifts_points() {
        let seg = make_test_segmentation();
        let aa_pixels = vec![AaPixelInfo {
            x: 2,
            y: 0,
            region_a: 0,
            region_b: 1,
            blend_ratio: 0.7,
        }];
        let mut boundaries = vec![Boundary {
            region_id: 0,
            points: vec![Point::new(2.0, 0.5)],
            is_closed: false,
        }];
        let orig_x = boundaries[0].points[0].x;
        refine_boundaries(&mut boundaries, &aa_pixels, &seg);
        // Point should have shifted
        let new_x = boundaries[0].points[0].x;
        assert!((new_x - orig_x).abs() > 0.001, "Expected point to shift");
    }

    #[test]
    fn test_refinement_preserves_distant_points() {
        let seg = make_test_segmentation();
        let aa_pixels = vec![AaPixelInfo {
            x: 100,
            y: 100,
            region_a: 0,
            region_b: 1,
            blend_ratio: 0.5,
        }];
        let mut boundaries = vec![Boundary {
            region_id: 0,
            points: vec![Point::new(0.0, 0.0)],
            is_closed: false,
        }];
        let original = boundaries[0].points.clone();
        refine_boundaries(&mut boundaries, &aa_pixels, &seg);
        assert_eq!(boundaries[0].points, original);
    }
}
