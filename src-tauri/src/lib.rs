//! Anti Leak Client — núcleo Rust: huellas locales, catálogo SQLite, bóveda,
//! escaneo por lotes, búsqueda inversa web (sin Google) e informes forenses.

pub mod catalog;
pub mod hash;
pub mod metadata;
pub mod report;
pub mod reporting;
pub mod scanner;
pub mod vault;
pub mod web;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub use catalog::{CatalogEntry, CatalogMatch};
pub use metadata::ImageMetadata;
pub use report::{ForensicReport, ReportResult};
pub use reporting::{ComplaintDraft, ComplaintDraftResult, PlatformInfo};
pub use scanner::{ScanFinding, ScanResult};
pub use web::{WebMatch, WebSearchResult};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HashResult {
    pub sha256: String,
    pub phash: String,
}

/// Procesa una imagen localmente: SHA-256 + pHash, sin salir de la computadora.
pub fn procesar_imagen_local(file_path: String) -> Result<HashResult, String> {
    if !Path::new(&file_path).exists() {
        return Err(format!("Archivo no encontrado: {}", file_path));
    }
    Ok(HashResult {
        sha256: hash::compute_sha256(&file_path)?,
        phash: hash::compute_phash(&file_path)?,
    })
}

/// Analiza una imagen y la compara con el catálogo en una sola llamada.
#[derive(Serialize, Debug, Clone)]
pub struct ImageAnalysis {
    pub hashes: HashResult,
    pub metadata: ImageMetadata,
    pub matches: Vec<CatalogMatch>,
}

pub fn analizar_imagen(
    conn: &Connection,
    file_path: &str,
    max_distance: u32,
) -> Result<ImageAnalysis, String> {
    let hashes = procesar_imagen_local(file_path.to_string())?;
    Ok(ImageAnalysis {
        metadata: metadata::read_metadata(file_path)?,
        matches: catalog::find_matches(conn, &hashes.sha256, &hashes.phash, max_distance)?,
        hashes,
    })
}
