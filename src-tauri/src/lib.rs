use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use sha2::{Sha256, Digest};
use image::io::Reader as ImageReader;
use image::GenericImageView;
use base64::{engine::general_purpose, Engine as _};

#[derive(Serialize, Deserialize, Debug)]
pub struct HashResult {
    pub sha256: String,
    pub phash: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WebMatch {
    pub url: String,
    /// "exacta" | "parcial" | "pagina"
    pub match_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WebSearchResult {
    pub full_matches: Vec<WebMatch>,
    pub partial_matches: Vec<WebMatch>,
    pub pages_with_matches: Vec<WebMatch>,
}

/// Calcula el hash SHA-256 de un archivo
fn compute_sha256(file_path: &str) -> Result<String, String> {
    let data = fs::read(file_path)
        .map_err(|e| format!("Error reading file: {}", e))?;
    
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let result = hasher.finalize();
    
    Ok(hex::encode(result))
}

/// Calcula el hash perceptual de una imagen (implementación simple)
fn compute_phash(file_path: &str) -> Result<String, String> {
    let img = ImageReader::open(file_path)
        .map_err(|e| format!("Error opening image: {}", e))?
        .decode()
        .map_err(|e| format!("Error decoding image: {}", e))?;

    // Redimensionar a 8x8 para el cálculo del hash
    let thumbnail = image::imageops::thumbnail(&img, 8, 8);
    let gray_img = image::DynamicImage::ImageLuma8(
        image::imageops::grayscale(&thumbnail)
    );

    // Calcular el promedio de píxeles
    let mut sum: u32 = 0;
    let mut count: u32 = 0;
    for pixel in gray_img.pixels() {
        sum += pixel.2[0] as u32;
        count += 1;
    }
    let avg = if count > 0 { sum / count } else { 0 };

    // Generar el hash basado en comparación con el promedio
    let mut hash: u64 = 0;
    let mut bit_pos = 0;
    for pixel in gray_img.pixels() {
        if pixel.2[0] as u32 >= avg {
            hash |= 1u64 << bit_pos;
        }
        bit_pos += 1;
        if bit_pos >= 64 {
            break;
        }
    }

    Ok(format!("{:016x}", hash))
}

/// Procesa una imagen localmente
pub fn procesar_imagen_local(file_path: String) -> Result<HashResult, String> {
    // Validar que el archivo existe
    if !Path::new(&file_path).exists() {
        return Err(format!("File not found: {}", file_path));
    }

    let sha256 = compute_sha256(&file_path)?;
    let phash = compute_phash(&file_path)?;

    Ok(HashResult {
        sha256,
        phash,
    })
}

fn extract_matches(value: &serde_json::Value, label: &str) -> Vec<WebMatch> {
    value
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    v["url"].as_str().map(|u| WebMatch {
                        url: u.to_string(),
                        match_type: label.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Busca la imagen en internet usando Google Cloud Vision (Web Detection).
/// Requiere una API key de Google Cloud con la Vision API habilitada.
/// Docs: https://cloud.google.com/vision/docs/detecting-web
pub fn buscar_imagen_en_internet(file_path: String, api_key: String) -> Result<WebSearchResult, String> {
    if !Path::new(&file_path).exists() {
        return Err(format!("File not found: {}", file_path));
    }
    if api_key.trim().is_empty() {
        return Err("Falta la API key de Google Cloud Vision".to_string());
    }

    let data = fs::read(&file_path).map_err(|e| format!("Error reading file: {}", e))?;
    let b64 = general_purpose::STANDARD.encode(&data);

    let body = serde_json::json!({
        "requests": [{
            "image": { "content": b64 },
            "features": [{ "type": "WEB_DETECTION", "maxResults": 20 }]
        }]
    });

    let url = format!("https://vision.googleapis.com/v1/images:annotate?key={}", api_key);
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .map_err(|e| format!("Error de red: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().unwrap_or_default();
        return Err(format!("Error de la API de Google Vision ({}): {}", status, text));
    }

    let json: serde_json::Value = resp.json().map_err(|e| format!("Error parseando respuesta: {}", e))?;
    let web_detection = &json["responses"][0]["webDetection"];

    Ok(WebSearchResult {
        full_matches: extract_matches(&web_detection["fullMatchingImages"], "exacta"),
        partial_matches: extract_matches(&web_detection["partialMatchingImages"], "parcial"),
        pages_with_matches: extract_matches(&web_detection["pagesWithMatchingImages"], "pagina"),
    })
}

