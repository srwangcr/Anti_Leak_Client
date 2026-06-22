// Prevenir que Tauri genere warning sobre main no utilizado
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use anti_leak_client::{procesar_imagen_local as process_image_impl, HashResult};

#[tauri::command]
fn procesar_imagen_local(file_path: String) -> Result<HashResult, String> {
    process_image_impl(file_path)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![procesar_imagen_local])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
