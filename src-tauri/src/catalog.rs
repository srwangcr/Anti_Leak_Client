//! Catálogo local persistente en SQLite (sin servicios externos).

use crate::hash::{compute_phash, compute_sha256, hamming_distance};
use crate::metadata::read_metadata;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CatalogEntry {
    pub id: i64,
    pub file_path: String,
    pub file_name: String,
    pub sha256: String,
    pub phash: String,
    pub width: u32,
    pub height: u32,
    pub file_size: u64,
    pub exif: BTreeMap<String, String>,
    pub registered_at: String,
    pub notes: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CatalogMatch {
    pub entry: CatalogEntry,
    pub kind: String, // "exacto" | "parecido"
    pub hamming_distance: u32,
}

pub fn init_db(db_path: &Path) -> Result<Connection, String> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("No se pudo crear la carpeta de datos: {}", e))?;
    }
    let conn = Connection::open(db_path).map_err(|e| format!("No se pudo abrir SQLite: {}", e))?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS images (
             id            INTEGER PRIMARY KEY AUTOINCREMENT,
             file_path     TEXT NOT NULL UNIQUE,
             file_name     TEXT NOT NULL,
             sha256        TEXT NOT NULL,
             phash         TEXT NOT NULL,
             width         INTEGER NOT NULL DEFAULT 0,
             height        INTEGER NOT NULL DEFAULT 0,
             file_size     INTEGER NOT NULL DEFAULT 0,
             exif_json     TEXT NOT NULL DEFAULT '{}',
             registered_at TEXT NOT NULL,
             notes         TEXT NOT NULL DEFAULT ''
         );
         CREATE INDEX IF NOT EXISTS idx_images_sha256 ON images(sha256);
         CREATE INDEX IF NOT EXISTS idx_images_phash ON images(phash);",
    )
    .map_err(|e| format!("No se pudo inicializar la base de datos: {}", e))?;
    Ok(conn)
}

fn entry_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CatalogEntry> {
    let exif_json: String = row.get("exif_json")?;
    let exif: BTreeMap<String, String> = serde_json::from_str(&exif_json).unwrap_or_default();
    Ok(CatalogEntry {
        id: row.get("id")?,
        file_path: row.get("file_path")?,
        file_name: row.get("file_name")?,
        sha256: row.get("sha256")?,
        phash: row.get("phash")?,
        width: row.get::<_, i64>("width")? as u32,
        height: row.get::<_, i64>("height")? as u32,
        file_size: row.get::<_, i64>("file_size")? as u64,
        exif,
        registered_at: row.get("registered_at")?,
        notes: row.get("notes")?,
    })
}

/// Registra (o actualiza) una imagen en el catálogo.
pub fn register_image(
    conn: &Connection,
    file_path: &str,
    notes: &str,
) -> Result<CatalogEntry, String> {
    let metadata = read_metadata(file_path)?;
    let sha256 = compute_sha256(file_path)?;
    let phash = compute_phash(file_path)?;
    let exif_json = serde_json::to_string(&metadata.exif)
        .map_err(|e| format!("No se pudo serializar EXIF: {}", e))?;
    let registered_at = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO images
            (file_path, file_name, sha256, phash, width, height, file_size, exif_json, registered_at, notes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(file_path) DO UPDATE SET
            file_name = excluded.file_name,
            sha256 = excluded.sha256,
            phash = excluded.phash,
            width = excluded.width,
            height = excluded.height,
            file_size = excluded.file_size,
            exif_json = excluded.exif_json,
            registered_at = excluded.registered_at,
            notes = excluded.notes",
        params![
            file_path,
            metadata.file_name,
            sha256,
            phash,
            metadata.width as i64,
            metadata.height as i64,
            metadata.file_size as i64,
            exif_json,
            registered_at,
            notes,
        ],
    )
    .map_err(|e| format!("No se pudo registrar la imagen: {}", e))?;

    let id = conn.last_insert_rowid();
    get_by_id(conn, id)
}

pub fn get_by_id(conn: &Connection, id: i64) -> Result<CatalogEntry, String> {
    conn.query_row(
        "SELECT * FROM images WHERE id = ?1",
        params![id],
        entry_from_row,
    )
    .map_err(|e| format!("No se encontró la entrada: {}", e))
}

pub fn list_images(conn: &Connection) -> Result<Vec<CatalogEntry>, String> {
    let mut stmt = conn
        .prepare("SELECT * FROM images ORDER BY registered_at DESC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], entry_from_row)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn delete_image(conn: &Connection, id: i64) -> Result<(), String> {
    let changes = conn
        .execute("DELETE FROM images WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    if changes == 0 {
        return Err(format!("No existe una entrada con id {}", id));
    }
    Ok(())
}

/// Busca coincidencias en el catálogo: SHA-256 idéntico o pHash cercano (distancia de Hamming).
pub fn find_matches(
    conn: &Connection,
    sha256: &str,
    phash: &str,
    max_distance: u32,
) -> Result<Vec<CatalogMatch>, String> {
    let entries = list_images(conn)?;
    let mut matches = Vec::new();
    for entry in entries {
        if entry.sha256 == sha256 {
            matches.push(CatalogMatch {
                entry,
                kind: "exacto".into(),
                hamming_distance: 0,
            });
            continue;
        }
        match hamming_distance(phash, &entry.phash) {
            Ok(distance) if distance <= max_distance => matches.push(CatalogMatch {
                entry,
                kind: "parecido".into(),
                hamming_distance: distance,
            }),
            _ => {}
        }
    }
    matches.sort_by_key(|m| (m.hamming_distance, m.entry.id));
    Ok(matches)
}

/// Busca una entrada por su ruta de archivo (para no duplicar reportes).
pub fn get_by_path(conn: &Connection, file_path: &str) -> Result<Option<CatalogEntry>, String> {
    conn.query_row(
        "SELECT * FROM images WHERE file_path = ?1",
        params![file_path],
        entry_from_row,
    )
    .optional()
    .map_err(|e| e.to_string())
}
