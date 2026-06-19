use crate::types::{CubicBezier, Point};

/// Fit cubic Bézier curves to a polyline, splitting at corners.
/// Implements Schneider's algorithm with Newton-Raphson refinement.
pub fn fit_curves(points: &[Point], corners: &[bool], tolerance: f64) -> Vec<CubicBezier> {
    if points.len() < 2 {
        return vec![];
    }

    let mut curves = Vec::new();

    // Split at corners into segments
    let mut segment_start = 0;
    for i in 1..points.len() {
        if corners[i] || i == points.len() - 1 {
            let segment = &points[segment_start..=i];
            if segment.len() >= 2 {
                let left_tangent = compute_tangent(points, segment_start, true);
                let right_tangent = compute_tangent(points, i, false);
                let mut segment_curves = Vec::new();
                fit_cubic(segment, left_tangent, right_tangent, tolerance, &mut segment_curves);
                curves.extend(segment_curves);
            }
            segment_start = i;
        }
    }

    curves
}

/// Recursively fit a cubic Bézier to a point sequence.
fn fit_cubic(
    points: &[Point],
    left_tangent: Point,
    right_tangent: Point,
    tolerance: f64,
    result: &mut Vec<CubicBezier>,
) {
    if points.len() == 2 {
        // Degenerate case: straight line
        let dist = points[0].distance_to(&points[1]) / 3.0;
        let bezier = CubicBezier {
            p0: points[0],
            p1: Point::new(
                points[0].x + left_tangent.x * dist,
                points[0].y + left_tangent.y * dist,
            ),
            p2: Point::new(
                points[1].x + right_tangent.x * dist,
                points[1].y + right_tangent.y * dist,
            ),
            p3: points[1],
        };
        result.push(bezier);
        return;
    }

    // Parameterize points by chord length
    let mut u = chord_length_parameterize(points);

    // Fit and refine
    let mut bezier = generate_bezier(points, &u, left_tangent, right_tangent);

    // Newton-Raphson refinement (max 4 iterations)
    for _ in 0..4 {
        let (max_error, _split_point) = compute_max_error(points, &bezier, &u);
        if max_error < tolerance {
            result.push(bezier);
            return;
        }
        u = reparameterize(points, &u, &bezier);
        bezier = generate_bezier(points, &u, left_tangent, right_tangent);
    }

    // Check final error
    let (max_error, split_point) = compute_max_error(points, &bezier, &u);
    if max_error < tolerance {
        result.push(bezier);
        return;
    }

    // Split at worst point and recurse
    let split = split_point.max(1).min(points.len() - 2);
    let left_points = &points[..=split];
    let right_points = &points[split..];

    let center_tangent = compute_center_tangent(points, split);
    let neg_center = Point::new(-center_tangent.x, -center_tangent.y);

    fit_cubic(left_points, left_tangent, neg_center, tolerance, result);
    fit_cubic(right_points, center_tangent, right_tangent, tolerance, result);
}

/// Compute tangent direction at a point.
fn compute_tangent(points: &[Point], index: usize, is_left: bool) -> Point {
    let (dx, dy) = if is_left {
        if index + 1 < points.len() {
            (points[index + 1].x - points[index].x, points[index + 1].y - points[index].y)
        } else {
            (1.0, 0.0)
        }
    } else {
        if index > 0 {
            (points[index - 1].x - points[index].x, points[index - 1].y - points[index].y)
        } else {
            (-1.0, 0.0)
        }
    };
    normalize(dx, dy)
}

fn compute_center_tangent(points: &[Point], index: usize) -> Point {
    let prev = if index > 0 { index - 1 } else { 0 };
    let next = if index + 1 < points.len() { index + 1 } else { points.len() - 1 };
    let dx = points[next].x - points[prev].x;
    let dy = points[next].y - points[prev].y;
    normalize(dx, dy)
}

fn normalize(dx: f64, dy: f64) -> Point {
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-12 {
        Point::new(1.0, 0.0)
    } else {
        Point::new(dx / len, dy / len)
    }
}

/// Parameterize by chord length (0.0 to 1.0).
fn chord_length_parameterize(points: &[Point]) -> Vec<f64> {
    let mut u = vec![0.0; points.len()];
    for i in 1..points.len() {
        u[i] = u[i - 1] + points[i].distance_to(&points[i - 1]);
    }
    let total = u[points.len() - 1];
    if total > 1e-12 {
        for val in u.iter_mut() {
            *val /= total;
        }
    }
    u
}

/// Generate a cubic Bézier that best fits the parameterized points.
fn generate_bezier(
    points: &[Point],
    u: &[f64],
    left_tangent: Point,
    right_tangent: Point,
) -> CubicBezier {
    let n = points.len();
    let first = points[0];
    let last = points[n - 1];

    // Compute A matrix elements
    let mut a = vec![[Point::new(0.0, 0.0); 2]; n];
    for i in 0..n {
        let b1 = bernstein1(u[i]);
        let b2 = bernstein2(u[i]);
        a[i][0] = Point::new(left_tangent.x * b1, left_tangent.y * b1);
        a[i][1] = Point::new(right_tangent.x * b2, right_tangent.y * b2);
    }

    // Compute C and X matrices
    let mut c = [[0.0f64; 2]; 2];
    let mut x = [0.0f64; 2];

    for i in 0..n {
        c[0][0] += a[i][0].x * a[i][0].x + a[i][0].y * a[i][0].y;
        c[0][1] += a[i][0].x * a[i][1].x + a[i][0].y * a[i][1].y;
        c[1][0] = c[0][1];
        c[1][1] += a[i][1].x * a[i][1].x + a[i][1].y * a[i][1].y;

        let b0 = bernstein0(u[i]);
        let b1 = bernstein1(u[i]);
        let b2 = bernstein2(u[i]);
        let b3 = bernstein3(u[i]);

        let tmp_x = points[i].x - (first.x * b0 + first.x * b1 + last.x * b2 + last.x * b3);
        let tmp_y = points[i].y - (first.y * b0 + first.y * b1 + last.y * b2 + last.y * b3);

        x[0] += a[i][0].x * tmp_x + a[i][0].y * tmp_y;
        x[1] += a[i][1].x * tmp_x + a[i][1].y * tmp_y;
    }

    // Solve 2x2 system
    let det = c[0][0] * c[1][1] - c[0][1] * c[1][0];
    let seg_len = first.distance_to(&last);
    let seg_third = seg_len / 3.0;
    // Clamp alpha to seg_len/2 to prevent control points from flying away.
    // Standard well-fitting cubics have alpha ~ seg_len/3; anything beyond
    // seg_len/2 indicates a poor tangent direction or ill-conditioned solve.
    let max_alpha = (seg_len / 2.0).max(0.5);
    let (alpha_l, alpha_r) = if det.abs() < 1e-12 {
        (seg_third, seg_third)
    } else {
        let al = (x[0] * c[1][1] - x[1] * c[0][1]) / det;
        let ar = (c[0][0] * x[1] - c[1][0] * x[0]) / det;
        (
            if al < 0.0 { seg_third } else { al.min(max_alpha) },
            if ar < 0.0 { seg_third } else { ar.min(max_alpha) },
        )
    };

    CubicBezier {
        p0: first,
        p1: Point::new(
            first.x + left_tangent.x * alpha_l,
            first.y + left_tangent.y * alpha_l,
        ),
        p2: Point::new(
            last.x + right_tangent.x * alpha_r,
            last.y + right_tangent.y * alpha_r,
        ),
        p3: last,
    }
}

/// Reparameterize using Newton-Raphson.
fn reparameterize(points: &[Point], u: &[f64], bezier: &CubicBezier) -> Vec<f64> {
    u.iter()
        .enumerate()
        .map(|(i, &t)| newton_raphson_root(bezier, &points[i], t))
        .collect()
}

fn newton_raphson_root(bezier: &CubicBezier, point: &Point, t: f64) -> f64 {
    let q = bezier.eval(t);
    let q1 = bezier_derivative(bezier, t);
    let q2 = bezier_second_derivative(bezier, t);

    let numerator = (q.x - point.x) * q1.x + (q.y - point.y) * q1.y;
    let denominator =
        q1.x * q1.x + q1.y * q1.y + (q.x - point.x) * q2.x + (q.y - point.y) * q2.y;

    if denominator.abs() < 1e-12 {
        t
    } else {
        (t - numerator / denominator).clamp(0.0, 1.0)
    }
}

fn bezier_derivative(b: &CubicBezier, t: f64) -> Point {
    let mt = 1.0 - t;
    Point::new(
        3.0 * mt * mt * (b.p1.x - b.p0.x)
            + 6.0 * mt * t * (b.p2.x - b.p1.x)
            + 3.0 * t * t * (b.p3.x - b.p2.x),
        3.0 * mt * mt * (b.p1.y - b.p0.y)
            + 6.0 * mt * t * (b.p2.y - b.p1.y)
            + 3.0 * t * t * (b.p3.y - b.p2.y),
    )
}

fn bezier_second_derivative(b: &CubicBezier, t: f64) -> Point {
    let mt = 1.0 - t;
    Point::new(
        6.0 * mt * (b.p2.x - 2.0 * b.p1.x + b.p0.x)
            + 6.0 * t * (b.p3.x - 2.0 * b.p2.x + b.p1.x),
        6.0 * mt * (b.p2.y - 2.0 * b.p1.y + b.p0.y)
            + 6.0 * t * (b.p3.y - 2.0 * b.p2.y + b.p1.y),
    )
}

/// Compute max error between points and fitted Bézier.
fn compute_max_error(points: &[Point], bezier: &CubicBezier, u: &[f64]) -> (f64, usize) {
    let mut max_dist = 0.0;
    let mut split_point = points.len() / 2;

    for i in 1..(points.len() - 1) {
        let p = bezier.eval(u[i]);
        let dist = points[i].distance_to(&p);
        if dist > max_dist {
            max_dist = dist;
            split_point = i;
        }
    }

    (max_dist, split_point)
}

// Bernstein basis functions
fn bernstein0(t: f64) -> f64 { (1.0 - t).powi(3) }
fn bernstein1(t: f64) -> f64 { 3.0 * t * (1.0 - t).powi(2) }
fn bernstein2(t: f64) -> f64 { 3.0 * t * t * (1.0 - t) }
fn bernstein3(t: f64) -> f64 { t.powi(3) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collinear_points() {
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(2.0, 0.0),
            Point::new(3.0, 0.0),
        ];
        let corners = vec![true, false, false, true];
        let curves = fit_curves(&points, &corners, 1.0);
        assert!(!curves.is_empty());
        // Should produce a near-degenerate Bézier (control points close to line)
        let c = &curves[0];
        assert!((c.p0.y).abs() < 0.01);
        assert!((c.p3.y).abs() < 0.01);
    }

    #[test]
    fn test_semicircle_fit() {
        // Generate semicircle points
        let n = 20;
        let points: Vec<Point> = (0..=n)
            .map(|i| {
                let angle = std::f64::consts::PI * (i as f64) / (n as f64);
                Point::new(angle.cos() * 10.0, angle.sin() * 10.0)
            })
            .collect();
        let mut corners = vec![false; points.len()];
        corners[0] = true;
        corners[points.len() - 1] = true;

        let curves = fit_curves(&points, &corners, 1.0);
        assert!(!curves.is_empty());
    }
}
