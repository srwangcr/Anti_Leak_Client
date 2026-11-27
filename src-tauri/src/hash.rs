//! Huellas locales: SHA-256 exacto y pHash (DCT 8x8) para imágenes parecidas.

use image::imageops;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// Tamaño de la miniatura usada antes de aplicar la DCT.
const PHASH_SIZE: u32 = 32;
/// Cantidad de coeficientes de la DCT que forman el hash (8x8 = 64 bits).
const PHASH_BITS: usize = 8;

/// Calcula el SHA-256 de un archivo, bit por bit.
pub fn compute_sha256(file_path: &str) -> Result<String, String> {
    let data = fs::read(file_path).map_err(|e| format!("No se pudo leer el archivo: {}", e))?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Ok(hex::encode(hasher.finalize()))
}

/// Calcula el pHash de una imagen con una DCT-II 2D y devuelve 16 caracteres hexadecimales.
pub fn compute_phash(file_path: &str) -> Result<String, String> {
    let img = image::open(Path::new(file_path))
        .map_err(|e| format!("No se pudo abrir/decodificar la imagen: {}", e))?;

    let thumb = imageops::resize(
        &img.to_luma8(),
        PHASH_SIZE,
        PHASH_SIZE,
        imageops::FilterType::Lanczos3,
    );

    let mut coords = [[0.0f64; PHASH_SIZE as usize]; PHASH_SIZE as usize];
    let size = PHASH_SIZE as usize;
    for (i, row) in coords.iter_mut().enumerate().take(size) {
        for (j, value) in row.iter_mut().enumerate().take(size) {
            let mut sum = 0.0f64;
            for y in 0..size {
                for x in 0..size {
                    let pixel = thumb.get_pixel(x as u32, y as u32).0[0] as f64;
                    let a = (((2 * x + 1) * i) as f64 * std::f64::consts::PI) / (2.0 * size as f64);
                    let b = (((2 * y + 1) * j) as f64 * std::f64::consts::PI) / (2.0 * size as f64);
                    sum += pixel * a.cos() * b.cos();
                }
            }
            let ci = if i == 0 { 1.0 / 2.0f64.sqrt() } else { 1.0 };
            let cj = if j == 0 { 1.0 / 2.0f64.sqrt() } else { 1.0 };
            *value = 0.25 * ci * cj * sum;
        }
    }

    // Descartamos el coeficiente DC (0,0): solo aporta el brillo promedio.
    let mut samples: Vec<f64> = Vec::with_capacity(PHASH_BITS * PHASH_BITS - 1);
    for i in 0..PHASH_BITS {
        for j in 0..PHASH_BITS {
            if i == 0 && j == 0 {
                continue;
            }
            samples.push(coords[i][j]);
        }
    }
    let mut sorted = samples.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = sorted[sorted.len() / 2];
    // Margen pequeño proportional a la energía de la imagen: evita que coeficientes
    // casi nulos (ruido de compresión/redimensionado) cambien de bit por nada.
    let max_energy = samples
        .iter()
        .fold(0.0f64, |acc, value| acc.max(value.abs()));
    let threshold = median + 0.002 * max_energy;

    let mut hash = 0u64;
    let mut bit = 0usize;
    for i in 0..PHASH_BITS {
        for j in 0..PHASH_BITS {
            if i == 0 && j == 0 {
                continue;
            }
            if coords[i][j] > threshold {
                hash |= 1u64 << bit;
            }
            bit += 1;
        }
    }

    Ok(format!("{:016x}", hash))
}

/// Convierte un pHash hexadecimal a u64 para comparar distancias.
pub fn phash_to_u64(phash: &str) -> Result<u64, String> {
    u64::from_str_radix(phash.trim(), 16).map_err(|e| format!("pHash inválido: {}", e))
}

/// Distancia de Hamming entre dos pHash (0 = idénticos visualmente).
pub fn hamming_distance(a: &str, b: &str) -> Result<u32, String> {
    let a = phash_to_u64(a)?;
    let b = phash_to_u64(b)?;
    Ok((a ^ b).count_ones())
}

/// Extensiones de imagen que se procesan en un escaneo por lotes.
pub fn es_imagen(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some("png" | "jpg" | "jpeg" | "gif" | "bmp" | "tif" | "tiff" | "webp")
    )
}
