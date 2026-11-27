use anti_leak_client::{
    catalog, hash, metadata, report, reporting, scanner, web, WebMatch, WebSearchResult,
};
use image::{Rgba, RgbaImage};
use sha2::Digest;
use std::fs;
use std::path::{Path, PathBuf};

fn temp_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("antileak_{}_{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

fn save_gradient(path: &Path, size: u32) {
    let mut img = RgbaImage::new(size, size);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        *pixel = Rgba([(x * 255 / size) as u8, (y * 255 / size) as u8, 100, 255]);
    }
    img.save(path).unwrap();
}

fn save_solid(path: &Path, size: u32, color: [u8; 3]) {
    let mut img = RgbaImage::new(size, size);
    for pixel in img.pixels_mut() {
        *pixel = Rgba([color[0], color[1], color[2], 255]);
    }
    img.save(path).unwrap();
}

#[test]
fn sha256_and_phash_are_stable() {
    let dir = temp_dir("hashes");
    let original = dir.join("original.png");
    let copy = dir.join("copy.png");
    save_gradient(&original, 128);
    save_gradient(&copy, 64); // misma imagen, otro tamaño

    let sha = hash::compute_sha256(original.to_str().unwrap()).unwrap();
    assert_eq!(sha.len(), 64);
    assert_eq!(
        sha,
        hash::compute_sha256(original.to_str().unwrap()).unwrap()
    );
    assert_ne!(sha, hash::compute_sha256(copy.to_str().unwrap()).unwrap());

    let a = hash::compute_phash(original.to_str().unwrap()).unwrap();
    let b = hash::compute_phash(copy.to_str().unwrap()).unwrap();
    assert_eq!(a.len(), 16);
    assert!(
        hash::hamming_distance(&a, &b).unwrap() <= 6,
        "pHash inestable: {} vs {}",
        a,
        b
    );
    assert!(hash::hamming_distance(&a, &a).unwrap() == 0);
}

#[test]
fn catalog_crud_and_matches() {
    let dir = temp_dir("catalog");
    let conn = catalog::init_db(&dir.join("catalog.sqlite")).unwrap();
    let image_path = dir.join("foto.png");
    save_gradient(&image_path, 96);

    let entry = catalog::register_image(&conn, image_path.to_str().unwrap(), "caso 1").unwrap();
    assert_eq!(entry.notes, "caso 1");
    assert_eq!(entry.width, 96);
    assert_eq!(catalog::list_images(&conn).unwrap().len(), 1);

    let matches = catalog::find_matches(&conn, &entry.sha256, &entry.phash, 8).unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].kind, "exacto");

    let near = catalog::find_matches(&conn, &"0".repeat(64), &entry.phash, 8).unwrap();
    assert_eq!(near.len(), 1);
    assert_eq!(near[0].kind, "parecido");

    catalog::delete_image(&conn, entry.id).unwrap();
    assert!(catalog::list_images(&conn).unwrap().is_empty());
}

#[test]
fn scan_detects_exact_and_similar_copies() {
    let dir = temp_dir("scan");
    let images = dir.join("images");
    fs::create_dir_all(&images).unwrap();
    save_gradient(&images.join("a.png"), 128);
    fs::copy(images.join("a.png"), images.join("copia_exacta.png")).unwrap();
    save_gradient(&images.join("redimensionada.png"), 80);
    save_solid(&images.join("otra.png"), 64, [210, 30, 20]);

    let conn = catalog::init_db(&dir.join("catalog.sqlite")).unwrap();
    let result = scanner::scan_folder(&conn, images.to_str().unwrap(), 10).unwrap();
    assert_eq!(result.scanned, 4);
    assert!(result.findings.iter().any(|f| f.kind == "duplicado_exacto"));
    assert!(result.findings.iter().any(|f| f.kind == "parecido"));
    assert!(result.findings.iter().any(|f| f.kind == "nuevo"));
}

#[test]
fn report_generates_json_and_valid_pdf() {
    let dir = temp_dir("report");
    let conn = catalog::init_db(&dir.join("catalog.sqlite")).unwrap();
    let image_path = dir.join("evidencia.png");
    save_gradient(&image_path, 64);
    catalog::register_image(&conn, image_path.to_str().unwrap(), "caso forense").unwrap();

    let out = dir.join("informes");
    let result = report::generate_report(
        &conn,
        image_path.to_str().unwrap(),
        "caso forense",
        out.to_str().unwrap(),
        false,
        8,
    )
    .unwrap();

    assert!(Path::new(&result.json_path).exists());
    assert!(Path::new(&result.pdf_path).exists());
    assert_eq!(result.report.local_matches.len(), 1);
    assert_eq!(result.report.hashes.sha256.len(), 64);

    let mut recomputed = result.report.clone();
    recomputed.integrity.digest.clear();
    let canonical = serde_json::to_string(&recomputed).unwrap();
    assert_eq!(
        result.report.integrity.digest,
        hex::encode(sha2::Sha256::digest(canonical.as_bytes()))
    );

    let pdf = fs::read(&result.pdf_path).unwrap();
    assert!(pdf.starts_with(b"%PDF-1.4"));
    assert!(pdf.windows(5).any(|w| w == b"%%EOF"));
    let xref_text = String::from_utf8_lossy(&pdf);
    let startxref = xref_text.rsplit("startxref\n").next().unwrap();
    let offset: usize = startxref
        .split_whitespace()
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(&pdf[offset..offset + 4], b"xref");
    assert!(xref_text.contains("/Root 1 0 R"));
}

#[test]
fn metadata_reads_local_file() {
    let dir = temp_dir("metadata");
    let image_path = dir.join("meta.png");
    save_gradient(&image_path, 48);
    let meta = metadata::read_metadata(image_path.to_str().unwrap()).unwrap();
    assert_eq!(meta.file_name, "meta.png");
    assert_eq!(meta.width, 48);
    assert_eq!(meta.mime_type, "image/png");
    assert!(meta.file_size > 0);
}

#[test]
fn complaint_draft_generates_structured_files() {
    let dir = temp_dir("denuncia");
    let conn = catalog::init_db(&dir.join("catalog.sqlite")).unwrap();
    let image_path = dir.join("foto.png");
    save_gradient(&image_path, 64);

    let out = dir.join("salida");
    let result = reporting::generate_complaint_draft(
        &conn,
        image_path.to_str().unwrap(),
        "instagram",
        "caso de prueba",
        out.to_str().unwrap(),
        false,
        8,
        None,
    )
    .unwrap();

    assert!(Path::new(&result.json_path).exists());
    assert!(Path::new(&result.markdown_path).exists());
    assert!(Path::new(&result.evidence.json_path).exists());
    assert!(Path::new(&result.evidence.pdf_path).exists());

    let draft = &result.draft;
    assert_eq!(draft.platform.id, "instagram");
    assert!(draft.status.contains("revisión humana"));
    assert!(draft.text.subject.contains("Solicitud de retirada"));
    assert!(draft.text.body.contains(&draft.hashes.sha256));
    assert!(draft.text.body.contains("Instagram"));
    assert!(draft.text.body.contains("[Pegá aquí la URL"));
    assert!(draft.text.body.contains("caso de prueba"));
    assert!(draft.infringing_urls.is_empty());
    assert_eq!(draft.attachments.len(), 3);
    assert_eq!(draft.checklist.len(), 5);
    assert!(draft.steps.len() >= 6);

    let markdown_text = fs::read_to_string(&result.markdown_path).unwrap();
    assert!(markdown_text.contains("# Borrador de denuncia"));
    assert!(markdown_text.contains("## Paso a paso"));
    assert!(markdown_text.contains("## Texto listo para copiar"));
    assert!(markdown_text.contains("NO se envió automáticamente"));
}

#[test]
fn complaint_draft_reuses_cached_web_result_without_network() {
    let dir = temp_dir("denuncia_web");
    let conn = catalog::init_db(&dir.join("catalog.sqlite")).unwrap();
    let image_path = dir.join("foto.png");
    save_gradient(&image_path, 64);

    let cached = WebSearchResult {
        provider: "Yandex Images (búsqueda inversa gratuita)".into(),
        search_url: "https://yandex.com/images/search?cbir_id=abc".into(),
        full_matches: vec![WebMatch {
            url: "https://ejemplo.com/fila".into(),
            title: "Foto filtrada".into(),
            domain: "ejemplo.com".into(),
            match_type: "exacta".into(),
        }],
        partial_matches: vec![],
        pages_with_matches: vec![],
    };
    let cached_json = serde_json::to_string(&cached).unwrap();

    let out = dir.join("salida");
    let result = reporting::generate_complaint_draft(
        &conn,
        image_path.to_str().unwrap(),
        "web",
        "",
        out.to_str().unwrap(),
        false, // sin red: solo se reutiliza el resultado cacheado
        8,
        Some(&cached_json),
    )
    .unwrap();

    assert_eq!(result.draft.infringing_urls.len(), 1);
    assert_eq!(
        result.draft.infringing_urls[0].url,
        "https://ejemplo.com/fila"
    );
    assert!(result.draft.text.body.contains("https://ejemplo.com/fila"));
    assert!(result.evidence.report.web_search.is_some());
    assert!(result.evidence.report.web_search_error.is_none());
}

#[test]
fn platforms_include_official_report_channels() {
    let platforms = reporting::platforms();
    let ids: Vec<&str> = platforms.iter().map(|p| p.id.as_str()).collect();
    for expected in ["instagram", "facebook", "tiktok", "x", "web"] {
        assert!(ids.contains(&expected), "Falta la plataforma {}", expected);
    }
    for platform in platforms {
        assert!(!platform.steps.is_empty());
        if platform.id != "web" {
            assert!(
                platform.report_url.starts_with("https://"),
                "URL no segura para {}",
                platform.id
            );
        }
    }
}

#[test]
fn complaint_draft_rejects_unknown_platform() {
    let dir = temp_dir("denuncia_error");
    let conn = catalog::init_db(&dir.join("catalog.sqlite")).unwrap();
    let image_path = dir.join("foto.png");
    save_gradient(&image_path, 64);

    let error = reporting::generate_complaint_draft(
        &conn,
        image_path.to_str().unwrap(),
        "plataforma_inexistente",
        "",
        dir.join("salida").to_str().unwrap(),
        false,
        8,
        None,
    )
    .err()
    .expect("Se esperaba error con plataforma desconocida");
    assert!(error.contains("Plataforma desconocida"));
}

/// Prueba de red: se ejecuta con `cargo test -- --ignored`.
#[test]
#[ignore]
fn yandex_web_search_returns_pages() {
    let dir = temp_dir("web");
    let image_path = dir.join("web.png");
    save_gradient(&image_path, 256);
    let result = web::buscar_imagen_en_internet(image_path.to_str().unwrap()).unwrap();
    assert_eq!(result.provider.contains("Yandex"), true);
    assert!(!result.search_url.is_empty());
}
