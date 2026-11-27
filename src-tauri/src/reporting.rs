//! Denuncia semiautomática: borrador estructurado + guía paso a paso para
//! reportar fotos filtradas en plataformas. Nada se envía automáticamente.

use crate::catalog::CatalogMatch;
use crate::metadata::ImageMetadata;
use crate::report::{generate_report_from, Hashes, ReportResult};
use crate::web::WebMatch;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlatformInfo {
    pub id: String,
    pub name: String,
    pub report_url: String,
    pub category: String,
    pub steps: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ComplaintText {
    pub subject: String,
    pub body: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ComplaintDraft {
    pub case_id: String,
    pub generated_at: String,
    pub status: String,
    pub platform: PlatformInfo,
    pub image: ImageMetadata,
    pub hashes: Hashes,
    pub infringing_urls: Vec<WebMatch>,
    pub local_matches: Vec<CatalogMatch>,
    pub text: ComplaintText,
    pub steps: Vec<String>,
    pub attachments: Vec<String>,
    pub checklist: Vec<String>,
    pub disclaimer: String,
    pub notes: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ComplaintDraftResult {
    pub json_path: String,
    pub markdown_path: String,
    pub evidence: ReportResult,
    pub draft: ComplaintDraft,
}

/// Plataformas con canal oficial de reporte. Las URLs pueden cambiar: verificar antes de enviar.
pub fn platforms() -> Vec<PlatformInfo> {
    vec![
        PlatformInfo {
            id: "instagram".into(),
            name: "Instagram / Threads".into(),
            report_url: "https://help.instagram.com/contact/552695131608132".into(),
            category: "Infracción de derechos de autor (copyright)".into(),
            steps: vec![
                "Abrí el formulario oficial de copyright de Instagram/Threads.".into(),
                "Elegí «Soy el titular de los derechos» y completá tus datos de contacto.".into(),
                "Pegá la URL exacta del post, reel o perfil donde aparece tu foto.".into(),
                "Adjuntá el archivo original y el informe PDF como prueba de titularidad.".into(),
            ],
        },
        PlatformInfo {
            id: "facebook".into(),
            name: "Facebook".into(),
            report_url: "https://www.facebook.com/help/325058084212425".into(),
            category: "Infracción de derechos de autor (copyright)".into(),
            steps: vec![
                "Entrá al centro de ayuda de Facebook y abrí el formulario de copyright.".into(),
                "Indicá tu relación con la imagen y tus datos de contacto.".into(),
                "Pegá el enlace directo al contenido que usa tu foto sin permiso.".into(),
                "Adjuntá el original y el informe PDF; guardá el número de reporte.".into(),
            ],
        },
        PlatformInfo {
            id: "tiktok".into(),
            name: "TikTok".into(),
            report_url: "https://ipr.tiktokforbusiness.com/legal/report/Copyright".into(),
            category: "Copyright infringement".into(),
            steps: vec![
                "Abrí el portal de propiedad intelectual de TikTok (no necesitás cuenta).".into(),
                "Seleccioná el tipo de infracción y completá tus datos.".into(),
                "Pegá la URL completa de cada video donde aparece tu imagen.".into(),
                "Firmá electrónicamente la declaración de buena fe y adjuntá las pruebas.".into(),
            ],
        },
        PlatformInfo {
            id: "x".into(),
            name: "X (antes Twitter)".into(),
            report_url: "https://help.x.com/rules-and-policies/twitter-copyright".into(),
            category: "DMCA / copyright".into(),
            steps: vec![
                "Revisá la política de copyright de X y abrí el canal de reporte indicado.".into(),
                "Completá nombre, correo, dirección y relación con la imagen.".into(),
                "Pegá la URL de cada publicación que contiene tu foto.".into(),
                "Incluí la declaración de buena fe y tu firma electrónica.".into(),
            ],
        },
        PlatformInfo {
            id: "web".into(),
            name: "Sitio web / hosting (genérico)".into(),
            report_url: String::new(),
            category: "Uso no autorizado de imagen / DMCA".into(),
            steps: vec![
                "Buscá en el sitio el enlace «Contacto», «Reportar contenido» o «DMCA».".into(),
                "Si el sitio está alojado en otro proveedor, identificá al hosting y usá su canal de abuso.".into(),
                "Enviá el texto del borrador con las URLs verificadas y el informe PDF.".into(),
                "Guardá el comprobante y el número de caso para seguimiento.".into(),
            ],
        },
    ]
}

fn find_platform(id: &str) -> Result<PlatformInfo, String> {
    platforms()
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| format!("Plataforma desconocida: {}", id))
}

fn build_text(platform: &PlatformInfo, draft: &ComplaintDraft) -> ComplaintText {
    let mut urls = String::new();
    if draft.infringing_urls.is_empty() {
        urls.push_str("1. [Pegá aquí la URL del contenido donde aparece tu foto]\n");
    } else {
        for (i, m) in draft.infringing_urls.iter().enumerate() {
            urls.push_str(&format!("{}. {}\n", i + 1, m.url));
        }
    }

    let body = format!(
        "Estimado equipo de {platform_name}:\n\n\
         Soy titular (o representante autorizado/a) de la imagen identificada abajo y \
         NO he autorizado su uso en el contenido señalado. Por medio de la presente \
         solicito la retirada del material por presunta infracción de derechos de autor \
         y uso no autorizado de imagen.\n\n\
         DATOS DE LA IMAGEN\n\
         - Archivo original: {file}\n\
         - SHA-256: {sha}\n\
         - pHash (DCT 8x8): {phash}\n\
         - Dimensiones: {width} x {height}\n\
         - Fecha de modificación del archivo: {modified}\n\n\
         URLs DONDE APARECE SIN AUTORIZACIÓN (verificar antes de enviar):\n\
         {urls}\n\
         Declaro que la información es veraz y que estoy autorizado/a a presentar esta \
         denuncia. Quedo a disposición para aportar el archivo original y evidencia adicional.\n\n\
         Nombre completo: [Tu nombre]\n\
         Correo de contacto: [Tu correo]\n\
         Relación con la imagen: [Titular / representante / autor]\n\
         Firma: [Tu nombre]\n\
         Fecha: {date}\n\n\
         Notas del caso: {notes}",
        platform_name = platform.name,
        file = draft.image.file_name,
        sha = draft.hashes.sha256,
        phash = draft.hashes.phash,
        width = draft.image.width,
        height = draft.image.height,
        modified = draft.image.modified_at,
        urls = urls,
        date = draft.generated_at,
        notes = if draft.notes.is_empty() {
            "—"
        } else {
            &draft.notes
        },
    );

    ComplaintText {
        subject: format!(
            "Solicitud de retirada por uso no autorizado de imagen — {} — {}",
            draft.image.file_name,
            &draft.hashes.sha256.chars().take(12).collect::<String>()
        ),
        body,
    }
}

fn markdown(draft: &ComplaintDraft) -> String {
    let urls = if draft.infringing_urls.is_empty() {
        "- [Pegá aquí la URL del contenido]".to_string()
    } else {
        draft
            .infringing_urls
            .iter()
            .map(|m| format!("- {} — {}", m.url, m.title))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let steps = draft
        .steps
        .iter()
        .enumerate()
        .map(|(i, s)| format!("{}. {}", i + 1, s))
        .collect::<Vec<_>>()
        .join("\n");
    let checklist = draft
        .checklist
        .iter()
        .map(|c| format!("- [ ] {}", c))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "# Borrador de denuncia — {}\n\n\
         **Caso:** {}\n\
         **Generado (UTC):** {}\n\
         **Plataforma:** {}\n\
         **Estado:** {}\n\n\
         > Este borrador NO se envió automáticamente. Revisalo y completá los campos entre corchetes.\n\n\
         ## Paso a paso\n\n{}\n\n\
         ## Asunto sugerido\n\n{}\n\n\
         ## Texto listo para copiar\n\n```text\n{}\n```\n\n\
         ## URLs detectadas\n\n{}\n\n\
         ## Archivos para adjuntar\n\n{}\n\n\
         ## Checklist antes de enviar\n\n{}\n\n\
         ## Aviso\n\n{}\n",
        draft.platform.name,
        draft.case_id,
        draft.generated_at,
        draft.platform.name,
        draft.status,
        steps,
        draft.text.subject,
        draft.text.body,
        urls,
        draft
            .attachments
            .iter()
            .map(|a| format!("- {}", a))
            .collect::<Vec<_>>()
            .join("\n"),
        checklist,
        draft.disclaimer,
    )
}

/// Genera el informe forense y, a partir de él, el borrador de denuncia.
pub fn generate_complaint_draft(
    conn: &rusqlite::Connection,
    file_path: &str,
    platform_id: &str,
    notes: &str,
    output_dir: &str,
    include_web: bool,
    max_distance: u32,
    cached_web_json: Option<&str>,
) -> Result<ComplaintDraftResult, String> {
    let platform = find_platform(platform_id)?;
    let web_override = match cached_web_json {
        Some(json) if !json.trim().is_empty() => Some(
            serde_json::from_str(json)
                .map_err(|e| format!("No se pudo reutilizar la búsqueda web: {}", e))?,
        ),
        _ => None,
    };

    let evidence = generate_report_from(
        conn,
        file_path,
        notes,
        output_dir,
        include_web,
        max_distance,
        web_override,
    )?;

    let infringing_urls = evidence
        .report
        .web_search
        .as_ref()
        .map(|w| {
            let mut all = w.pages_with_matches.clone();
            all.extend(w.full_matches.clone());
            all.extend(w.partial_matches.clone());
            all
        })
        .unwrap_or_default();

    let mut steps: Vec<String> = vec![
        "Revisá el informe forense PDF/JSON y confirmá que las URLs corresponden a tu imagen."
            .into(),
        "Completá los campos entre corchetes del texto (nombre, correo y relación con la imagen)."
            .into(),
    ];
    steps.extend(platform.steps.clone());
    steps.push("Guardá el número de caso o comprobante que te devuelva la plataforma.".into());

    let mut draft = ComplaintDraft {
        case_id: evidence.report.report_id.clone(),
        generated_at: evidence.report.generated_at.clone(),
        status: "Borrador — requiere revisión humana antes de enviar".into(),
        platform: platform.clone(),
        image: evidence.report.image.clone(),
        hashes: evidence.report.hashes.clone(),
        infringing_urls,
        local_matches: evidence.report.local_matches.clone(),
        text: ComplaintText {
            subject: String::new(),
            body: String::new(),
        },
        steps,
        attachments: vec![
            evidence.report.image.file_name.clone(),
            PathBuf::from(&evidence.pdf_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("informe.pdf")
                .to_string(),
            PathBuf::from(&evidence.json_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("informe.json")
                .to_string(),
        ],
        checklist: vec![
            "Confirmá que sos el titular o representante autorizado de la imagen.".into(),
            "Verificá una por una las URLs antes de incluirlas en la denuncia.".into(),
            "No envíes una denuncia con datos falsos: puede tener consecuencias legales.".into(),
            "Guardá una copia del envío y del número de caso.".into(),
            "Si la plataforma no responde, podés reenviar con el número de caso.".into(),
        ],
        disclaimer: "Herramienta de apoyo: genera evidencia y un borrador. No envía datos \
                     a ninguna plataforma ni constituye asesoría legal. Las URLs de reporte \
                     pueden cambiar; verificalas en el sitio oficial."
            .into(),
        notes: notes.to_string(),
    };
    draft.text = build_text(&platform, &draft);

    let dir = PathBuf::from(output_dir);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("No se pudo crear la carpeta de denuncias: {}", e))?;
    let base = format!("denuncia_{}_{}", platform.id, draft.case_id);
    let json_path = dir.join(format!("{}.json", base));
    let markdown_path = dir.join(format!("{}.md", base));
    std::fs::write(
        &json_path,
        serde_json::to_string_pretty(&draft).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("No se pudo guardar el JSON de la denuncia: {}", e))?;
    std::fs::write(&markdown_path, markdown(&draft))
        .map_err(|e| format!("No se pudo guardar el borrador: {}", e))?;

    Ok(ComplaintDraftResult {
        json_path: json_path.to_string_lossy().to_string(),
        markdown_path: markdown_path.to_string_lossy().to_string(),
        evidence,
        draft,
    })
}
