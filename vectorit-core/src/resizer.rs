use crate::types::RawImage;

/// Resize an image for analysis if it exceeds the megapixel threshold.
/// Uses pixel-averaging (box filter) downsampling to preserve color information.
/// Returns the original image unmodified if it's already within the limit.
pub fn resize_for_analysis(image: &RawImage, max_megapixels: f64) -> RawImage {
    let pixel_count = image.width as f64 * image.height as f64;
    let max_pixels = max_megapixels * 1_000_000.0;

    if pixel_count <= max_pixels {
        return image.clone();
    }

    let scale = (max_pixels / pixel_count).sqrt();
    let new_width = ((image.width as f64 * scale).round() as u32).max(1);
    let new_height = ((image.height as f64 * scale).round() as u32).max(1);

    let mut output = Vec::with_capacity((new_width * new_height) as usize);

    for out_y in 0..new_height {
        for out_x in 0..new_width {
            // Determine the source rectangle that maps to this output pixel
            let src_x_start = (out_x as f64 / new_width as f64 * image.width as f64) as u32;
            let src_x_end =
                (((out_x + 1) as f64 / new_width as f64 * image.width as f64).ceil() as u32)
                    .min(image.width);
            let src_y_start = (out_y as f64 / new_height as f64 * image.height as f64) as u32;
            let src_y_end =
                (((out_y + 1) as f64 / new_height as f64 * image.height as f64).ceil() as u32)
                    .min(image.height);

            // Average all source pixels in this rectangle
            let mut r_sum = 0u64;
            let mut g_sum = 0u64;
            let mut b_sum = 0u64;
            let mut a_sum = 0u64;
            let mut count = 0u64;

            for sy in src_y_start..src_y_end {
                for sx in src_x_start..src_x_end {
                    let idx = (sy * image.width + sx) as usize;
                    let pixel = image.pixels[idx];
                    r_sum += pixel[0] as u64;
                    g_sum += pixel[1] as u64;
                    b_sum += pixel[2] as u64;
                    a_sum += pixel[3] as u64;
                    count += 1;
                }
            }

            if count > 0 {
                output.push([
                    (r_sum / count) as u8,
                    (g_sum / count) as u8,
                    (b_sum / count) as u8,
                    (a_sum / count) as u8,
                ]);
            } else {
                output.push([0, 0, 0, 255]);
            }
        }
    }

    RawImage {
        width: new_width,
        height: new_height,
        pixels: output,
        has_alpha: image.has_alpha,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_resize_when_under_limit() {
        let image = RawImage {
            width: 100,
            height: 100,
            pixels: vec![[255, 0, 0, 255]; 10_000],
            has_alpha: false,
        };
        let result = resize_for_analysis(&image, 1.0);
        assert_eq!(result.width, 100);
        assert_eq!(result.height, 100);
    }

    #[test]
    fn test_resize_when_over_limit() {
        let image = RawImage {
            width: 2000,
            height: 2000, // 4 MP
            pixels: vec![[255, 128, 0, 255]; 4_000_000],
            has_alpha: false,
        };
        let result = resize_for_analysis(&image, 1.0); // max 1 MP
        let mp = result.width as f64 * result.height as f64 / 1_000_000.0;
        assert!(mp <= 1.1, "Resized image should be ~1MP, got {:.2}MP", mp);
        assert!(result.width < 2000);
        assert!(result.height < 2000);
    }

    #[test]
    fn test_aspect_ratio_preserved() {
        let image = RawImage {
            width: 4000,
            height: 2000, // 8 MP, 2:1 ratio
            pixels: vec![[0, 0, 0, 255]; 8_000_000],
            has_alpha: false,
        };
        let result = resize_for_analysis(&image, 2.0);
        let ratio = result.width as f64 / result.height as f64;
        assert!(
            (ratio - 2.0).abs() < 0.1,
            "Aspect ratio should be ~2:1, got {:.2}",
            ratio
        );
    }
}
