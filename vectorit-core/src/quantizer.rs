use palette::{FromColor, Lab, Srgb};

use crate::types::{LabColor, Palette, QuantizedImage, RawImage, Result, VectorItError};

/// Quantize an image to a fixed number of colors using K-means++ in CIE Lab space.
pub fn quantize(image: &RawImage, num_colors: u16) -> Result<QuantizedImage> {
    let num_colors = num_colors as usize;
    if num_colors == 0 {
        return Err(VectorItError::QuantizationFailed(
            "num_colors must be > 0".into(),
        ));
    }

    // Convert pixels to Lab in chunks to avoid allocating full Lab image at once
    const CHUNK_SIZE: usize = 65536;
    let pixel_count = image.pixels.len();
    let mut lab_pixels: Vec<Lab> = Vec::with_capacity(pixel_count);
    for chunk in image.pixels.chunks(CHUNK_SIZE) {
        lab_pixels.extend(chunk.iter().map(|rgba| {
            let srgb = Srgb::new(
                rgba[0] as f32 / 255.0,
                rgba[1] as f32 / 255.0,
                rgba[2] as f32 / 255.0,
            );
            Lab::from_color(srgb)
        }));
    }

    // K-means++ initialization
    let mut centroids = kmeans_plus_plus_init(&lab_pixels, num_colors);

    // K-means iteration
    let max_iterations = 50;
    let convergence_threshold = 0.01_f32;
    let mut labels = vec![0u16; lab_pixels.len()];

    for _ in 0..max_iterations {
        // Assignment step
        for (i, pixel) in lab_pixels.iter().enumerate() {
            let mut best_dist = f32::MAX;
            let mut best_idx = 0u16;
            for (c_idx, centroid) in centroids.iter().enumerate() {
                let dist = lab_distance_sq(pixel, centroid);
                if dist < best_dist {
                    best_dist = dist;
                    best_idx = c_idx as u16;
                }
            }
            labels[i] = best_idx;
        }

        // Update step
        let mut new_centroids = vec![Lab::new(0.0, 0.0, 0.0); num_colors];
        let mut counts = vec![0u32; num_colors];

        for (i, pixel) in lab_pixels.iter().enumerate() {
            let idx = labels[i] as usize;
            new_centroids[idx].l += pixel.l;
            new_centroids[idx].a += pixel.a;
            new_centroids[idx].b += pixel.b;
            counts[idx] += 1;
        }

        let mut max_movement = 0.0_f32;
        for (i, centroid) in new_centroids.iter_mut().enumerate() {
            if counts[i] > 0 {
                centroid.l /= counts[i] as f32;
                centroid.a /= counts[i] as f32;
                centroid.b /= counts[i] as f32;
            } else {
                // Keep old centroid for empty clusters
                *centroid = centroids[i];
            }
            let movement = lab_distance_sq(&centroids[i], centroid).sqrt();
            if movement > max_movement {
                max_movement = movement;
            }
        }

        centroids = new_centroids;

        if max_movement < convergence_threshold {
            break;
        }
    }

    let palette = Palette {
        colors: centroids
            .iter()
            .map(|c| LabColor {
                l: c.l,
                a: c.a,
                b: c.b,
            })
            .collect(),
    };

    Ok(QuantizedImage {
        width: image.width,
        height: image.height,
        labels,
        palette,
    })
}

/// K-means++ initialization: select initial centroids with distance-weighted probability.
fn kmeans_plus_plus_init(pixels: &[Lab], k: usize) -> Vec<Lab> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let n = pixels.len();
    if n == 0 || k == 0 {
        return vec![];
    }

    // Deterministic seed based on pixel count
    let mut hasher = DefaultHasher::new();
    n.hash(&mut hasher);
    let mut seed = hasher.finish();

    let mut rng = move || -> f64 {
        // Simple xorshift64
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed as f64) / (u64::MAX as f64)
    };

    let mut centroids = Vec::with_capacity(k);

    // First centroid: pick based on seed
    let first_idx = (rng() * n as f64) as usize % n;
    centroids.push(pixels[first_idx]);

    // Remaining centroids
    let mut distances = vec![f32::MAX; n];

    for _ in 1..k {
        // Update distances to nearest centroid
        let last_centroid = centroids.last().unwrap();
        for (i, pixel) in pixels.iter().enumerate() {
            let d = lab_distance_sq(pixel, last_centroid);
            if d < distances[i] {
                distances[i] = d;
            }
        }

        // Weighted random selection
        let total: f64 = distances.iter().map(|d| *d as f64).sum();
        if total <= 0.0 {
            // All pixels are same color
            centroids.push(pixels[0]);
            continue;
        }

        let threshold = rng() * total;
        let mut cumulative = 0.0_f64;
        let mut chosen = 0;
        for (i, d) in distances.iter().enumerate() {
            cumulative += *d as f64;
            if cumulative >= threshold {
                chosen = i;
                break;
            }
        }
        centroids.push(pixels[chosen]);
    }

    centroids
}

#[inline]
fn lab_distance_sq(a: &Lab, b: &Lab) -> f32 {
    let dl = a.l - b.l;
    let da = a.a - b.a;
    let db = a.b - b.b;
    dl * dl + da * da + db * db
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_solid_image(r: u8, g: u8, b: u8, w: u32, h: u32) -> RawImage {
        let pixels = vec![[r, g, b, 255]; (w * h) as usize];
        RawImage {
            width: w,
            height: h,
            pixels,
            has_alpha: false,
        }
    }

    #[test]
    fn test_solid_color_quantizes_to_one() {
        let img = make_solid_image(255, 0, 0, 2, 2);
        let result = quantize(&img, 1).unwrap();
        assert_eq!(result.palette.colors.len(), 1);
        assert!(result.labels.iter().all(|&l| l == 0));
    }

    #[test]
    fn test_four_color_image() {
        let mut pixels = Vec::new();
        // 2x2 image with 4 distinct colors
        pixels.push([255, 0, 0, 255]); // red
        pixels.push([0, 255, 0, 255]); // green
        pixels.push([0, 0, 255, 255]); // blue
        pixels.push([255, 255, 0, 255]); // yellow
        let img = RawImage {
            width: 2,
            height: 2,
            pixels,
            has_alpha: false,
        };
        let result = quantize(&img, 4).unwrap();
        assert_eq!(result.palette.colors.len(), 4);
    }

    #[test]
    fn test_convergence_within_limit() {
        // Large-ish solid image should converge quickly
        let img = make_solid_image(128, 128, 128, 10, 10);
        let result = quantize(&img, 3);
        assert!(result.is_ok());
    }
}
