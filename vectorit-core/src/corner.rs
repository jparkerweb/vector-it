use crate::types::Point;

/// Detect corners in a polyline based on angle threshold.
/// Returns a boolean vector parallel to input indicating corner status.
pub fn detect_corners(points: &[Point], threshold_degrees: f64) -> Vec<bool> {
    let n = points.len();
    if n < 3 {
        return vec![true; n];
    }

    let threshold_rad = threshold_degrees.to_radians();
    let mut corners = vec![false; n];

    // Always mark first and last as corners for open paths
    corners[0] = true;
    corners[n - 1] = true;

    for i in 1..(n - 1) {
        let angle = compute_angle(&points[i - 1], &points[i], &points[i + 1]);
        if angle < threshold_rad {
            corners[i] = true;
        }
    }

    // Handle wraparound for closed paths (first point == last point case)
    if n >= 3 && points[0].distance_to(&points[n - 1]) < 1e-6 {
        // Check angle at the wrap point
        let angle = compute_angle(&points[n - 2], &points[0], &points[1]);
        if angle < threshold_rad {
            corners[0] = true;
            corners[n - 1] = true;
        }
    }

    corners
}

/// Compute the angle at point `b` formed by segments ba and bc.
/// Returns the angle in radians (π = straight line, 0 = fold-back/U-turn).
fn compute_angle(a: &Point, b: &Point, c: &Point) -> f64 {
    let ba_x = a.x - b.x;
    let ba_y = a.y - b.y;
    let bc_x = c.x - b.x;
    let bc_y = c.y - b.y;

    let dot = ba_x * bc_x + ba_y * bc_y;
    let cross = ba_x * bc_y - ba_y * bc_x;

    let angle = cross.atan2(dot).abs();
    // Return the interior angle (smaller angle)
    if angle > std::f64::consts::PI {
        2.0 * std::f64::consts::PI - angle
    } else {
        angle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_90_degree_corner_detected() {
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(5.0, 0.0),
            Point::new(5.0, 5.0),
        ];
        let corners = detect_corners(&points, 120.0); // 90° < 120° threshold
        assert!(corners[1]); // The 90° turn should be a corner
    }

    #[test]
    fn test_170_degree_not_corner() {
        // Nearly straight line
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(5.0, 0.1), // Slight deviation
            Point::new(10.0, 0.0),
        ];
        let corners = detect_corners(&points, 60.0);
        assert!(!corners[1]); // ~170° angle should NOT be a corner with 60° threshold
    }
}
