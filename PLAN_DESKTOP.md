# Plan de Migración: media-manager CLI (Terminal) → App de Escritorio (Linux)

> Estado: borrador inicial  
> Objetivo: migrar la capa de presentación de una CLI de terminal (Ink + React) a una aplicación de escritorio nativa Linux, manteniendo la lógica de negocio existente en `core/`, `shared/` y `db/`.

---

## 1. Stack elegido

| Capa | Tecnología | Motivo |
|------|-----------|--------|
| **Backend** | Rust | Mejor rendimiento, SQLite nativo, FFI a `libmpv`, seguridad de memoria |
| **Frontend** | TypeScript + React | Tipado estricto, ecosistema de UI de escritorio maduro, componentes reutilizables |
| **Framework desktop** | Tauri | Ligero (WebView del sistema), llamadas Rust↔TS, instaladores nativos Linux |
| **Base de datos** | SQLite (`rusqlite` o `sqlx`) | Reutiliza esquema actual, WAL + FTS5 |
| **Reproductor integrado** | `libmpv` vía FFI o GStreamer | Reproducción dentro de la ventana, sin `spawn` externo |
| **Imágenes / pósters** | HTML `` o canvas dentro de WebView | Alta resolución, dentro de contenedor real, sin límite de 1 color por celda de terminal |
| **Configuración** | YAML + `serde` en Rust | Reemplaza `zod` del lado de Rust |

---

## 2. Qué se conserva y qué se reescribe

### Conservar (casi sin cambios)

| Componente | Qué hace | Destino |
|------------|----------|---------|
| `core/db/` | Migraciones, repositorios, FTS5, better-sqlite3 | Puerto a Rust (`rusqlite`/`sqlx`) |
| `core/library/` | LibraryService, escaneo, indexado | Puerto a Rust |
| `core/player/` | Lanzador de mpv/vlc, tracker IPC | Puerto a Rust (bindings a `libmpv`) |
| `core/metadata/` | TMDB/TVDB, rate limiter, caché TTL | Puerto a Rust (`reqwest` + `serde_json`) |
| `config/` | YAML + Zod | YAML + `serde` en Rust |
| `shared/` | Utilidades, `imageToAnsi`, `kittyProtocol` | Parcial: lógica sí, render ANSI no |

### Reescribir por completo

| Componente | Nueva implementación |
|------------|---------------------|
| `ui/` (Ink) | React + TypeScript dentro de WebView de Tauri |
| `commands/` (Commander) | Comandos Tauri (Rust → frontend) |
| `PosterImage` (ANSI/chafa) | `` HTML o canvas Rust dentro del contenedor de React |
| Reproductor vía `spawn` | `libmpv` integrado o GStreamer dentro de la app |
| Tests de Ink (`ink-testing-library`) | Tests de React + Tauri (vitest + playwright o similar) |

---

## 3. Arquitectura propuesta

```
frontend/                 ← React + TypeScript (WebView de Tauri)
  ├── components/         ← Poster, MediaList, Player, Sidebar, Details
  ├── services/           ← Llamadas a comandos Tauri (Rust)
  ├── state/              ← Zustand o React Query
  └── pages/              ← Home, Search, Player, Settings

src-tauri/                ← Rust
  ├── core/               ← Lógica de negocio portada (library, metadata, db)
  ├── db/                 ← rusqlite/sqlx, migraciones, FTS5
  ├── player/             ← bindings a libmpv o GStreamer
  ├── commands/           ← comandos Tauri expuestos al frontend
  └── config/             ← serde + YAML
```

---

## 4. Hitos del plan

### Hito 1 — Prototipo mínimo (Semanas 1–2)
- Ventana Tauri en Linux con WebView.
- Lista de medios leída desde SQLite existente.
- Un póster renderizado en `` dentro de un contenedor React.
- Barra de búsqueda básica.
- **Criterio de aceptación:** el póster se ve nítido dentro de su box, sin pixelado.

### Hito 2 — Capa de dominio en Rust (Semanas 3–6)
- Puerto de `core/db` a `rusqlite`/`sqlx` (migraciones, repositorios, FTS5).
- Puerto de `core/library` (indexado, escaneo, consultas).
- Puerto de `core/metadata` (TMDB/TVDB, caché TTL).
- Comandos Tauri que exponen esta lógica al frontend.
- **Criterio de aceptación:** la app puede indexar una carpeta de medios y consultarla desde React.

### Hito 3 — Reproductor integrado (Semanas 7–9)
- Bindings a `libmpv` (o GStreamer) desde Rust.
- Componente de video en React que reciba frames del reproductor integrado.
- Controles: play, pause, stop, next, prev, seek, volumen.
- **Criterio de aceptación:** se reproduce un archivo de video dentro de la ventana de la app, sin procesos externos.

### Hito 4 — UI completa (Semanas 10–13)
- Vista de detalles con póster grande, metadatos, sinopsis.
- Favoritos, en curso, vistos/no vistos.
- Historial de reproducción.
- Sincronización automática de metadatos con TTL.
- **Criterio de aceptación:** flujo completo: indexar → buscar → ver detalles → reproducir.

### Hito 5 — Empaquetado Linux (Semanas 14–15)
- Instalador nativo Linux (`.deb`, AppImage o `.rpm`).
- Icono de aplicación, entrada de menú, archivo `.desktop`.
- **Criterio de aceptación:** se instala y ejecuta en Linux sin dependencias manuales de Rust/Node. ✅ cumplido

### Hito 6 — Configuración YAML en el backend (Semanas 16–17)
- Carga de configuración desde archivo YAML en Rust usando `serde` + `serde_yaml`.
- Comandos Tauri para leer y escribir claves de configuración desde el frontend.
- Reemplazo progresivo de valores hardcodeados por lectura del archivo YAML.
- **Criterio de aceptación:** la app lee la configuración desde un archivo YAML en startup y expone comandos para modificarla desde la UI.

### Hito 7 — Internacionalización (i18n) de la UI (Semanas 18–19)
- Extraer cadenas de texto de la UI a archivos de traducción (español e inglés).
- Selector de idioma en la UI con persistencia de la elección.
- **Criterio de aceptación:** la app cambia de idioma sin recargar y todos los textos visibles usan el sistema i18n.

### Hito 8 — Metadatos automáticos TMDB/TVDB + historial de reproducción (Semanas 20–23)
- Servicio de sincronización automática de metadatos con TMDB y TVDB (póster, sinopsis, año, género).
- Llenado automático de columnas `poster_path`, `synopsis` y `metadata_fetched_at` en la BD.
- Comandos Tauri para registrar reproducciones en la tabla `playback_history` (media_id, fecha, posición, duración).
- Vista de historial de reproducción en la UI.
- **Criterio de aceptación:** al buscar un medio sin metadatos, la app los rellena automáticamente desde TMDB/TVDB; al reproducir un video se registra en el historial.

### Hito 9 — Publicación automática en GitHub Releases (Semanas 24–25)
- Configurar `release.yml` y `release-please.yml` para generar releases automáticos.
- Subir artefactos `.deb`, AppImage y `.rpm` a GitHub Releases con checksums y notas de release.
- **Criterio de aceptación:** al hacer push de un tag, se genera un release en GitHub con los instaladores listos para descargar.

---

## 5. Desafíos clave

| Desafío | Solución propuesta |
|---------|-------------------|
| Reproductor integrado | FFI a `libmpv` o GStreamer desde Rust; renderizar frames en una superficie compartida con WebView o en ventana nativa aparte |
| Posters en alta resolución | `` HTML con `object-fit: cover` dentro de un contenedor React con dimensiones fijas |
| Migración de datos SQLite | Leer el archivo `.db` existente directamente desde Rust, sin migración de datos |
| Configuración YAML | Reemplazar `zod` por `serde` + `serde_yaml` en Rust |
| Comunicación Rust↔TS | Comandos Tauri con tipado estricto en ambos extremos |
| Metadatos externos | Rate limiting y caché TTL para llamadas a TMDB/TVDB; manejo de errores cuando no hay conexión |
| i18n | Estructura de archivos de traducción y selector de idioma persistente |

---

## 6. Estimación de tiempo

**Total:** 25 semanas (tiempo parcial, ~10–15 h/semana)

---

## 7. Decisiones tomadas (basadas en respuestas del usuario)

| Pregunta | Respuesta |
|----------|-----------|
| Mejor rendimiento | Tauri + Rust |
| Backend | Rust |
| Frontend | TypeScript + React |
| Plataformas | Solo Linux |
| Reproductor | Integrado en la app (no `spawn` externo) |

---

## 8. Estado de hitos

| Hito | Contenido | Estado |
|------|-----------|--------|
| Hito 1 | Prototipo mínimo: ventana Tauri, lista desde SQLite, póster, búsqueda | ✅ completado |
| Hito 2 | Capa de dominio en Rust (db, library, metadata) | ✅ completado |
| Hito 3 | Reproductor integrado (libmpv) | ✅ completado |
| Hito 4 | UI completa (detalles, favoritos, visto, navegación) | ✅ completado |
| Hito 5 | Empaquetado Linux (.deb, AppImage, .rpm) | ✅ completado |
| Hito 6 | Configuración YAML en el backend | ⏳ pendiente |
| Hito 7 | Internacionalización (i18n) de la UI | ⏳ pendiente |
| Hito 8 | Metadatos automáticos TMDB/TVDB + historial de reproducción | ⏳ pendiente |
| Hito 9 | Publicación automática en GitHub Releases | ⏳ pendiente |

## 9. Próximo paso inmediato

El siguiente hito a implementar es el **Hito 6 — Configuración YAML en el backend**, seguido del Hito 7 (i18n), Hito 8 (metadatos automáticos e historial) y Hito 9 (publicación en releases).

---

*Nota: Este plan es un borrador inicial. A medida que se avanza en la implementación, se deben actualizar los hitos y criterios de aceptación.*