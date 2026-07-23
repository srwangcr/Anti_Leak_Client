// Prevenir que Tauri genere warning sobre main no utilizado
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use anti_leak_client::{
    buscar_imagen_en_internet as web_search_impl, procesar_imagen_local as process_image_impl,
    HashResult, WebSearchResult,
};

#[tauri::command]
fn procesar_imagen_local(file_path: String) -> Result<HashResult, String> {
    process_image_impl(file_path)
}

#[tauri::command]
fn buscar_imagen_en_internet(file_path: String, api_key: String) -> Result<WebSearchResult, String> {
    web_search_impl(file_path, api_key)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            procesar_imagen_local,
            buscar_imagen_en_internet
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
