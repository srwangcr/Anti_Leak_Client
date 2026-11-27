//! Metadatos_file: tamaño, fechas, dimensiones y EXIF leído localmente.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ImageMetadata {
    pub file_path: String,
    pub file_name: String,
    pub file_size: u64,
    pub modified_at: String,
    pub width: u32,
    pub height: u32,
    pub mime_type: String,
    /// Etiquetas EXIF (Make, Model, DateTimeOriginal, GPSLatitude, ...).
    pub exif: BTreeMap<String, String>,
}

fn timestamp_as_rfc3339(time: SystemTime) -> String {
    let duration = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    DateTime::<Utc>::from_timestamp(duration.as_secs() as i64, 0)
        .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap())
        .to_rfc3339()
}

fn mime_from_extension(path: &Path) -> String {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("jpg" | "jpeg") => "image/jpeg".into(),
        Some("png") => "image/png".into(),
        Some("gif") => "image/gif".into(),
        Some("bmp") => "image/bmp".into(),
        Some("tif" | "tiff") => "image/tiff".into(),
        Some("webp") => "image/webp".into(),
        _ => "application/octet-stream".into(),
    }
}

/// Lee EXIF sin decodificar la imagen completa; si no hay EXIF devuelve un mapa vacío.
pub fn read_exif(path: &Path) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return result,
    };
    let mut reader = BufReader::new(file);
    let exif_data = match exif::Reader::new().read_from_container(&mut reader) {
        Ok(data) => data,
        Err(_) => return result,
    };
    for field in exif_data.fields() {
        let key = field.tag.to_string();
        let value = field.value.display_as(field.tag).to_string();
        result.entry(key).or_insert(value);
    }
    result
}

/// Obtiene todos los metadatos de un archivo de imagen.
pub fn read_metadata(file_path: &str) -> Result<ImageMetadata, String> {
    let path = Path::new(file_path);
    let info = std::fs::metadata(path).map_err(|e| format!("No se pudo leer el archivo: {}", e))?;
    let (width, height) = image::image_dimensions(path)
        .map_err(|e| format!("No se pudo determinar las dimensiones: {}", e))?;

    Ok(ImageMetadata {
        file_path: file_path.to_string(),
        file_name: path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string(),
        file_size: info.len(),
        modified_at: info
            .modified()
            .map(timestamp_as_rfc3339)
            .unwrap_or_default(),
        width,
        height,
        mime_type: mime_from_extension(path),
        exif: read_exif(path),
    })
}
