import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';

const $ = (id) => document.getElementById(id);
const escapeHtml = (value) =>
  String(value ?? '').replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  })[c]);

const fmtBytes = (n) => {
  if (!n && n !== 0) return '—';
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
};

const errorHtml = (error) => `<div class="msg error">${escapeHtml(String(error))}</div>`;
const okHtml = (text) => `<div class="msg ok">${escapeHtml(text)}</div>`;
const infoHtml = (text) => `<div class="msg info">${escapeHtml(text)}</div>`;

const maxDistance = () => {
  const value = parseInt($('maxDistance').value, 10);
  return Number.isFinite(value) ? Math.max(0, Math.min(64, value)) : 8;
};

const currentPath = () => $('filePath').value.trim();

// Cache de la última búsqueda web para no volver a subir la imagen al generar un borrador.
let lastWebResult = null;
let lastWebPath = '';

function showAnalysisError(message) {
  $('analysisResult').innerHTML = errorHtml(message);
  $('imageActions').style.display = 'none';
}

async function pickImage() {
  try {
    const selected = await open({
      multiple: false,
      filters: [{ name: 'Imágenes', extensions: ['png', 'jpg', 'jpeg', 'gif', 'bmp', 'tif', 'tiff', 'webp'] }],
    });
    if (typeof selected === 'string') {
      $('filePath').value = selected;
      await processImage();
    }
  } catch (error) {
    showAnalysisError(error);
  }
}

async function processImage() {
  const filePath = currentPath();
  if (!filePath) {
    showAnalysisError('Ingresá la ruta de una imagen o usá el botón Examinar.');
    return;
  }
  try {
    $('analysisResult').innerHTML = '<div class="msg info">Calculando huellas y metadatos…</div>';
    $('imageActions').style.display = 'none';
    $('webResult').innerHTML = '';
    if (lastWebPath !== filePath) {
      lastWebResult = null;
      lastWebPath = '';
    }
    const analysis = await invoke('analizar_imagen', { filePath, maxDistance: maxDistance() });
    const { hashes, metadata, matches } = analysis;
    const exifRows = Object.entries(metadata.exif || {})
      .slice(0, 12)
      .map(([k, v]) => `<tr><th>${escapeHtml(k)}</th><td>${escapeHtml(v)}</td></tr>`)
      .join('');

    $('analysisResult').innerHTML = `
      <div class="grid">
        <div class="kv"><div class="label">Archivo</div><div class="value">${escapeHtml(metadata.file_name)}</div></div>
        <div class="kv"><div class="label">Dimensiones</div><div class="value">${metadata.width} × ${metadata.height}</div></div>
        <div class="kv"><div class="label">Tamaño</div><div class="value">${fmtBytes(metadata.file_size)}</div></div>
        <div class="kv"><div class="label">Modificado</div><div class="value small">${escapeHtml(metadata.modified_at || '—')}</div></div>
      </div>
      <p class="small">SHA-256 (huella exacta)</p>
      <code>${escapeHtml(hashes.sha256)}</code>
      <p class="small" style="margin-top:10px">pHash DCT 8x8 (huella aproximada)</p>
      <code>${escapeHtml(hashes.phash)}</code>
      ${matches.length ? `<p class="small" style="margin-top:10px">${matches.length} coincidencia(s) en el catálogo local</p>${renderMatchList(matches)}` : ''}
      ${exifRows ? `<p class="small" style="margin-top:12px">EXIF (primeras ${Math.min(12, Object.keys(metadata.exif).length)} etiquetas)</p><table>${exifRows}</table>` : '<p class="small" style="margin-top:12px">Sin metadatos EXIF.</p>'}
    `;
    $('imageActions').style.display = 'flex';
  } catch (error) {
    showAnalysisError(error);
  }
}

function renderMatchList(matches) {
  return `<ul class="list">${matches.map((m) => `
    <li><span class="badge ${escapeHtml(m.kind)}">${escapeHtml(m.kind)}</span>
      ${escapeHtml(m.entry.file_path)}
      <span class="small"> · Hamming: ${m.hamming_distance}</span>
    </li>`).join('')}</ul>`;
}

async function registerImage() {
  const filePath = currentPath();
  if (!filePath) return;
  try {
    const entry = await invoke('registrar_imagen', { filePath, notes: $('notes').value });
    $('analysisResult').insertAdjacentHTML('beforeend', okHtml(`Registrada en el catálogo (id ${entry.id}).`));
    await loadCatalog();
  } catch (error) {
    $('analysisResult').insertAdjacentHTML('beforeend', errorHtml(error));
  }
}

async function loadCatalog() {
  try {
    const entries = await invoke('listar_catalogo');
    $('catalogCount').textContent = `${entries.length} imagen(es) registrada(s)`;
    if (!entries.length) {
      $('catalogResult').innerHTML = infoHtml('El catálogo está vacío. Procesá una imagen y presioná «Registrar en catálogo».');
      return;
    }
    $('catalogResult').innerHTML = `<ul class="list">${entries.map((e) => `
      <li>
        <strong>${escapeHtml(e.file_name)}</strong>
        <div class="small">${escapeHtml(e.file_path)}</div>
        <div class="mono small">SHA-256 ${escapeHtml(e.sha256)} · pHash ${escapeHtml(e.phash)}</div>
        <div class="small">Registrada: ${escapeHtml(e.registered_at)}${e.notes ? ' · ' + escapeHtml(e.notes) : ''}</div>
        <button class="danger" data-delete="${e.id}" style="margin-top:6px">Eliminar</button>
      </li>`).join('')}</ul>`;
  } catch (error) {
    $('catalogResult').innerHTML = errorHtml(error);
  }
}

async function pickFolder() {
  try {
    const selected = await open({ directory: true, multiple: false, title: 'Elegí una carpeta para escanear' });
    if (typeof selected === 'string') await scanFolder(selected);
  } catch (error) {
    $('scanResult').innerHTML = errorHtml(error);
  }
}

async function scanFolder(folderPath) {
  try {
    $('scanResult').innerHTML = '<div class="msg info">Escaneando imágenes… esto puede tardar un momento.</div>';
    const result = await invoke('escanear_carpeta', { folderPath, maxDistance: maxDistance() });
    const duplicates = result.findings.filter((f) => f.kind !== 'nuevo');
    const badges = {
      duplicado_exacto: 'duplicado_exacto', parecido: 'parecido', nuevo: 'nuevo',
    };
    $('scanResult').innerHTML = `
      ${okHtml(`Escaneadas ${result.scanned} imagen(es) · ${duplicates.length} hallazgo(s) · ${result.errors.length} error(es)`)}
      ${result.skipped ? infoHtml(`${result.skipped} imagen(es) omitidas por superar el límite de 5000.`) : ''}
      ${duplicates.length ? `<table><thead><tr><th>Tipo</th><th>Imagen</th><th>Coincide con</th><th>Hamming</th></tr></thead><tbody>
        ${duplicates.map((f) => `<tr>
          <td><span class="badge ${badges[f.kind]}">${escapeHtml(f.kind)}</span></td>
          <td>${escapeHtml(f.file_path)}</td>
          <td>${escapeHtml(f.matched_path)} <span class="small">(${escapeHtml(f.source)})</span></td>
          <td>${f.hamming_distance}</td>
        </tr>`).join('')}
      </tbody></table>` : '<p class="small">No se encontraron duplicados ni copias manipuladas.</p>'}
      ${result.errors.length ? `<div class="msg error">${result.errors.map(escapeHtml).join('<br>')}</div>` : ''}
    `;
  } catch (error) {
    $('scanResult').innerHTML = errorHtml(error);
  }
}

function renderMatchGroup(title, matches) {
  if (!matches || !matches.length) return `<p class="small">${escapeHtml(title)}: ninguna</p>`;
  return `<p class="small"><strong>${escapeHtml(title)} (${matches.length}):</strong></p><ul class="list">${matches.map((m) => `
    <li>
      ${m.title ? `<strong>${escapeHtml(m.title)}</strong><br>` : ''}
      <a href="${escapeHtml(m.url)}" target="_blank" rel="noopener noreferrer">${escapeHtml(m.url)}</a>
      ${m.domain ? `<span class="small"> · ${escapeHtml(m.domain)}</span>` : ''}
    </li>`).join('')}</ul>`;
}

async function searchWeb() {
  const filePath = currentPath();
  if (!filePath) return;
  try {
    $('webResult').innerHTML = '<div class="msg info">Subiendo la imagen a Yandex Images… no se requiere API key.</div>';
    const result = await invoke('buscar_imagen_en_internet', { filePath });
    lastWebResult = result;
    lastWebPath = filePath;
    const total = result.full_matches.length + result.partial_matches.length + result.pages_with_matches.length;
    $('webResult').innerHTML = `
      ${total ? `<div class="msg error">Se encontraron ${total} coincidencia(s) pública(s)</div>` : okHtml('No se encontraron coincidencias públicas.')}
      <p class="small">Proveedor: ${escapeHtml(result.provider)} ·
        <a href="${escapeHtml(result.search_url)}" target="_blank" rel="noopener noreferrer">Abrir búsqueda completa</a></p>
      ${renderMatchGroup('Páginas donde aparece', result.pages_with_matches)}
      ${renderMatchGroup('Imágenes exactas / otros tamaños', result.full_matches)}
      ${renderMatchGroup('Imágenes similares', result.partial_matches)}
    `;
  } catch (error) {
    $('webResult').innerHTML = errorHtml(error);
  }
}

async function generateReport() {
  const filePath = currentPath();
  if (!filePath) return;
  try {
    const outputDir = await open({ directory: true, multiple: false, title: '¿Dónde guardar el informe?' });
    if (typeof outputDir !== 'string') return;
    $('webResult').insertAdjacentHTML('beforeend', infoHtml('Generando informe forense (JSON + PDF)…'));
    const result = await invoke('generar_reporte', {
      filePath,
      notes: $('notes').value,
      outputDir,
      includeWeb: $('includeWeb').checked,
      maxDistance: maxDistance(),
    });
    $('webResult').insertAdjacentHTML('beforeend', okHtml(`Informe generado:<br>JSON: ${result.json_path}<br>PDF: ${result.pdf_path}`));
    if (result.report.web_search_error) {
      $('webResult').insertAdjacentHTML('beforeend', infoHtml(`La búsqueda web no se pudo completar: ${result.report.web_search_error}`));
    }
  } catch (error) {
    $('webResult').insertAdjacentHTML('beforeend', errorHtml(error));
  }
}

async function loadPlatforms() {
  try {
    const platforms = await invoke('listar_plataformas');
    $('platformSelect').innerHTML = platforms
      .map((p) => `<option value="${escapeHtml(p.id)}">${escapeHtml(p.name)}</option>`)
      .join('');
  } catch (error) {
    $('draftResult').innerHTML = errorHtml(error);
  }
}

async function generateDraft() {
  const filePath = currentPath();
  if (!filePath) {
    $('draftResult').innerHTML = errorHtml('Primero analizá una imagen en el paso 1.');
    return;
  }
  try {
    const outputDir = await open({ directory: true, multiple: false, title: '¿Dónde guardar el borrador y las pruebas?' });
    if (typeof outputDir !== 'string') return;
    $('draftResult').innerHTML = '<div class="msg info">Generando informe y borrador…</div>';
    const platformId = $('platformSelect').value;
    const includeWeb = $('includeWeb').checked;
    const result = await invoke('generar_borrador_denuncia', {
      filePath,
      platformId,
      notes: $('notes').value,
      outputDir,
      includeWeb,
      maxDistance: maxDistance(),
      webResultJson: includeWeb && lastWebPath === filePath && lastWebResult
        ? JSON.stringify(lastWebResult)
        : null,
    });
    const draft = result.draft;
    if (!draft.infringing_urls.length) {
      lastWebResult = null;
      lastWebPath = '';
    }
    $('draftResult').innerHTML = `
      ${okHtml('Borrador generado. Revisalo y completá los campos entre corchetes antes de enviar.')}
      <div class="grid">
        <div class="kv"><div class="label">Plataforma</div><div class="value">${escapeHtml(draft.platform.name)}</div></div>
        <div class="kv"><div class="label">Caso</div><div class="value small">${escapeHtml(draft.case_id)}</div></div>
      </div>
      <p class="small">Archivos:</p>
      <ul class="list">
        <li>Borrador Markdown: <code>${escapeHtml(result.markdown_path)}</code></li>
        <li>Borrador JSON: <code>${escapeHtml(result.json_path)}</code></li>
        <li>Prueba PDF: <code>${escapeHtml(result.evidence.pdf_path)}</code></li>
        <li>Prueba JSON: <code>${escapeHtml(result.evidence.json_path)}</code></li>
      </ul>
      ${draft.platform.report_url ? `<p class="small">Formulario oficial: <a href="${escapeHtml(draft.platform.report_url)}" target="_blank" rel="noopener noreferrer">${escapeHtml(draft.platform.report_url)}</a></p>` : '<p class="small">No hay URL fija: buscá «Reportar contenido» o «DMCA» en el sitio.</p>'}
      <p class="small"><strong>Paso a paso</strong></p>
      <ol class="small">${draft.steps.map((s) => `<li>${escapeHtml(s)}</li>`).join('')}</ol>
      <p class="small"><strong>Asunto sugerido</strong></p>
      <code>${escapeHtml(draft.text.subject)}</code>
      <p class="small"><strong>Texto listo para copiar</strong></p>
      <textarea id="draftText" readonly>${escapeHtml(draft.text.body)}</textarea>
      <div class="actions">
        <button class="secondary" data-copy="draftText">Copiar texto</button>
      </div>
      ${result.evidence.report.web_search_error ? infoHtml(`Búsqueda web: ${result.evidence.report.web_search_error}`) : ''}
      <p class="small">${escapeHtml(draft.disclaimer)}</p>
    `;
  } catch (error) {
    $('draftResult').innerHTML = errorHtml(error);
  }
}

async function vaultAction(action) {
  const key = $('vaultKey').value.trim();
  const value = $('vaultValue').value;
  if (!key) {
    $('vaultResult').innerHTML = errorHtml('Ingresá una clave.');
    return;
  }
  try {
    if (action === 'save') {
      await invoke('guardar_credencial', { key, value });
      $('vaultResult').innerHTML = okHtml(`Secreto «${key}» guardado en el keychain del sistema.`);
    } else if (action === 'read') {
      const secret = await invoke('obtener_credencial', { key });
      $('vaultResult').innerHTML = secret
        ? `<div class="kv"><div class="label">${escapeHtml(key)}</div><code>${escapeHtml(secret)}</code></div>`
        : infoHtml('No hay ningún secreto guardado con esa clave.');
    } else {
      await invoke('eliminar_credencial', { key });
      $('vaultResult').innerHTML = okHtml(`Secreto «${key}» eliminado.`);
    }
  } catch (error) {
    $('vaultResult').innerHTML = errorHtml(error);
  }
}

async function setupDragAndDrop() {
  const zone = $('dropzone');
  await listen('tauri://drag-over', () => zone.classList.add('over'));
  await listen('tauri://drag-leave', () => zone.classList.remove('over'));
  await listen('tauri://drag-drop', (event) => {
    zone.classList.remove('over');
    const paths = event.payload?.paths || [];
    if (paths.length) scanFolder(paths[0]);
  });
}

window.addEventListener('DOMContentLoaded', () => {
  $('processBtn').addEventListener('click', processImage);
  $('filePath').addEventListener('keydown', (e) => { if (e.key === 'Enter') processImage(); });
  $('pickFileBtn').addEventListener('click', pickImage);
  $('registerBtn').addEventListener('click', registerImage);
  $('webBtn').addEventListener('click', searchWeb);
  $('reportBtn').addEventListener('click', generateReport);
  $('reloadCatalogBtn').addEventListener('click', loadCatalog);
  $('pickFolderBtn').addEventListener('click', pickFolder);
  $('draftBtn').addEventListener('click', generateDraft);
  $('draftResult').addEventListener('click', async (event) => {
    const button = event.target.closest('[data-copy]');
    if (!button) return;
    try {
      await navigator.clipboard.writeText($(button.dataset.copy).value);
      button.textContent = '¡Copiado!';
    } catch (error) {
      button.textContent = 'No se pudo copiar';
    }
  });
  loadPlatforms();
  $('vaultSaveBtn').addEventListener('click', () => vaultAction('save'));
  $('vaultReadBtn').addEventListener('click', () => vaultAction('read'));
  $('vaultDeleteBtn').addEventListener('click', () => vaultAction('delete'));
  $('catalogResult').addEventListener('click', async (event) => {
    const button = event.target.closest('[data-delete]');
    if (!button) return;
    try {
      await invoke('eliminar_del_catalogo', { id: Number(button.dataset.delete) });
      await loadCatalog();
    } catch (error) {
      $('catalogResult').insertAdjacentHTML('afterbegin', errorHtml(error));
    }
  });
  setupDragAndDrop();
  loadCatalog();
});
