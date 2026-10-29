<p align="center">
  <img src="https://raw.githubusercontent.com/Angel-p-v/app-media-manager/main/docs/assets/logo.svg" alt="media-manager" width="200"/>
</p>

<h1 align="center">media-manager</h1>

<p align="center">
  <strong>Gestor de contenido multimedia de escritorio (Linux)</strong><br/>
  Indexa tu colección de series, anime y películas en una base de datos SQLite local<br/>
  y reprodúcela dentro de la propia aplicación, sin reproductores externos.
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

`media-manager` es una **aplicación de escritorio nativa Linux** que indexa tu colección de series, anime y películas en una **base de datos SQLite local** y te permite buscarla y **reproducirla dentro de la propia app**, sin depender de reproductores externos (`spawn` de mpv/vlc).

Está construida sobre:

| Capa | Tecnología |
|------|------------|
| **Backend** | Rust (edición 2024) |
| **Frontend** | TypeScript + React |
| **Framework desktop** | Tauri v2 (WebView del sistema) |
| **Base de datos** | SQLite (`rusqlite`), WAL + FTS5 |
| **Reproductor integrado** | `libmpv` vía FFI o GStreamer (Hito 3) |

---

## ⚡ Comenzando en 30 segundos

```bash
# 1. Clona e instala dependencias
git clone https://github.com/Angel-p-v/app-media-manager.git
cd app-media-manager
npm install

# 2. Ejecuta en modo desarrollo (frontend Vite + Tauri)
npm run tauri:dev

# 3. Compila para producción (instaladores nativos Linux)
npm run tauri:build
```

---

## 📦 Instalación desde instaladores

Descarga el instalador para tu distribución desde [GitHub Releases](https://github.com/Angel-p-v/app-media-manager/releases):

| Formato | Distribuciones | Comando de instalación |
|---------|---------------|----------------------|
| `.deb` | Debian, Ubuntu, Mint | `sudo dpkg -i media-manager_amd64.deb` |
| `.rpm` | Fedora, RHEL, openSUSE | `sudo rpm -i media-manager.x86_64.rpm` |
| `.AppImage` | Cualquier Linux | `chmod +x media-manager.AppImage && ./media-manager.AppImage` |

Después de instalar con `.deb` o `.rpm`, el comando `media-manager` estará disponible en tu PATH y aparecerá en el menú de aplicaciones.

---

## ✨ Características principales

| Capacidad | Estado |
|-----------|--------|
| 🖥️ Ventana Tauri con WebView del sistema | ✅ Hito 1 completado |
| 🗃️ Índice local en SQLite con FTS5 | ✅ Hito 2 completado |
| 🔍 Búsqueda de medios desde el frontend | ✅ Hito 1 completado |
| 🖼️ Pósteres de medios en alta resolución | ✅ Hito 1 completado |
| ▶️ Reproductor integrado (`libmpv` / GStreamer) | ✅ Hito 3 completado |
| 📦 Instaladores nativos Linux (`.deb`, AppImage, `.rpm`) | ✅ Hito 5 completado |

---

## 🚀 Scripts disponibles

Desde la raíz del repositorio:

| Comando | Descripción |
|---------|-------------|
| `npm run dev` | Inicia el frontend (Vite, puerto 3000) |
| `npm run build` | Compila el frontend hacia `../build` |
| `npm run test` | Ejecuta los tests de vitest (frontend) |
| `npm run lint` | ESLint del frontend |
| `npm run format` | Prettier del frontend |
| `npm run typecheck` | `tsc --noEmit` del frontend |
| `npm run verify` | lint + format:check + typecheck + test |
| `npm run tauri:dev` | App de escritorio en modo desarrollo |
| `npm run tauri:build` | Instaladores nativos Linux |

Desde `src-tauri/`:

| Comando | Descripción |
|---------|-------------|
| `cargo build` | Compila el backend Rust |
| `cargo clippy` | Linter de Rust |
| `cargo test` | Tests de Rust |
| `cargo fmt` | Formato de Rust |

---

## 🏗️ Estructura del repositorio

```
app-media-manager/
├── frontend/               ← React + TypeScript (Vite), WebView de Tauri
│   ├── src/
│   │   ├── components/     Poster, MediaList, SearchBar, …
│   │   ├── services/       llamadas a comandos Tauri (invoke)
│   │   ├── types/          contratos TS compartidos con Rust
│   │   ├── App.tsx, main.tsx, index.css
│   │   └── __tests__/      tests de vitest
│   ├── package.json
│   └── vite.config.ts
├── src-tauri/              ← Rust backend
│   ├── src/
│   │   ├── main.rs         punto de entrada
│   │   ├── lib.rs          run(), invoke_handler con comandos Tauri
│   │   ├── commands.rs     comandos expuestos al frontend
│   │   ├── media.rs        entidad Media
│   │   ├── db.rs           apertura SQLite, seed, list_media
│   │   └── core/           lógica de negocio (library, metadata, db)
│   ├── Cargo.toml
│   └── tauri.conf.json     (frontendDist ../build, devUrl :3000)
├── PLAN_DESKTOP.md         plan de migración (estado de hitos)
├── AGENTS.md               instrucciones para el agente
└── package.json            scripts raíz que delegan a frontend/
```

---

## ⚙️ Arquitectura

```
frontend/  (React + TS, WebView de Tauri)
   │  servicios (invoke a comandos Tauri) ──▶  src-tauri/  (Rust)
   │                                             ├── core/        lógica de negocio
   │                                             ├── db/          rusqlite, migraciones, FTS5
   │                                             ├── player/      bindings a libmpv o GStreamer
   │                                             ├── commands/    comandos Tauri expuestos
   │                                             └── config/      serde + YAML
```

**Reglas que no se rompen:**

1. El dominio (`src-tauri/core/`, `src-tauri/db/`) **nunca** importa desde `frontend/`.
2. `frontend/` **nunca** escribe SQL: accede a datos mediante comandos Tauri (`invoke`).
3. `src-tauri/db/` habla con SQLite; nadie más importa `rusqlite` fuera de `db/`.
4. El acceso a datos se inyecta (DIP): nada de singletons globales ni estado mutable.

---

## 🛠️ Desarrollo

```bash
# Desde la raíz del repo
npm install                       # instala dependencias del frontend (Node ≥ 22)
npm run dev                       # frontend dev server (Vite, puerto 3000)
npm run build                     # compila el frontend hacia ../build
npm test                          # suite completa de vitest (frontend)
npm run lint                      # eslint del frontend
npm run format                    # prettier --write del frontend
npm run typecheck                 # tsc --noEmit del frontend
npm run verify                    # typecheck + test del frontend  ← puerta de calidad

# Backend (desde src-tauri/)
cd src-tauri
cargo build                       # compila el backend Rust
cargo clippy                      # linter de Rust ← puerta de calidad
cargo test                        # tests de Rust ← puerta de calidad
cargo fmt                         # formateo de Rust

# App completa (Tauri)
npm run tauri dev                 # app de escritorio en modo desarrollo
npm run tauri build               # instaladores nativos Linux (.deb, AppImage, .rpm)
```

### Puerta de calidad

Antes de pushear, ambos deben pasar:

- `npm run verify` (frontend)
- `cargo clippy` + `cargo test` (backend)

---

## 📊 Estado de hitos

| Hito | Contenido | Estado |
|------|-----------|--------|
| Hito 1 | Prototipo mínimo: ventana Tauri, lista desde SQLite, póster, búsqueda | ✅ completado |
| Hito 2 | Capa de dominio en Rust (db, library, metadata) | ✅ completado |
| Hito 3 | Reproductor integrado (`libmpv` / GStreamer) | ✅ completado |
| Hito 4 | UI completa (detalles, favoritos, historial) | ✅ completado |
| Hito 5 | Empaquetado Linux (`.deb`, AppImage, `.rpm`) | ✅ completado |

Ver [`PLAN_DESKTOP.md`](./PLAN_DESKTOP.md) para el plan detallado.

---

## 📚 Documentación

| Documento | Descripción |
|-----------|-------------|
| [`PLAN_DESKTOP.md`](./PLAN_DESKTOP.md) | Plan de migración desktop por hitos |
| [`AGENTS.md`](./AGENTS.md) | Instrucciones para el agente de desarrollo |
| [`docs/README.md`](./docs/README.md) | Índice de documentación |
| [`docs/adr/`](./docs/adr) | Architecture Decision Records |
| [`CHANGELOG.md`](./CHANGELOG.md) | Historial de versiones |
| [`SECURITY.md`](./SECURITY.md) | Reportar vulnerabilidades |
| [`CONTRIBUTING.md`](./CONTRIBUTING.md) | Flujo de desarrollo y PRs |

---

## 📄 Licencia

[MIT](./LICENSE) © 2026 media-manager contributors

<p align="center">
  <sub>Hecho con ❤️ para gestión de contenido multimedia en Linux.</sub><br/>
  <sub><a href="https://github.com/Angel-p-v/app-media-manager/issues">Reportar bug</a> • <a href="https://github.com/Angel-p-v/app-media-manager/discussions">Preguntar</a> • <a href="https://github.com/Angel-p-v/app-media-manager/blob/main/CONTRIBUTING.md">Contribuir</a></sub>
</p>
