use std::collections::HashSet;

use crate::types::{Boundary, Point, Segmentation};

/// Trace boundaries of all regions using marching squares with midpoint interpolation.
pub fn trace_boundaries(seg: &Segmentation, width: u32, height: u32) -> Vec<Boundary> {
    let w = width as usize;
    let h = height as usize;
    let mut boundaries = Vec::new();

    for region in &seg.regions {
        let mask: Vec<bool> = seg.label_map.iter().map(|&label| label == region.id).collect();
        boundaries.extend(trace_region_boundaries(&mask, w, h, region.id));
    }

    boundaries
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Edge {
    Left,
    Top,
    Right,
    Bottom,
}

impl Edge {
    fn opposite(self) -> Self {
        match self {
            Edge::Left => Edge::Right,
            Edge::Top => Edge::Bottom,
            Edge::Right => Edge::Left,
            Edge::Bottom => Edge::Top,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CellPath {
    entry: Edge,
    exit: Edge,
}

/// Trace every contour for a single region.
fn trace_region_boundaries(mask: &[bool], w: usize, h: usize, region_id: u32) -> Vec<Boundary> {
    let get_val = |x: i32, y: i32| -> bool {
        if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
            false
        } else {
            mask[y as usize * w + x as usize]
        }
    };

    let mut visited = HashSet::new();
    let mut boundaries = Vec::new();

    for cy in 0..=h as i32 {
        for cx in 0..=w as i32 {
            let case = cell_case(cx, cy, &get_val);
            if case == 0 || case == 15 {
                continue;
            }

            let center_inside = sample_cell_center(cx, cy, &get_val);
            for path in cell_paths(case, center_inside) {
                if visited.contains(&(cx, cy, path)) {
                    continue;
                }

                if let Some(boundary) = trace_contour(
                    cx,
                    cy,
                    path,
                    w,
                    h,
                    region_id,
                    &get_val,
                    &mut visited,
                ) {
                    if boundary.points.len() >= 4 {
                        boundaries.push(boundary);
                    }
                }
            }
        }
    }

    boundaries
}

fn trace_contour(
    start_cx: i32,
    start_cy: i32,
    start_path: CellPath,
    w: usize,
    h: usize,
    region_id: u32,
    get_val: &impl Fn(i32, i32) -> bool,
    visited: &mut HashSet<(i32, i32, CellPath)>,
) -> Option<Boundary> {
    let mut points = Vec::new();
    let mut current_cx = start_cx;
    let mut current_cy = start_cy;
    let mut current_path = start_path;
    let max_steps = (w + 1) * (h + 1) * 2;

    for _ in 0..max_steps {
        if !visited.insert((current_cx, current_cy, current_path)) {
            break;
        }

        push_point(&mut points, edge_point(current_cx, current_cy, current_path.entry));
        push_point(&mut points, edge_point(current_cx, current_cy, current_path.exit));

        let (next_cx, next_cy) = step_to_neighbor(current_cx, current_cy, current_path.exit);
        if next_cx < 0 || next_cy < 0 || next_cx > w as i32 || next_cy > h as i32 {
            break;
        }

        let next_case = cell_case(next_cx, next_cy, get_val);
        if next_case == 0 || next_case == 15 {
            break;
        }

        let next_center_inside = sample_cell_center(next_cx, next_cy, get_val);
        let next_entry = current_path.exit.opposite();
        let Some(next_path) = find_path_for_entry(next_case, next_center_inside, next_entry) else {
            break;
        };

        if next_cx == start_cx && next_cy == start_cy && next_path == start_path {
            break;
        }

        current_cx = next_cx;
        current_cy = next_cy;
        current_path = next_path;
    }

    if points.len() < 3 {
        return None;
    }

    if points.first() != points.last() {
        points.push(points[0]);
    }

    Some(Boundary {
        region_id,
        points,
        is_closed: true,
    })
}

fn push_point(points: &mut Vec<Point>, point: Point) {
    if points.last() != Some(&point) {
        points.push(point);
    }
}

/// Compute the marching squares case (0-15) for a cell at (cx, cy).
/// Cell corners map to pixels: TL=(cx-1,cy-1), TR=(cx,cy-1), BL=(cx-1,cy), BR=(cx,cy)
fn cell_case(cx: i32, cy: i32, get_val: &impl Fn(i32, i32) -> bool) -> u8 {
    let tl = get_val(cx - 1, cy - 1) as u8;
    let tr = get_val(cx, cy - 1) as u8;
    let bl = get_val(cx - 1, cy) as u8;
    let br = get_val(cx, cy) as u8;
    (tl << 3) | (tr << 2) | (br << 1) | bl
}

/// Sample the region value at the center of a marching-squares cell.
fn sample_cell_center(cx: i32, cy: i32, get_val: &impl Fn(i32, i32) -> bool) -> bool {
    get_val(cx, cy)
}

fn cell_paths(case: u8, center_inside: bool) -> Vec<CellPath> {
    use Edge::{Bottom, Left, Right, Top};

    match case {
        0 | 15 => vec![],
        1 => vec![CellPath { entry: Left, exit: Bottom }],
        2 => vec![CellPath { entry: Bottom, exit: Right }],
        3 => vec![CellPath { entry: Left, exit: Right }],
        4 => vec![CellPath { entry: Right, exit: Top }],
        5 => {
            if center_inside {
                vec![
                    CellPath { entry: Top, exit: Left },
                    CellPath { entry: Bottom, exit: Right },
                ]
            } else {
                vec![
                    CellPath { entry: Right, exit: Top },
                    CellPath { entry: Left, exit: Bottom },
                ]
            }
        }
        6 => vec![CellPath { entry: Bottom, exit: Top }],
        7 => vec![CellPath { entry: Left, exit: Top }],
        8 => vec![CellPath { entry: Top, exit: Left }],
        9 => vec![CellPath { entry: Top, exit: Bottom }],
        10 => {
            if center_inside {
                vec![
                    CellPath { entry: Right, exit: Top },
                    CellPath { entry: Bottom, exit: Left },
                ]
            } else {
                vec![
                    CellPath { entry: Top, exit: Left },
                    CellPath { entry: Bottom, exit: Right },
                ]
            }
        }
        11 => vec![CellPath { entry: Top, exit: Right }],
        12 => vec![CellPath { entry: Right, exit: Left }],
        13 => vec![CellPath { entry: Right, exit: Bottom }],
        14 => vec![CellPath { entry: Bottom, exit: Left }],
        _ => vec![],
    }
}

fn find_path_for_entry(case: u8, center_inside: bool, entry: Edge) -> Option<CellPath> {
    cell_paths(case, center_inside)
        .into_iter()
        .find(|path| path.entry == entry)
}

fn edge_point(cx: i32, cy: i32, edge: Edge) -> Point {
    let x = cx as f64;
    let y = cy as f64;

    match edge {
        Edge::Left => Point::new(x - 0.5, y),
        Edge::Top => Point::new(x, y - 0.5),
        Edge::Right => Point::new(x + 0.5, y),
        Edge::Bottom => Point::new(x, y + 0.5),
    }
}

fn step_to_neighbor(cx: i32, cy: i32, exit: Edge) -> (i32, i32) {
    match exit {
        Edge::Right => (cx + 1, cy),
        Edge::Bottom => (cx, cy + 1),
        Edge::Left => (cx - 1, cy),
        Edge::Top => (cx, cy - 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Region, Segmentation};

    #[test]
    fn test_single_square_region() {
        // 3x3 image with a 1x1 region in the center
        // 0 0 0
        // 0 1 0
        // 0 0 0
        let label_map = vec![0, 0, 0, 0, 1, 0, 0, 0, 0];
        let seg = Segmentation {
            regions: vec![
                Region { id: 0, color_index: 0, pixel_count: 8 },
                Region { id: 1, color_index: 1, pixel_count: 1 },
            ],
            label_map,
            width: 3,
            height: 3,
        };

        let boundaries = trace_boundaries(&seg, 3, 3);
        // Region 1 should have a boundary
        let region1_boundary = boundaries.iter().find(|b| b.region_id == 1);
        assert!(region1_boundary.is_some());
        let boundary = region1_boundary.unwrap();
        assert!(boundary.is_closed);
        assert!(boundary.points.len() >= 4); // At least 4 points for a square
    }

    #[test]
    fn test_region_with_hole_traces_multiple_boundaries() {
        let label_map = vec![
            0, 0, 0, 0, 0,
            0, 1, 1, 1, 0,
            0, 1, 0, 1, 0,
            0, 1, 1, 1, 0,
            0, 0, 0, 0, 0,
        ];
        let seg = Segmentation {
            regions: vec![
                Region { id: 0, color_index: 0, pixel_count: 17 },
                Region { id: 1, color_index: 1, pixel_count: 8 },
            ],
            label_map,
            width: 5,
            height: 5,
        };

        let boundaries = trace_boundaries(&seg, 5, 5);
        let region1_boundaries: Vec<_> = boundaries.iter().filter(|b| b.region_id == 1).collect();

        assert_eq!(region1_boundaries.len(), 2);
        assert!(region1_boundaries.iter().all(|boundary| boundary.is_closed));
    }

    #[test]
    fn test_saddle_case_traces_disconnected_diagonals() {
        // 2x2 checkerboard for region 1:
        // 0 1
        // 1 0
        let label_map = vec![0, 1, 1, 0];
        let seg = Segmentation {
            regions: vec![
                Region { id: 0, color_index: 0, pixel_count: 2 },
                Region { id: 1, color_index: 1, pixel_count: 2 },
            ],
            label_map,
            width: 2,
            height: 2,
        };

        let boundaries = trace_boundaries(&seg, 2, 2);
        let region1_boundaries: Vec<_> = boundaries.iter().filter(|b| b.region_id == 1).collect();

        assert_eq!(region1_boundaries.len(), 2);
        assert!(region1_boundaries.iter().all(|boundary| boundary.points.len() >= 4));
    }
}
