use crate::types::{CubicBezier, Point};

/// Optimize curves by merging adjacent pairs that can be represented
/// as a single Bézier within tolerance, and converting near-straight
/// curves to true straight lines.
pub fn optimize(curves: &[CubicBezier], tolerance: f64) -> Vec<CubicBezier> {
    if curves.is_empty() {
        return vec![];
    }

    // Step 1: Straighten near-straight curves (use generous threshold to
    // smooth out sub-pixel wobble from anti-aliased boundaries)
    let straighten_threshold = tolerance.max(1.0);
    let mut straightened: Vec<CubicBezier> = curves
        .iter()
        .map(|c| straighten_if_linear(c, straighten_threshold))
        .collect();

    if straightened.len() <= 1 {
        return straightened;
    }

    // Step 2: Merge adjacent curves iteratively
    let mut changed = true;
    let max_passes = 5;
    let mut pass = 0;
    while changed && pass < max_passes {
        changed = false;
        pass += 1;
        let mut merged: Vec<CubicBezier> = vec![straightened[0]];

        for i in 1..straightened.len() {
            let last = merged.last().unwrap();
            let current = &straightened[i];

            if let Some(m) = try_merge(last, current, tolerance) {
                *merged.last_mut().unwrap() = m;
                changed = true;
            } else {
                merged.push(*current);
            }
        }

        straightened = merged;
        if straightened.len() <= 1 {
            break;
        }
    }

    straightened
}

/// If a Bézier curve is nearly a straight line, replace it with a
/// degenerate Bézier (control points on the line).
fn straighten_if_linear(curve: &CubicBezier, tolerance: f64) -> CubicBezier {
    let p0 = curve.p0;
    let p3 = curve.p3;

    // Check if control points are close to the p0-p3 line
    let d1 = point_to_line_distance(&curve.p1, &p0, &p3);
    let d2 = point_to_line_distance(&curve.p2, &p0, &p3);

    if d1 < tolerance && d2 < tolerance {
        // Make it a perfect straight line as a degenerate Bézier
        let third = 1.0 / 3.0;
        CubicBezier {
            p0,
            p1: Point::new(
                p0.x + (p3.x - p0.x) * third,
                p0.y + (p3.y - p0.y) * third,
            ),
            p2: Point::new(
                p0.x + (p3.x - p0.x) * (2.0 * third),
                p0.y + (p3.y - p0.y) * (2.0 * third),
            ),
            p3,
        }
    } else {
        *curve
    }
}

/// Try to merge two adjacent Bézier curves into one.
/// Uses least-squares fitting through sampled points from both curves.
fn try_merge(a: &CubicBezier, b: &CubicBezier, tolerance: f64) -> Option<CubicBezier> {
    // Check continuity
    if a.p3.distance_to(&b.p0) > 1e-6 {
        return None;
    }

    // Don't merge if either has a corner (large angle change)
    let a_end_tangent = Point::new(a.p3.x - a.p2.x, a.p3.y - a.p2.y);
    let b_start_tangent = Point::new(b.p1.x - b.p0.x, b.p1.y - b.p0.y);
    let dot = a_end_tangent.x * b_start_tangent.x + a_end_tangent.y * b_start_tangent.y;
    let a_len = (a_end_tangent.x.powi(2) + a_end_tangent.y.powi(2)).sqrt();
    let b_len = (b_start_tangent.x.powi(2) + b_start_tangent.y.powi(2)).sqrt();
    if a_len > 1e-6 && b_len > 1e-6 {
        let cos_angle = dot / (a_len * b_len);
        if cos_angle < 0.0 {
            // More than 90° angle change — definitely a corner, don't merge
            return None;
        }
    }

    // Sample both curves densely
    let n_samples = 20;
    let all_points: Vec<Point> = (0..=n_samples)
        .map(|i| {
            let t = i as f64 / n_samples as f64;
            if t <= 0.5 {
                a.eval(t * 2.0)
            } else {
                b.eval((t - 0.5) * 2.0)
            }
        })
        .collect();

    // Compute merged curve preserving end tangents
    let left_tangent = normalize_vec(a.p1.x - a.p0.x, a.p1.y - a.p0.y);
    let right_tangent = normalize_vec(b.p2.x - b.p3.x, b.p2.y - b.p3.y);

    // Use chord-length parameterization and fit
    let merged = fit_single_bezier(&all_points, left_tangent, right_tangent);

    // Check max error
    let max_err = max_fitting_error(&merged, &all_points);
    if max_err < tolerance {
        Some(merged)
    } else {
        None
    }
}

/// Fit a single cubic Bézier to a set of ordered points with given end tangents.
fn fit_single_bezier(points: &[Point], left_tan: Point, right_tan: Point) -> CubicBezier {
    let n = points.len();
    if n < 2 {
        return CubicBezier {
            p0: points[0],
            p1: points[0],
            p2: points[0],
            p3: points[0],
        };
    }

    let first = points[0];
    let last = points[n - 1];

    // Chord-length parameterization
    let mut u = vec![0.0; n];
    for i in 1..n {
        u[i] = u[i - 1] + points[i].distance_to(&points[i - 1]);
    }
    let total = u[n - 1];
    if total > 1e-12 {
        for v in u.iter_mut() {
            *v /= total;
        }
    }

    // Least-squares solve for alpha_l, alpha_r
    let mut c = [[0.0f64; 2]; 2];
    let mut x = [0.0f64; 2];

    for i in 0..n {
        let b0 = (1.0 - u[i]).powi(3);
        let b1 = 3.0 * u[i] * (1.0 - u[i]).powi(2);
        let b2 = 3.0 * u[i].powi(2) * (1.0 - u[i]);
        let b3 = u[i].powi(3);

        let a0 = Point::new(left_tan.x * b1, left_tan.y * b1);
        let a1 = Point::new(right_tan.x * b2, right_tan.y * b2);

        c[0][0] += a0.x * a0.x + a0.y * a0.y;
        c[0][1] += a0.x * a1.x + a0.y * a1.y;
        c[1][1] += a1.x * a1.x + a1.y * a1.y;

        let tmp_x = points[i].x - (first.x * (b0 + b1) + last.x * (b2 + b3));
        let tmp_y = points[i].y - (first.y * (b0 + b1) + last.y * (b2 + b3));

        x[0] += a0.x * tmp_x + a0.y * tmp_y;
        x[1] += a1.x * tmp_x + a1.y * tmp_y;
    }
    c[1][0] = c[0][1];

    let det = c[0][0] * c[1][1] - c[0][1] * c[1][0];
    let seg_len = first.distance_to(&last);
    let seg_third = seg_len / 3.0;
    let max_alpha = (seg_len / 2.0).max(0.5);
    let (alpha_l, alpha_r) = if det.abs() < 1e-12 {
        (seg_third, seg_third)
    } else {
        let al = (x[0] * c[1][1] - x[1] * c[0][1]) / det;
        let ar = (c[0][0] * x[1] - c[1][0] * x[0]) / det;
        (
            if al < 1e-6 { seg_third } else { al.min(max_alpha) },
            if ar < 1e-6 { seg_third } else { ar.min(max_alpha) },
        )
    };

    CubicBezier {
        p0: first,
        p1: Point::new(first.x + left_tan.x * alpha_l, first.y + left_tan.y * alpha_l),
        p2: Point::new(last.x + right_tan.x * alpha_r, last.y + right_tan.y * alpha_r),
        p3: last,
    }
}

fn max_fitting_error(bezier: &CubicBezier, points: &[Point]) -> f64 {
    let mut max_err = 0.0f64;
    let n_check = 100;
    for point in points {
        let mut min_dist = f64::MAX;
        for j in 0..=n_check {
            let t = j as f64 / n_check as f64;
            let bp = bezier.eval(t);
            let dist = point.distance_to(&bp);
            if dist < min_dist {
                min_dist = dist;
            }
        }
        if min_dist > max_err {
            max_err = min_dist;
        }
    }
    max_err
}

fn point_to_line_distance(point: &Point, line_start: &Point, line_end: &Point) -> f64 {
    let dx = line_end.x - line_start.x;
    let dy = line_end.y - line_start.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-12 {
        return point.distance_to(line_start);
    }
    let num = ((point.x - line_start.x) * dy - (point.y - line_start.y) * dx).abs();
    num / len_sq.sqrt()
}

fn normalize_vec(x: f64, y: f64) -> Point {
    let len = (x * x + y * y).sqrt();
    if len < 1e-12 {
        Point::new(1.0, 0.0)
    } else {
        Point::new(x / len, y / len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collinear_segments_merge() {
        let a = CubicBezier {
            p0: Point::new(0.0, 0.0),
            p1: Point::new(1.0, 0.0),
            p2: Point::new(2.0, 0.0),
            p3: Point::new(3.0, 0.0),
        };
        let b = CubicBezier {
            p0: Point::new(3.0, 0.0),
            p1: Point::new(4.0, 0.0),
            p2: Point::new(5.0, 0.0),
            p3: Point::new(6.0, 0.0),
        };
        let result = optimize(&[a, b], 1.0);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_straight_line_detection() {
        let curve = CubicBezier {
            p0: Point::new(0.0, 0.0),
            p1: Point::new(3.3, 0.01),
            p2: Point::new(6.6, -0.01),
            p3: Point::new(10.0, 0.0),
        };
        let result = straighten_if_linear(&curve, 0.1);
        // Control points should be on the line
        assert!((result.p1.y).abs() < 1e-10);
        assert!((result.p2.y).abs() < 1e-10);
    }
}
