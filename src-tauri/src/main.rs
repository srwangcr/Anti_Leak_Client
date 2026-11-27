// Prevenir que Tauri genere warning sobre main no utilizado
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use anti_leak_client::{
    analizar_imagen as analyze_impl, catalog, hash, metadata,
    procesar_imagen_local as process_image_impl, report, reporting, scanner, vault, web,
    HashResult, ImageAnalysis, ImageMetadata, ReportResult, ScanResult, WebSearchResult,
};
use anti_leak_client::{CatalogEntry, CatalogMatch, ComplaintDraftResult, PlatformInfo};
use rusqlite::Connection;
use tauri::{AppHandle, Manager};

fn catalog_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("No se pudo obtener la carpeta de datos: {}", e))?;
    Ok(dir.join("catalog.sqlite"))
}

fn open_catalog(app: &AppHandle) -> Result<Connection, String> {
    catalog::init_db(&catalog_path(app)?)
}

#[tauri::command]
fn procesar_imagen_local(file_path: String) -> Result<HashResult, String> {
    process_image_impl(file_path)
}

#[tauri::command]
fn obtener_metadatos(file_path: String) -> Result<ImageMetadata, String> {
    metadata::read_metadata(&file_path)
}

#[tauri::command]
fn analizar_imagen(
    app: AppHandle,
    file_path: String,
    max_distance: u32,
) -> Result<ImageAnalysis, String> {
    analyze_impl(&open_catalog(&app)?, &file_path, max_distance)
}

#[tauri::command]
fn registrar_imagen(
    app: AppHandle,
    file_path: String,
    notes: String,
) -> Result<CatalogEntry, String> {
    catalog::register_image(&open_catalog(&app)?, &file_path, &notes)
}

#[tauri::command]
fn listar_catalogo(app: AppHandle) -> Result<Vec<CatalogEntry>, String> {
    catalog::list_images(&open_catalog(&app)?)
}

#[tauri::command]
fn eliminar_del_catalogo(app: AppHandle, id: i64) -> Result<(), String> {
    catalog::delete_image(&open_catalog(&app)?, id)
}

#[tauri::command]
fn buscar_en_catalogo(
    app: AppHandle,
    sha256: String,
    phash: String,
    max_distance: u32,
) -> Result<Vec<CatalogMatch>, String> {
    catalog::find_matches(&open_catalog(&app)?, &sha256, &phash, max_distance)
}

#[tauri::command]
fn escanear_carpeta(
    app: AppHandle,
    folder_path: String,
    max_distance: u32,
) -> Result<ScanResult, String> {
    scanner::scan_folder(&open_catalog(&app)?, &folder_path, max_distance)
}

/// Búsqueda inversa gratuita (Yandex). Ya no se requiere ninguna API key.
#[tauri::command]
fn buscar_imagen_en_internet(file_path: String) -> Result<WebSearchResult, String> {
    web::buscar_imagen_en_internet(&file_path)
}

#[tauri::command]
fn generar_reporte(
    app: AppHandle,
    file_path: String,
    notes: String,
    output_dir: String,
    include_web: bool,
    max_distance: u32,
) -> Result<ReportResult, String> {
    report::generate_report(
        &open_catalog(&app)?,
        &file_path,
        &notes,
        &output_dir,
        include_web,
        max_distance,
    )
}

#[tauri::command]
fn listar_plataformas() -> Vec<PlatformInfo> {
    reporting::platforms()
}

/// Genera informe + borrador de denuncia. Si el frontend ya hizo la búsqueda web,
/// reutiliza ese resultado para no volver a subir la imagen.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn generar_borrador_denuncia(
    app: AppHandle,
    file_path: String,
    platform_id: String,
    notes: String,
    output_dir: String,
    include_web: bool,
    max_distance: u32,
    web_result_json: Option<String>,
) -> Result<ComplaintDraftResult, String> {
    reporting::generate_complaint_draft(
        &open_catalog(&app)?,
        &file_path,
        &platform_id,
        &notes,
        &output_dir,
        include_web,
        max_distance,
        web_result_json.as_deref(),
    )
}

#[tauri::command]
fn guardar_credencial(key: String, value: String) -> Result<(), String> {
    vault::save_credential(&key, &value)
}

#[tauri::command]
fn obtener_credencial(key: String) -> Result<Option<String>, String> {
    vault::load_credential(&key)
}

#[tauri::command]
fn eliminar_credencial(key: String) -> Result<(), String> {
    vault::delete_credential(&key)
}

#[tauri::command]
fn ruta_datos(app: AppHandle) -> Result<String, String> {
    Ok(catalog_path(&app)?
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .to_string_lossy()
        .to_string())
}

#[tauri::command]
fn distancia_phash(a: String, b: String) -> Result<u32, String> {
    hash::hamming_distance(&a, &b)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            procesar_imagen_local,
            obtener_metadatos,
            analizar_imagen,
            registrar_imagen,
            listar_catalogo,
            eliminar_del_catalogo,
            buscar_en_catalogo,
            escanear_carpeta,
            buscar_imagen_en_internet,
            generar_reporte,
            listar_plataformas,
            generar_borrador_denuncia,
            guardar_credencial,
            obtener_credencial,
            eliminar_credencial,
            ruta_datos,
            distancia_phash,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
