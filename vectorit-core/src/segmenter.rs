use crate::types::{QuantizedImage, Region, Segmentation};

/// Segment a quantized image into connected regions using flood-fill.
pub fn segment(image: &QuantizedImage) -> Segmentation {
    let w = image.width as usize;
    let h = image.height as usize;
    let total = w * h;

    let mut label_map = vec![u32::MAX; total];
    let mut regions: Vec<Region> = Vec::new();
    let mut region_id: u32 = 0;

    for y in 0..h {
        for x in 0..w {
            let idx = y * w + x;
            if label_map[idx] != u32::MAX {
                continue;
            }

            let color_index = image.labels[idx];
            let pixel_count = flood_fill(
                &image.labels,
                &mut label_map,
                w,
                h,
                x,
                y,
                color_index,
                region_id,
            );

            regions.push(Region {
                id: region_id,
                color_index,
                pixel_count,
            });
            region_id += 1;
        }
    }

    Segmentation {
        regions,
        label_map,
        width: image.width,
        height: image.height,
    }
}

/// Flood-fill from (start_x, start_y) marking all 4-connected pixels with same color_index.
fn flood_fill(
    labels: &[u16],
    label_map: &mut [u32],
    w: usize,
    h: usize,
    start_x: usize,
    start_y: usize,
    color_index: u16,
    region_id: u32,
) -> u32 {
    let mut stack = vec![(start_x, start_y)];
    let mut count = 0u32;

    while let Some((x, y)) = stack.pop() {
        let idx = y * w + x;
        if label_map[idx] != u32::MAX {
            continue;
        }
        if labels[idx] != color_index {
            continue;
        }

        label_map[idx] = region_id;
        count += 1;

        if x > 0 {
            stack.push((x - 1, y));
        }
        if x + 1 < w {
            stack.push((x + 1, y));
        }
        if y > 0 {
            stack.push((x, y - 1));
        }
        if y + 1 < h {
            stack.push((x, y + 1));
        }
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{LabColor, Palette, QuantizedImage};

    #[test]
    fn test_solid_image_one_region() {
        let img = QuantizedImage {
            width: 3,
            height: 3,
            labels: vec![0; 9],
            palette: Palette {
                colors: vec![LabColor { l: 50.0, a: 0.0, b: 0.0 }],
            },
        };
        let seg = segment(&img);
        assert_eq!(seg.regions.len(), 1);
        assert_eq!(seg.regions[0].pixel_count, 9);
    }

    #[test]
    fn test_checkerboard_four_regions() {
        // 2x2 checkerboard with 2 colors - each pixel is isolated
        let img = QuantizedImage {
            width: 2,
            height: 2,
            labels: vec![0, 1, 1, 0],
            palette: Palette {
                colors: vec![
                    LabColor { l: 50.0, a: 0.0, b: 0.0 },
                    LabColor { l: 80.0, a: 0.0, b: 0.0 },
                ],
            },
        };
        let seg = segment(&img);
        // In a 2x2 checkerboard, each pixel of same color is NOT 4-connected to the other
        assert_eq!(seg.regions.len(), 4);
    }
}
