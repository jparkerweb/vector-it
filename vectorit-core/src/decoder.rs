use std::path::Path;

use image::{GenericImageView, DynamicImage};

use crate::types::{RawImage, Result, VectorItError};

/// Decode an image file into a RawImage.
/// Supports PNG, JPG, BMP, GIF (first frame only), TIFF, and SVG.
pub fn decode_image(path: &Path) -> Result<RawImage> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext == "svg" {
        return decode_svg(path);
    }

    let img: DynamicImage = match ext.as_str() {
        "gif" => {
            // GIF: decode first frame only, handle transparent color index
            let file = std::fs::File::open(path)
                .map_err(|e| VectorItError::ImageDecode(e.to_string()))?;
                let reader = std::io::BufReader::new(file);
                let decoder = image::codecs::gif::GifDecoder::new(reader)
                    .map_err(|e| VectorItError::ImageDecode(e.to_string()))?;
                DynamicImage::from_decoder(decoder)
                    .map_err(|e| VectorItError::ImageDecode(e.to_string()))?
            }
        "tiff" | "tif" => {
            // TIFF: the image crate handles LZW, PackBits, Deflate, and CMYK→RGB conversion
            image::open(path).map_err(|e| VectorItError::ImageDecode(e.to_string()))?
        }
        _ => {
            image::open(path).map_err(|e| VectorItError::ImageDecode(e.to_string()))?
        }
    };

    let (width, height) = img.dimensions();
    let rgba = img.to_rgba8();
    let has_alpha = rgba.pixels().any(|p| p.0[3] < 255);

    let pixels: Vec<[u8; 4]> = rgba.pixels().map(|p| p.0).collect();

    Ok(RawImage {
        width,
        height,
        pixels,
        has_alpha,
    })
}

/// Decode an SVG file by rasterizing it into a RawImage.
/// Renders at the SVG's intrinsic size, defaulting to 800×600 if no dimensions are specified.
fn decode_svg(path: &Path) -> Result<RawImage> {
    let svg_data = std::fs::read(path)
        .map_err(|e| VectorItError::ImageDecode(format!("Failed to read SVG file: {e}")))?;

    let tree = resvg::usvg::Tree::from_data(&svg_data, &resvg::usvg::Options::default())
        .map_err(|e| VectorItError::ImageDecode(format!("Failed to parse SVG: {e}")))?;

    let size = tree.size();
    let width = size.width().ceil() as u32;
    let height = size.height().ceil() as u32;

    if width == 0 || height == 0 {
        return Err(VectorItError::ImageDecode(
            "SVG has zero-size dimensions".to_string(),
        ));
    }

    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| VectorItError::ImageDecode("Failed to create pixmap for SVG".to_string()))?;

    resvg::render(&tree, resvg::usvg::Transform::default(), &mut pixmap.as_mut());

    let data = pixmap.data();
    let pixel_count = (width * height) as usize;
    let mut pixels = Vec::with_capacity(pixel_count);
    let mut has_alpha = false;

    for i in 0..pixel_count {
        let offset = i * 4;
        let a = data[offset + 3];
        // resvg outputs premultiplied alpha; convert to straight alpha
        let (r, g, b) = if a == 0 {
            (0, 0, 0)
        } else if a == 255 {
            (data[offset], data[offset + 1], data[offset + 2])
        } else {
            has_alpha = true;
            let af = a as f32 / 255.0;
            (
                (data[offset] as f32 / af).round() as u8,
                (data[offset + 1] as f32 / af).round() as u8,
                (data[offset + 2] as f32 / af).round() as u8,
            )
        };
        if a < 255 {
            has_alpha = true;
        }
        pixels.push([r, g, b, a]);
    }

    Ok(RawImage {
        width,
        height,
        pixels,
        has_alpha,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_nonexistent_file() {
        let result = decode_image(Path::new("nonexistent.png"));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VectorItError::ImageDecode(_)));
    }
}
