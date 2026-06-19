use arboard::Clipboard;
use image::ImageEncoder;

/// Read image data from the system clipboard (CF_BITMAP / CF_DIB on Windows).
/// Saves the image to a temp file in PNG format and returns the path.
pub fn read_clipboard_image() -> std::result::Result<String, String> {
    let mut clipboard = Clipboard::new().map_err(|e| format!("Clipboard init failed: {}", e))?;

    let img = clipboard
        .get_image()
        .map_err(|_| "No image found on clipboard".to_string())?;

    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join("vectorit_clipboard.png");

    // Convert arboard ImageData to PNG bytes
    let mut png_bytes = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
    encoder
        .write_image(
            &img.bytes,
            img.width as u32,
            img.height as u32,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| format!("Failed to encode clipboard image: {}", e))?;

    std::fs::write(&temp_path, &png_bytes)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;

    Ok(temp_path.to_string_lossy().to_string())
}
