# Plan: Mejoras continuas post-M22 (Fase 2)

**Estado:** ✅ Completado — todos los ítems implementados en código y verificados con `npm run verify` (599 tests pasando).

**Objetivo:** Implementar mejoras y funcionalidades adicionales tras completar todos los hitos M0–M22 y los 10 ítems del plan post-M22. Cada ítem se ejecuta en su propia rama, se verifica con `npm run verify`, se abre PR y se mergea con squash antes de pasar al siguiente.

## Contexto

Todos los hitos M0–M22 están completados y mergeados en `main`. El plan post-M22 anterior (`docs/plans/mejoras-pendientes-post-m22.md`) está marcado como completado (PR #122). El proyecto está estable: `npm run verify` pasa verde (lint, format, typecheck, unit, integration, e2e).

Este plan documenta la siguiente ola de mejoras, ordenadas por valor e impacto. No es obligatorio implementarlas todas; se pueden seleccionar por prioridad.

---

## Ítems del plan

### Fase A — Calidad y testing (recomendado empezar aquí)

| #   | Área  | Descripción                                                                                                                                                  | Prioridad | Esfuerzo | Rama sugerida               |
| --- | ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------- | -------- | --------------------------- |
| A1  | Tests | Tests de contrato para `mm doctor --json`: `migrated: false` (BD sin migrar) y salida `text` (no JSON).                                                      | Alta      | Bajo     | `test/doctor-contrato`      | ✅  |
| A2  | Tests | Añadir tests para ramas no cubiertas en `watcher.ts`, `mpvTracker.ts`, `size.ts`, `open.ts` para subir cobertura de branches por encima del 85 % con margen. | Media     | Bajo     | `test/coverage-branches`    | ✅  |
| A3  | Tests | Tests E2E con TTY real para `mm list --export json\|csv` y `mm index --folder`.                                                                              | Media     | Medio    | `test/e2e-export-folder`    | ✅  |
| A4  | Tests | Tests de resiliencia de red en `MetadataService`: forzar 429, timeout y errores de red; verificar que la TUI no se bloquea.                                  | Media     | Medio    | `test/metadata-resiliencia` | ✅  |

### Fase B — Indexador y watcher

| #   | Área      | Descripción                                                                                           | Prioridad | Esfuerzo   | Rama sugerida                |
| --- | --------- | ----------------------------------------------------------------------------------------------------- | --------- | ---------- | ---------------------------- |
| B1  | Indexador | `mm index --folder`: limitar `scanLibrary` al subdirectorio pedido en lugar de recorrer toda la raíz. | Media     | Medio      | `feat/indexer-carpeta-real`  | ✅  |
| B2  | Watcher   | Detectar eventos `rename`/`move` de chokidar y actualizar `file_path` en lugar de borrar+insertar.    | Media     | Medio-Alto | `feat/watcher-renombrados`   | ✅  |
| B3  | Config    | Permitir recargar `library.exclude` sin reiniciar `mm` (watch de config).                             | Baja      | Bajo       | `feat/config-exclude-reload` | ✅  |

### Fase C — Metadatos y caché

| #   | Área      | Descripción                                                                                  | Prioridad | Esfuerzo | Rama sugerida                         | Estado |
| --- | --------- | -------------------------------------------------------------------------------------------- | --------- | -------- | ------------------------------------- | ------ |
| C1  | Metadatos | Fallback de póster local: si falla la red, usar el último póster cacheado o uno por defecto. | Media     | Bajo     | `feat/metadatos-poster-fallback`      | ✅     |
| C2  | Metadatos | Añadir proveedor secundario (OMDB, AniList o fanart.tv) como fallback de TMDB/TVDB.          | Baja      | Medio    | `feat/metadatos-proveedor-secundario` | ✅     |
| C3  | Metadatos | Comando `mm metadata cache --clear` y TTL para imágenes no usadas en 30 días.                | Baja      | Bajo     | `feat/metadatos-cache-limpiar`        | ✅     |

### Fase D — TUI

| #   | Área | Descripción                                                                                         | Prioridad | Esfuerzo | Rama sugerida                 | Estado |
| --- | ---- | --------------------------------------------------------------------------------------------------- | --------- | -------- | ----------------------------- | ------ |
| D1  | TUI  | Filtros rápidos con teclas: letras para filtrar por tipo (serie/anime/película) sin entrar en menú. | Baja      | Bajo     | `feat/tui-filtros-teclas`     | ✅     |
| D2  | TUI  | Vista compacta/densa alternativa a la vista con portada para bibliotecas grandes.                   | Baja      | Medio    | `feat/tui-vista-compacta`     | ✅     |
| D3  | TUI  | Selección múltiple y acciones en lote (marcar varios como visto/favorito/eliminar).                 | Media     | Medio    | `feat/tui-seleccion-multiple` | ✅     |
| D4  | TUI  | Soporte de ratón: clic en filas y botones de la TUI.                                                | Baja      | Bajo     | `feat/tui-raton`              | ✅     |

### Fase E — CLI y comandos

| #   | Área | Descripción                                                                         | Prioridad | Esfuerzo | Rama sugerida                  | Estado |
| --- | ---- | ----------------------------------------------------------------------------------- | --------- | -------- | ------------------------------ | ------ |
| E1  | CLI  | Comandos `mm db backup` y `mm db restore` para backup/restauración de la BD SQLite. | Media     | Medio    | `feat/cli-db-backup-restore`   | ✅     |
| E2  | CLI  | `mm config check`: validar el YAML de configuración sin ejecutar subcomandos.       | Baja      | Bajo     | `feat/cli-config-check`        | ✅     |
| E3  | CLI  | `--dry-run` en operaciones destructivas (`prune`, `delete`, `mark`).                | Media     | Bajo     | `feat/cli-dry-run-destructivo` | ✅     |

### Fase F — Observabilidad y doctor

| #   | Área     | Descripción                                                                                                | Prioridad | Esfuerzo | Rama sugerida           |
| --- | -------- | ---------------------------------------------------------------------------------------------------------- | --------- | -------- | ----------------------- |
| F1  | Doctor   | `mm doctor --full`: uso de disco, versión de mpv/vlc, conectividad TMDB/TVDB, tamaño de caché de pósteres. | Media     | Bajo     | `feat/doctor-full`      | ✅  |
| F2  | Logs     | Opción `--log-file` con rotación de logs (actualmente solo stderr).                                        | Baja      | Medio    | `feat/logs-archivo`     | ✅  |
| F3  | Métricas | Contadores simples: indexaciones, errores de metadatos, tiempo medio de consulta.                          | Baja      | Medio    | `feat/metricas-basicas` | ✅  |

### Fase G — Empaquetado y distribución

| #   | Área         | Descripción                                                                   | Prioridad | Esfuerzo | Rama sugerida               |
| --- | ------------ | ----------------------------------------------------------------------------- | --------- | -------- | --------------------------- |
| G1  | Distribución | `Dockerfile` multi-stage con Node oficial y `mm` instalado.                   | Media     | Medio    | `feat/docker-image`         | ✅  |
| G2  | Distribución | Fórmulas Homebrew (macOS) y winget (Windows).                                 | Baja      | Medio    | `feat/instaladores-native`  | ✅  |
| G3  | Releases     | Verificar que el SBOM CycloneDX se adjunta a cada release en GitHub Releases. | Media     | Bajo     | `ci/release-sbom-verificar` | ✅  |

### Fase H — Internacionalización (i18n)

| #   | Área | Descripción                                                                           | Prioridad | Esfuerzo | Rama sugerida  | Estado |
| --- | ---- | ------------------------------------------------------------------------------------- | --------- | -------- | -------------- | ------ |
| H1  | i18n | Añadir soporte de idiomas (`en` y `es`) en la TUI; código en inglés, UI configurable. | Baja      | Medio    | `feat/i18n-ui` | ✅     |

### Fase I — Seguridad

| #   | Área      | Descripción                                                   | Prioridad | Esfuerzo | Rama sugerida             |
| --- | --------- | ------------------------------------------------------------- | --------- | -------- | ------------------------- |
| I2  | Seguridad | Añadir `npm audit` y `dependabot` como paso bloqueante en CI. | Media     | Bajo     | `ci/auditoria-bloqueante` |

---

## Orden recomendado de implementación

Todos los ítems del plan están completados:

1. **A1–A4** — Testing (doctor, cobertura, export/folder, resiliencia) ✅ completados
2. **E1, E2, E3** — CLI (db backup/restore, config check, dry-run) ✅ completados
3. **F1, F2, F3** — Observabilidad (doctor --full, --log-file, métricas) ✅ completados
4. **B1** — `mm index --folder` real ✅ completado
5. **G1, G2, G3** — Distribución (Docker, Homebrew/winget, SBOM) ✅ completados
6. **B2** — Watcher rename/move ✅ completado (PR #161)
7. **B3** — Config exclude reload ✅ completado (PR #163)
8. **D1–D4** — TUI mejoras (filtros, compacto, selección múltiple, ratón) ✅ completado (PR #165)
9. **H1** — i18n (infraestructura de traducciones) ✅ completado (PR #167)

No es necesario implementar todos los ítems. Cada uno es independiente y se puede aceptar o rechazar individualmente.

> **Nota:** El ítem **I1 (firmado GPG de commits)** se ha pospuesto indefinidamente, ya que requiere claves GPG privadas del mantenedor (infraestructura, no código). Si en el futuro se dispone de esas claves, se puede reabrir como un ítem nuevo.

---

## Estrategia de trabajo (obligatorio, del AGENTS.md)

1. **Una rama por ítem**: `feat/…`, `fix/…`, `test/…` o `ci/…` según corresponda.
2. **Commits con Conventional Commits y scope del proyecto**:
   `feat(indexer): soporta el patrón 1x02`
   Scopes válidos: `config`, `db`, `indexer`, `parser`, `player`, `ui`, `cli`, `deps`, `ci`, `docs`, `test`, `release`.
3. **`npm run verify` debe pasar antes de cada push** (lo comprueba el hook de pre-push; el de pre-commit ejecuta lint-staged y typecheck).
4. **`git push -u origin <rama>` y PR a `main`** con `gh pr create`.
5. Revisar que la CI de la PR está en verde y **mergear con squash**: `gh pr merge <n> --squash --delete-branch`.
6. `main` queda siempre desplegable: cada commit compila, pasa lint y tiene tests.

### Política de commits pequeños y atómicos

- Un commit representa **una sola intención revisable**, no trabajo acumulado.
- Objetivo: cerrar un commit cada **5–10 archivos** o **200–300 líneas** de un mismo cambio coherente, en cuanto sus tests estén en verde.
- Límite: no superar **15 archivos ni 500 líneas modificadas** por commit.
- Código y tests del mismo comportamiento van juntos.
- Documentación, formatting masivo y dependencias van en commits separados.
- Antes de commitear revisar `git diff --stat`, `git diff --check` y `git status`.
- Nunca `git add -A` si hay cambios mezclados; usar `git add <rutas>` o `git add -p`.

### Reglas de arquitectura (no se rompen)

```
ui/  commands/  ──▶  core/  ──▶  shared/
                     └──▶ core/db/
```

1. `core/**` **nunca** importa desde `ui/**` ni `commands/**`.
2. `ui/**` y `commands/**` **nunca** escriben SQL: acceden a datos por repositorios.
3. `core/db/` habla con SQLite; nadie más importa `better-sqlite3`.
4. `shared/**` no depende de nada del proyecto (solo librerías estándar).
5. El acceso a datos se **inyecta** (DIP): `openDatabase()` devuelve la instancia y los repositorios la reciben como parámetro. Nada de singletons globales.

### Reglas de código

- Escribir código en **inglés** (identificadores, comentarios técnicos) y documentación de cara al usuario en **español**.
- Nada de `any`, nada de `as` sin motivo justificado, nada de estados globales.
- Cada función nueva va con su test; si el bug es curioso, el test lo documenta.
- Comentar **por qué**, no el qué obvio.
- Si hay más de un camino válido, escribir un ADR en `docs/adr/` y anotarlo en `PLAN.md`.

---

## Criterios de aceptación (DoD) por ítem

- [ ] Código en inglés, sin `any`, sin `as` sin justificación, sin estados globales.
- [ ] Cada función nueva tiene su test.
- [ ] `npm run lint`, `npm run format:check`, `npm run typecheck` pasan.
- [ ] `npm run test` pasa (unit + integration + e2e).
- [ ] Commit atómico con Conventional Commits y scope del proyecto.
- [ ] PR mergeado con squash en `main`, CI verde, rama borrada.
- [ ] Si el ítem cambia comportamiento de usuario, actualizar `README.md` y/o docs correspondientes.

---

## Notas

- No ampliar el alcance de un ítem sin avisar; es un proyecto por hitos.
- No pasar al siguiente ítem hasta que el PR del anterior esté mergeado y en `main`.
- Al finalizar un hito: `npm run verify`, commit, push, PR, merge, y actualizar la tabla de estado de este plan y del `README.md`.
- Este plan vive en `docs/plans/mejoras-continuas-fase2.md` y se actualiza a medida que los ítems se completan.
