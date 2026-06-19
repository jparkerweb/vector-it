use palette::{FromColor, Lab, Srgb};

use crate::types::{AaPixelInfo, QuantizedImage, RawImage, Segmentation};

/// Detect anti-aliasing pixels in the image.
///
/// For each pixel, checks if its Lab color is intermediate between two adjacent
/// region colors (within 30% blend threshold). Returns information about each
/// detected AA pixel including the two regions it borders and the blend ratio.
pub fn detect_aa_pixels(image: &RawImage, quantized: &QuantizedImage, seg: &Segmentation) -> Vec<AaPixelInfo> {
    let w = image.width as usize;
    let h = image.height as usize;
    let mut aa_pixels = Vec::new();

    // Convert quantized palette to Lab for comparison
    let palette_lab: Vec<Lab> = quantized
        .palette
        .colors
        .iter()
        .map(|c| Lab::new(c.l, c.a, c.b))
        .collect();

    // For each pixel, check if it's intermediate between two neighboring region colors
    for y in 0..h {
        for x in 0..w {
            let idx = y * w + x;
            let pixel_label = quantized.labels[idx];

            // Convert this pixel's actual color to Lab
            let rgba = image.pixels[idx];
            let srgb = Srgb::new(
                rgba[0] as f32 / 255.0,
                rgba[1] as f32 / 255.0,
                rgba[2] as f32 / 255.0,
            );
            let pixel_lab: Lab = Lab::from_color(srgb);

            // Check 4-connected neighbors for different labels
            let neighbors = [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ];

            for &(nx, ny) in &neighbors {
                if nx >= w || ny >= h {
                    continue;
                }
                let nidx = ny * w + nx;
                let neighbor_label = quantized.labels[nidx];

                if neighbor_label == pixel_label {
                    continue;
                }

                let label_a = pixel_label as usize;
                let label_b = neighbor_label as usize;

                if label_a >= palette_lab.len() || label_b >= palette_lab.len() {
                    continue;
                }

                let color_a = &palette_lab[label_a];
                let color_b = &palette_lab[label_b];

                // Check if pixel is intermediate between the two colors
                if let Some(blend_ratio) = compute_blend_ratio(&pixel_lab, color_a, color_b) {
                    if blend_ratio > 0.05 && blend_ratio < 0.95 {
                        // Use segmentation label_map for actual region IDs
                        aa_pixels.push(AaPixelInfo {
                            x: x as u32,
                            y: y as u32,
                            region_a: seg.label_map[idx],
                            region_b: seg.label_map[nidx],
                            blend_ratio,
                        });
                        break; // Only record once per pixel
                    }
                }
            }
        }
    }

    aa_pixels
}

/// Compute the blend ratio of a pixel between two colors in Lab space.
/// Returns Some(ratio) where 0.0 = exactly color_a, 1.0 = exactly color_b,
/// or None if the pixel is not on the line between the two colors (within threshold).
fn compute_blend_ratio(pixel: &Lab, color_a: &Lab, color_b: &Lab) -> Option<f64> {
    let ab_dist = lab_distance(color_a, color_b);
    if ab_dist < 1.0 {
        return None; // Colors too similar
    }

    let ap_dist = lab_distance(color_a, pixel);
    let bp_dist = lab_distance(color_b, pixel);

    // Check if pixel lies approximately on the line between a and b
    let total_via_pixel = ap_dist + bp_dist;
    let deviation = (total_via_pixel - ab_dist) / ab_dist;

    // 15% blend threshold — pixel must be close to the interpolation line
    // between the two region colors. Lower = stricter AA detection.
    if deviation > 0.15 {
        return None;
    }

    let ratio = ap_dist / ab_dist;
    Some(ratio.clamp(0.0, 1.0) as f64)
}

fn lab_distance(a: &Lab, b: &Lab) -> f32 {
    let dl = a.l - b.l;
    let da = a.a - b.a;
    let db = a.b - b.b;
    (dl * dl + da * da + db * db).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{LabColor, Palette, QuantizedImage, RawImage, Region, Segmentation};

    fn make_solid_segmentation() -> Segmentation {
        Segmentation {
            regions: vec![Region { id: 0, color_index: 0, pixel_count: 16 }],
            label_map: vec![0; 16],
            width: 4,
            height: 4,
        }
    }

    #[test]
    fn test_no_aa_in_solid_image() {
        let image = RawImage {
            width: 4,
            height: 4,
            pixels: vec![[255, 0, 0, 255]; 16],
            has_alpha: false,
        };
        let quantized = QuantizedImage {
            width: 4,
            height: 4,
            labels: vec![0; 16],
            palette: Palette {
                colors: vec![LabColor { l: 53.23, a: 80.11, b: 67.22 }],
            },
        };
        let seg = make_solid_segmentation();
        let result = detect_aa_pixels(&image, &quantized, &seg);
        assert!(result.is_empty());
    }

    #[test]
    fn test_aa_detected_at_boundary() {
        // 4x1 strip: pure red | blended | blended | pure blue
        let pixels = vec![
            [255, 0, 0, 255],   // pure red
            [192, 0, 63, 255],  // mostly red
            [63, 0, 192, 255],  // mostly blue
            [0, 0, 255, 255],   // pure blue
        ];
        let image = RawImage {
            width: 4,
            height: 1,
            pixels,
            has_alpha: false,
        };
        // Label all as either 0 (red) or 1 (blue)
        let quantized = QuantizedImage {
            width: 4,
            height: 1,
            labels: vec![0, 0, 1, 1],
            palette: Palette {
                colors: vec![
                    LabColor { l: 53.23, a: 80.11, b: 67.22 },  // red
                    LabColor { l: 32.30, a: 79.20, b: -107.86 }, // blue
                ],
            },
        };
        let seg = Segmentation {
            regions: vec![
                Region { id: 0, color_index: 0, pixel_count: 2 },
                Region { id: 1, color_index: 1, pixel_count: 2 },
            ],
            label_map: vec![0, 0, 1, 1],
            width: 4,
            height: 1,
        };
        let result = detect_aa_pixels(&image, &quantized, &seg);
        // Should detect AA pixels at boundary positions
        assert!(!result.is_empty());
    }

    #[test]
    fn test_blend_ratio_is_valid() {
        let a = Lab::new(50.0, 0.0, 0.0);
        let b = Lab::new(100.0, 0.0, 0.0);
        let mid = Lab::new(75.0, 0.0, 0.0);
        let ratio = compute_blend_ratio(&mid, &a, &b);
        assert!(ratio.is_some());
        let r = ratio.unwrap();
        assert!((r - 0.5).abs() < 0.05);
    }
}
