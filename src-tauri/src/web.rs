//! Búsqueda inversa en la web usando Yandex Images (CBIR).
//! No requiere API key, cuenta ni pago: es el mismo formulario público que usa el navegador.
//! Reemplaza por completo a Google Cloud Vision.

use crate::metadata::read_metadata;
use reqwest::blocking::{
    multipart::{Form, Part},
    Client,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WebMatch {
    pub url: String,
    pub title: String,
    pub domain: String,
    /// "exacta" | "parcial" | "pagina"
    pub match_type: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WebSearchResult {
    pub provider: String,
    pub search_url: String,
    pub full_matches: Vec<WebMatch>,
    pub partial_matches: Vec<WebMatch>,
    pub pages_with_matches: Vec<WebMatch>,
}

fn build_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(90))
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0 Safari/537.36")
        .build()
        .map_err(|e| format!("No se pudo crear el cliente HTTP: {}", e))
}

fn html_unescape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '&' {
            out.push(c);
            continue;
        }
        let mut entity = String::new();
        while let Some(&next) = chars.peek() {
            chars.next();
            if next == ';' {
                break;
            }
            if entity.len() > 12 {
                break;
            }
            entity.push(next);
        }
        let decoded = match entity.as_str() {
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" | "#39" => Some('\''),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "nbsp" => Some(' '),
            _ => {
                if let Some(hex) = entity
                    .strip_prefix("#x")
                    .or_else(|| entity.strip_prefix("#X"))
                {
                    u32::from_str_radix(hex, 16).ok().and_then(char::from_u32)
                } else if let Some(dec) = entity.strip_prefix('#') {
                    dec.parse::<u32>().ok().and_then(char::from_u32)
                } else {
                    None
                }
            }
        };
        match decoded {
            Some(ch) => out.push(ch),
            None => {
                out.push('&');
                out.push_str(&entity);
                out.push(';');
            }
        }
    }
    out
}

fn data_states(html: &str) -> Vec<String> {
    const MARKER: &str = "data-state=\"";
    let mut result = Vec::new();
    let mut rest = html;
    while let Some(idx) = rest.find(MARKER) {
        rest = &rest[idx + MARKER.len()..];
        match rest.find('"') {
            Some(end) => {
                if end > 100 {
                    result.push(rest[..end].to_string());
                }
                rest = &rest[end + 1..];
            }
            None => break,
        }
    }
    result
}

fn clean_url(url: &str) -> String {
    let url = url.trim();
    if url.starts_with("//") {
        return format!("https:{}", url);
    }
    match url.split_once('?') {
        Some((base, query)) => {
            let kept: Vec<&str> = query
                .split('&')
                .filter(|part| !part.starts_with("utm_") && !part.is_empty())
                .collect();
            if kept.is_empty() {
                base.to_string()
            } else {
                format!("{}?{}", base, kept.join("&"))
            }
        }
        None => url.to_string(),
    }
}

fn match_from(value: &Value, match_type: &str) -> Option<WebMatch> {
    let url = value
        .get("url")
        .or_else(|| value.get("origUrl"))
        .or_else(|| value.get("imageUrl"))
        .and_then(Value::as_str)?;
    Some(WebMatch {
        url: clean_url(url),
        title: value
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        domain: value
            .get("domain")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        match_type: match_type.to_string(),
    })
}

fn push_all(target: &mut Vec<WebMatch>, value: Option<&Value>, label: &str) {
    if let Some(items) = value.and_then(Value::as_array) {
        for item in items {
            if let Some(m) = match_from(item, label) {
                if !target.iter().any(|existing| existing.url == m.url) {
                    target.push(m);
                }
            }
        }
    }
}

/// Sube la imagen al buscador público de Yandex y devuelve las páginas donde aparece.
pub fn buscar_imagen_en_internet(file_path: &str) -> Result<WebSearchResult, String> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(format!("Archivo no encontrado: {}", file_path));
    }
    let data = std::fs::read(path).map_err(|e| format!("No se pudo leer la imagen: {}", e))?;
    let mime = read_metadata(file_path)?.mime_type;
    let client = build_client()?;

    // 1) Subir la imagen. El ID CBIR identifica la búsqueda inversa.
    let part = Part::bytes(data)
        .file_name("blob".to_string())
        .mime_str(&mime)
        .map_err(|e| format!("No se pudo preparar la imagen: {}", e))?;
    let form = Form::new().part("upfile", part);
    let request_body = r#"{"blocks":[{"block":"b-page_type_search-by-image__link"}]}"#;
    let upload: Value = client
        .post("https://yandex.com/images/search")
        .query(&[
            ("rpt", "imageview"),
            ("format", "json"),
            ("request", request_body),
        ])
        .multipart(form)
        .send()
        .map_err(|e| format!("Error de red al subir la imagen: {}", e))?
        .error_for_status()
        .map_err(|e| format!("Yandex rechazó la imagen: {}", e))?
        .json()
        .map_err(|e| format!("No se pudo interpretar la respuesta de Yandex: {}", e))?;

    let cbir_id = upload["blocks"][0]["params"]["cbirId"]
        .as_str()
        .ok_or_else(|| {
            "Yandex no devolvió un identificador de búsqueda (¿captcha o imagen no válida?)"
                .to_string()
        })?
        .to_string();

    // 2) Pedir la pestaña de páginas donde aparece la imagen.
    let response = client
        .get("https://yandex.com/images/search")
        .query(&[
            ("rpt", "imageview"),
            ("cbir_page", "sites"),
            ("cbir_id", cbir_id.as_str()),
        ])
        .send()
        .map_err(|e| format!("Error de red al buscar coincidencias: {}", e))?
        .error_for_status()
        .map_err(|e| format!("Yandex devolvió un error: {}", e))?
        .text()
        .map_err(|e| format!("No se pudo leer la respuesta de Yandex: {}", e))?;

    if response.contains("Are you not a robot?") || response.contains("SmartCaptcha") {
        return Err(
            "Yandex pidió verificación anti-bot. Probá de nuevo en unos minutos o desde otra red."
                .into(),
        );
    }

    let search_url = format!(
        "https://yandex.com/images/search?rpt=imageview&cbir_page=sites&cbir_id={}",
        cbir_id
    );

    let mut full_matches: Vec<WebMatch> = Vec::new();
    let mut partial_matches: Vec<WebMatch> = Vec::new();
    let mut pages_with_matches: Vec<WebMatch> = Vec::new();

    for state in data_states(&response) {
        let unescaped = html_unescape(&state);
        let json: Value = match serde_json::from_str(&unescaped) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let initial = match json.get("initialState") {
            Some(v) => v,
            None => continue,
        };

        push_all(
            &mut pages_with_matches,
            initial.pointer("/cbirSites/sites"),
            "pagina",
        );

        if let Some(sizes) = initial.pointer("/cbirOtherSizes/items") {
            for key in ["small_dups", "medium_dups", "large_dups"] {
                push_all(&mut full_matches, sizes.get(key), "exacta");
            }
        }

        if let Some(thumbs) = initial
            .pointer("/cbirSimilar/thumbs")
            .and_then(Value::as_array)
        {
            for thumb in thumbs {
                if let Some(m) = match_from(thumb, "parcial") {
                    if !partial_matches.iter().any(|e| e.url == m.url) {
                        partial_matches.push(m);
                    }
                }
            }
        }

        if !pages_with_matches.is_empty() || !full_matches.is_empty() {
            break;
        }
    }

    Ok(WebSearchResult {
        provider: "Yandex Images (búsqueda inversa gratuita)".into(),
        search_url,
        full_matches,
        partial_matches,
        pages_with_matches,
    })
}
