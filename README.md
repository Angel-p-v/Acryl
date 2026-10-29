<p align="center">
  <img src="https://raw.githubusercontent.com/Angel-p-v/app-media-manager/main/docs/assets/logo.svg" alt="media-manager" width="200"/>
</p>

<h1 align="center">media-manager</h1>

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

## 🎯 ¿Qué es media-manager?

**media-manager** es una aplicación de escritorio nativa para Linux que te permite indexar, buscar y reproducir tu colección local de contenido multimedia sin depender de reproductores externos ni servicios en la nube.

Todo se ejecuta localmente en tu máquina: la base de datos SQLite, el índice de búsqueda de texto completo y la reproducción de vídeo integrada mediante libmpv.

---

## ✨ Características principales

| Capacidad | Descripción |
|-----------|-------------|
| 📚 **Biblioteca local** | Indexa películas, series y anime en una base de datos SQLite con búsqueda FTS5 (full-text search) |
| 🔍 **Búsqueda rápida** | Busca por título con filtrado por tipo y paginación |
| 🖼️ **Pósteres y metadatos** | Visualiza pósteres, sinopsis, año y tipo de cada medio |
| ⭐ **Favoritos y visto** | Marca contenido como favorito o visto con un solo clic |
| ▶️ **Reproducción integrada** | Reproduce archivos de vídeo dentro de la aplicación sin reproductores externos |
| ⚙️ **Configuración YAML** | Ajustes persistentes en archivo YAML (idioma, ruta de base de datos, reproductor predeterminado) |
| 🌐 **Idiomas** | Interfaz disponible en español e inglés |
| 📦 **Instaladores Linux** | Genera paquetes `.deb`, AppImage y `.rpm` para instalación nativa |

---

## 🏗️ Arquitectura

media-manager sigue una arquitectura de capas separadas que garantiza mantenibilidad y pruebas:

```
┌─────────────────────────────────────────────────────────────┐
│                    Frontend (React + TypeScript)            │
│  ┌─────────────┐  ┌─────────────┐  ┌───────────────────┐   │
│  │   SearchBar │  │  MediaList  │  │   MediaDetails    │   │
│  │   VideoPlayer│  │   Poster    │  │   (metadatos)     │   │
│  └─────────────┘  └─────────────┘  └───────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                           │
                    Tauri IPC (invoke)
                           │
┌─────────────────────────────────────────────────────────────┐
│                      Backend (Rust + Tauri)                 │
│  ┌─────────────┐  ┌─────────────┐  ┌───────────────────┐   │
│  │ Commands    │  │  Config     │  │   Metadata        │   │
│  │ (adaptador) │  │  (YAML)     │  │   (TVmaze API)    │   │
│  └─────────────┘  └─────────────┘  └───────────────────┘   │
│  ┌─────────────┐  ┌─────────────┐  ┌───────────────────┐   │
│  │   DB Repo   │  │   Player    │  │   Playback        │   │
│  │  (SQLite)   │  │  (libmpv)   │  │   History         │   │
│  └─────────────┘  └─────────────┘  └───────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                           │
                    SQLite (WAL + FTS5)
                           │
                    ┌──────┴──────┐
                    │ media.db    │
                    └─────────────┘
```

### Flujo de datos

1. El usuario busca o navega por medios en el frontend (React).
2. El frontend invoca un comando Tauri que llega al backend Rust.
3. El backend consulta SQLite y devuelve los resultados al frontend.
4. Para la reproducción, el backend utiliza libmpv vía FFI para abrir y controlar el archivo de vídeo.
5. Los metadatos pueden sincronizarse con TVmaze para obtener pósteres y sinopsis automáticamente.

---

## 📋 Requisitos del sistema

### Dependencias de sistema (Linux)

La aplicación requiere las siguientes bibliotecas del sistema para compilar y ejecutar:

| Paquete | Propósito |
|---------|-----------|
| `libwebkit2gtk-4.0-dev` | WebView del sistema (Tauri) |
| `libgtk-3-dev` | Interfaz gráfica (Tauri) |
| `librsvg2-dev` | Renderizado de SVG (Tauri) |
| `libayatana-appindicator3-dev` | Indicadores de bandeja (opcional) |
| `libmpv-dev` | Reproducción de vídeo integrado |
| `xvfb` | Servidor virtual X11 (solo para pruebas E2E) |

### Instalación de dependencias (Debian/Ubuntu)

```bash
sudo apt-get update
sudo apt-get install -y \
  libwebkit2gtk-4.0-dev \
  libgtk-3-dev \
  librsvg2-dev \
  libayatana-appindicator3-dev \
  libmpv-dev
```

### Requisitos de desarrollo

| Herramienta | Versión mínima |
|-------------|---------------|
| Node.js | ≥ 22 |
| npm | ≥ 10 |
| Rust / Cargo | ≥ 1.90 |
| Tauri CLI | Instalado automáticamente |

---

## 🚀 Instalación

La única forma soportada de instalar media-manager es clonando el repositorio y compilando desde el código fuente. No se distribuyen instaladores precompilados públicamente; generas los instaladores tú mismo localmente.

### Paso 1: Clonar el repositorio

```bash
git clone https://github.com/Angel-p-v/app-media-manager.git
cd app-media-manager
```

### Paso 2: Instalar dependencias del frontend

```bash
npm install
```

### Paso 3: Ejecutar en modo desarrollo

```bash
npm run tauri:dev
```

Esto iniciará el servidor de desarrollo de Vite (puerto 3000) y la aplicación de escritorio Tauri simultáneamente.

### Paso 4: Generar instaladores de producción

```bash
npm run tauri:build
```

Los instaladores se generan en:
```
src-tauri/target/release/bundle/
```

Formatos disponibles: `.deb`, AppImage, `.rpm`.

---

## 💻 Uso

Una vez iniciada la aplicación:

1. **Indexar medios**: La aplicación carga automáticamente los medios desde la base de datos SQLite configurada (`media-manager.db` por defecto).
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
│   └── tauri.conf.json       Configuración de Tauri (ventana, seguridad, bundle)
├── PLAN_DESKTOP.md           Plan de migración y hitos del proyecto
├── AGENTS.md                 Instrucciones para el desarrollo
├── CONTRIBUTING.md           Flujo de trabajo y Pull Requests
├── SECURITY.md               Política de seguridad
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

## 🏗️ Arquitectura y decisiones técnicas

### Separación de responsabilidades

- El **dominio** no depende del frontend ni del WebView.
- El **frontend** no ejecuta SQL ni accede directamente al sistema de archivos.
- El acceso a SQLite se concentra en la capa de persistencia (`src-tauri/src/db/`).
- Los **comandos de Tauri** actúan como adaptadores de entrada: validan, delegan y devuelven resultados serializables.
- La lógica de negocio no se duplica entre React y Rust.

### Base de datos

Se utiliza **SQLite** con `rusqlite` (modo WAL y búsqueda FTS5). Las migraciones son versionadas y transaccionales. El esquema incluye tablas para medios, archivos indexados, historial de reproducción y metadatos sincronizados.

### Reproducción

La aplicación utiliza **libmpv** vía FFI para reproducir archivos de vídeo dentro de la propia ventana de la aplicación, sin lanzar procesos externos.

---

## 📄 Licencia

[MIT](./LICENSE) © 2026 media-manager contributors

---

<p align="center">
  <sub>Hecho con ❤️ para gestión de contenido multimedia en Linux.</sub><br/>
  <sub><a href="https://github.com/Angel-p-v/app-media-manager/issues">Reportar bug</a> • <a href="https://github.com/Angel-p-v/app-media-manager/discussions">Preguntar</a> • <a href="https://github.com/Angel-p-v/app-media-manager/blob/main/CONTRIBUTING.md">Contribuir</a></sub>
</p>