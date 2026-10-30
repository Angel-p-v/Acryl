# Plan: UI visual moderna, pósters renderizados, pistas de audio/subtítulos y soporte a datos masivos

> **Estado:** propuesta  
> **Objetivo:** convertir Acryl en una aplicación de escritorio Linux (Rust + GTK4 + Libadwaita) con una interfaz visual moderna, renderizado de pósters de alta resolución, detección y selección de pistas de audio y subtítulos, y arquitectura preparada para colecciones masivas (miles de medios, decenas de miles de archivos) sin bloquear la UI.

---

## 1. Contexto y motivación

La aplicación actual (tras la migración a GTK4/Libadwaita y al reproductor `mpv` como proceso externo) tiene una UI funcional pero básica:

- La lista de medios es una tabla simple sin virtualización.
- Los pósters no se renderizan (o se renderizan de forma rudimentaria).
- No hay detección de pistas de audio ni subtítulos de los archivos de vídeo.
- No hay forma de que el usuario seleccione pista de audio o subtítulos antes de reproducir.
- No está diseñada para colecciones grandes; cargar todos los medios en memoria no escala.

Este plan aborda esos límites desde el principio, con una arquitectura pensada para volumen masivo de datos.

---

## 2. Stack tecnológico (sin cambios)

| Capa | Tecnología | Motivo |
|------|-----------|--------|
| Backend / UI | Rust, edición 2024 | Seguridad de memoria, rendimiento, propiedad clara |
| Interfaz gráfica | GTK4 + Libadwaita (gdk-rs) | Renderizado nativo, Wayland/Hyprland, sin WebView |
| Base de datos | SQLite + `rusqlite` | Consultas parametrizadas, WAL, FTS5, índices |
| Reproducción | `mpv` (proceso externo vía `std::process::Command`) | Fiabilidad de codecs, delegación de reproducción |
| Pósters / imágenes | `gdk-pixbuf` / `cairo` | Renderizado nativo dentro de GTK4 |
| Metadatos externos | `reqwest` + APIs públicas (`public-apis`) | Descarga de pósters e información mediante APIs listadas en https://github.com/public-apis/public-apis |
| Descarga de pósters | Hilos secundarios + canales | No bloquear el hilo principal de GTK4 |
| API de metadatos y pósters | https://github.com/public-apis/public-apis | Directorio de APIs públicas para descubrir y usar servicios de metadatos y pósters sin acoplarse a un proveedor único |
| Detección de pistas | `ffprobe` (JSON) | Extraer streams de audio y subtítulos |

No se añaden dependencias nuevas salvo que sea estrictamente necesario. `ffmpeg`/`ffprobe` ya están disponibles en el entorno.

### Uso de APIs públicas (`public-apis`)

Para metadatos y pósters se consultará el directorio https://github.com/public-apis/public-apis para seleccionar una API pública adecuada (cine, series, anime, etc.) en lugar de acoplarse a un único proveedor como TMDB o TVDB. Esto permite:

- Elegir la API con mejor cobertura según el tipo de contenido (películas, series, anime).
- No depender de una API key específica de un solo proveedor.
- Cambiar de proveedor sin modificar la arquitectura, solo la implementación del cliente HTTP.
- Respetar los límites de tasa (rate limits) documentados en cada API pública.

La implementación abstraerá el cliente de metadatos detrás de una interfaz para poder cambiar el proveedor fácilmente.

---

## 3. Pilares para soporte de datos masivos

### 3.1. Virtualización de la UI (lista de medios)

- **Nunca cargar todos los medios en memoria a la vez.**
- Usar `GtkListView` con `GtkFactoryListItemFactory` (o `GtkColumnView` para modo tabla/galería).
- Carga paginada desde SQLite: `LIMIT` / `OFFSET` o cursor-based.
- Scroll infinito o carga por bloques (ej. 100 items) cuando el usuario se acerca al final del visible.

### 3.2. Base de datos escalable

- Migración versionada para tabla `media_streams` (audio/subtítulos por archivo).
- Índices en columnas de consulta frecuente: `folder_path`, `type`, `title`, `year`, `media_id`.
- FTS5 para búsqueda de texto completo; mantenerlo actualizado.
- Consultas parametrizadas siempre; nunca concatenar valores de usuario.
- WAL mode para lecturas concurrentes sin bloquear escrituras.

### 3.3. Pósters: descarga y caché en segundo plano

- Directorio de caché local: `~/.cache/acryl/posters/` (configurable).
- Descarga asíncrona de pósters desde una API pública seleccionada de https://github.com/public-apis/public-apis con `reqwest`.
- No bloquear el hilo principal de GTK4: descarga en hilo separado con canales o `glib::MainContext`.
- Placeholder (color / ícono) mientras se descarga o si falla.
- Renderizado con `gdk-pixbuf` cargado en `gtk4::Picture`; bordes redondeados o máscaras con `cairo` sobre `GtkDrawingArea` si es necesario.
- Solo renderizar lo visible en pantalla (virtualización).

### 3.4. Detección de pistas con `ffprobe`

- Al indexar o actualizar un medio, ejecutar:
  ```
  ffprobe -v quiet -print_format json -show_streams -select_streams a,s <archivo>
  ```
- Almacenar en tabla `media_streams`:
  - `media_id`, `file_path`, `stream_type` (`audio` / `subtitle`), `stream_index_relativo` (relativo al tipo), `idioma`, `codec`, `título`.
- Hacerlo solo una vez por archivo (o cuando cambie), no en cada renderizado.
- Mapeo correcto de índices: `ffprobe` numera streams globalmente; `mpv` numera audio y subtítulos por separado (`--aid`, `--sid`).

### 3.5. Selección de pistas y reproducción con `mpv`

- UI en vista de detalles: listas seleccionables para audio y subtítulos (incluyendo "Ninguno" para subtítulos).
- Guardar preferencia del usuario por medio (BD o sesión; definir en implementación).
- Al lanzar `mpv`: `mpv --aid=<N> --sid=<M> <archivo.mkv>`.
- Si no hay selección, lanzar `mpv` sin esos argumentos (comportamiento predeterminado).

---

## 4. Hitos de implementación

### Hito G.1 — Infraestructura de datos masivos (semanas 1-2)

- Migración versionada: tabla `media_streams` + índices necesarios.
- Función para extraer streams con `ffprobe` y guardar en BD.
- Consulta paginada de medios (`list` con `limit` / `offset`).
- Pruebas: integración con SQLite temporal, verificación de índices y paginación.

### Hito G.2 — Galería de pósters virtualizada (semanas 3-4)

- Reemplazar lista simple por `GtkListView` / `GtkColumnView` con factory de ítems.
- Renderizado de pósters con `gdk-pixbuf` (placeholder si no hay).
- Descarga asíncrona de pósters desde una API pública de https://github.com/public-apis/public-apis con caché en disco.
- Pruebas: renderizado de muchos ítems sin bloquear UI; descarga de pósters en segundo plano.

### Hito G.3 — UI moderna con Libadwaita y CSS (semanas 5-6)

- CSS de GTK4 para colores, bordes redondeados, estados `:hover` / `:selected`.
- `AdwStatusPage` para pantallas vacías, `AdwToast` para notificaciones, `AdwClamp` para pantallas anchas.
- Vista de detalles mejorada con póster grande y metadatos.
- Pruebas: pruebas de unidad de componentes visuales puros.

### Hito G.4 — Selección de pistas audio / subtítulos (semanas 7-8)

- UI en vista de detalles: seleccionar pista de audio y subtítulos por medio.
- Persistencia de la elección (BD o sesión).
- `play_file()` lanza `mpv` con `--aid` / `--sid` según selección.
- Pruebas: integración real con `ffprobe` + `mpv` en archivo de prueba; verificar que se aplican las pistas.

### Hito G.5 — Pruebas de estrés y ajuste (semana 9)

- Cargar colección grande (miles de filas) y medir tiempo de arranque y scroll.
- Verificar que no hay fugas de memoria en descarga de pósters.
- Pruebas de regresión de todos los hitos anteriores.

---

## 5. Archivos y módulos afectados (estimado)

| Archivo | Cambio principal |
|---------|------------------|
| `src/db/migrations.rs` | Nueva tabla `media_streams` + índices |
| `src/db/media_repo.rs` / `file_repo.rs` | Consultas paginadas, CRUD de `media_streams` |
| `src/core/player.rs` | (opcional) construir argumentos de mpv con pistas |
| `src/ui/main_window.rs` | `play_file()` pasa `--aid` / `--sid` a `mpv`; CSS provider |
| `src/ui/media_details.rs` | Sección de selección de audio / subtítulos |
| `src/ui/media_list.rs` | Galería virtualizada con `GtkListView` |
| `src/ui/poster.rs` | Carga y renderizado de pósters con `gdk-pixbuf` |
| `Cargo.toml` | Revisar si falta alguna dependencia (probablemente no: ya tiene `gtk4`, `libadwaita`, `gdk-pixbuf`, `reqwest`, `rusqlite`) |

---

## 6. Contratos e interfaces clave

```rust
pub enum StreamType { Audio, Subtitle }

pub struct MediaStream {
    pub stream_type: StreamType,
    pub stream_index_rel: i32,      // Índice relativo al tipo (para --aid/--sid de mpv)
    pub language: Option<String>,
    pub codec: Option<String>,
    pub title: Option<String>,
}

pub struct ListMediaOptions {
    pub r#type: Option<String>,
    pub query: Option<String>,      // para FTS5
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub order_by: Option<String>,
}

pub fn build_mpv_args_with_tracks(
    file_path: &str,
    socket_path: &str,
    audio_track: Option<i32>,
    subtitle_track: Option<i32>,
) -> Vec<String>
```

---

## 7. Riesgos y mitigaciones

| Riesgo | Mitigación |
|--------|-----------|
| Índices de pista `ffprobe` ≠ índices de `mpv` | Filtrar solo streams del tipo deseado y numerar por tipo antes de pasar a `--aid` / `--sid` |
| Bloquear UI con `ffprobe` o descargas | Ejecutar en hilos secundarios o canales; nunca en el hilo principal de GTK4 |
| Miles de pósters cargados a la vez | Virtualización con `GtkListView`; solo renderizar lo visible |
| Consulta SQL lenta en colección grande | Índices en columnas de filtro / orden; paginación estricta |
| Fugas de memoria en `gdk-pixbuf` | `Drop` correcto de objetos; no retener píxmaps innecesarios |
| Pistas sin idioma detectado | Mostrar "Desconocido" y permitir selección por índice |

---

## 8. Estrategia de pruebas

- **Unitarios:** construcción de argumentos de `mpv`, mapeo de índices de pista, consultas paginadas, serialización de streams.
- **Integración:** base de datos temporal con miles de filas; descarga de pósters (mock o real); ejecución real de `ffprobe` + `mpv` sobre archivo de prueba generado con `ffmpeg`.
- **Regresión:** los tests existentes deben seguir pasando (`cargo test --all-targets --all-features`).
- **Estrés:** colección grande; medir tiempo de arranque, scroll y uso de memoria.

---

## 9. Criterios de aceptación

- [ ] La lista de medios se renderiza con `GtkListView` y no carga todos los ítems en memoria.
- [ ] Los pósters se descargan en segundo plano y se muestran con placeholder mientras tanto.
- [ ] Se pueden ver y seleccionar pistas de audio y subtítulos en la vista de detalles.
- [ ] Al reproducir, `mpv` se lanza con `--aid` / `--sid` correspondientes a la selección del usuario.
- [ ] La aplicación no bloquea la UI al indexar, buscar o scrollar con miles de medios.
- [ ] Todas las pruebas pasan (`cargo test --all-targets --all-features`).
- [ ] `cargo fmt --check` y `cargo clippy -D warnings` limpios.

---

## 10. Próximo paso inmediato

Crear la rama `feat/plan-ui-visual-pistas-escala-masiva` e iniciar el **Hito G.1** (infraestructura de datos masivos: tabla `media_streams`, índices, extracción con `ffprobe`, consultas paginadas).

---

*Nota: este plan es una propuesta de arquitectura. Antes de implementar cada hito, revisar `AGENTS.md` y `PLAN_DESKTOP.md` para garantizar coherencia con el resto del proyecto.*
