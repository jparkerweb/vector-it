use std::path::Path;

/// Initiate a file drag-and-drop operation as drag source.
/// Uses the Tauri drag-and-drop API to start dragging the specified file.
pub fn start_file_drag(file_path: &str) -> std::result::Result<(), String> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }
    // The actual drag initiation is handled by the frontend via Tauri's
    // startDrag API. This function validates the file exists.
    Ok(())
}
