pub mod commands;
pub mod clipboard;
pub mod dragdrop;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::load_image,
            commands::load_svg_file,
            commands::parse_svg_content,
            commands::paste_image,
            commands::detect_type,
            commands::vectorize,
            commands::suggest_palette,
            commands::export_svg,
            commands::render_svg_string,
            commands::write_file,
            commands::export_eps,
            commands::export_pdf,
            commands::export_dxf,
            commands::init_editor,
            commands::apply_edit,
            commands::undo_edit,
            commands::reset_edits,
            commands::get_segmentation,
            commands::find_artifacts,
            commands::fix_artifact,
            commands::sample_color,
            commands::zap_region,
            commands::re_vectorize,
            commands::get_documents_dir,
            commands::paste_from_clipboard,
            commands::start_drag,
            commands::export_bitmap,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
