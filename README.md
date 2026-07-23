# Anti Leak Client (contraconte)

Aplicación de escritorio (Tauri 2 + Rust) que procesa imágenes localmente,
calcula sus hashes, y opcionalmente busca si esa imagen ya aparece en
internet.

## ¿Qué hace esto, explicado sin tecnicismos?

Pensalo como una "huella digital" para una foto:

1. **Le das una foto** a la app (indicando dónde está guardada en tu
   computadora).
2. La app calcula dos tipos de "huella":
   - Una **huella exacta** (SHA-256): si alguien cambia un solo píxel de
     la imagen, esta huella cambia por completo. Sirve para probar que
     un archivo es *idéntico, bit por bit*, a otro.
   - Una **huella aproximada** (pHash): sigue pareciéndose aunque la
     imagen se comprima, se le cambie el tamaño o se recorte un poco.
     Sirve para detectar copias "parecidas", no solo idénticas.
3. Con esa huella guardada, más adelante podés comparar cualquier otra
   imagen y saber si es la misma foto (o una versión editada de ella)
   sin tener que mirarla vos mismo.
4. Opcionalmente, la app le pregunta a **Google** (a través de su API de
   Vision) si esa foto específica aparece en alguna página pública de
   internet, y te devuelve los links donde la encontró.

En corto: es una herramienta para **verificar el origen de una imagen y
detectar si se filtró o se está usando sin permiso en otro lado**.

## Flujo de trabajo (técnico)

```
[Usuario ingresa ruta de archivo]
        │
        ▼
[Frontend (src/index.html) llama invoke('procesar_imagen_local')]
        │
        ▼
[Comando Tauri en Rust (src-tauri/src/main.rs)]
        │
        ▼
[lib.rs: lee el archivo, calcula SHA-256 y pHash localmente]
        │
        ▼
[Resultado (HashResult) vuelve al frontend y se muestra en pantalla]
        │
        ▼  (opcional, botón "Buscar en internet")
[invoke('buscar_imagen_en_internet') con la API key del usuario]
        │
        ▼
[lib.rs: lee la imagen, la codifica en base64 y llama a la API de
 Google Cloud Vision (Web Detection)]
        │
        ▼
[Google devuelve páginas / imágenes exactas / imágenes parecidas
 donde aparece esa foto en la web pública]
        │
        ▼
[El frontend muestra la lista de coincidencias como links]
```

**Importante sobre la búsqueda en internet:** no existe un buscador
público de "hashes"; ninguna herramienta te deja preguntar "¿alguien
tiene un archivo con este hash exacto?". Lo que hacemos es mandarle la
imagen real a la API de reconocimiento de imágenes de Google, que
compara su contenido visual contra su propio índice de la web y
devuelve páginas donde encontró esa imagen (igual o parecida). El
SHA-256/pHash calculados localmente son útiles para comparar archivos
que vos mismo tengas, no para la búsqueda web.

### Configurar la búsqueda en internet

1. Creá un proyecto en [Google Cloud Console](https://console.cloud.google.com/).
2. Habilitá la **Cloud Vision API**.
3. Generá una **API key** (Credenciales → Crear credenciales → Clave de API).
4. Pegá esa key en el campo correspondiente dentro de la app. No se
   guarda en disco ni se commitea a ningún lado — se usa solo para esa
   consulta puntual.
5. Google da 1000 unidades gratis por mes; superado eso, Web Detection
   cuesta USD 3.50 por cada 1000 unidades (precio sujeto a cambios,
   revisá la [página oficial de precios](https://cloud.google.com/vision/pricing)).

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

## Build de producción (para tu propio sistema operativo)

```bash
npm run build
```

Genera el instalador en `src-tauri/target/release/bundle/`, pero
**solo para el sistema operativo en el que corras el comando**. Tauri
empaqueta contra las librerías nativas del SO (WebView2 en Windows,
WebKit en macOS, webkit2gtk en Linux), así que no hay "compilación
cruzada" real: un Mac genera `.dmg`/`.app`, un Windows genera
`.msi`/`.exe`, un Linux genera `.deb`/`.AppImage`/`.rpm`.

## Exportar a multiplataforma (Windows + macOS + Linux)

Como no podés generar los tres instaladores desde una sola máquina,
la forma estándar es compilar en la nube, una vez por sistema
operativo, usando GitHub Actions. Este repo ya incluye
`.github/workflows/release.yml`, que:

1. Se dispara cuando hacés push de un tag tipo `v0.1.0`.
2. Corre en paralelo sobre runners de `macos-latest`, `ubuntu-latest`
   y `windows-latest`.
3. En cada uno instala Rust + Node, corre `npm install`, y usa la
   acción oficial `tauri-apps/tauri-action` para compilar y adjuntar
   los instaladores a un GitHub Release (como borrador).

Para usarlo:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Y revisá la pestaña **Actions** de GitHub — al terminar vas a tener un
Release con los tres instaladores listos para descargar.

Alternativa sin CI: pedirle a alguien con Mac que corra `npm run
build` ahí, y lo mismo con alguien en Windows — funciona, pero no
escala y es más manual.

## Uso

1. Abrí la app.
2. Pegá la ruta completa de una imagen local (ej: `/home/usuario/foto.png`).
3. Presioná **Procesar** para obtener su SHA-256 y pHash.
4. (Opcional) Ingresá tu API key de Google Cloud Vision y presioná
   **Buscar en internet** para ver si esa imagen aparece en alguna
   página pública.

## Estructura

- `src/` — frontend (HTML/JS, sin framework)
- `src-tauri/` — backend Rust / configuración de Tauri
- `.github/workflows/` — pipeline de release multiplataforma
