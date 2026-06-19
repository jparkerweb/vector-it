use crate::types::{ImageType, RawImage};

/// Result of image type detection with confidence score.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DetectionResult {
    pub image_type: ImageType,
    pub confidence: f64,
}

/// Auto-detect image type (Photo, AntiAliased, or Aliased).
///
/// Detection logic:
/// - Compute edge frequency using Sobel filter on grayscale
/// - Compute color histogram entropy
/// - Classify: Photo if entropy > 6.0 AND edge frequency is continuous;
///   Aliased if edge transitions are strictly 1px; AntiAliased otherwise
pub fn detect_image_type(image: &RawImage) -> DetectionResult {
    let grayscale = to_grayscale(image);
    let (edge_magnitude, edge_frequency) = compute_sobel_edges(&grayscale, image.width, image.height);
    let entropy = compute_color_entropy(image);
    let has_continuous_edges = edge_frequency > 0.15;
    let has_sharp_transitions = check_sharp_transitions(&grayscale, image.width, image.height);

    if entropy > 6.0 && has_continuous_edges {
        let confidence = ((entropy - 6.0) / 2.0).min(1.0) * 0.7
            + (edge_frequency.min(1.0)) * 0.3;
        DetectionResult {
            image_type: ImageType::Photo,
            confidence: confidence.clamp(0.5, 1.0),
        }
    } else if has_sharp_transitions && !has_continuous_edges {
        let sharpness_score = if edge_magnitude > 0.0 {
            1.0 - edge_frequency.min(1.0)
        } else {
            0.8
        };
        DetectionResult {
            image_type: ImageType::Aliased,
            confidence: sharpness_score.clamp(0.5, 1.0),
        }
    } else {
        // Default: AntiAliased (logo with smoothing)
        let aa_score = if has_continuous_edges { 0.6 } else { 0.8 };
        DetectionResult {
            image_type: ImageType::AntiAliased,
            confidence: aa_score,
        }
    }
}

fn to_grayscale(image: &RawImage) -> Vec<f32> {
    image
        .pixels
        .iter()
        .map(|p| 0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32)
        .collect()
}

/// Compute Sobel edge magnitude and edge frequency (fraction of edge pixels).
fn compute_sobel_edges(gray: &[f32], width: u32, height: u32) -> (f64, f64) {
    let w = width as usize;
    let h = height as usize;
    if w < 3 || h < 3 {
        return (0.0, 0.0);
    }

    let mut edge_count = 0u64;
    let mut total_magnitude = 0.0f64;
    let edge_threshold = 30.0;
    let total_pixels = ((w - 2) * (h - 2)) as u64;

    for y in 1..(h - 1) {
        for x in 1..(w - 1) {
            let tl = gray[(y - 1) * w + (x - 1)];
            let tc = gray[(y - 1) * w + x];
            let tr = gray[(y - 1) * w + (x + 1)];
            let ml = gray[y * w + (x - 1)];
            let mr = gray[y * w + (x + 1)];
            let bl = gray[(y + 1) * w + (x - 1)];
            let bc = gray[(y + 1) * w + x];
            let br = gray[(y + 1) * w + (x + 1)];

            let gx = -tl + tr - 2.0 * ml + 2.0 * mr - bl + br;
            let gy = -tl - 2.0 * tc - tr + bl + 2.0 * bc + br;
            let mag = (gx * gx + gy * gy).sqrt();

            total_magnitude += mag as f64;
            if mag > edge_threshold {
                edge_count += 1;
            }
        }
    }

    let avg_magnitude = if total_pixels > 0 {
        total_magnitude / total_pixels as f64
    } else {
        0.0
    };
    let frequency = if total_pixels > 0 {
        edge_count as f64 / total_pixels as f64
    } else {
        0.0
    };

    (avg_magnitude, frequency)
}

/// Compute color histogram entropy (bits).
fn compute_color_entropy(image: &RawImage) -> f64 {
    // Quantize to 4-bit per channel (4096 bins)
    let mut histogram = vec![0u32; 4096];
    let total = image.pixels.len() as f64;

    for pixel in &image.pixels {
        let r = (pixel[0] >> 4) as usize;
        let g = (pixel[1] >> 4) as usize;
        let b = (pixel[2] >> 4) as usize;
        let bin = (r << 8) | (g << 4) | b;
        histogram[bin] += 1;
    }

    let mut entropy = 0.0f64;
    for &count in &histogram {
        if count > 0 {
            let p = count as f64 / total;
            entropy -= p * p.log2();
        }
    }

    entropy
}

/// Check if edge transitions are strictly 1px (no intermediate colors).
fn check_sharp_transitions(gray: &[f32], width: u32, height: u32) -> bool {
    let w = width as usize;
    let h = height as usize;
    if w < 3 || h < 3 {
        return true;
    }

    let edge_threshold = 30.0;
    let mut sharp_edges = 0u64;
    let mut total_edges = 0u64;

    // Sample horizontal transitions
    for y in 0..h {
        for x in 0..(w - 2) {
            let left = gray[y * w + x];
            let mid = gray[y * w + x + 1];
            let right = gray[y * w + x + 2];

            let diff_lr = (left - right).abs();
            if diff_lr > edge_threshold {
                total_edges += 1;
                // Sharp if middle is close to one side (no gradient)
                let diff_lm = (left - mid).abs();
                let diff_mr = (mid - right).abs();
                if diff_lm < edge_threshold * 0.3 || diff_mr < edge_threshold * 0.3 {
                    sharp_edges += 1;
                }
            }
        }
    }

    if total_edges == 0 {
        return true;
    }

    let sharp_ratio = sharp_edges as f64 / total_edges as f64;
    sharp_ratio > 0.8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_solid_image(r: u8, g: u8, b: u8, w: u32, h: u32) -> RawImage {
        RawImage {
            width: w,
            height: h,
            pixels: vec![[r, g, b, 255]; (w * h) as usize],
            has_alpha: false,
        }
    }

    #[test]
    fn test_solid_color_detected_as_aliased() {
        let img = make_solid_image(255, 0, 0, 10, 10);
        let result = detect_image_type(&img);
        // Solid color should be Aliased (no edges, no gradients)
        assert_eq!(result.image_type, ImageType::Aliased);
    }

    #[test]
    fn test_noisy_image_detected_as_photo() {
        // Create a noisy image with many distinct colors
        let mut pixels = Vec::new();
        for i in 0..10000u32 {
            let r = ((i * 7) % 256) as u8;
            let g = ((i * 13) % 256) as u8;
            let b = ((i * 23) % 256) as u8;
            pixels.push([r, g, b, 255]);
        }
        let img = RawImage {
            width: 100,
            height: 100,
            pixels,
            has_alpha: false,
        };
        let result = detect_image_type(&img);
        assert_eq!(result.image_type, ImageType::Photo);
    }

    #[test]
    fn test_two_color_sharp_image() {
        // Create a sharp 2-color image (black and white halves)
        let mut pixels = Vec::new();
        for _y in 0..20u32 {
            for x in 0..20u32 {
                if x < 10 {
                    pixels.push([0, 0, 0, 255]);
                } else {
                    pixels.push([255, 255, 255, 255]);
                }
            }
        }
        let img = RawImage {
            width: 20,
            height: 20,
            pixels,
            has_alpha: false,
        };
        let result = detect_image_type(&img);
        assert!(result.image_type == ImageType::Aliased || result.image_type == ImageType::AntiAliased);
    }

    #[test]
    fn test_confidence_in_range() {
        let img = make_solid_image(128, 128, 128, 10, 10);
        let result = detect_image_type(&img);
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
    }
}
