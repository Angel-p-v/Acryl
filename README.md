<p align="center">
  <img src="icons/acryl.svg" alt="Acryl" width="200"/>
</p>

<h1 align="center">Acryl</h1>

<p align="center">
  <strong>Gestor de contenido multimedia de escritorio para Linux</strong><br/>
  Indexa tu colección de películas, series y anime en una base de datos SQLite local<br/>
  y reproduce tus episodios con mpv, el reproductor de referencia en Linux.
</p>

<p align="center">
  <a href="https://github.com/Angel-p-v/app-media-manager/actions/workflows/ci.yml">
    <img src="https://img.shields.io/github/actions/workflow/status/Angel-p-v/app-media-manager/ci.yml?branch=main&style=for-the-badge" alt="CI Status"/>
  </a>
  <a href="https://github.com/Angel-p-v/app-media-manager/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/license-MIT-blue?style=for-the-badge" alt="License"/>
  </a>
  <a href="https://www.rust-lang.org/">
    <img src="https://img.shields.io/badge/rust-%3E%3D1.90-000000?style=for-the-badge" alt="Rust"/>
  </a>
  <a href="https://gtk.org/">
    <img src="https://img.shields.io/badge/gtk-4.x-497975?style=for-the-badge" alt="GTK4"/>
  </a>
</p>

---

## 🎯 ¿Qué es Acryl?

**Acryl** es una aplicación de escritorio nativa para Linux que te permite indexar, buscar y reproducir tu colección local de películas, series y anime. La interfaz es Rust + GTK4/Libadwaita y la reproducción se delega a `mpv` en una ventana independiente.

Todo se ejecuta localmente en tu máquina: la base de datos SQLite, el índice de búsqueda de texto completo (FTS5) y la integración con `mpv` mediante `std::process::Command`.

---

## ✨ Características principales

| Capacidad | Descripción |
|-----------|-------------|
| 📚 **Biblioteca local** | Indexa películas, series y anime en una base de datos SQLite con búsqueda FTS5 |
| 🔍 **Búsqueda rápida** | Busca por título con filtrado por tipo y paginación |
| 🏷️ **Categorías** | Filtra la biblioteca por géneros/etiquetas obtenidos de los metadatos (TVmaze) |
| 🖼️ **Pósteres, banners y metadatos** | Descarga pósteres, sinopsis y puntuación (vote_average) desde TVmaze; banners anchos desde TVmaze/AniList. Imágenes individuales por episodio. |
| ▶️ **Reproducción con mpv** | Abre el episodio seleccionado en `mpv` (ventana independiente) |
| ⚙️ **Configuración YAML** | Ajustes persistentes en archivo YAML (idioma, ruta de base de datos, reproductor predeterminado) |
| 🌐 **Idiomas** | Interfaz disponible en español e inglés |
| 📦 **Instalación** | Clona el repositorio y ejecuta `./scripts/install.sh` |

---

## 🏗️ Arquitectura

media-manager (Acryl) sigue una arquitectura de capas separadas que garantiza mantenibilidad, pruebas y seguridad:

```mermaid
graph TD
    Usuario[["Usuario"]]
    UI[["UI: GTK4 / Libadwaita (Rust)"]]
    Logic[["Lógica: Rust puro"]]
    Decision{{"¿Consulta o reproduce?"}}
    SQLite[["SQLite: acryl.db"]]
    MPV[["mpv (proceso externo)"]]
    TVmaze[["TVmaze: API pública"]]

    Usuario -->|busca| UI
    UI -->|llama| Logic
    Logic --> Decision
    Decision -->|consulta| SQLite
    Decision -->|reproduce| MPV
    SQLite -->|datos| UI
    MPV -->|ventana independiente| Usuario
    Logic -.->|sincroniza| TVmaze
    TVmaze -.->|metadatos| Logic
    UI -->|resultado| Usuario

    classDef usuario fill:#e0e7ff,stroke:#6366f1,stroke-width:1px
    classDef ui fill:#f0fdf4,stroke:#497975,stroke-width:1px
    classDef logic fill:#fffbeb,stroke:#f59e0b,stroke-width:1px
    classDef db fill:#fce7f3,stroke:#ec4899,stroke-width:1px
    classDef ext fill:#f3f4f6,stroke:#9ca3af,stroke-width:1px

    class Usuario usuario
    class UI ui
    class Logic logic
    class SQLite db
    class MPV logic
    class TVmaze ext
```

### Flujo de datos

1. El usuario busca o navega por medios en la UI (GTK4 / Libadwaita, Rust).
2. La UI llama directamente a funciones Rust del backend.
3. El backend consulta **una única base de datos SQLite** (`acryl.db`) y devuelve los resultados a la UI.
4. Para la reproducción, el backend lanza `mpv` como proceso externo (`std::process::Command`) en su propia ventana; `mpv` gestiona play, pausa, seek, volumen y pistas.
5. Los metadatos se sincronizan en segundo plano con TVmaze (API pública, sin API key) para obtener pósteres y sinopsis automáticamente. Para el banner ancho de la vista de detalles se consultan las imágenes `background`/`banner` de TVmaze y, en anime, el banner de AniList (con Kitsu como respaldo). Desde Ajustes se puede forzar una actualización completa que sobrescribe los metadatos de todos los medios indexados.

### Principios de diseño

- **Separación de responsabilidades**: el dominio no depende de la UI; la UI no ejecuta SQL ni accede directamente al sistema de archivos.
- **Inyección de dependencias**: dependencias explícitas, sin singletons globales ni estado mutable innecesario.
- **Contratos tipados**: Serde para serialización interna de Rust, con validación en el límite de entrada.
- **Seguridad por diseño**: mínimo privilegio, sin acceso general al sistema de archivos, sin ejecución de procesos externos arbitrarios, sin WebView.

---

## 📋 Requisitos del sistema

### Dependencias de sistema (Linux)

| Paquete | Propósito | Distribución |
|---------|-----------|--------------|
| `libgtk-4-dev` | Interfaz gráfica (GTK4) | Todas |
| `libadwaita-1-dev` | Componentes de UI (Libadwaita) | Todas |
| `librsvg2-dev` | Renderizado de SVG | Todas |
| `libayatana-appindicator3-dev` | Indicadores de bandeja (opcional) | Todas |
| `mpv` | Reproducción de vídeo (proceso externo) | Todas |
| `libgdk-pixbuf2.0-dev` | Decodificación de imágenes (pósters) | Todas |

> **GTK 4.10 o superior:** Acryl usa el selector nativo de carpetas `GtkFileDialog`,
> disponible a partir de GTK 4.10. Las distribuciones con versiones anteriores
> deberán actualizar GTK para poder compilar y usar la aplicación.

### Herramientas de desarrollo

| Herramienta | Versión mínima |
|-------------|---------------|
| Rust / Cargo | ≥ 1.90 |
| `pkg-config` | Detección de librerías |

---

## 📦 Instalación

### 🚀 Cómo instalar

Acryl no tiene releases publicados. La única forma de instalarlo es clonando
el repositorio y ejecutando el script de instalación:

```bash
git clone https://github.com/Angel-p-v/app-media-manager.git acryl
cd acryl
./scripts/install.sh
```

#### Cómo instalar

Acryl no tiene releases publicados. La única forma de instalarlo es clonando
el repositorio y ejecutando el script de instalación:

```bash
git clone https://github.com/Angel-p-v/app-media-manager.git acryl
cd acryl
./scripts/install.sh
```

El script **instala todo lo necesario** para que la aplicación funcione:

1. Detecta tu gestor de paquetes (`apt`, `pacman`, `dnf`, `zypper` o `apk`) e
   instala las dependencias de sistema (`libgtk-4`, `libadwaita-1`,
   `librsvg2`, `gdk-pixbuf2`, `mpv`, `pkg-config`). Usa `sudo` solo para este
   paso.
2. Instala Rust / Cargo (≥ 1.90) si falta o está desactualizado.
3. Compila la aplicación con `cargo build --release`.
4. Instala el binario en `~/.local/bin/acryl`.
5. Crea la entrada `.desktop` en `~/.local/share/applications/` para que puedas
   buscar y ejecutar Acryl desde el menú de aplicaciones.
6. Instala el icono en `~/.local/share/icons/hicolor/scalable/apps/`.

Al terminar, busca **Acryl** en el menú de aplicaciones o ejecuta `acryl`.

> **Nota:** el binario y la entrada `.desktop` se instalan a nivel de usuario
> (`~/.local`), sin necesidad de root para esos archivos. Solo se requiere
> `sudo` para instalar las dependencias de sistema. Si quieres instalar para
> todos los usuarios, usa `sudo ./scripts/install.sh --system`. **No requiere
> Node.js ni npm.**

---

### 🗑️ Cómo desinstalar

Si ya no necesitas Acryl, ejecuta el desinstalador desde el directorio clonado:

```bash
cd acryl
./scripts/uninstall.sh
```

El desinstalador elimina:

- La entrada `.desktop` de `~/.local/share/applications`.
- El icono de `~/.local/share/icons/hicolor/scalable/apps/`.
- El binario de `~/.local/bin/acryl`.

**Por defecto no borra** el código fuente clonado ni la base de datos de tu biblioteca (`acryl.db`), para que no pierdas tus medios indexados.

Además, el desinstalador te preguntará:

- Si quieres eliminar las **dependencias de sistema** que instaló el installer
  (GTK4, Libadwaita, mpv, etc.). Se te advierte que esto podría afectar a otras
  aplicaciones que las usen.
- Si quieres desinstalar **Rust / Cargo**. Se te advierte que solo lo hagas si
  no lo usas para otros proyectos.

#### Borrado absoluto (`--purge` / `-p`)

Si quieres eliminar absolutamente **TODO** lo relacionado con Acryl —incluida la base de datos `acryl.db` (tus medios indexados), la configuración YAML y el caché de pósters— usa la opción `--purge`:

```bash
./scripts/uninstall.sh --purge
```

Si instalaste con `--system`, desinstala también con `--system`:

```bash
sudo ./scripts/uninstall.sh --system --purge
```

> **⚠️ Advertencia:** `--purge` borra tu base de datos `acryl.db`, la configuración y el caché de pósters, y **no se puede deshacer**. El script pedirá confirmación antes de proceder (usa `-f` para saltarte la confirmación).

---

## 🔄 Actualización

Para actualizar Acryl a la última versión sin tener que hacer `git pull` y luego ejecutar el instalador manualmente, usa el script de actualización:

```bash
./scripts/update.sh
```

El script hace lo siguiente:

1. Ejecuta `git pull` para traer los últimos cambios del repositorio.
2. Ejecuta `scripts/install.sh` para recompilar e instalar la aplicación.

Si instalaste con `--system`, actualiza también con `--system`:

```bash
sudo ./scripts/update.sh --system
```

> **Nota:** si tienes cambios sin confirmar en el working tree, el script te pedirá confirmación antes de hacer `git pull` para evitar conflictos.

---

## 💻 Uso

Una vez iniciada la aplicación:

1. **Indexar medios**: Acryl carga automáticamente los medios desde la base de datos SQLite configurada (`acryl.db` por defecto).
2. **Buscar**: Escribe un título en el campo de búsqueda para filtrar resultados.
3. **Ver detalles**: Haz clic en cualquier medio para ver su banner, sinopsis, año y archivos indexados.
4. **Volver atrás**: En la vista de detalles, el botón «← Volver» regresa a la serie desde una temporada o episodio, y a la lista principal desde una serie, película o anime.
5. **Filtrar por categorías**: Desde la barra superior, pulsa «Categorías» para ver los géneros disponibles y filtrar la biblioteca. Usa «Ver todo» para quitar el filtro.
6. **Reproducir**: Pulsa "▶ Reproducir" en la vista de detalles o en un episodio concreto; el archivo se abrirá en `mpv`.
7. **Puntuación de metadatos**: En la vista de detalles se muestra la puntuación obtenida de TVmaze (vote_average, 0-10) cuando está disponible.
8. **Actualizar metadatos**: En Ajustes, pulsa "Actualizar todos los metadatos" para volver a consultar TVmaze y sobrescribir título, año, sinopsis, puntuación, póster y banner de todos los medios indexados. La operación se ejecuta en segundo plano y pide confirmación.
9. **Cambiar idioma**: Usa el selector de idioma en la parte superior de la aplicación.
10. **Gestionar carpetas**: En Ajustes puedes añadir carpetas con el selector nativo
   ("Examinar...") o escribiendo la ruta. Al añadir una carpeta se indexa en segundo
   plano y la galería se actualiza al terminar. "Reindexar todas las carpetas" vuelve
   a escanear las rutas configuradas. Al eliminar una carpeta se pide confirmación y
   solo se borran sus entradas de la biblioteca: **los archivos de vídeo del disco no
   se tocan**.

### Estructura de carpetas e indexado

Acryl detecta automáticamente películas, series y anime a partir de la estructura:

- Los vídeos sueltos en la raíz de una carpeta configurada se indexan como películas.
- Cada subcarpeta se indexa como una serie (o anime si algún componente de la ruta
  se llama `Anime`/`Animes`).
- Las carpetas con un único vídeo cuyo nombre no parece un episodio se indexan como
  película.
- Las carpetas de temporada se detectan con variantes como `S01`, `s01`,
  `Season1`, `Season 1`, `Temporada 1`, `1ª Temporada`, `T2` o con el título
  delante (`Solo Leveling S01`, `Solo Leveling Season 2`), incluso con metadatos
  entre corchetes (`Solo Leveling S01 [1080p]`).
- Las carpetas que solo contienen otras carpetas (`Peliculas/`, `Series/`) se
  recorren como contenedores y no crean un medio propio.
- Los nombres se limpian de resolución, códecs, fuentes, audio y grupos de
  release (`1080p`, `x265`, `H.264`, `BluRay`, `WEB-DL`, `DDP5.1`, `[YTS.MX]`...)
  para obtener títulos correctos y mejorar las búsquedas de metadatos.
- Se omiten los archivos ocultos y los enlaces simbólicos, y se limita la
  profundidad de escaneo para evitar recorridos infinitos.

Reindexar es idempotente y conserva los metadatos ya sincronizados (título, año,
sinopsis, póster y banner).

---

## 📁 Estructura del repositorio

```
app-media-manager/
├── src/                      ← Rust puro (sin Tauri, sin WebView, sin Node.js)
│   ├── main.rs               Punto de entrada de la aplicación GTK4
│   ├── lib.rs                Biblioteca (config, core, db, ui)
│   ├── config/               Gestión de configuración YAML
│   ├── core/                 Lógica de negocio (library, metadata, player, poster_service)
│   ├── db/                   SQLite, conexiones, repositorios, migraciones, FTS5
│   └── ui/                   Interfaz GTK4 / Libadwaita
│       ├── main_window.rs    Ventana principal
│       ├── media_list.rs     Galería de medios con pósters
│       ├── media_details.rs  Vista de detalles
│       ├── search_bar.rs     Barra de búsqueda
│       ├── settings_window.rs Vista de ajustes
│       └── poster.rs         Renderizado de pósters con gdk-pixbuf
├── scripts/                  Scripts de instalación, desinstalación y actualización
│   ├── install.sh            Instalador (sin Node.js, sin privilegios de root por defecto)
│   ├── uninstall.sh          Desinstalador (con opción --purge para borrar todo)
│   └── update.sh             Actualizador (git pull + install en un solo paso)
├── diagnostic/               Scripts de diagnóstico Wayland/Hyprland
├── tests/                    Pruebas de integración (SQLite temporal, mpv)
├── docs/                     Documentación y ADRs
│   └── adr/                  Decisiones de arquitectura
├── icons/                    Recursos gráficos (icono, etc.)
├── PLAN_DESKTOP.md           Plan de migración y hitos del proyecto
├── AGENTS.md                 Instrucciones para el desarrollo
├── CONTRIBUTING.md           Flujo de trabajo y Pull Requests
├── SECURITY.md               Política de seguridad
├── LICENSE                   Licencia MIT
├── README.md                 Este archivo
└── Cargo.toml                Proyecto Rust único (edición 2024, rust 1.90)
├── docker-compose.yml        Sonarr + Radarr (contenedores Docker)
└── config.example.yaml       Configuración de ejemplo de Acryl
```

---

## 📺 Gestión de descargas con Sonarr y Radarr

El repositorio incluye un [`docker-compose.yml`](./docker-compose.yml) que levanta
Sonarr (series/anime) y Radarr (películas) en contenedores Docker. Ambos comparten
la misma carpeta de descargas y escriben **hardlinks** a la biblioteca organizada
de Acryl.

### Estructura de carpetas en el host

```
/home/coronel/Videos/
├── downloads/   ← Descargas entrantes (nombres sucios, ej. "Serie.S01E01.1080p")
└── media/       ← Biblioteca organizada para Acryl (hardlinks desde downloads)
```

> ⚠️ **Importante:** `downloads` y `media` deben estar en el mismo filesystem para
> que los hardlinks funcionen. En este entorno ambos están bajo `/home/coronel/Videos/`
> (btrfs).

### Puertos

| Servicio | Puerto | URL |
|----------|--------|-----|
| Sonarr   | 8989   | http://localhost:8989 |
| Radarr   | 7878   | http://localhost:7878 |

### Levantar los contenedores

```bash
docker compose up -d
```

### Configuración en Sonarr y Radarr (UI web)

En cada aplicación, tras el primer arranque:

1. **Root folders (biblioteca):**
   - Sonarr → `/tv` (mapeado a `/home/coronel/Videos/media`)
   - Radarr → `/movies` (mapeado a `/home/coronel/Videos/media`)
2. **Carpeta de descargas:** `/downloads` (mapeado a `/home/coronel/Videos/downloads`)
3. **Hardlinks:** Habilita la opción de hardlinks en la configuración de descargas
   de cada aplicación para que no duplique espacio en disco.

Cuando Sonarr/Radarr importe un archivo desde `/downloads`, creará un hardlink en
`/media` con el nombre limpio y la estructura organizada (serie/temporada/episodio
o película). Acryl indexará desde `/media` y verá los nombres correctos.

### Configurar Acryl para usar la biblioteca de media

Copia el archivo de ejemplo y adapta las rutas si es necesario:

```bash
cp config.example.yaml ~/.config/acryl/config.yaml
```

O ejecuta Acryl directamente señalando el archivo de ejemplo:

```bash
ACRYL_CONFIG=./config.example.yaml \
XDG_DATA_HOME=/home/coronel/.local/share/acryl \
./target/release/acryl
```

En la clave `media_folders` del YAML está configurada la ruta
`/home/coronel/Videos/media`. Desde Ajustes en Acryl puedes añadir o quitar
carpetas de indexado en cualquier momento.

---

## 🛠️ Comandos disponibles

### Desde la raíz del repositorio

| Comando | Descripción |
|---------|-------------|
| `cargo build` | Compila la aplicación Rust + GTK4 |
| `cargo run` | Ejecuta la aplicación en modo desarrollo |
| `cargo test` | Ejecuta las pruebas de Rust |
| `cargo clippy` | Ejecuta el linter de Rust |
| `cargo fmt` | Aplica formato de Rust |
| `cargo build --release` | Genera el binario nativo Linux optimizado |


---

## 📄 Dependencias y reconocimientos

Acryl utiliza las siguientes dependencias principales. Todas son de código abierto y sus licencias se respetan íntegramente.

### Backend (Rust)

| Crate | Propósito | Licencia |
|-------|-----------|----------|
| `rusqlite` | Bindings a SQLite | MIT |
| `serde` / `serde_json` / `serde_yaml` | Serialización | MIT |
| `reqwest` | Cliente HTTP (TVmaze) | MIT / Apache-2.0 |
| `libc` | Tipos C para FFI | MIT / Apache-2.0 |
| `thiserror` | Errores tipados | MIT |
| `log` | Facade de logging | MIT |
| `pkg-config` | Detección de librerías | MIT |

### UI (GTK4 / Libadwaita)

| Crate | Propósito | Licencia |
|-------|-----------|----------|
| `gtk4` | Interfaz gráfica | LGPL |
| `libadwaita` | Componentes de UI (Libadwaita) | LGPL |
| `gdk-pixbuf` | Renderizado de imágenes (pósters) | LGPL |
| `cairo` | Gráficos 2D | LGPL / MPL |

### Sistema

| Paquete | Propósito | Licencia |
|---------|-----------|----------|
| `gtk4` | Interfaz gráfica | LGPL |
| `mpv` | Reproducción de vídeo externa | GPLv2+ |
| `libgtk-4` | Interfaz gráfica | LGPL |
| `libadwaita-1` | Componentes de UI | LGPL |
| `librsvg2` | Renderizado SVG | LGPL |

> Todas las dependencias de terceros conservan sus licencias originales. Este proyecto se distribuye bajo la licencia **MIT**. Consulta `LICENSE` para más detalles.

---

## 📄 Licencia

[MIT](./LICENSE) © 2026 Angel-p-v

---

## 🤝 Contribución

Consulta [`CONTRIBUTING.md`](./CONTRIBUTING.md) para el flujo de trabajo, convenciones de commits y reglas de calidad.

---

## 🔒 Seguridad

Consulta [`SECURITY.md`](./SECURITY.md) para la política de seguridad y cómo reportar vulnerabilidades.

---

<p align="center">
  <sub>Hecho con ❤️ para gestión de contenido multimedia en Linux.</sub><br/>
  <sub><a href="https://github.com/Angel-p-v/app-media-manager/issues">Reportar bug</a> • <a href="https://github.com/Angel-p-v/app-media-manager/discussions">Preguntar</a> • <a href="https://github.com/Angel-p-v/app-media-manager/blob/main/CONTRIBUTING.md">Contribuir</a></sub>
</p>