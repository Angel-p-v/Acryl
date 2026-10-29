# AGENTS.md — media-manager (app de escritorio)

Instrucciones para el agente que trabaje en este repositorio. Léete esto antes de tocar nada.

## Qué es este proyecto

`media-manager` es un gestor de contenido multimedia **de escritorio (Linux)** que indexa la colección de series, anime y películas del usuario en una **base de datos SQLite local** y permite buscarla y **reproducirla dentro de la propia aplicación**, sin depender de reproductores externos (`spawn` de mpv/vlc).

Es una **migración de la capa de presentación** de la CLI/TUI anterior (Ink + React en terminal, proyecto `media-manager`) a una aplicación de escritorio nativa Linux, manteniendo la lógica de negocio. Es un fork alojado en `app-media-manager/`.

- Plan de desarrollo: [`PLAN_DESKTOP.md`](./PLAN_DESKTOP.md)
- Hito 1 (prototipo mínimo): ✅ completado (ventana Tauri, lista desde SQLite, póster, búsqueda)
- Siguiente hito: **Hito 2** — capa de dominio en Rust (db, library, metadata)

## Stack (fijado, no cambiar sin ADR)

| Capa | Elección | Versión / nota |
|------|----------|----------------|
| Backend | Rust | edición 2024 |
| Frontend | TypeScript + React | tipado `strict`, Node ≥ 22 |
| Framework desktop | Tauri v2 | WebView del sistema, llamadas Rust↔TS, instaladores Linux |
| Base de datos | SQLite (`rusqlite`) | WAL + FTS5, esquema reusado del proyecto original |
| Reproductor integrado | `libmpv` vía FFI o GStreamer | frames dentro de la ventana (Hito 3) |
| Imágenes / pósters | HTML `img` o canvas dentro del WebView | alta resolución |
| Configuración | YAML + `serde` en Rust | reemplaza `zod` del lado de Rust |
| Tests | vitest + cargo test | frontend y backend |

## Estructura del repositorio

```
app-media-manager/                 ← raíz del fork (repo privado Angel-p-v/app-media-manager)
├── frontend/                      ← React + TypeScript (Vite), WebView de Tauri
│   ├── src/
│   │   ├── components/            Poster, MediaList, SearchBar, …
│   │   ├── services/              llamadas a comandos Tauri (invoke)
│   │   ├── types/                 contratos TS compartidos con Rust
│   │   ├── App.tsx, main.tsx, index.css
│   │   └── __tests__/             tests de vitest
│   ├── package.json
│   ├── vite.config.ts             (build → ../build, dev en :3000)
│   └── tsconfig.json
├── src-tauri/                     ← Rust backend
│   ├── src/
│   │   ├── main.rs                punto de entrada de la app
│   │   ├── lib.rs                 run(), invoke_handler con comandos Tauri
│   │   ├── commands.rs            comandos expuestos al frontend
│   │   ├── media.rs               entidad Media
│   │   ├── db.rs                  apertura SQLite, seed, list_media
│   │   └── core/                  (Hito 2+) lógica de negocio portada
│   ├── Cargo.toml
│   ├── tauri.conf.json            (frontendDist ../build, devUrl :3000)
│   └── Cargo.lock
├── PLAN_DESKTOP.md                plan de migración (estado de hitos)
├── AGENTS.md                      este archivo
├── package.json                   scripts raíz que delegan a frontend/
└── tests/                         e2e del binario Tauri (futuro)
```

## Arquitectura: la regla que no se rompe

```
frontend/  (React + TS, WebView de Tauri)
   │  servicios (invoke a comandos Tauri) ──▶  src-tauri/  (Rust)
   │                                             ├── core/        lógica de negocio (library, metadata, db)
   │                                             ├── db/          rusqlite, migraciones, FTS5
   │                                             ├── player/      bindings a libmpv o GStreamer
   │                                             ├── commands/    comandos Tauri expuestos al frontend
   │                                             └── config/      serde + YAML
```

1. **El dominio (`src-tauri/core/`, `src-tauri/db/`) nunca importa desde `frontend/`.** El dominio no sabe que existe una interfaz gráfica.
2. **`frontend/` nunca escribe SQL:** accede a datos mediante comandos Tauri que expone Rust (`invoke`).
3. **`src-tauri/db/` habla con SQLite; nadie más importa `rusqlite` directamente fuera de `db/`.**
4. El acceso a datos se **inyecta** (DIP): nada de singletons globales ni estado global mutable.

## Flujo de trabajo (obligatorio)

1. **Una rama por hito o feature**: `feat/…`, `fix/…`, `refactor/…`, `chore/…`.
   Ejemplo: `feat/hito-2-capa-dominio-rust`.
2. **Nunca se commitea directamente en `main`.** Todo el trabajo va en su propia rama.
3. Commits con **Conventional Commits** y scope del proyecto:
   `config`, `db`, `indexer`, `parser`, `player`, `ui`, `cli`, `deps`, `ci`, `docs`, `test`, `release`:
   `feat(db): puerto de mediaRepo a rusqlite`
4. `npm run verify` (frontend) y `cargo clippy` + `cargo test` (backend) **deben pasar** antes de push.
5. `git push -u origin <rama>` y **PR a `main`** con `gh pr create`.
6. Revisar que la CI de la PR está en verde y **mergear con squash**:
   `gh pr merge <n> --squash --delete-branch`.
7. `main` queda siempre desplegable: cada commit compila, pasa lint y tiene tests.

### Política de commits pequeños y atómicos

Un commit debe representar **una sola intención revisable**, no el trabajo acumulado de varias sesiones.

- **Objetivo:** cerrar un commit cada **5–10 archivos** o **200–300 líneas** de un mismo cambio coherente, en cuanto sus tests estén en verde. No esperes a terminar la feature completa.
- **Límite:** no superes **15 archivos ni 500 líneas modificadas** por commit. Si el cambio previsto los supera, párate y divídelo antes de seguir.
- El código y los tests del mismo comportamiento van juntos. La documentación, el _formatting_ masivo y las dependencias van en commits separados.
- Separa siempre que sea posible:
  - refactor mecánico (renombres, extracciones) sin cambio de comportamiento;
  - cambio funcional;
  - tests adicionales que no cubren el cambio principal;
  - documentación;
  - dependencias y lockfile;
  - formato o movimiento masivo de archivos.
- Los cambios masivos inevitables (movimientos, migraciones, lockfile, formato) se aíslan en su propio commit y se justifican en el cuerpo.
- Antes de commitear revisa `git diff --stat`, `git diff --check` y `git status`. Usa `git add <rutas>` o `git add -p`; evita `git add -A` si hay cambios mezclados.
- Si aparecen cambios no relacionados, no los mezcles: commitéalos aparte o déjalos sin _stage_. Cada commit debe dejar el árbol compilando y los tests relevantes en verde.

## Comandos

```bash
# Desde la raíz del repo
npm install                       # instala dependencias del frontend (Node >= 22)
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

*(Los scripts `tauri:*` se definen en `package.json` de la raíz al crear el esqueleto en el Hito 1.)*

## Estrategia de tests

| Nivel | Ubicación | Qué cubre | Objetivo |
|-------|-----------|-----------|----------|
| unitario (Rust) | `src-tauri/src/**` (`#[cfg(test)]`) | lógica pura, consultas DB | < 1 s |
| unitario (TS) | `frontend/src/**` (vitest) | componentes, hooks, servicios | < 1 s |
| integración (Rust) | `src-tauri/tests/` | SQLite real, fs real | < 10 s |
| e2e | `tests/e2e/` | binario compilado de Tauri | < 60 s |

Reglas: se testea **comportamiento público**, no implementación · cada test es independiente (su propio directorio temporal, su propia BD) · nombres `describe > it` en Given/When/Then · los umbrales de cobertura viven en `vitest.config.ts`.

## Gotchas conocidos (no repetirlos)

- **FFI a `libmpv` / GStreamer**: manejar punteros raw con `unsafe` acotado; nunca dejar fugas de recursos (`mpv_terminate_destroy` / equivalente en `Drop`).
- **Comunicación Rust↔TS**: comandos Tauri con tipado estricto en ambos extremos (`struct` con `serde` en Rust y interfaces TS correspondientes; `invoke<Tipo>` en el frontend).
- **SQLite**: leer el archivo `.db` existente directamente desde Rust, sin migración de datos (el esquema viaja con las migraciones en Rust).
- **Configuración YAML**: reemplazar `zod` por `serde` + `serde_yaml` en Rust.
- **WebView de Tauri**: no asumir APIs de Node (`fs`, `child_process`) en el frontend; todo el acceso al sistema va por comandos Tauri.
- **Reproductor integrado**: renderizar frames de `libmpv` en una superficie compartida con el WebView o en ventana nativa aparte; evitar bloquear el hilo UI de Tauri.
- **Cierre de SQLite**: nunca dejar conexiones abiertas; abrir y cerrar por comando Tauri.

## Reglas para el agente

- Escribe código en **inglés** (identificadores, comentarios técnicos) y la documentación de cara al usuario en **español**.
- Nada de `any`, nada de `as` sin motivo justificado, nada de estados globales.
- Cada función nueva va con su test; si el bug es curioso, el test lo documenta.
- Comenta **por qué**, no el qué obvio.
- Si hay que tomar una decisión con más de un camino válido, escribe un ADR en `docs/adr/` y anótala en el `PLAN_DESKTOP.md`.
- No amplíes el alcance del hito en curso sin avisar; es un proyecto por hitos.
- Trocea el trabajo según la política de commits pequeños; no acumules cambios para un único commit final.
- Al terminar un hito: verificar, commit, push, PR, review, merge con squash, y actualizar la tabla de estado de `PLAN_DESKTOP.md` y del `README.md`.
- **No pasar al siguiente hito hasta que la PR del hito actual esté mergeada y en `main`.** Cada hito debe cerrar su PR y verificarse en CI antes de empezar el siguiente.

## Reglas de tipado estricto y modelado de dominio

Estas reglas se aplican en **cada línea de código, refactorización o componente** que se escriba en el proyecto.

### 1. Tipado estricto y absoluto

- Todo debe estar tipado de forma explícita: argumentos de funciones, valores de retorno, variables, constantes y estructuras de datos.
- Queda **totalmente PROHIBIDO** el uso de `any`. Si un tipo es desconocido en tiempo de compilación, utiliza `unknown` y aplica Type Guards o aserciones de control (narrowing) antes de operar con él.
- Para datos dinámicos o respuestas de APIs externas, utiliza esquemas de validación en tiempo de ejecución.
- Deshabilita cualquier asunción implícita (actúa como si `noImplicitAny`, `strictNullChecks` y `strict` estuvieran siempre activados).

### 2. Manejo de nulos y valores opcionales

- Trata de forma explícita los posibles valores `null` o `undefined`. No uses aserciones de no-nulo (`!`) a menos que exista una justificación indiscutible.
- Utiliza encadenamiento opcional (`?.`) y el operador de fusión nula (`??`) cuando corresponda.

### 3. Modelado de datos y dominio

- Prioriza `interface` para contratos de objetos/entidades expansibles y `type` para uniones, intersecciones, tuplas y tipos utilitarios.
- Define Union Types discriminadas para gestionar estados complejos o respuestas de éxito/error.
- Emplea `readonly` e inmutabilidad por defecto.

### 4. Control de errores

- No lances excepciones genéricas (`throw new Error(...)`). Crea clases de error personalizadas tipadas o utiliza tipos de resultado explícitos como `Result<T, E>`.
- En bloques `catch`, tipa el parámetro de error como `unknown` y valida su estructura antes de procesarlo.

### 5. Arquitectura y estilo de código

- Diseña funciones puras de responsabilidad única. Evita efectos secundarios no tipados.
- Utiliza nombres claros y descriptivos para interfaces, tipos y genéricos.
- Exporta todos los tipos compartidos para que puedan ser reutilizados de forma consistente.

Si identificas código no tipado o ambiguo, corrígelo e infórmalo al usuario.
