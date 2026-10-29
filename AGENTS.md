# AGENTS.md — media-manager

Instrucciones para el agente que trabaje en este repositorio. Léete esto antes de tocar nada.

## Qué es este proyecto

`media-manager` (CLI: `mm`) es un gestor de contenido multimedia **100 % terminal**:
indexa la colección de series, anime y películas del usuario en una **base de datos SQLite
local** y permite buscarla y **lanzarla en `mpv`/`vlc`** sin salir de la terminal.

No hay servidor, no hay web, no hay streaming: solo índice local + reproductor local.

- Plan de desarrollo completo: [`PLAN.md`](./PLAN.md)
- Índice de documentación: [`docs/README.md`](./docs/README.md)
- Documentación técnica: [`docs/guides/architecture.md`](./docs/guides/architecture.md)
- Decisiones registradas (ADR): [`docs/adr/`](./docs/adr)

## Stack (fijado, no cambiar sin ADR)

| Capa     | Elección                                                                         | Versión                       |
| -------- | -------------------------------------------------------------------------------- | ----------------------------- |
| Runtime  | Node.js                                                                          | `>=22` (CI prueba 22, 24, 26) |
| Lenguaje | TypeScript `strict`                                                              | `~6.0.3`                      |
| TUI      | Ink + React + `@inkjs/ui`                                                        | 8.x / 19.x / 2.x              |
| CLI      | Commander                                                                        | 15.x                          |
| Config   | YAML + Zod                                                                       | 2.x / 4.x                     |
| BD       | better-sqlite3 (WAL + FTS5)                                                      | 13.x                          |
| Tests    | Vitest                                                                           | 5.x                           |
| Calidad  | ESLint 10 (type-aware) + Prettier 3.9 + Husky 9 + lint-staged 17 + commitlint 21 | —                             |

## Arquitectura: la regla que no se rompe

```
ui/  commands/  ──▶  core/  ──▶  shared/
                     └──▶ core/db/
```

1. `core/**` **nunca** importa desde `ui/**` ni `commands/**`. El dominio no sabe que
   existe una terminal.
2. `ui/**` y `commands/**` **nunca** escriben SQL: acceden a datos por repositorios.
3. `core/db/` habla con SQLite; nadie más importa `better-sqlite3`.
4. `shared/**` no depende de nada del proyecto (solo librerías estándar).
5. El acceso a datos se **inyecta** (DIP): `openDatabase()` devuelve la instancia y los
   repositorios la reciben como parámetro. Nada de singletons globales.

## Flujo de trabajo (obligatorio)

1. **Una rama por hito o feature**: `feat/…`, `fix/…`, `refactor/…`, `chore/…`.
2. Commits con **Conventional Commits** y scope del proyecto
   (`config`, `db`, `indexer`, `parser`, `player`, `ui`, `cli`, `deps`, `ci`, `docs`,
   `test`, `release`):
   `feat(indexer): soporta el patrón 1x02`
3. `npm run verify` **debe pasar** antes de hacer push (lo comprueba el hook de
   pre-push; el de pre-commit ejecuta lint-staged y typecheck).
4. `git push -u origin <rama>` y **PR a `main`** con `gh pr create`.
5. Revisar que la CI de la PR está en verde y **mergear con squash**:
   `gh pr merge <n> --squash --delete-branch`.
6. `main` queda siempre desplegable: cada commit compila, pasa lint y tiene tests.

**Nunca se commitea directamente en `main`.**

### Política de commits pequeños y atómicos

Un commit debe representar **una sola intención revisable**, no el trabajo acumulado de
varias sesiones.

- **Objetivo:** cerrar un commit cada **5–10 archivos** o **200–300 líneas** de un mismo
  cambio coherente, en cuanto sus tests estén en verde. No esperes a terminar la feature
  completa.
- **Límite:** no superes **15 archivos ni 500 líneas modificadas** por commit. Si el
  cambio previsto los supera, párate y divídelo antes de seguir.
- El código y los tests del mismo comportamiento van juntos. La documentación, el
  _formatting_ masivo y las dependencias van en commits separados.
- Separa siempre que sea posible:
  - refactor mecánico (renombres, extracciones) sin cambio de comportamiento;
  - cambio funcional;
  - tests adicionales que no cubren el cambio principal;
  - documentación;
  - dependencias y lockfile;
  - formato o movimiento masivo de archivos.
- Los cambios masivos inevitables (movimientos, migraciones, lockfile, formato) se
  aíslan en su propio commit y se justifican en el cuerpo.
- Antes de commitear revisa `git diff --stat`, `git diff --check` y `git status`. Usa
  `git add <rutas>` o `git add -p`; evita `git add -A` si hay cambios mezclados.
- Si aparecen cambios no relacionados, no los mezcles: commitéalos aparte o déjalos sin
  _stage_. Cada commit debe dejar el árbol compilando y los tests relevantes en verde.

## Comandos

```bash
npm install            # instala y prepara los hooks (husky)
npm run dev            # CLI en modo watch (tsx)
npm run verify         # lint + format:check + typecheck + test  ← puerta de calidad
npm test               # suite completa
npm run test:unit      # solo unitarias (rápidas)
npm run test:integration
npm run test:e2e
npm run coverage       # informe de cobertura
npm run bench          # benchmarks de escaneo, indexado y consultas
npm run smoke          # npm pack → instalación limpia → smoke de la CLI
npm run standalone     # bundle portable con Node oficial verificado (--node-source official)
npm run build          # compila a dist/
npm run lint:fix       # eslint --fix
npm run format         # prettier --write
```

## Estrategia de tests

| Nivel       | Ubicación            | Qué cubre                                  | Objetivo |
| ----------- | -------------------- | ------------------------------------------ | -------- |
| unitario    | `tests/unit/`        | lógica pura, parsers, config               | < 1 s    |
| integración | `tests/integration/` | SQLite real, `fs` real, render de Ink      | < 10 s   |
| e2e         | `tests/e2e/`         | binario compilado: argv, stdout, exit code | < 60 s   |

Reglas: se testea **comportamiento público**, no implementación · cada test es
independiente (su propio `mkdtemp`, su propia BD) · nombres `describe > it` en
Given/When/Then · los umbrales de cobertura viven en `vitest.config.ts` (85/85/80/85).

## Gotchas conocidos (no repetirlos)

- **npm 11.19+ bloquea los scripts de instalación.** `package.json` tiene el campo
  `allowScripts` con `better-sqlite3`, `esbuild` y `unrs-resolver`. Si añades una
  dependencia con script de instalación, apruébala con
  `npm install-scripts approve <pkg>` y **commitea** el cambio (si no, el CI falla).
- **TypeScript está fijado a `~6.0.3`** porque `typescript-eslint@8` declara
  `typescript: ">=4.8.4 <6.1.0"`. No subir a TS 7 hasta que el linter lo soporte.
- **better-sqlite3 y los parámetros con nombre**: si una sentencia usa `@nombre`, hay
  que pasar **un objeto** (`stmt.all({ tipo: … })`). Pasar valores posicionales lanza
  `RangeError: Too many parameter values were provided`.
- **`mkdirSync` sobre `/proc` se queda bloqueado** en Linux: nunca usar rutas bajo
  `/proc` en tests; usa un fichero normal como "bloqueo" para provocar `ENOTDIR`.
- **Migraciones**: se declaran en TypeScript (`src/core/db/migrations/index.ts`), no como
  ficheros `.sql`, para que viajen compiladas en `dist/` y el paquete instalado no
  dependa de rutas en disco. La v2 solo añade índices de rendimiento; su test parte de
  una base v1 con datos y FTS5.
- **Logging**: `createLogger({ level, format, stream, bindings, now })` escribe siempre en
  `stderr`; `text` es para humanos y `json` emite una línea por evento. El valor por
  defecto es `warn`/`text`. `--verbose` es global y el hook `preAction` lo copia al
  subcomando: no añadas un flag local duplicado.
- **Cierre de SQLite**: `closeDatabase()` ejecuta `PRAGMA optimize` como _best effort_ y
  nunca debe tapar el error original. `VACUUM` solo se lanza con
  `mm db optimize --vacuum`, porque bloquea y necesita espacio temporal.
- **`zod` v4**: `.default()` exige el tipo de _salida_; para objetos con subcampos
  opcionales usar `.prefault({})`.
- **Reproductor**: la plantilla de `players.*.args` debe incluir `{file}`. Se usa
  `spawn` con `shell: false`; nunca se construye una línea de shell. La TUI fuerza
  `detach` para no bloquearse, aunque `player.onLaunch` sea `foreground`.
- **Watch**: chokidar v6 no recibe globs; se filtran eventos con `isVideoFile` y se
  reindexa la biblioteca completa (incremental) para que los borrados también se poden.
- **Paquete npm interno**: el nombre sigue siendo `media-manager-cli` y el binario `mm`,
  pero npm solo se usa para construir y probar. `files` es una lista blanca: si añades
  documentación que el instalador copie o el README enlace, inclúyela. `prepack` compila
  `dist/` y `npm run smoke` prueba el tarball real.
- **Instaladores**: `install.sh` y `uninstall.sh` son POSIX `sh`, sin `eval` ni `sudo`.
  Cualquier cambio debe pasar `shellcheck`, `sh -n` y los tests de
  `tests/integration/installer/`. El checksum es obligatorio incluso con `--file`; no
  ejecutes el bundle antes de verificarlo ni confíes en las rutas del `install.json`
  instalado para borrar nada.
- **Release**: las releases se publican desde GitHub Releases (ADR 0009), sin `NPM_TOKEN`.
  `release.yml` y `release-please.yml` llaman a `release-artifacts.yml`, que descarga el
  Node oficial verificado, genera `.sha256` y SBOM, atesta la procedencia y sube los
  assets sin sobrescribir. No muevas un tag ya publicado; ver `docs/guides/release.md`.
- **`CHANGELOG.md` es generado por release-please** y está en `.prettierignore`: su formato
  (incluidos los dobles saltos de línea) lo fija la herramienta y `format:check` no debe
  reformatearlo.

## Estado actual y siguiente paso

Hitos del plan (ver `PLAN.md` §7):

| Hito | Contenido                                             | Estado   |
| ---- | ----------------------------------------------------- | -------- |
| M0   | Cimientos: tooling, hooks, CI                         | ✅ hecho |
| M1   | Núcleo de configuración (YAML + Zod)                  | ✅ hecho |
| M2   | Persistencia SQLite (migraciones, repositorios, FTS5) | ✅ hecho |
| M3   | Indexación y parseo (scanner + parsers)               | ✅ hecho |
| M4   | CLI headless + búsqueda                               | ✅ hecho |
| M5   | TUI (Ink)                                             | ✅ hecho |
| M6   | Reproductor mpv/vlc + modo watch                      | ✅ hecho |
| M7   | Robustez y rendimiento                                | ✅ hecho |
| M8   | Release v1.0.0                                        | ✅ hecho |
| M9   | Esquema v3, estado reproducción, doctor               | ✅ hecho |
| M10  | Reproducción con memoria (tracker IPC, TUI)           | ✅ hecho |
| M11  | Biblioteca sana y descubrimiento                      | ✅ hecho |
| M12  | macOS y experiencia v2                                | ✅ hecho |
| M13  | Release v2.0.1 (Linux x64/arm64 + SBOM)               | ✅ hecho |
| M14  | v2.1: Windows + auto-update + tags                    | ✅ hecho |
| M15  | Metadatos externos TMDB/TVDB                          | ✅ hecho |
| M16  | Config YAML metadatos + TVDB proveedor secundario     | ✅ hecho |
| M17  | Sincronización automática + metadatos con TTL         | ✅ hecho |
| M18  | TUI con metadatos visuales                            | ✅ hecho |
| M19  | Sincronización en segundo plano (watch metadatos)     | ✅ hecho |
| M20  | Auto-completado de shell (bash, zsh, fish)            | ✅ hecho |
| M21  | E2E interactivos con TTY real (node-pty)              | ✅ hecho |
| M22  | Mejorar cobertura de pruebas (watch, play, watcher)   | ✅ hecho |

M9 está completado: esquema v3 (playback_state, media_preferences), migración con backup,
repositorios playback/preferences, dominio de reproducción, comandos CLI (play --resume/restart/next,
continue, history, mark, favorite), mm doctor. Tests de migración y repositorios en verde.

M10 está completado: tracker IPC de mpv, sección "Continuar" en la TUI, y tests E2E con IPC falso y `--dry-run` completados.

M11 está completado: filtros de favoritos/en curso/vistos/no vistos en `mm list` y TUI, sección "Favoritos" en la barra lateral, benchmarks de consultas con estado de reproducción. Tests integrados y verificados en CI.

M12 está completado: CI con runners macOS, instalador POSIX con eliminación automática de cuarentena, install-test en Ubuntu y macOS, README documenta instalación vía `sh install.sh` y Gatekeeper.

M13 está completado: release v2.0.1 publicada en GitHub Releases con assets Linux x64/arm64, checksums SHA-256, SBOM CycloneDX y atestaciones de procedencia. El asset macOS arm64 quedó pendiente por limitación de capacidad de runners de GitHub Actions (exit code 127).

**M14 completado:** v2.1 (menor) con todos los sub-hitos:

- **M14b** — Windows (instalador PowerShell, .zip, named pipes IPC) ✅
- **M14c** — Auto-actualización (`mm self-update`) ✅
- **M14d** — Etiquetas/colecciones (migración v4, CLI) ✅

macOS sin assets oficiales en v2.1 (sin hardware macOS).

**M15 completado:** Metadatos externos TMDB/TVDB:

- Migración v5: tabla media_metadata
- metadataRepo: CRUD de metadatos externos
- MetadataService: llamadas a API de TMDB (búsqueda y detalles)
- CLI: mm metadata fetch/search/clear [--json]
- LibraryService: getMetadata, setMetadata, removeMetadata

**M17 completado:** Sincronización automática de metadatos:

- LibraryService.getOrFetchMetadata(): verifica TTL y refresca automáticamente
- MetadataService.search() y getDetails(): métodos genéricos multi-proveedor
- openLibrary() acepta config y logger para crear MetadataService
- context.ts y run.tsx pasan config y logger a openLibrary
- Tests actualizados para metadataService y metadataConfig

Todos los hitos M0–M17 están completados.

**M18 — TUI con metadatos visuales · completado**

- Panel de detalles en TUI muestra metadatos (título, año, sinopsis, calificación, fecha estreno, idioma)
- Mostrar información del proveedor (TMDB/TVDB)
- Si no hay metadatos, mostrar opción para obtenerlos desde la TUI
- Tecla `r` en detalles para refrescar metadatos
- Tecla `m` en listado para mostrar/ocultar el panel de detalles

**M19 — Sincronización en segundo plano · completado**

- Watch de metadatos al abrir TUI: verifica títulos con metadatos expirados
- Refrescar metadatos expirados al iniciar (automático, configurable vía `cacheTtlHours`)
- Manejo elegante de errores de red (catch blocks, TUI no se bloquea)
- Rate-limiting respetado: TMDB 40 req/10s, TVDB 10 req/10s (`src/core/metadata/rateLimiter.ts`)

**M20 — Auto-completado de shell · completado**

- `mm completion bash|zsh|fish` (ya existía en `src/cli.tsx` y `src/commands/completion.ts`)
- Documentación de instalación por shell en README.md (sección "Autocompletado de shell")

**M21 — E2E interactivos con TTY real · completado**

- `node-pty` integrado en `tests/e2e/tui.e2e.test.ts`
- Test de navegación por TUI con teclas reales
- Test de selección y reproducción desde TUI
- 30 tests E2E pasando

**M22 — Mejorar cobertura de pruebas · completado**

- Tests para launcher.ts, watcher.ts, watch.ts, service.ts (ya existían)
- Cobertura global: Statements 89.65 %, Branches 81.31 %, Functions 92.25 %, Lines 91.81 % (excluyendo test de estrés pre-existing en ARM64)
- Todos los umbrales de `vitest.config.ts` (85/85/80/85) se cumplen

## Reglas para el agente

- Escribe código en **inglés** (identificadores, comentarios técnicos) y la
  documentación de cara al usuario en **español**.
- Nada de `any`, nada de `as` sin motivo justificado, nada de estados globales.
- Cada función nueva va con su test; si el bug es curioso, el test lo documenta.
- Comenta **por qué**, no el qué obvious.
- Si hay que tomar una decisión con más de un camino válido, escribe un ADR en
  `docs/adr/` y anótala en el `PLAN.md`.
- No amplíes el alcance del hito en curso sin avisar; es un proyecto por hitos.
- Trocea el trabajo según la política de commits pequeños; no acumules cambios para un
  único commit final.
- Al terminar un hito: `npm run verify`, commit, push, PR, merge, y actualizar la
  tabla de estado de `PLAN.md` y del `README.md`.
- **No pasar al siguiente hito hasta que la PR del hito actual esté mergeada y en `main`.**
  Cada hito debe cerrar su PR y verificarse en CI antes de empezar el siguiente.
- **Revisión y merge automático de PR:** al crear la PR al finalizar un hito, el agente
  debe hacer una **preview/revisión completa** de la PR (diff, tests, CI, documentación)
  y, **si la CI está en verde y no hay bloqueadores**, proceder al **merge con squash**
  (`gh pr merge --squash --delete-branch`) y limpiar la rama remota.
- **Releases descriptivas:** al publicar una release (manual o via release-please), las
  notas deben ser **descriptivas y completas**, no solo una lista de commits. Deben
  incluir:
  - **Nuevas funcionalidades** (`Features`): qué se añade y para qué sirve.
  - **Cambios** (`Changes`): qué se modifica y por qué (breaking changes destacados).
  - **Correcciones** (`Bug Fixes`): qué fallaba y cómo se soluciona.
  - **Documentación**: qué se actualiza en docs/README/ADRs.
  - Usar el formato Keep a Changelog y agrupar por tipo (`### Features`, `### Changes`,
    `### Bug Fixes`, `### Documentation`). Evitar notas genéricas como "v2.0.0" sin
    contenido; release-please genera el CHANGELOG pero el agente debe revisar y
    completar las notas de la GitHub Release antes de publicarla.
- **Scripts de instalación en la misma PR:** al modificar la aplicación, si es
  necesario modificar los scripts de instalación (`install.sh`, `install.ps1` u otros
  instaladores), esos cambios se incluirán en la misma PR que los cambios de la
  aplicación, no en una PR separada.
- **README actualizado en la misma PR:** si los cambios requieren actualizar el README
  (nuevas funcionalidades, cambios de instalación, pasos adicionales, información que el
  usuario deba ver), se incluirá en la misma PR los cambios del README correspondientes.

## Reglas de tipado estricto y modelado de dominio

Estas reglas se aplican en **cada línea de código, refactorización o componente** que se escriba en el proyecto.

### 1. Tipado estricto y absoluto

- Todo debe estar tipado de forma explícita: argumentos de funciones, valores de retorno, variables, constantes y estructuras de datos.
- Queda **totalmente PROHIBIDO** el uso de `any`. Si un tipo es desconocido en tiempo de compilación, utiliza `unknown` y aplica Type Guards o aserciones de control (narrowing) antes de operar con él.
- Para datos dinámicos o respuestas de APIs externas, utiliza esquemas de validación en tiempo de ejecución (como Zod, Valibot o TypeBox) para garantizar la integridad de las entradas.
- Deshabilita cualquier asunción implícita (actúa como si `noImplicitAny`, `strictNullChecks` y `strict` estuvieran siempre activados en `tsconfig`).

### 2. Manejo de nulos y valores opcionales

- Trata de forma explícita los posibles valores `null` o `undefined`. No uses aserciones de no-nulo (`!`) a menos que exista una justificación matemática/lógica indiscutible en el contexto.
- Utiliza encadenamiento opcional (`?.`) y el operador de fusión nula (`??`) cuando corresponda.

### 3. Modelado de datos y dominio

- Prioriza el uso de `interface` para definir contratos de objetos/entidades expansibles y `type` para uniones, intersecciones, tuplas y tipos utilitarios.
- Define Union Types discriminadas (Discriminated Unions) para gestionar estados complejos o respuestas que puedan ser de éxito o error.
- Emplea `readonly` e inmutabilidad por defecto para prevenir mutaciones no deseadas en arreglos u objetos.

### 4. Control de errores

- No lances excepciones genéricas (`throw new Error(...)`). Crea clases de error personalizadas tipadas o utiliza tipos de resultado explícitos como `Result<T, E>`.
- En bloques `catch`, tipa el parámetro de error como `unknown` y valida su estructura antes de procesarlo.

### 5. Arquitectura y estilo de código

- Diseña funciones puras de responsabilidad única. Evita efectos secundarios no tipados.
- Utiliza nombres claros y descriptivos para interfaces, tipos y genéricos (`TData`, `TError` en lugar de una sola letra si hay contexto ambiguo).
- Exporta todos los tipos compartidos para que puedan ser reutilizados de forma consistente.

Si identificas código no tipado o ambiguo, corrígelo e infórmalo al usuario.
