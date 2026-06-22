use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use sha2::{Sha256, Digest};
use image::io::Reader as ImageReader;
use image::GenericImageView;

#[derive(Serialize, Deserialize, Debug)]
pub struct HashResult {
    pub sha256: String,
    pub phash: String,
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

