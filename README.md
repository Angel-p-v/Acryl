<p align="center">
  <img src="src-tauri/icons/acryl.svg" alt="Acryl" width="200"/>
</p>

<h1 align="center">Acryl</h1>

<p align="center">
  <strong>Gestor de contenido multimedia de escritorio para Linux</strong><br/>
  Indexa tu colección de películas, series y anime en una base de datos SQLite local<br/>
  y reprodúcelo dentro de la propia aplicación, sin depender de reproductores externos.
</p>

<p align="center">
  <a href="https://github.com/Angel-p-v/app-media-manager/releases/latest">
    <img src="https://img.shields.io/github/v/release/Angel-p-v/app-media-manager?label=latest%20release&style=for-the-badge" alt="Latest Release"/>
  </a>
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
  <a href="https://github.com/Angel-p-v/app-media-manager/releases">
    <img src="https://img.shields.io/github/downloads/Angel-p-v/app-media-manager/total?style=for-the-badge" alt="Downloads"/>
  </a>
</p>

---

## 🎯 ¿Qué es Acryl?

**Acryl** es una aplicación de escritorio nativa para Linux que te permite indexar, buscar y reproducir tu colección local de películas, series y anime —todo dentro de la propia aplicación, sin depender de reproductores externos ni servicios en la nube.

Todo se ejecuta localmente en tu máquina: la base de datos SQLite, el índice de búsqueda de texto completo (FTS5) y la reproducción de vídeo integrada mediante libmpv.

---

## ✨ Características principales

| Capacidad | Descripción |
|-----------|-------------|
| 📚 **Biblioteca local** | Indexa películas, series y anime en una base de datos SQLite con búsqueda FTS5 |
| 🔍 **Búsqueda rápida** | Busca por título con filtrado por tipo y paginación |
| 🖼️ **Pósteres y metadatos** | Visualiza pósteres, sinopsis, año y tipo de cada medio |
| ⭐ **Favoritos y visto** | Marca contenido como favorito o visto con un solo clic |
| ▶️ **Reproducción integrada** | Reproduce archivos de vídeo dentro de la aplicación sin reproductores externos |
| ⚙️ **Configuración YAML** | Ajustes persistentes en archivo YAML (idioma, ruta de base de datos, reproductor predeterminado) |
| 🌐 **Idiomas** | Interfaz disponible en español e inglés |
| 📦 **Instaladores Linux** | Genera paquetes `.deb`, AppImage y `.rpm` para instalación nativa |

---

## 🏗️ Arquitectura

media-manager (Acryl) sigue una arquitectura de capas separadas que garantiza mantenibilidad, pruebas y seguridad:

```mermaid
graph TD
    Usuario[[Usuario]]
    UI[UI<br/>GTK4 / Libadwaita (Rust)]
    Logic[Lógica<br/>Rust puro]
    Decision{¿Consulta<br/>o reproduce?}
    SQLite[(SQLite<br/>acryl.db)]
    MPV[[libmpv<br/>FFI]]
    TVmaze[TVmaze<br/>API pública]

    Usuario -->|busca| UI
    UI -->|llama| Logic
    Logic --> Decision
    Decision -->|consulta| SQLite
    Decision -->|reproduce| MPV
    SQLite -->|datos| UI
    MPV -->|frames| UI
    Logic -.->|sincroniza| TVmaze
    TVmaze -.->|metadatos| Logic
    UI -->|resultado| Usuario

    classDef usuario fill:#e0e7ff,stroke:#6366f1,stroke-width:1px
    classDef ui fill:#f0fdf4,stroke:#497975,stroke-width:1px
    classDef logic fill:#fffbeb,stroke:#f59e0b,stroke-width:1px
    classDef db fill:#fce7f3,stroke:#ec4899,stroke-width:1px
    classDef ext fill:#f3f4f6,stroke:#9ca3af,stroke-width:1px,stroke-dasharray: 3 3

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
4. Para la reproducción, el backend utiliza libmpv vía FFI para abrir y controlar el archivo de vídeo dentro de la misma ventana GTK4.
5. Los metadatos pueden sincronizarse con TVmaze (API pública, sin API key) para obtener pósteres y sinopsis automáticamente.

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
| `libmpv-dev` | Reproducción de vídeo integrado | Todas |
| `libgdk-pixbuf2.0-dev` | Decodificación de imágenes (pósters) | Todas |

### Herramientas de desarrollo

| Herramienta | Versión mínima |
|-------------|---------------|
| Rust / Cargo | ≥ 1.90 |
| `pkg-config` | Detección de librerías |

---

## 📦 Instalación

### 🚀 Cómo instalar

Acryl incluye un script que detecta tu gestor de paquetes, instala las
dependencias del sistema y de desarrollo, clona el repositorio, compila la
aplicación y crea la entrada `.desktop` para que la encuentres en el
buscador de aplicaciones.

Elige una de las dos opciones:

**Opción A — Ejecutar directamente (sin clonar):**

```bash
curl -fsSL https://raw.githubusercontent.com/Angel-p-v/app-media-manager/main/scripts/install.sh | bash
```

**Opción B — Clonar el repositorio y ejecutar desde allí:**

```bash
git clone https://github.com/Angel-p-v/app-media-manager.git acryl
cd acryl
./scripts/install.sh
```

El script hace lo siguiente:

1. Detecta el gestor de paquetes (`apt`, `pacman`, `dnf`, `zypper` o `apk`).
2. Instala las dependencias de sistema (`libgtk-4`, `libadwaita-1`, `librsvg2`, `libmpv`, `gdk-pixbuf2`, etc.).
3. Instala Rust/Cargo (≥ 1.90) si falta o está desactualizado.
4. Clona el repositorio (o lo actualiza si ya existe).
5. Compila la aplicación con `cargo build --release`.
6. Instala el binario generado y crea el archivo `.desktop` en
   `~/.local/share/applications`.

Al terminar, busca **Acryl** en el menú de aplicaciones o ejecuta `acryl`.

> **Nota:** el script usa `sudo` para instalar dependencias de sistema y el paquete generado. La entrada `.desktop` y el icono se instalan a nivel de usuario (`~/.local`), sin necesidad de root. **No requiere Node.js ni npm.**

---

### 🗑️ Cómo desinstalar

Si ya no necesitas Acryl, ejecuta el desinstalador:

**Opción A — Ejecutar directamente (sin clonar):**

```bash
curl -fsSL https://raw.githubusercontent.com/Angel-p-v/app-media-manager/main/scripts/uninstall.sh | bash
```

### 🗑️ Cómo desinstalar

Si ya no necesitas Acryl, ejecuta el desinstalador:

**Opción A — Ejecutar directamente (sin clonar):**

```bash
curl -fsSL https://raw.githubusercontent.com/Angel-p-v/app-media-manager/main/scripts/uninstall.sh | bash
```

**Opción B — Clonar el repositorio y ejecutar desde allí:**

```bash
git clone https://github.com/Angel-p-v/app-media-manager.git acryl
cd acryl
./scripts/uninstall.sh
```

El desinstalador elimina:

- La entrada `.desktop` de `~/.local/share/applications`.
- El icono de `~/.local/share/icons/hicolor/scalable/apps/`.
- El binario AppImage de `~/.local/bin/` (si se usó como fallback).
- El paquete nativo (`.deb` o `.rpm`) si fue instalado con el script.

**Por defecto no borra** el código fuente clonado ni la base de datos de tu biblioteca (`acril.db`), para que no pierdas tus medios indexados.

#### Borrado absoluto (`--purge`)

Si quieres eliminar absolutamente TODO —incluido el código fuente clonado y la base de datos `acril.db` (tus medios indexados)— usa la opción `--purge`:

```bash
# Opción A
curl -fsSL https://raw.githubusercontent.com/Angel-p-v/app-media-manager/main/scripts/uninstall.sh | bash -s -- --purge

# Opción B
git clone https://github.com/Angel-p-v/app-media-manager.git acryl
cd acryl
./scripts/uninstall.sh --purge
```

> **⚠️ Advertencia:** `--purge` borra tu base de datos `acril.db` y no se puede deshacer. El script pedirá confirmación antes de proceder.

---

### Instaladores nativos

Una vez compilado, los instaladores también se generan en:

```
src-tauri/target/release/bundle/
```

Formatos disponibles: `.deb`, AppImage, `.rpm`.

---

### Modo desarrollo

Si quieres ejecutar la aplicación en modo desarrollo (sin instalar):

```bash
git clone https://github.com/Angel-p-v/app-media-manager.git acryl
cd acryl
cargo run
```

---

## 💻 Uso

Una vez iniciada la aplicación:

1. **Indexar medios**: Acryl carga automáticamente los medios desde la base de datos SQLite configurada (`acryl.db` por defecto).
2. **Buscar**: Escribe un título en el campo de búsqueda para filtrar resultados.
3. **Ver detalles**: Haz clic en cualquier medio para ver su póster, sinopsis, año y archivos indexados.
4. **Marcar favorito/visto**: Usa los botones de estado en la vista de detalles.
5. **Reproducir**: Escribe la ruta de un archivo de vídeo en el reproductor integrado y pulsa "Abrir".
6. **Cambiar idioma**: Usa el selector de idioma en la parte superior de la aplicación.

---

## 📁 Estructura del repositorio

```
app-media-manager/
├── src/                      ← Rust puro (sin Tauri, sin WebView)
│   ├── main.rs               Punto de entrada de la aplicación GTK4
│   ├── core/                 ← Lógica de negocio (library, metadata, db)
│   ├── db/                   SQLite, repositorios, migraciones, FTS5
│   ├── player/               Bindings a libmpv (reproducción integrada)
│   ├── config/               Gestión de configuración YAML
│   ├── ui/                   Interfaz GTK4 / Libadwaita
│   │   ├── main_window.rs    Ventana principal
│   │   ├── search_bar.rs     Barra de búsqueda
│   │   ├── media_list.rs     Lista de medios
│   │   ├── media_details.rs  Vista de detalles
│   │   ├── video_player.rs   Reproductor embebido
│   │   └── poster.rs         Renderizado de pósters con gdk-pixbuf
│   ├── Cargo.toml
│   └── Cargo.lock
├── scripts/                  Scripts de instalación y desinstalación
│   ├── install.sh            Instalador automático (sin Node.js)
│   ├── uninstall.sh          Desinstalador
│   ├── diagnose-deep.sh      Diagnóstico profundo Wayland/Hyprland
│   ├── diagnose-final.sh     Diagnóstico final de renderizado
│   ├── fix-blank-screen.sh   Script de solución para pantalla en blanco
│   ├── fix-blank-screen-final.sh Script final de solución
│   └── acryl-launcher.sh     Lanzador con captura de errores
├── PLAN_DESKTOP.md           Plan de migración y hitos del proyecto
├── AGENTS.md                 Instrucciones para el desarrollo
├── CONTRIBUTING.md           Flujo de trabajo y Pull Requests
├── SECURITY.md               Política de seguridad
├── LICENSE                   Licencia MIT
├── README.md                 Este archivo
└── docs/                     Documentación y ADRs
    └── adr/                  Decisiones de arquitectura
```

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
| `cargo tauri build` | Genera instaladores nativos Linux (`.deb`, AppImage, `.rpm`) |


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
| `libmpv` | Reproducción de vídeo | GPLv2+ / LGPL |
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