use crate::types::{BitmapFormat, CubicBezier, Point, Result, VectorizationResult, VectorItError};

/// Export vectorization result as a rasterized bitmap at the specified resolution.
/// Uses 4× SSAA (supersampling anti-aliasing) for edge quality.
pub fn export_bitmap(
    result: &VectorizationResult,
    width: u32,
    height: u32,
    format: BitmapFormat,
) -> Result<Vec<u8>> {
    let (orig_w, orig_h) = result.dimensions;
    let scale_x = width as f64 / orig_w as f64;
    let scale_y = height as f64 / orig_h as f64;

    // SSAA factor
    let ss = 2u32; // 2×2 = 4× supersampling
    let ss_w = width * ss;
    let ss_h = height * ss;

    // Rasterize at supersampled resolution
    let mut pixels = vec![[255u8, 255u8, 255u8, 255u8]; (ss_w * ss_h) as usize];

    for path in &result.paths {
        if path.segments.is_empty() {
            continue;
        }

        // Collect all scaled points for winding number test
        let segments: Vec<CubicBezier> = path
            .segments
            .iter()
            .map(|seg| {
                let c = &seg.curve;
                CubicBezier {
                    p0: Point::new(c.p0.x * scale_x, c.p0.y * scale_y),
                    p1: Point::new(c.p1.x * scale_x, c.p1.y * scale_y),
                    p2: Point::new(c.p2.x * scale_x, c.p2.y * scale_y),
                    p3: Point::new(c.p3.x * scale_x, c.p3.y * scale_y),
                }
            })
            .collect();

        // Compute bounding box to limit pixel iteration
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        for seg in &segments {
            for t_i in 0..=8 {
                let t = t_i as f64 / 8.0;
                let pt = seg.eval(t);
                min_x = min_x.min(pt.x);
                min_y = min_y.min(pt.y);
                max_x = max_x.max(pt.x);
                max_y = max_y.max(pt.y);
            }
        }

        let px_min_x = (min_x * ss as f64).floor().max(0.0) as u32;
        let px_min_y = (min_y * ss as f64).floor().max(0.0) as u32;
        let px_max_x = ((max_x * ss as f64).ceil() as u32).min(ss_w);
        let px_max_y = ((max_y * ss as f64).ceil() as u32).min(ss_h);

        let color = path.fill_color;

        // Linearize segments to polyline for winding number test
        let polyline = linearize_path(&segments, 16);

        for py in px_min_y..px_max_y {
            for px in px_min_x..px_max_x {
                let x = (px as f64 + 0.5) / ss as f64;
                let y = (py as f64 + 0.5) / ss as f64;
                let pt = Point::new(x, y);

                if winding_number(&pt, &polyline) != 0 {
                    let idx = (py * ss_w + px) as usize;
                    pixels[idx] = [color.r, color.g, color.b, 255];
                }
            }
        }
    }

    // Downsample from supersampled resolution to target resolution
    let mut final_pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let mut r = 0u32;
            let mut g = 0u32;
            let mut b = 0u32;
            let mut a = 0u32;
            for sy in 0..ss {
                for sx in 0..ss {
                    let idx = ((y * ss + sy) * ss_w + (x * ss + sx)) as usize;
                    let p = pixels[idx];
                    r += p[0] as u32;
                    g += p[1] as u32;
                    b += p[2] as u32;
                    a += p[3] as u32;
                }
            }
            let count = (ss * ss) as u32;
            final_pixels.push((r / count) as u8);
            final_pixels.push((g / count) as u8);
            final_pixels.push((b / count) as u8);
            final_pixels.push((a / count) as u8);
        }
    }

    // Encode to the requested format
    encode_bitmap(&final_pixels, width, height, format)
}

/// Linearize Bézier path to polyline points for winding number test.
fn linearize_path(segments: &[CubicBezier], steps: usize) -> Vec<Point> {
    let mut points = Vec::new();
    for seg in segments {
        for i in 0..steps {
            let t = i as f64 / steps as f64;
            points.push(seg.eval(t));
        }
    }
    // Close with the last point
    if let Some(last_seg) = segments.last() {
        points.push(last_seg.eval(1.0));
    }
    points
}

/// Compute the winding number of point p relative to polygon.
fn winding_number(p: &Point, polygon: &[Point]) -> i32 {
    let mut wn = 0i32;
    let n = polygon.len();
    if n < 2 {
        return 0;
    }

    for i in 0..n {
        let j = (i + 1) % n;
        let v1 = &polygon[i];
        let v2 = &polygon[j];

        if v1.y <= p.y {
            if v2.y > p.y {
                if is_left(v1, v2, p) > 0.0 {
                    wn += 1;
                }
            }
        } else if v2.y <= p.y {
            if is_left(v1, v2, p) < 0.0 {
                wn -= 1;
            }
        }
    }
    wn
}

fn is_left(p0: &Point, p1: &Point, p2: &Point) -> f64 {
    (p1.x - p0.x) * (p2.y - p0.y) - (p2.x - p0.x) * (p1.y - p0.y)
}

fn encode_bitmap(
    rgba_pixels: &[u8],
    width: u32,
    height: u32,
    format: BitmapFormat,
) -> Result<Vec<u8>> {
    use image::ImageEncoder;

    let mut buf = Vec::new();

    match format {
        BitmapFormat::Png => {
            let encoder = image::codecs::png::PngEncoder::new(&mut buf);
            encoder
                .write_image(rgba_pixels, width, height, image::ExtendedColorType::Rgba8)
                .map_err(|e| VectorItError::ExportFailed(format!("PNG encode error: {}", e)))?;
        }
        BitmapFormat::Bmp => {
            let encoder = image::codecs::bmp::BmpEncoder::new(&mut buf);
            encoder
                .write_image(rgba_pixels, width, height, image::ExtendedColorType::Rgba8)
                .map_err(|e| VectorItError::ExportFailed(format!("BMP encode error: {}", e)))?;
        }
        BitmapFormat::Jpg(quality) => {
            // JPEG doesn't support alpha — convert to RGB
            let mut rgb_pixels = Vec::with_capacity((width * height * 3) as usize);
            for chunk in rgba_pixels.chunks(4) {
                rgb_pixels.push(chunk[0]);
                rgb_pixels.push(chunk[1]);
                rgb_pixels.push(chunk[2]);
            }
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
            encoder
                .write_image(&rgb_pixels, width, height, image::ExtendedColorType::Rgb8)
                .map_err(|e| VectorItError::ExportFailed(format!("JPEG encode error: {}", e)))?;
        }
    }

    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    #[test]
    fn test_export_bitmap_png() {
        let result = VectorizationResult {
            paths: vec![VectorPath {
                segments: vec![
                    BezierSegment {
                        curve: CubicBezier {
                            p0: Point::new(0.0, 0.0),
                            p1: Point::new(50.0, 0.0),
                            p2: Point::new(50.0, 0.0),
                            p3: Point::new(50.0, 0.0),
                        },
                        is_corner_start: true,
                    },
                    BezierSegment {
                        curve: CubicBezier {
                            p0: Point::new(50.0, 0.0),
                            p1: Point::new(50.0, 50.0),
                            p2: Point::new(50.0, 50.0),
                            p3: Point::new(50.0, 50.0),
                        },
                        is_corner_start: true,
                    },
                    BezierSegment {
                        curve: CubicBezier {
                            p0: Point::new(50.0, 50.0),
                            p1: Point::new(0.0, 50.0),
                            p2: Point::new(0.0, 50.0),
                            p3: Point::new(0.0, 50.0),
                        },
                        is_corner_start: true,
                    },
                    BezierSegment {
                        curve: CubicBezier {
                            p0: Point::new(0.0, 50.0),
                            p1: Point::new(0.0, 0.0),
                            p2: Point::new(0.0, 0.0),
                            p3: Point::new(0.0, 0.0),
                        },
                        is_corner_start: true,
                    },
                ],
                fill_color: RgbColor {
                    r: 255,
                    g: 0,
                    b: 0,
                },
                is_closed: true,
                stroke_color: None,
                stroke_width: None,
            }],
            palette: Palette { colors: vec![] },
            dimensions: (100, 100),
            segmentation: Segmentation {
                regions: vec![],
                label_map: vec![],
                width: 100,
                height: 100,
            },
        };

        let png_data = export_bitmap(&result, 50, 50, BitmapFormat::Png).unwrap();
        // PNG magic bytes
        assert_eq!(&png_data[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }
}
