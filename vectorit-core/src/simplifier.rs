use crate::types::Point;

/// Simplify a polyline using a hybrid approach:
/// 1. VTracer-style area/length penalty for initial reduction (O(n))
/// 2. Douglas-Peucker refinement pass for quality assurance
///
/// The area/length penalty naturally retains more points in
/// high-curvature areas while aggressively simplifying straight runs.
pub fn simplify(points: &[Point], tolerance: f64) -> Vec<Point> {
    if points.len() <= 2 {
        return points.to_vec();
    }

    // Step 1: Staircase removal — remove collinear points first
    let cleaned = remove_collinear(points, tolerance * 0.1);
    if cleaned.len() <= 2 {
        return cleaned;
    }

    // Step 2: Area/length penalty simplification (VTracer-inspired)
    // This retains points where the local curvature is high.
    let area_simplified = area_penalty_simplify(&cleaned, tolerance);
    if area_simplified.len() <= 2 {
        return area_simplified;
    }

    // Step 3: Douglas-Peucker refinement to catch anything the greedy pass missed
    douglas_peucker(&area_simplified, tolerance)
}

/// Remove points that are collinear with their neighbors (staircase artifacts).
/// Uses signed triangle area: if area/base_length < threshold, point is redundant.
fn remove_collinear(points: &[Point], threshold: f64) -> Vec<Point> {
    if points.len() <= 2 {
        return points.to_vec();
    }

    let mut result = vec![points[0]];

    for i in 1..points.len() - 1 {
        let a = result.last().unwrap();
        let b = &points[i];
        let c = &points[i + 1];

        let base_len = a.distance_to(c);
        if base_len < 1e-12 {
            continue;
        }

        // Signed triangle area / base length = perpendicular height
        let area = ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)).abs();
        let height = area / base_len;

        if height > threshold {
            result.push(*b);
        }
    }

    result.push(*points.last().unwrap());
    result
}

/// VTracer-inspired simplification using area/length penalty.
/// Greedily extends segments until the penalty exceeds tolerance.
/// Penalty = max(area(triangle(start, mid, end)) / distance(start, end))
/// for all intermediate points.
fn area_penalty_simplify(points: &[Point], tolerance: f64) -> Vec<Point> {
    let n = points.len();
    if n <= 2 {
        return points.to_vec();
    }

    let mut result = vec![points[0]];
    let mut seg_start = 0;

    for i in 2..n {
        let start = &points[seg_start];
        let end = &points[i];
        let base_len = start.distance_to(end);

        if base_len < 1e-12 {
            continue;
        }

        // Check max penalty for all points in the current segment
        let mut max_penalty = 0.0;
        for k in (seg_start + 1)..i {
            let mid = &points[k];
            let area = ((mid.x - start.x) * (end.y - start.y)
                - (end.x - start.x) * (mid.y - start.y))
                .abs();
            let penalty = area / base_len;
            if penalty > max_penalty {
                max_penalty = penalty;
            }
        }

        if max_penalty > tolerance {
            // The previous point was the last acceptable extension
            result.push(points[i - 1]);
            seg_start = i - 1;
        }
    }

    result.push(points[n - 1]);
    result
}

/// Classic Douglas-Peucker as a refinement pass.
fn douglas_peucker(points: &[Point], tolerance: f64) -> Vec<Point> {
    if points.len() <= 2 {
        return points.to_vec();
    }

    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;

    dp_recursive(points, 0, points.len() - 1, tolerance, &mut keep);

    points
        .iter()
        .zip(keep.iter())
        .filter(|(_, k)| **k)
        .map(|(p, _)| *p)
        .collect()
}

fn dp_recursive(
    points: &[Point],
    start: usize,
    end: usize,
    tolerance: f64,
    keep: &mut [bool],
) {
    if end <= start + 1 {
        return;
    }

    let mut max_dist = 0.0;
    let mut max_idx = start;

    for i in (start + 1)..end {
        let d = perpendicular_distance(&points[i], &points[start], &points[end]);
        if d > max_dist {
            max_dist = d;
            max_idx = i;
        }
    }

    if max_dist > tolerance {
        keep[max_idx] = true;
        dp_recursive(points, start, max_idx, tolerance, keep);
        dp_recursive(points, max_idx, end, tolerance, keep);
    }
}

/// Compute perpendicular distance from point to line segment (start, end).
fn perpendicular_distance(point: &Point, start: &Point, end: &Point) -> f64 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let len_sq = dx * dx + dy * dy;

    if len_sq < 1e-12 {
        return point.distance_to(start);
    }

    let numerator = ((point.x - start.x) * dy - (point.y - start.y) * dx).abs();
    numerator / len_sq.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_straight_line_simplified() {
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(2.0, 0.0),
            Point::new(3.0, 0.0),
        ];
        let result = simplify(&points, 1.0);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], points[0]);
        assert_eq!(result[1], points[3]);
    }

    #[test]
    fn test_l_shape_preserved() {
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(5.0, 0.0),
            Point::new(5.0, 5.0),
        ];
        let result = simplify(&points, 1.0);
        assert_eq!(result.len(), 3);
    }
}
