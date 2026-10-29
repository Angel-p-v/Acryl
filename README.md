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
  <a href="https://nodejs.org/">
    <img src="https://img.shields.io/badge/node-%3E%3D22-5FA04E?style=for-the-badge" alt="Node.js"/>
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

```text
┌─────────────────────────────────────────────────────────────────────────┐
│                         FRONTEND (React + TypeScript)                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │   SearchBar  │  │  MediaList   │  │ MediaDetails │  │  VideoPlayer │   │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘   │
│           │                  │                  │                  │         │
│           └──────────────────┴──────────────────┴────────────────┘         │
│                               │ invoke (Tauri IPC)                          │
│                               ▼                                             │
┌─────────────────────────────────────────────────────────────────────────────┐
│                         BACKEND (Rust + Tauri v2)                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │   Commands   │  │   Config     │  │   Metadata   │  │   Player     │   │
│  │  (adaptador) │  │   (YAML)     │  │   (TVmaze)   │  │   (libmpv)   │   │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘   │
│                               │                                             │
│                               ▼                                             │
│                    ┌─────────────────────────────────────────┐              │
│                    │           BASE DE DATOS SQLite           │              │
│                    │  ┌─────────────┐  ┌─────────────┐  ┌─────────┐  │      │
│                    │  │   media     │  │   files     │  │ playback  │  │      │
│                    │  │   ┌─────┐   │  │   ┌─────┐   │  │   history   │  │      │
│                    │  │   │ id  │   │  │   │ id  │   │  │   ┌─────┐   │  │      │
│                    │  │   │ title│   │  │   │ media_id│   │  │ media_id│   │  │      │
│                    │  │   │ year │   │  │   │ season│   │  │   │played_at│   │  │      │
│                    │  │   │ type │   │  │   │ episode│   │  │   │ position│   │  │      │
│                    │  │   │ folder│   │  │   │ title│   │  │   │ duration│   │  │      │
│                    │  │   │ path │   │  │   │ file_path│   │  │   └─────┘   │  │      │
│                    │  │   │ poster│   │  │   │ size   │   │  │             │  │      │
│                    │  │   │_path │   │  │   │_bytes │   │  │             │  │      │
│                    │  │   │ synopsis│   │  │   │mtime│   │  │             │  │      │
│                    │  │   │_path │   │  │   │_ms │   │  │             │  │      │
│                    │  │   │is_fav│   │  │   │ indexed│   │  │             │  │      │
│                    │  │   │_ite │   │  │   │_at │   │  │             │  │      │
│                    │  │   │is_watc│   │  │   │ │   │   │  │             │  │      │
│                    │  │   │_hed │   │  │   │ │   │   │  │             │  │      │
│                    │  │   │metadata│   │  │   │ │   │   │  │             │  │      │
│                    │  │   │_fetched│   │  │   │ │   │   │  │             │  │      │
│                    │  │   │_at   │   │  │   │ │   │   │  │             │  │      │
│                    │  │   └─────┘   │  │   │ │   │   │  │             │  │      │
│                    │  └─────────────┘  │   │ │   │   │  │             │  │      │
│                    │                   │   │ │   │   │  │             │  │      │
│                    │                   │   └─────┘   │  │             │  │      │
│                    │                   └─────────────┘  │             │  │      │
│                    └─────────────────────────────────────┘             │      │
│                                                                          │      │
└─────────────────────────────────────────────────────────────────────────┘
```

### Flujo de datos

1. El usuario busca o navega por medios en el frontend (React + TypeScript).
2. El frontend invoca un comando Tauri que llega al backend Rust.
3. El backend consulta **una única base de datos SQLite** (`acryl.db`) y devuelve los resultados al frontend.
4. Para la reproducción, el backend utiliza libmpv vía FFI para abrir y controlar el archivo de vídeo dentro de la misma ventana.
5. Los metadatos pueden sincronizarse con TVmaze (API pública, sin API key) para obtener pósteres y sinopsis automáticamente.

### Principios de diseño

- **Separación de responsabilidades**: el dominio no depende del frontend ni del WebView; el frontend no ejecuta SQL ni accede directamente al sistema de archivos.
- **Inyección de dependencias**: dependencias explícitas, sin singletons globales ni estado mutable innecesario.
- **Contratos tipados**: Serde para serialización Rust ↔ TypeScript, con validación en el límite de entrada.
- **Seguridad por diseño**: mínimo privilegio, sin acceso general al sistema de archivos, sin ejecución de procesos externos arbitrarios.

---

## 📋 Requisitos del sistema

### Dependencias de sistema (Linux)

| Paquete | Propósito | Distribución |
|---------|-----------|--------------|
| `libwebkit2gtk-4.0-dev` | WebView del sistema (Tauri) | Todas |
| `libgtk-3-dev` | Interfaz gráfica (Tauri) | Todas |
| `librsvg2-dev` | Renderizado de SVG (Tauri) | Todas |
| `libayatana-appindicator3-dev` | Indicadores de bandeja (opcional) | Todas |
| `libmpv-dev` | Reproducción de vídeo integrado | Todas |
| `xvfb` | Servidor virtual X11 (solo pruebas E2E) | Todas |

### Herramientas de desarrollo

| Herramienta | Versión mínima |
|-------------|---------------|
| Node.js | ≥ 22 |
| npm | ≥ 10 |
| Rust / Cargo | ≥ 1.90 |
| Tauri CLI | Instalado automáticamente |

---

## 📦 Instalación

### Instalación por distribución

Elige tu distribución y ejecuta el bloque correspondiente:

#### 🟦 Debian / Ubuntu / Mint / Pop!_OS

```bash
sudo apt-get update -qq && sudo apt-get install -y -qq \
    libwebkit2gtk-4.0-dev \
    libgtk-3-dev \
    librsvg2-dev \
    libayatana-appindicator3-dev \
    libmpv-dev

# 2. Clonar el repositorio
git clone https://github.com/Angel-p-v/app-media-manager.git acryl
cd acryl

# 3. Instalar dependencias de Node.js y ejecutar en modo desarrollo
npm install
npm run tauri:dev
```

#### 🟦 Arch Linux / Manjaro / EndeavourOS

```bash
sudo pacman -Sy --needed \
    webkit2gtk \
    gtk3 \
    librsvg \
    libayatana-appindicator \
    libmpv

# 2. Clonar el repositorio
git clone https://github.com/Angel-p-v/app-media-manager.git acryl
cd acryl

# 3. Instalar dependencias de Node.js y ejecutar en modo desarrollo
npm install
npm run tauri:dev
```

#### 🟦 Fedora / RHEL / CentOS / AlmaLinux / Rocky Linux

```bash
sudo dnf install -y \
    webkit2gtk3-devel \
    gtk3-devel \
    librsvg2-devel \
    libayatana-appindicator3-devel \
    libmpv-devel

# 2. Clonar el repositorio
git clone https://github.com/Angel-p-v/app-media-manager.git acryl
cd acryl

# 3. Instalar dependencias de Node.js y ejecutar en modo desarrollo
npm install
npm run tauri:dev
```

#### 🟦 openSUSE / SUSE Linux Enterprise

```bash
sudo zypper install -y \
    libwebkit2gtk-4_0-devel \
    gtk3-devel \
    librsvg2-devel \
    libayatana-appindicator3-devel \
    libmpv-devel

# 2. Clonar el repositorio
git clone https://github.com/Angel-p-v/app-media-manager.git acryl
cd acryl

# 3. Instalar dependencias de Node.js y ejecutar en modo desarrollo
npm install
npm run tauri:dev
```

#### 🟦 Alpine Linux

```bash
sudo apk add \
    webkit2gtk-dev \
    gtk+3-dev \
    librsvg-dev \
    libayatana-appindicator3-dev \
    libmpv-dev

# 2. Clonar el repositorio
git clone https://github.com/Angel-p-v/app-media-manager.git acryl
cd acryl

# 3. Instalar dependencias de Node.js y ejecutar en modo desarrollo
npm install
npm run tauri:dev
```

### Instaladores nativos

Una vez compilado, los instaladores se generan en:

```
src-tauri/target/release/bundle/
```

Formatos disponibles: `.deb`, AppImage, `.rpm`.

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
├── frontend/                 ← React + TypeScript (Vite), WebView de Tauri
│   ├── src/
│   │   ├── components/       Poster, MediaList, SearchBar, MediaDetails, VideoPlayer
│   │   ├── services/         Llamadas a comandos Tauri (invoke)
│   │   ├── contexts/         Contexto de idioma (i18n)
│   │   ├── locales/          Archivos de traducción (es.json, en.json)
│   │   ├── types/            Contratos TypeScript compartidos con Rust
│   │   ├── App.tsx, main.tsx, index.css
│   │   └── __tests__/        Pruebas de Vitest
│   ├── package.json
│   └── vite.config.ts
├── src-tauri/                ← Backend Rust (Tauri v2)
│   ├── src/
│   │   ├── main.rs           Punto de entrada de la aplicación
│   │   ├── lib.rs            Registro de comandos Tauri y estado gestionado
│   │   ├── commands.rs       Comandos expuestos al frontend
│   │   ├── config/           Gestión de configuración YAML
│   │   ├── db/               SQLite, repositorios, migraciones, FTS5
│   │   ├── metadata/         Cliente TVmaze y sincronización de metadatos
│   │   └── player/           Bindings a libmpv (reproducción integrada)
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── build.rs              Configuración de compilación (enlaza libmpv)
│   ├── tauri.conf.json       Configuración de Tauri (ventana, seguridad, bundle)
│   └── resources/
│       └── acryl.desktop     Entrada .desktop para Linux
├── PLAN_DESKTOP.md           Plan de migración y hitos del proyecto
├── AGENTS.md                 Instrucciones para el desarrollo
├── CONTRIBUTING.md           Flujo de trabajo y Pull Requests
├── SECURITY.md               Política de seguridad
├── LICENSE                   Licencia MIT
├── README.md                 Este archivo
└── package.json              Scripts raíz que delegan a frontend/
```

---

## 🛠️ Comandos disponibles

### Desde la raíz del repositorio

| Comando | Descripción |
|---------|-------------|
| `npm install` | Instala dependencias del frontend (Node.js ≥ 22) |
| `npm run dev` | Inicia el servidor de desarrollo Vite (puerto 3000) |
| `npm run build` | Compila el frontend hacia `../build` |
| `npm run test` | Ejecuta las pruebas de Vitest (frontend) |
| `npm run lint` | Ejecuta ESLint del frontend |
| `npm run format` | Aplica Prettier al código fuente |
| `npm run format:check` | Verifica el formato de Prettier |
| `npm run typecheck` | Ejecuta `tsc --noEmit` del frontend |
| `npm run verify` | Ejecuta lint + format:check + typecheck + test |
| `npm run tauri:dev` | Inicia la aplicación de escritorio en modo desarrollo |
| `npm run tauri:build` | Genera instaladores nativos Linux (`.deb`, AppImage, `.rpm`) |

### Desde `src-tauri/`

| Comando | Descripción |
|---------|-------------|
| `cargo build` | Compila el backend Rust |
| `cargo clippy` | Ejecuta el linter de Rust |
| `cargo test` | Ejecuta las pruebas de Rust |
| `cargo fmt` | Aplica formato de Rust |

---

## 📄 Dependencias y reconocimientos

Acryl utiliza las siguientes dependencias principales. Todas son de código abierto y sus licencias se respetan íntegramente.

### Backend (Rust)

| Crate | Propósito | Licencia |
|-------|-----------|----------|
| `tauri` | Framework de escritorio | MIT / Apache-2.0 |
| `rusqlite` | Bindings a SQLite | MIT |
| `serde` / `serde_json` / `serde_yaml` | Serialización | MIT |
| `reqwest` | Cliente HTTP (TVmaze) | MIT / Apache-2.0 |
| `libc` | Tipos C para FFI | MIT / Apache-2.0 |
| `thiserror` | Errores tipados | MIT |
| `log` | Facade de logging | MIT |
| `tauri-plugin-log` | Plugin de logs para Tauri | MIT / Apache-2.0 |
| `pkg-config` | Detección de librerías | MIT |

### Frontend (TypeScript / React)

| Paquete | Propósito | Licencia |
|---------|-----------|----------|
| `react` / `react-dom` | Interfaz de usuario | MIT |
| `@tauri-apps/api` | API de comandos Tauri | MIT / Apache-2.0 |
| `vite` | Herramienta de construcción | MIT |
| `vitest` | Tests unitarios | MIT |
| `eslint` | Linter | MIT |
| `prettier` | Formateador | MIT |
| `typescript` | Tipado estricto | Apache-2.0 |
| `better-sqlite3` | SQLite (desarrollo) | ISC |

### Sistema

| Paquete | Propósito | Licencia |
|---------|-----------|----------|
| `libmpv` | Reproducción de vídeo | GPLv2+ / LGPL |
| `libwebkit2gtk` | WebView del sistema | LGPL |
| `libgtk-3` | Interfaz gráfica | LGPL |
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