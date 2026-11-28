# Anti Leak Client

Aplicación de escritorio (Tauri 2 + Rust) que procesa imágenes localmente,
calcula sus hashes, mantiene un catálogo forense en SQLite, detecta duplicados
en carpetas completas, genera informes PDF/JSON y prepara **borradores de
denuncia** para reportar fotos filtradas. **No depende de Google ni de API keys
de pago.**

## ¿Qué hace esto, explicado sin tecnicismos?

Pensalo como una "huella digital" para una foto:

1. **Le das una foto** a la app (ruta completa o botón *Examinar*).
2. La app calcula dos tipos de "huella" en tu computadora:
   - Una **huella exacta** (SHA-256): si alguien cambia un solo píxel, cambia
     por completo. Sirve para probar que un archivo es *idéntico, bit por bit*.
   - Una **huella aproximada** (pHash con DCT 8x8): sigue pareciéndose aunque
     la imagen se comprima, se le cambie el tamaño o se recorte un poco.
3. Podés **registrar la imagen en un catálogo local** (SQLite) junto con sus
   hashes, dimensiones, fecha y metadatos EXIF.
4. Podés **arrastrar una carpeta entera** y la app compara todas las imágenes
   contra el catálogo y entre sí, usando distancia de Hamming sobre el pHash.
   Detecta duplicados exactos y copias manipuladas sin consumir cuota de ninguna API.
5. Opcionalmente, con un solo botón, busca la imagen en la web mediante
   **Yandex Images** (búsqueda inversa pública, gratuita y sin cuenta) y muestra
   las páginas donde aparece.
6. Podés generar un **informe forense en PDF y JSON** con fecha, metadatos EXIF,
   hashes, coincidencias locales, URLs encontradas en la web y un digest SHA-256
   que permite detectar si el informe fue modificado después.
7. Con el informe listo, podés generar un **borrador de denuncia semiautomático**:
   texto ya redactado con hashes y URLs, el enlace al formulario oficial de la
   plataforma y un paso a paso. Vos revisás, completás tus datos y enviás; la app
   **nunca envía nada por vos**.

## Flujo de trabajo (técnico)

```
[Usuario ingresa ruta / elige archivo]
        │
        ▼
[Frontend (src/main.js) llama invoke('analizar_imagen')]
        │
        ▼
[lib.rs: SHA-256 + pHash DCT + EXIF + dimensiones, todo local]
        │
        ├──► registrar_imagen ──► SQLite (catálogo persistente)
        ├──► escanear_carpeta ──► walkdir + distancia de Hamming (lotes)
        ├──► buscar_imagen_en_internet ──► Yandex Images (sin API key)
        ├──► generar_reporte ──► informe_*.json + informe_*.pdf
        └──► generar_borrador_denuncia ──► denuncia_*.json + denuncia_*.md
                                            (reutiliza la búsqueda web previa)
```

### Búsqueda en internet sin Google

No existe un buscador público de "hashes"; ninguna herramienta te deja
preguntar "¿alguien tiene un archivo con este hash exacto?". Para encontrar
dónde aparece una foto en la web hay que enviar la imagen real a un buscador
de imágenes inverso.

Antes se usaba Google Cloud Vision (API key, cuota y costo). **Esa dependencia
fue eliminada por completo.** Ahora se usa el formulario público de
**Yandex Images (CBIR)**: no necesita registro, API key ni tarjeta, y es el
mismo flujo que usa una persona desde el navegador.

- La imagen **solo se sube** cuando el usuario presiona *Buscar en internet*
  o cuando genera un informe con la casilla *Incluir búsqueda web* activada.
- El catálogo, el escaneo, los hashes, el EXIF y los informes funcionan
  **100% offline**.
- Si Yandex muestra una verificación anti-bot o hay un problema de red, la app
  lo informa y el informe se genera igual, indicando el motivo en el apartado
  "Coincidencias en la web".

### Denuncia semiautomática (borrador para reportar fotos)

Es una mezcla entre "informe automático" y "borrador de denuncia": la app hace
el trabajo pesado (evidencia + redacción) y la persona conserva el control del
envío. **No se envía nada automáticamente a ninguna plataforma ni autoridad.**

En la sección **4. Denuncia semiautomática** de la interfaz:

1. Elegís la plataforma (Instagram/Threads, Facebook, TikTok, X o un sitio web
   genérico) y presionás *Generar borrador de denuncia*.
2. La app genera primero el informe forense (PDF + JSON) y después un borrador
   con:
   - **Texto listo para copiar** con nombre de archivo, SHA-256, pHash,
     dimensiones, fecha y las URLs detectadas.
   - **Asunto sugerido** y campos entre corchetes para completar tu nombre,
     correo y relación con la imagen.
   - **Enlace al formulario oficial** de la plataforma y un **paso a paso**.
   - **Checklist** de verificación (confirmar titularidad, revisar cada URL,
     guardar el número de caso, etc.).
   - **Aviso legal**: es una herramienta de apoyo, no asesoría legal; las URLs
     pueden cambiar y hay que verificarlas en el sitio oficial.
3. Guarda los archivos en la carpeta que elijas: `denuncia_<plataforma>_<caso>.md`
   (legible), `denuncia_<plataforma>_<caso>.json` (estructurado) y, junto a ellos,
   el `informe_*.pdf` / `informe_*.json` ya generado como prueba.
4. Vos copiás el texto, abrís el enlace oficial, pegás las URLs verificadas,
   adjuntás la foto original y el PDF, y guardás el comprobante.

Notas importantes:

- Si ya hiciste *Buscar en internet* para esa misma imagen, el resultado se
  **reutiliza** y la foto **no se sube dos veces** a Yandex.
- Si no hiciste la búsqueda, se sube solo si la casilla *Incluir búsqueda web*
  está activada. Si no hay URLs, el borrador deja un marcador `[Pegá aquí la URL]`.
- Los enlaces de reporte son los canales públicos de copyright de cada
  plataforma al momento de escribir esta documentación; pueden cambiar.

### Bóveda segura (opcional)

Las credenciales de posibles integraciones futuras se guardan en el
**keychain nativo del sistema operativo** (Keychain en macOS, Credential
Manager en Windows, libsecret/gnome-keyring en Linux) mediante `keyring`.
Nunca se escriben secretos en la base de datos ni en archivos de texto.
Yandex no requiere credenciales, así que esta sección es opcional.

## Requisitos

- [Rust](https://www.rust-lang.org/tools/install) (edición 2021+, 1.89+)
- [Node.js](https://nodejs.org/) 18+ y npm
- Dependencias de sistema de Tauri para tu plataforma: ver la
  [guía de prerrequisitos de Tauri](https://v2.tauri.app/start/prerequisites/)
  (en Linux normalmente `webkit2gtk`, `libayatana-appindicator3-dev`, etc.)
- Para la bóveda en Linux: `libsecret-1-dev` y un keyring activo
  (por ejemplo `gnome-keyring`). En macOS y Windows no hace falta instalar nada.

## Instalación

```bash
npm install
```

## Desarrollo

```bash
npm run dev
```

Esto levanta el servidor de Vite y abre la ventana nativa de Tauri con
hot-reload.

## Pruebas

```bash
cd src-tauri
cargo test                       # hashes, catálogo, escaneo, informes y borradores
cargo test -- --ignored          # incluye la prueba de red contra Yandex
```

## Build de producción (para tu propio sistema operativo)

```bash
npm run build:app
```

Compila el frontend y genera el instalador en
`src-tauri/target/release/bundle/`, pero
**solo para el sistema operativo en el que corras el comando**. Tauri
empaqueta contra las librerías nativas del SO (WebView2 en Windows,
WebKit en macOS, webkit2gtk en Linux), así que no hay "compilación
cruzada" real: un Mac genera `.dmg`/`.app`, un Windows genera
`.msi`/`.exe`, un Linux genera `.deb`/`.rpm` (el AppImage se desactivó
porque depende de `linuxdeploy`/FUSE y fallaba en CI).

## Exportar a multiplataforma (Windows + macOS + Linux)

**Sí, se puede exportar a Windows.** El proyecto ya está preparado: el workflow
de GitHub Actions compila un `.msi` / `.exe` en un runner `windows-latest`, sin
necesidad de tener Windows en tu máquina. Lo mismo vale para macOS y Linux.

Como no podés generar los tres instaladores desde una sola máquina,
la forma estándar es compilar en la nube, una vez por sistema
operativo, usando GitHub Actions. Este repo incluye
`.github/workflows/release.yml`, que:

1. Se dispara cuando hacés push de un tag tipo `v0.2.0`.
2. Corre en paralelo sobre runners de `macos-latest`, `ubuntu-latest`
   y `windows-latest`.
3. En cada uno instala Rust + Node, corre `npm install`, y usa la
   acción oficial `tauri-apps/tauri-action` para compilar y adjuntar
   los instaladores a un GitHub Release (como borrador).

Para usarlo:

```bash
git tag v0.2.0
git push origin v0.2.0
```

Y revisá la pestaña **Actions** de GitHub — al terminar vas a tener un
Release con los tres instaladores listos para descargar.

## ¿Y Android?

**También es posible, pero hoy no está configurado**: Tauri 2 soporta Android,
así que el núcleo Rust (hashes, pHash, EXIF, SQLite, informes y borradores)
podría reutilizarse casi sin cambios. Lo que impide publicar un `.apk` hoy es
que el proyecto aún no tiene el proyecto móvil inicializado. Para hacerlo:

1. Instalar Android Studio y el **Android SDK / NDK**, y configurar
   `JAVA_HOME` / `ANDROID_HOME` (la [guía oficial de Tauri](https://v2.tauri.app/start/prerequisites/#android)
   lista los pasos).
2. Ejecutar `npm run tauri android init` para generar `src-tauri/gen/android`.
3. Adaptar la UI y los permisos móviles (`src-tauri/capabilities/mobile.toml`):
   - El plugin `dialog` en Android **no tiene selector de carpetas**: hay que
     usar el selector de archivos (`content://` URIs) y, para la bóveda,
     `tauri-plugin-fs` con los permisos correspondientes.
   - El keyring debe usar el **Android Keystore** en vez de libsecret/Keychain.
   - La interfaz necesita un layout responsive y el escaneo por carpetas debe
     reformularse como "seleccionar varias fotos".
4. Agregar al workflow un job que corra `npm run tauri android build` y firme
   el `.aab`/`.apk` con un keystore propio (Google Play exige firma).
5. Probar en un dispositivo real o emulador; la búsqueda inversa y el keyring
   se comportan distinto que en escritorio.

Recomendación: primero consolidar la versión de escritorio (Fase 5) y después
abrir Android como un hito separado, porque implica otra curva de pruebas y
firma. No es un cambio de una línea, pero **es alcanzable con la arquitectura
actual**.

## Uso

1. Abrí la app.
2. En **1. Analizar una imagen**, pegá la ruta completa o presioná
   *Examinar…* y después *Procesar*. Vas a ver SHA-256, pHash, dimensiones
   y las primeras etiquetas EXIF.
3. Escribí notas del caso si querés y presioná *Registrar en catálogo*.
4. En **2. Catálogo local** podés ver y eliminar las imágenes registradas.
5. En **3. Escaneo por lotes**, arrastrá una carpeta o elegila con el botón.
   Ajustá la tolerancia de pHash (por defecto 8 sobre 64 bits) y revisá los
   duplicados/copias manipuladas detectados.
6. Para buscar en internet, presioná *Buscar en internet* (no pedimos ninguna
   API key). Para dejar evidencia, elegí la carpeta destino y presioná
   *Generar informe PDF + JSON*.
7. En **4. Denuncia semiautomática**, elegí la plataforma y presioná
   *Generar borrador de denuncia*: se crean el borrador y las pruebas. Revisá
   el texto, completá tus datos y enviá el reporte vos mismo desde el enlace
   oficial de la plataforma.

Los datos se guardan en la carpeta de configuración de la app
(`~/.config/com.antileak.client/` en Linux; equivalente en macOS/Windows):

- `catalog.sqlite` — catálogo de imágenes registradas.
- `informe_<fecha>_<hash>.json` / `.pdf` — informes que elijas generar.
- `denuncia_<plataforma>_<caso>.md` / `.json` — borradores de denuncia, en la
  carpeta que elijas al generarlos (junto con el informe PDF/JSON).

## Estructura

- `src/index.html` y `src/main.js` — frontend (HTML/JS con Vite, sin framework).
- `src-tauri/src/hash.rs` — SHA-256, pHash DCT y distancia de Hamming.
- `src-tauri/src/metadata.rs` — dimensiones, fecha de modificación y EXIF.
- `src-tauri/src/catalog.rs` — persistencia SQLite del catálogo.
- `src-tauri/src/vault.rs` — secretos en el keychain nativo.
- `src-tauri/src/scanner.rs` — escaneo recursivo de carpetas y comparación local.
- `src-tauri/src/web.rs` — búsqueda inversa en Yandex (reemplaza a Google).
- `src-tauri/src/report.rs` — generación de informes JSON y PDF.
- `src-tauri/src/reporting.rs` — borradores de denuncia y canales oficiales.
- `src-tauri/tests/integration.rs` — pruebas automáticas del núcleo.
- `.github/workflows/release.yml` — pipeline de release multiplataforma.

## Hoja de Ruta y Madurez del Producto

Estado actual: **v0.2.0 — Evidencia forense local** (catálogo SQLite, keychain,
escaneo por lotes, búsqueda inversa gratuita sin Google, informes PDF/JSON y
borradores de denuncia semiautomáticos).

- [x] **Fase 1 — Core Local & Búsqueda Puntual:** interfaz Tauri 2 + backend Rust
  calculando SHA-256 y pHash localmente, búsqueda inversa bajo demanda y workflow
  de compilación para Linux/Windows/macOS. La integración original con Google
  Cloud Vision fue reemplazada por Yandex Images (gratuita, sin API key).
- [x] **Fase 2 — Almacenamiento Seguro & Base de Datos Local:** credenciales
  opcionales en el keychain nativo (`keyring`) y catálogo SQLite persistente de
  imágenes con SHA-256, pHash, dimensiones, EXIF y notas.
- [x] **Fase 3 — Escaneo por Lotes y Comparación Local:** selección de carpetas
  y drag & drop, escaneo recursivo con `walkdir`, detección de duplicados
  exactos y copias manipuladas por distancia de Hamming, sin consumir cuota de API.
- [x] **Fase 4 — Evidencia Forense y Reportes:** informes PDF y JSON con marca
  de tiempo UTC, metadatos EXIF, hashes criptográficos, coincidencias locales,
  URLs de coincidencias web y digest SHA-256 de integridad del informe. Además,
  borradores de denuncia semiautomáticos (texto + paso a paso + checklist) para
  reportar la foto en plataformas, con revisión humana antes de enviar.
- [ ] **Fase 5 — Distribución Firmada:** firma digital de binarios (Apple
  Notarization / Windows Authenticode) y actualizaciones automáticas integradas
  mediante `tauri-plugin-updater`.
- [ ] **Fase 6 — Android (plan):** inicializar el proyecto móvil de Tauri 2,
  adaptar permisos, selector de archivos, Android Keystore y UI responsive,
  y publicar un `.aab` firmado. Ver la sección «¿Y Android?» más arriba.
