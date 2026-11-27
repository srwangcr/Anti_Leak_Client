//! Evidencia forense: informes JSON y PDF generados localmente.

use crate::catalog::{find_matches, CatalogMatch};
use crate::hash::{compute_phash, compute_sha256};
use crate::metadata::read_metadata;
use crate::web::{buscar_imagen_en_internet, WebSearchResult};
use chrono::Utc;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Hashes {
    pub sha256: String,
    pub phash: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Integrity {
    pub algorithm: String,
    pub digest: String,
    pub note: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ForensicReport {
    pub report_id: String,
    pub generated_at: String,
    pub case_notes: String,
    pub image: crate::metadata::ImageMetadata,
    pub hashes: Hashes,
    pub local_matches: Vec<CatalogMatch>,
    pub web_search: Option<WebSearchResult>,
    pub web_search_error: Option<String>,
    pub integrity: Integrity,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReportResult {
    pub json_path: String,
    pub pdf_path: String,
    pub report: ForensicReport,
}

pub fn generate_report(
    conn: &Connection,
    file_path: &str,
    notes: &str,
    output_dir: &str,
    include_web: bool,
    max_distance: u32,
) -> Result<ReportResult, String> {
    generate_report_from(
        conn,
        file_path,
        notes,
        output_dir,
        include_web,
        max_distance,
        None,
    )
}

/// Variante que permite reutilizar una búsqueda web ya hecha (evita subir la imagen dos veces).
pub fn generate_report_from(
    conn: &Connection,
    file_path: &str,
    notes: &str,
    output_dir: &str,
    include_web: bool,
    max_distance: u32,
    web_override: Option<WebSearchResult>,
) -> Result<ReportResult, String> {
    let metadata = read_metadata(file_path)?;
    let sha256 = compute_sha256(file_path)?;
    let phash = compute_phash(file_path)?;
    let matches = find_matches(conn, &sha256, &phash, max_distance)?;

    let (web_search, web_search_error) = if let Some(result) = web_override {
        (Some(result), None)
    } else if include_web {
        match buscar_imagen_en_internet(file_path) {
            Ok(result) => (Some(result), None),
            Err(e) => (None, Some(e)),
        }
    } else {
        (None, None)
    };

    let now = Utc::now();
    let report_id = format!(
        "{}_{}",
        now.format("%Y%m%dT%H%M%SZ"),
        &sha256.chars().take(8).collect::<String>()
    );

    let mut report = ForensicReport {
        report_id: report_id.clone(),
        generated_at: now.to_rfc3339(),
        case_notes: notes.to_string(),
        image: metadata,
        hashes: Hashes { sha256, phash },
        local_matches: matches,
        web_search,
        web_search_error,
        integrity: Integrity {
            algorithm: "SHA-256 del contenido del informe".into(),
            digest: String::new(),
            note: "Permite detectar si el informe fue modificado después de generarlo.".into(),
        },
    };

    let canonical = serde_json::to_string(&report)
        .map_err(|e| format!("No se pudo serializar el informe: {}", e))?;
    let digest = Sha256::digest(canonical.as_bytes());
    report.integrity.digest = hex::encode(digest);

    let dir = PathBuf::from(output_dir);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("No se pudo crear la carpeta de informes: {}", e))?;
    let json_path = dir.join(format!("informe_{}.json", report_id));
    let pdf_path = dir.join(format!("informe_{}.pdf", report_id));

    let json_text = serde_json::to_string_pretty(&report)
        .map_err(|e| format!("No se pudo escribir el JSON: {}", e))?;
    std::fs::write(&json_path, json_text)
        .map_err(|e| format!("No se pudo guardar el JSON: {}", e))?;

    let lines = report_lines(&report);
    std::fs::write(&pdf_path, build_pdf(&lines))
        .map_err(|e| format!("No se pudo guardar el PDF: {}", e))?;

    Ok(ReportResult {
        json_path: json_path.to_string_lossy().to_string(),
        pdf_path: pdf_path.to_string_lossy().to_string(),
        report,
    })
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    if text.chars().count() <= width {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.chars().count() + 1 + word.chars().count() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn add(lines: &mut Vec<String>, label: &str, value: &str) {
    for line in wrap(&format!("{}: {}", label, value), 98) {
        lines.push(line);
    }
}

fn report_lines(report: &ForensicReport) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push("ANTI LEAK CLIENT - INFORME FORENSE".into());
    add(&mut lines, "ID del informe", &report.report_id);
    add(&mut lines, "Generado (UTC)", &report.generated_at);
    add(&mut lines, "Notas del caso", &report.case_notes);
    lines.push(String::new());

    lines.push("1. IMAGEN ANALIZADA".into());
    add(&mut lines, "Archivo", &report.image.file_name);
    add(&mut lines, "Ruta original", &report.image.file_path);
    add(
        &mut lines,
        "Tamano (bytes)",
        &report.image.file_size.to_string(),
    );
    add(&mut lines, "Modificado", &report.image.modified_at);
    add(
        &mut lines,
        "Dimensiones",
        &format!("{} x {}", report.image.width, report.image.height),
    );
    add(&mut lines, "Tipo MIME", &report.image.mime_type);
    lines.push(String::new());

    lines.push("2. HUELLAS CRIPTOGRAFICAS".into());
    add(&mut lines, "SHA-256", &report.hashes.sha256);
    add(&mut lines, "pHash (DCT 8x8)", &report.hashes.phash);
    lines.push(String::new());

    lines.push("3. METADATOS EXIF".into());
    if report.image.exif.is_empty() {
        lines.push("Sin metadatos EXIF disponibles.".into());
    } else {
        for (tag, value) in &report.image.exif {
            add(&mut lines, tag, value);
        }
    }
    lines.push(String::new());

    lines.push("4. COINCIDENCIAS LOCALES".into());
    if report.local_matches.is_empty() {
        lines.push("No se encontraron coincidencias en el catalogo local.".into());
    } else {
        for (i, m) in report.local_matches.iter().enumerate() {
            add(
                &mut lines,
                &format!("Coincidencia {}", i + 1),
                m.kind.as_str(),
            );
            add(&mut lines, "  Archivo", &m.entry.file_path);
            add(
                &mut lines,
                "  Distancia Hamming",
                &m.hamming_distance.to_string(),
            );
            add(&mut lines, "  SHA-256", &m.entry.sha256);
        }
    }
    lines.push(String::new());

    lines.push("5. COINCIDENCIAS EN LA WEB".into());
    match &report.web_search {
        Some(web) => {
            add(&mut lines, "Proveedor", &web.provider);
            add(&mut lines, "Busqueda", &web.search_url);
            for group in [
                ("Paginas", &web.pages_with_matches),
                ("Imagenes exactas", &web.full_matches),
                ("Imagenes similares", &web.partial_matches),
            ] {
                if group.1.is_empty() {
                    continue;
                }
                lines.push(format!("{} ({}):", group.0, group.1.len()));
                for m in group.1 {
                    if !m.title.is_empty() {
                        add(&mut lines, "  Titulo", &m.title);
                    }
                    add(&mut lines, "  URL", &m.url);
                }
            }
        }
        None => {
            let reason = report
                .web_search_error
                .as_deref()
                .unwrap_or("Busqueda web no solicitada.");
            add(&mut lines, "Resultado", reason);
        }
    }
    lines.push(String::new());

    lines.push("6. INTEGRIDAD DEL INFORME".into());
    add(&mut lines, "Algoritmo", &report.integrity.algorithm);
    add(&mut lines, "Digest", &report.integrity.digest);
    add(&mut lines, "Nota", &report.integrity.note);

    lines
}

/// Escapa texto para una cadena literal PDF y lo convierte a bytes WinAnsi.
fn pdf_escape(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => out.extend_from_slice(b"\\\\"),
            '(' => out.extend_from_slice(b"\\("),
            ')' => out.extend_from_slice(b"\\)"),
            '\n' | '\r' => out.push(b' '),
            c if (c as u32) <= 255 => out.push(c as u8),
            _ => out.push(b'?'),
        }
    }
    out
}

/// Escritor PDF mínimo (Helvetica/WinAnsi) sin dependencias externas.
fn build_pdf(lines: &[String]) -> Vec<u8> {
    const LINES_PER_PAGE: usize = 55;
    let pages = lines.len().div_ceil(LINES_PER_PAGE).max(1);
    let max_obj = 4 + pages * 2;
    let mut offsets = vec![0usize; max_obj + 1];
    let mut out = b"%PDF-1.4\n".to_vec();

    fn write_object(out: &mut Vec<u8>, offsets: &mut [usize], num: usize, body: &[u8]) {
        offsets[num] = out.len();
        out.extend_from_slice(format!("{} 0 obj\n", num).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }

    write_object(
        &mut out,
        &mut offsets,
        1,
        b"<< /Type /Catalog /Pages 2 0 R >>",
    );

    let kids: Vec<String> = (0..pages).map(|i| format!("{} 0 R", 6 + i * 2)).collect();
    write_object(
        &mut out,
        &mut offsets,
        2,
        format!(
            "<< /Type /Pages /Kids [{}] /Count {} >>",
            kids.join(" "),
            pages
        )
        .as_bytes(),
    );
    write_object(
        &mut out,
        &mut offsets,
        3,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    );
    write_object(
        &mut out,
        &mut offsets,
        4,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>",
    );

    for page in 0..pages {
        let content_num = 5 + page * 2;
        let page_num = 6 + page * 2;
        let slice = &lines[page * LINES_PER_PAGE..((page + 1) * LINES_PER_PAGE).min(lines.len())];
        let mut content = b"BT\n/F1 9.5 Tf\n12 TL\n0.12 0.12 0.12 rg\n45 800 Td\n".to_vec();
        for line in slice {
            let font = if line.ends_with(':')
                || line.starts_with("ANTI LEAK CLIENT")
                || line
                    .chars()
                    .next()
                    .map_or(false, |c| c.is_ascii_digit() && line.contains('.'))
            {
                "/F2"
            } else {
                "/F1"
            };
            content.extend_from_slice(format!("{} 9.5 Tf (", font).as_bytes());
            content.extend_from_slice(&pdf_escape(line));
            content.extend_from_slice(b") Tj\nT*\n");
        }
        content.extend_from_slice(b"ET\n");

        let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
        stream.extend_from_slice(&content);
        stream.extend_from_slice(b"endstream");
        write_object(&mut out, &mut offsets, content_num, &stream);

        let page_body = format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] \
             /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents {} 0 R >>",
            content_num
        );
        write_object(&mut out, &mut offsets, page_num, page_body.as_bytes());
    }

    let xref_offset = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", max_obj + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        out.extend_from_slice(format!("{:020} 00000 n \n", offset).as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            max_obj + 1,
            xref_offset
        )
        .as_bytes(),
    );
    out
}
