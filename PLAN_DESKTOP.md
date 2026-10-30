# Plan de Migración: media-manager CLI (Terminal) → App de Escritorio (Linux)

> Estado: borrador inicial  
> Objetivo: migrar la capa de presentación de una CLI de terminal (Ink + React) a una aplicación de escritorio nativa Linux, manteniendo la lógica de negocio existente en `core/`, `shared/` y `db/`.

---

## 1. Stack elegido

| Capa | Tecnología | Motivo |
|------|-----------|--------|
| **Backend** | Rust | Mejor rendimiento, SQLite nativo, integración nativa GTK4, seguridad de memoria |
| **Frontend** | TypeScript + React | Tipado estricto, ecosistema de UI de escritorio maduro, componentes reutilizables |
| **Framework desktop** | Tauri | Ligero (WebView del sistema), llamadas Rust↔TS, instaladores nativos Linux |
| **Base de datos** | SQLite (`rusqlite` o `sqlx`) | Reutiliza esquema actual, WAL + FTS5 |
| **Reproductor** | `mpv` (proceso externo) | Delegación de reproducción a reproductor externo `mpv` vía `std::process::Command` |
| **Imágenes / pósters** | HTML `` o canvas dentro de WebView | Alta resolución, dentro de contenedor real, sin límite de 1 color por celda de terminal |
| **Configuración** | YAML + `serde` en Rust | Reemplaza `zod` del lado de Rust |

---

## 2. Qué se conserva y qué se reescribe

### Conservar (casi sin cambios)

| Componente | Qué hace | Destino |
|------------|----------|---------|
| `core/db/` | Migraciones, repositorios, FTS5, better-sqlite3 | Puerto a Rust (`rusqlite`/`sqlx`) |
| `core/library/` | LibraryService, escaneo, indexado | Puerto a Rust |
| `core/player/` | Lanzador de reproductor (mpv), historial | Puerto a Rust (`std::process::Command` → `mpv`) |
| `core/metadata/` | TMDB/TVDB, rate limiter, caché TTL | Puerto a Rust (`reqwest` + `serde_json`) |
| `config/` | YAML + Zod | YAML + `serde` en Rust |
| `shared/` | Utilidades, `imageToAnsi`, `kittyProtocol` | Parcial: lógica sí, render ANSI no |

### Reescribir por completo

| Componente | Nueva implementación |
|------------|---------------------|
| `ui/` (Ink) | React + TypeScript dentro de WebView de Tauri |
| `commands/` (Commander) | Comandos Tauri (Rust → frontend) |
| `PosterImage` (ANSI/chafa) | `` HTML o canvas Rust dentro del contenedor de React |
| Reproductor | `mpv` como proceso externo |
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
  ├── player/             ← integración con `mpv` (proceso externo vía `std::process::Command`)
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

### Hito 3 — Reproductor con mpv (Semanas 7–9)
- Integración con `mpv` como proceso externo desde Rust (`std::process::Command`).
- Componente de video que lance `mpv` para reproducir el archivo seleccionado.
- Controles: play, pause, stop, next, prev, seek, volumen (enviados a `mpv`).
- **Criterio de aceptación:** se reproduce un archivo de video usando `mpv` como proceso externo, sin `GtkMediaFile`.

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
- Eliminado: Acryl no publica releases automáticos. La única forma de instalar es clonando el repositorio y ejecutando `scripts/install.sh`.
- **Criterio de aceptación:** N/A (no aplica).

---

## 5. Desafíos clave

| Desafío | Solución propuesta |
|---------|-------------------|
| Reproductor | `mpv` como proceso externo vía `std::process::Command` |
| Posters en alta resolución | `` HTML con `object-fit: cover` dentro de un contenedor React con dimensiones fijas |
| Migración de datos SQLite | Leer el archivo `.db` existente directamente desde Rust, sin migración de datos |
| Configuración YAML | Reemplazar `zod` por `serde` + `serde_yaml` en Rust |
| Comunicación Rust↔TS | Comandos Tauri con tipado estricto en ambos extremos |
| Metadatos externos | TVmaze API (sin API key, plug-and-play); rate limiting 20 req/10s; manejo de errores cuando no hay conexión |
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
| Reproductor | `mpv` como proceso externo (`std::process::Command`) |

---

## 8. Estado de hitos

| Hito | Contenido | Estado |
|------|-----------|--------|
| Hito 1 | Prototipo mínimo: ventana Tauri, lista desde SQLite, póster, búsqueda | ✅ completado |
| Hito 2 | Capa de dominio en Rust (db, library, metadata) | ✅ completado |
| Hito 3 | Reproductor integrado (mpv, proceso externo) | ✅ completado |
| Hito 4 | UI completa (detalles, favoritos, visto, navegación) | ✅ completado |
| Hito 5 | Empaquetado Linux (.deb, AppImage, .rpm) | ✅ completado |
| Hito 6 | Configuración YAML en el backend | ✅ completado |
| Hito 7 | Internacionalización (i18n) de la UI | ✅ completado |
| Hito 8 | Metadatos automáticos TMDB/TVDB + historial de reproducción | ✅ completado |
| Hito 9 | Publicación automática en GitHub Releases | ✅ completado |

## 9. Próximo paso inmediato

Todos los hitos del plan están completados.

---

## 10. Migración E: Arquitectura nativa GTK4 / Libadwaita (sin motor Web)

> **Motivo:** WebKit2GTK (usado por Tauri v2) no renderiza correctamente en Wayland/Hyprland. Se han agotado todas las configuraciones de entorno (`GDK_BACKEND=x11`, `WEBKIT_DISABLE_DMABUF_RENDERER=1`, `WEBKIT_DISABLE_COMPOSITING_MODE=1`, `LIBGL_ALWAYS_SOFTWARE=1`, `XDG_SESSION_TYPE=x11`, etc.). La única solución robusta a largo plazo es eliminar el WebView y usar GTK4/Libadwaita nativo.

### 10.1. Stack objetivo

| Capa | Tecnología actual | Tecnología objetivo | Motivo |
|------|-------------------|---------------------|--------|
| **Backend** | Rust + Tauri | Rust puro | Mantener lógica de negocio, SQLite; reproductor delegado a `mpv` |
| **Frontend** | React + TypeScript (WebView) | Rust + GTK4 / Libadwaita | Renderizado nativo GTK, sin WebView |
| **Framework desktop** | Tauri v2 | gtk4-rs / libadwaita-rs | Ventana nativa, sin WebView, sin problemas Wayland |
| **Comunicación** | Tauri IPC (invoke) | Llamadas directas Rust → Rust | Sin serialización JSON innecesaria |
| **Base de datos** | SQLite (`rusqlite`) | SQLite (`rusqlite`) | Sin cambios |
| **Reproductor** | GtkMediaFile (GTK4) | `mpv` (proceso externo vía `std::process::Command`) | Cambio por fiabilidad de codecs y Wayland |
| **Imágenes / pósters** | HTML `` en WebView | `gdk-pixbuf` / `cairo` / `gtk4::Picture` | Renderizado nativo |
| **Configuración** | YAML + `serde` | YAML + `serde` | Sin cambios |
| **Instaladores** | Tauri bundle (.deb, AppImage, .rpm) | `cargo-deb` / `cargo-rpm` / AppImage manual | Empaquetado nativo sin Tauri |

### 10.2. Qué se conserva y qué se reescribe

#### Conservar (casi sin cambios)

| Componente | Qué hace | Destino |
|------------|----------|---------|
| `core/db/` | Migraciones, repositorios, FTS5 | Mantener (`rusqlite`) |
| `core/library/` | LibraryService, escaneo, indexado | Mantener |
| `core/player/` | Lanzador de reproductor (mpv), historial | Mantener (integración con `mpv` vía `std::process::Command`) |
| `core/metadata/` | TMDB/TVDB, rate limiter, caché TTL | Mantener (`reqwest` + `serde_json`) |
| `config/` | YAML + `serde` | Mantener |
| `shared/` | Utilidades, `imageToAnsi` | Lógica sí, render ANSI no |

#### Reescribir por completo

| Componente | Implementación actual | Nueva implementación |
|------------|---------------------|---------------------|
| `ui/` (React + TypeScript + WebView) | React + TypeScript dentro de WebView de Tauri | Rust + GTK4 / Libadwaita (código o GtkBuilder XML) |
| `commands/` (Comandos Tauri) | Comandos Tauri expuestos al frontend | Funciones Rust internas llamadas directamente |
| `PosterImage` | HTML `` en WebView | `gdk-pixbuf` / `cairo` dentro de contenedor GTK4 |
| Reproductor integrado | GtkMediaFile | `mpv` (proceso externo vía `std::process::Command`) |
| Tests de UI | Vitest + Tauri | `gtk4-rs` tests + pruebas de integración |
| Instaladores | Tauri bundle | `cargo-deb`, `cargo-rpm`, AppImage manual |
| Scripts de instalación | `install.sh` con detectar Node.js + Tauri | `install.sh` sin Node.js, solo Rust + GTK dependencias |

### 10.3. Arquitectura propuesta

```
frontend/                 ← React + TypeScript (WebView de Tauri)  → ELIMINAR
src-tauri/                ← Rust + Tauri                          → REEMPLAZAR
```

Se convierte en:

```
src/                      ← Rust puro (binario único)
  ├── core/               ← Lógica de negocio (library, metadata, db)
  ├── db/                 ← rusqlite, migraciones, FTS5
  ├── player/             ← integración con `mpv` (proceso externo vía `std::process::Command`)
  ├── config/             ← serde + YAML
  ├── ui/                 ← GTK4 / Libadwaita (ventana, widgets, vistas)
  │   ├── main_window.rs  ← Ventana principal
  │   ├── search_bar.rs   ← Barra de búsqueda
  │   ├── media_list.rs   ← Lista de medios
  │   ├── media_details.rs│ Vista de detalles
  │   ├── video_player.rs ← Reproductor embebido
  │   └── poster.rs       ← Renderizado de pósters con gdk-pixbuf
  └── main.rs             ← Punto de entrada GTK4
```

### 10.4. Hitos del plan (Migración E)

#### Hito E.1 — Prototipo mínimo GTK4 (Semanas 1–2)
- Ventana GTK4 con `libadwaita`
- Lista de medios leída desde SQLite
- Un póster renderizado con `gdk-pixbuf`
- Barra de búsqueda básica
- **Criterio de aceptación:** la ventana se abre en Wayland/Hyprland y muestra la lista de medios

#### Hito E.2 — UI completa con GTK4 (Semanas 3–6)
- Vista de detalles con póster grande, metadatos, sinopsis
- Favoritos, en curso, vistos/no vistos
- Historial de reproducción
- **Criterio de aceptación:** flujo completo: indexar → buscar → ver detalles → reproducir

#### Hito E.3 — Reproductor integrado con mpv (proceso externo) (Semanas 7–8)
- Integración con `mpv` como proceso externo desde la ventana GTK4 (`std::process::Command`)
- Controles: play, pause, stop, next, prev, seek, volumen (enviados a `mpv`)
- **Criterio de aceptación:** se reproduce un archivo de video usando `mpv` como proceso externo

#### Hito E.4 — Empaquetado nativo Linux (Semanas 9–10)
- `install.sh` crea la entrada `.desktop` en `~/.local/share/applications/` para ejecutar Acryl desde el menú de aplicaciones
- Icono de aplicación instalado en `~/.local/share/icons/hicolor/scalable/apps/`
- Lanzador con captura de errores instalado en `~/.local/bin/acryl-launcher`
- **Criterio de aceptación:** al ejecutar `./scripts/install.sh` en Linux limpio, Acryl se instala y es ejecutable desde el menú de aplicaciones/herramientas de búsqueda, sin dependencias de Node.js/Rust en el sistema de destino. No hay releases de GitHub ni paquetes `.deb`/`.rpm`/AppImage.

#### Hito E.5 — Limpieza y documentación (Semana 11)
- Eliminar código de frontend (React, TypeScript, Vite)
- Eliminar dependencias de Node.js
- Actualizar README, AGENTS.md, PLAN_DESKTOP.md
- **Criterio de aceptación:** el repositorio no contiene referencias a Tauri, Node.js o WebView

---

## 11. Tecnologías a eliminar (descontinuar)

| Tecnología | Motivo de eliminación | Reemplazo |
|------------|----------------------|-----------|
| **Tauri v2** | Depende de WebView del sistema (WebKit2GTK) que no funciona en Wayland | Eliminado (se usa Rust puro + GTK4) |
| **WebView / WebKit2GTK** | No renderiza en Wayland/Hyprland | Eliminado |
| **Node.js** | Solo se usaba para el frontend React | Eliminado |
| **npm / pnpm** | Solo para dependencias del frontend | Eliminado |
| **Vite** | Herramienta de construcción del frontend | Eliminado |
| **React** | Framework de UI web | Eliminado |
| **TypeScript** | Lenguaje del frontend | Eliminado |
| **@tauri-apps/api** | API de comandos Tauri | Eliminado (llamadas directas Rust) |
| **better-sqlite3** | SQLite para desarrollo frontend | Eliminado |
| **Tauri CLI** | Herramienta de construcción Tauri | Eliminado |

### Archivos a eliminar

```
frontend/                 ← Carpeta completa (React + TypeScript + Vite)
src-tauri/                ← Carpeta completa (Tauri + Rust bindings a WebView)
scripts/install.sh        ← Se reescribe (sin Node.js, solo Rust + GTK)
scripts/uninstall.sh      ← Se actualiza si es necesario
package.json (raíz)       ← Se reescribe (sin scripts de Tauri/Node)
```

### Archivos a conservar

```
src/                      ← Nueva carpeta Rust pura (db, library, player, config, ui, main.rs)
src-tauri/Cargo.toml      ← Se reescribe como Cargo.toml de proyecto Rust puro
Cargo.toml (raíz)         ← Se actualiza (workspace o proyecto único)
Cargo.lock                ← Se regenera
scripts/                  ← Se mantienen (install.sh, uninstall.sh, diagnose-*.sh)
docs/                     ← Se mantienen (ADRs, etc.)
README.md                 ← Se reescribe completamente
PLAN_DESKTOP.md           ← Se actualiza (este documento)
AGENTS.md                 ← Se actualiza (referencias a tecnologías)
CONTRIBUTING.md           ← Se actualiza (instrucciones de contribución)
SECURITY.md               ← Se revisa (permisos de Tauri ya no aplican)
LICENSE                   ← Se mantiene
```

---

## 12. Cambios en documentación y scripts

### README.md

| Sección | Cambio |
|---------|--------|
| Badges | Eliminar Node.js, Tauri, WebView; agregar GTK4, Rust, Libadwaita |
| Arquitectura | Reemplazar diagrama de Tauri por diagrama de GTK4 + Rust |
| Requisitos del sistema | Eliminar Node.js, npm, Tauri CLI; agregar libgtk-4-dev, libadwaita-1-dev, libwebkit2gtk (opcional) |
| Instalación | `install.sh` ya no instala Node.js; solo dependencias de sistema GTK + Rust |
| Estructura del repositorio | Eliminar `frontend/` y `src-tauri/`; agregar `src/` |
| Comandos disponibles | Eliminar comandos de Tauri; agregar comandos de Rust/Cargo |

### Scripts

| Archivo | Cambio |
|---------|--------|
| `install.sh` | Eliminar instalación de Node.js/npm; instalar `libgtk-4-dev`, `libadwaita-1-dev`, `libwebkit2gtk-4.0-dev` (opcional para webview), `rust-cargo`; compilar con `cargo build --release` |
| `uninstall.sh` | No requiere cambios significativos |
| `diagnose-blank.sh` | Eliminar referencias a WebView/Tauri; enfocar en GTK4 |

### Otros archivos

| Archivo | Cambio |
|---------|--------|
| `AGENTS.md` | Actualizar referencias a Tauri, WebView, frontend React |
| `CONTRIBUTING.md` | Actualizar instrucciones de desarrollo (Rust + GTK4 en lugar de React + Tauri) |
| `SECURITY.md` | Revisar permisos (ya no hay WebView ni comandos Tauri) |
| `docs/adr/` | Agregar ADR para la decisión de migrar a GTK4/Libadwaita |

---

## 13. Estimación de tiempo adicional

| Fase | Tiempo estimado |
|------|-----------------|
| Análisis y ADR | 1 semana |
| Prototipo GTK4 | 2 semanas |
| Migración completa UI | 4-6 semanas |
| Pruebas y ajuste | 2 semanas |
| Empaquetado y documentación | 1 semana |
| **Total adicional** | **10-12 semanas** |

---

## 14. Decisiones pendientes

| Pregunta | Opciones | Decisión |
|----------|----------|----------|
| Toolkit GTK | GTK3 vs GTK4 | GTK4 (implementado) |
| Librería de UI | Libadwaita vs GTK puro | Libadwaita (implementado) |
| Definición UI | Código Rust vs GtkBuilder XML | Código Rust (implementado) |
| Empaquetado | `cargo-deb` / `cargo-rpm` vs `cargo-appimage` | Pendiente (Hito E.4) |

---

## 15. Estado de hitos

| Hito | Contenido | Estado |
|------|-----------|--------|
| Hito 1 | Prototipo mínimo: ventana Tauri, lista desde SQLite, póster, búsqueda | ✅ completado |
| Hito 2 | Capa de dominio en Rust (db, library, metadata) | ✅ completado |
| Hito 3 | Reproductor integrado (mpv, proceso externo) | ✅ completado |
| Hito 4 | UI completa (detalles, favoritos, visto, navegación) | ✅ completado |
| Hito 5 | Empaquetado Linux (.deb, AppImage, .rpm) | ✅ completado |
| Hito 6 | Configuración YAML en el backend | ✅ completado |
| Hito 7 | Internacionalización (i18n) de la UI | ✅ completado |
| Hito 8 | Metadatos automáticos TMDB/TVDB + historial de reproducción | ✅ completado |
| Hito 9 | Publicación automática en GitHub Releases | ✅ completado |
| **Hito E.1** | Prototipo mínimo GTK4 (ventana, lista, póster, búsqueda) | ✅ completado |
| **Hito E.2** | UI completa GTK4 (detalles, favoritos, visto, navegación) | ✅ completado |
| **Hito E.3** | Reproductor integrado con `mpv` (proceso externo) en GTK4 | ✅ completado |
| **Hito E.4** | Empaquetado nativo Linux sin Tauri | ✅ completado |
| **Hito E.5** | Limpieza y documentación (eliminar frontend/Tauri) | ✅ completado |

---

### Hito F.1 — Reproducción de episodios desde la vista de detalles (Semanas 1–2)
- Conectar los botones "▶ Reproducir" de cada episodio en la vista de detalles al reproductor integrado (`VideoPlayer`)
- Al hacer clic en un episodio, cargar su archivo en el reproductor y navegar a la vista de reproducción
- **Criterio de aceptación:** al hacer clic en el botón de reproducción de cualquier episodio, el video se reproduce dentro de la ventana de la aplicación

### Hito F.2 — Controles de reproducción integrados (Semanas 3–4)
- Controles funcionales en el reproductor: play, pausa, stop, seek, volumen
- Mostrar información de reproducción (tiempo actual, duración)
- **Criterio de aceptación:** se puede pausar, reanudar, detener y ajustar el volumen de un video en reproducción

---

## 16. Fase G — UI visual moderna, pósters, pistas de audio/subtítulos y datos masivos

Plan detallado en `docs/plan-ui-visual-pistas-escala-masiva.md`.

Objetivo: interfaz visual bonita con Libadwaita + CSS, renderizado de pósters de alta resolución, detección y selección de pistas de audio y subtítulos, y arquitectura preparada para colecciones masivas (miles de medios, decenas de miles de archivos) sin bloquear la UI.

### Hitos de la Fase G

| Hito | Contenido | Estado |
|------|-----------|--------|
| **Hito G.1** | Infraestructura de datos masivos: tabla `media_streams`, índices, extracción con `ffprobe`, consultas paginadas | Pendiente |
| **Hito G.2** | Galería de pósters virtualizada con `GtkListView` y descarga asíncrona desde TMDB/TVDB | Pendiente |
| **Hito G.3** | UI moderna con Libadwaita y CSS de GTK4 (`AdwStatusPage`, `AdwToast`, `AdwClamp`) | Pendiente |
| **Hito G.4** | Selección de pistas audio / subtítulos y reproducción con `mpv` aplicando `--aid` / `--sid` | Pendiente |
| **Hito G.5** | Pruebas de estrés con colección grande y ajuste de rendimiento | Pendiente |

---

## 17. Próximo paso inmediato

Crear la rama `feat/plan-ui-visual-pistas-escala-masiva` e iniciar el **Hito G.1** (infraestructura de datos masivos).

---

---

*Nota: Este plan es un borrador inicial. A medida que se avanza en la implementación, se deben actualizar los hitos y criterios de aceptación.*