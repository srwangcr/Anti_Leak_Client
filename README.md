# Anti Leak Client (contraconte)

Aplicación de escritorio (Tauri 2 + Rust) que procesa imágenes localmente y
calcula su hash SHA-256 y un hash perceptual (pHash) simplificado.

## Requisitos

- [Rust](https://www.rust-lang.org/tools/install) (edición 2021+)
- [Node.js](https://nodejs.org/) 18+ y npm
- Dependencias de sistema de Tauri para tu plataforma: ver la
  [guía de prerrequisitos de Tauri](https://v2.tauri.app/start/prerequisites/)
  (en Linux normalmente `webkit2gtk`, `libayatana-appindicator3-dev`, etc.)

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

## Build de producción

```bash
npm run build
```

Genera el bundle instalable en `src-tauri/target/release/bundle/`.

## Uso

1. Abrí la app.
2. Pegá la ruta completa de una imagen local (ej: `/home/usuario/foto.png`).
3. Presioná **Procesar** para obtener su SHA-256 y pHash.

## Estructura

- `src/` — frontend (HTML/JS, sin framework)
- `src-tauri/` — backend Rust / configuración de Tauri
