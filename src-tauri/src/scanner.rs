//! Escaneo por lotes de carpetas y comparación local (sin consumir cuota de API).

use crate::catalog::{find_matches, CatalogMatch};
use crate::hash::{compute_phash, compute_sha256, es_imagen, hamming_distance};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::Path;
use walkdir::WalkDir;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScanFinding {
    pub file_path: String,
    pub file_name: String,
    pub sha256: String,
    pub phash: String,
    /// "duplicado_exacto" | "parecido" | "nuevo"
    pub kind: String,
    pub matched_path: String,
    pub hamming_distance: u32,
    pub source: String, // "catalogo" | "escaneo"
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScanResult {
    pub scanned: usize,
    pub skipped: usize,
    pub findings: Vec<ScanFinding>,
    pub errors: Vec<String>,
}

fn finding_from_match(
    file_path: &str,
    sha256: &str,
    phash: &str,
    m: &CatalogMatch,
    source: &str,
) -> ScanFinding {
    ScanFinding {
        file_path: file_path.to_string(),
        file_name: Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string(),
        sha256: sha256.to_string(),
        phash: phash.to_string(),
        kind: if m.kind == "exacto" {
            "duplicado_exacto".into()
        } else {
            "parecido".into()
        },
        matched_path: m.entry.file_path.clone(),
        hamming_distance: m.hamming_distance,
        source: source.into(),
    }
}

/// Recorre una carpeta de forma recursiva y compara cada imagen con el catálogo
/// y con las demás imágenes del mismo escaneo.
pub fn scan_folder(
    conn: &Connection,
    folder: &str,
    max_distance: u32,
) -> Result<ScanResult, String> {
    if !Path::new(folder).is_dir() {
        return Err(format!("La carpeta no existe: {}", folder));
    }

    let mut result = ScanResult {
        scanned: 0,
        skipped: 0,
        findings: Vec::new(),
        errors: Vec::new(),
    };
    let mut seen_sha: Vec<(String, String)> = Vec::new();
    let mut seen_phash: Vec<(String, String)> = Vec::new();

    for entry in WalkDir::new(folder)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() || !es_imagen(entry.path()) {
            continue;
        }
        if result.scanned >= 5000 {
            result.skipped += 1;
            continue;
        }
        let file_path = entry.path().to_string_lossy().to_string();
        let sha = match compute_sha256(&file_path) {
            Ok(v) => v,
            Err(e) => {
                result
                    .errors
                    .push(format!("{}: {}", file_name(&file_path), e));
                continue;
            }
        };
        let ph = match compute_phash(&file_path) {
            Ok(v) => v,
            Err(e) => {
                result
                    .errors
                    .push(format!("{}: {}", file_name(&file_path), e));
                continue;
            }
        };
        result.scanned += 1;

        // 1) Comparar contra el catálogo SQLite.
        let catalog_matches = find_matches(conn, &sha, &ph, max_distance).unwrap_or_default();
        if let Some(m) = catalog_matches.first() {
            result
                .findings
                .push(finding_from_match(&file_path, &sha, &ph, m, "catalogo"));
            continue;
        }

        // 2) Comparar contra las imágenes ya vistas en este escaneo.
        if let Some((_, previous)) = seen_sha.iter().find(|(s, _)| s == &sha) {
            result.findings.push(ScanFinding {
                file_path: file_path.clone(),
                file_name: file_name(&file_path),
                sha256: sha.clone(),
                phash: ph.clone(),
                kind: "duplicado_exacto".into(),
                matched_path: previous.clone(),
                hamming_distance: 0,
                source: "escaneo".into(),
            });
            continue;
        }
        let mut nearest: Option<(u32, String)> = None;
        for (other_phash, other_path) in &seen_phash {
            if let Ok(distance) = hamming_distance(&ph, other_phash) {
                if distance <= max_distance && nearest.as_ref().map_or(true, |(d, _)| distance < *d)
                {
                    nearest = Some((distance, other_path.clone()));
                }
            }
        }
        if let Some((distance, other_path)) = nearest {
            result.findings.push(ScanFinding {
                file_path: file_path.clone(),
                file_name: file_name(&file_path),
                sha256: sha.clone(),
                phash: ph.clone(),
                kind: "parecido".into(),
                matched_path: other_path,
                hamming_distance: distance,
                source: "escaneo".into(),
            });
        } else {
            result.findings.push(ScanFinding {
                file_path: file_path.clone(),
                file_name: file_name(&file_path),
                sha256: sha.clone(),
                phash: ph.clone(),
                kind: "nuevo".into(),
                matched_path: String::new(),
                hamming_distance: 0,
                source: "escaneo".into(),
            });
        }
        seen_sha.push((sha, file_path.clone()));
        seen_phash.push((ph, file_path));
    }

    Ok(result)
}

fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string()
}
