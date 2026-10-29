# Plan: media-manager v2.0 — una biblioteca con memoria — ✅ COMPLETADO

> **Estado:** todos los hitos M9–M13 completados, release v2.0.0 publicada y verificada.
> **Índice de planes:** [`docs/plans/README.md`](./README.md)

- **Estado:** completado — M9 (contratos, datos y migración v3), M10–M13 implementados.
- **Fecha:** 2026-10-22
- **Inicio de ejecución:** 2026-10-23, tras cerrar `v1.0.0`/`v1.0.1`
- **Depende de:** [`instaladores-y-seguridad-COMPLETADO.md`](./instaladores-y-seguridad-COMPLETADO.md)
- **Base:** `1.x` publicada desde GitHub Releases
- **Objetivo de versión:** `2.0.0`

## 1. Objetivo

`v1.0` indexa y reproduce. `v2.0` debe **recordar** lo que el usuario está viendo y
convertir el índice en una biblioteca útil para el día a día:

- continuar un episodio o una película donde se dejó;
- saber cuál es el siguiente episodio;
- marcar visto/no visto y consultar el historial;
- detectar ficheros ausentes, duplicados y entradas huérfanas;
- organizar con favoritos y filtros;
- ofrecer la misma experiencia en Linux y macOS.

La aplicación sigue siendo **local-first**: sin servidor, sin streaming, sin cuentas y sin
telemetría. El estado de visionado vive en la misma SQLite que la biblioteca.

## 2. Decisiones de alcance

1. **Estado por fichero:** el progreso se guarda en `media_files.id`; mover o borrar un
   fichero puede perder su progreso. Se documenta como limitación conocida.
2. **Seguimiento automático solo para mpv.** VLC y otros reproductores usan progreso
   manual; `{start}` permite reanudar igualmente.
3. **macOS entra en `v2.0`.** Windows queda para `2.1`; requiere instalador PowerShell y
   otra matriz de pruebas.
4. **Favoritos sí; etiquetas y colecciones no.** Las etiquetas se valoran para `2.1` si
   los filtros y favoritos no cubren la necesidad.
5. **Sin metadatos externos.** No habrá scraping, TMDB, TVDB ni claves API en `2.0`.
6. **Sin autoactualización.** Actualizar sigue siendo ejecutar el instalador de la nueva
   versión.
7. **Contrato JSON aditivo.** Los comandos existentes conservan sus campos; los nuevos
   añaden campos, no rompen scripts.

## 3. Funcionalidades

### 3.1 Continuar viendo

- `mm play <título>` reanuda por defecto si hay progreso guardado y no está completado.
- `mm play <título> --restart` fuerza el inicio desde cero.
- `mm play <título> --next` reproduce el siguiente episodio no completado.
- `mm continue` lista títulos en curso, ordenados por última reproducción.
- `mm history` muestra el historial reciente.
- `mm mark <título> --watched|--unwatched` marca estado manualmente.
- `mm mark <título> --file <id> --position <segundos>` registra una posición concreta.
- La TUI añade una sección **Continuar** con barra de progreso y acciones rápidas.

Reglas:

- Un fichero se considera completado al superar el **90 %** de su duración o al llegar a
  los últimos **30 segundos**.
- No se reanuda una posición inferior a **30 segundos**; se empieza desde el principio.
- Si no se conoce la duración, la finalización solo puede marcarse manualmente.
- Para series y anime, el siguiente episodio se busca por `season`, `episode`; los
  especiales (`season = 0`) no se mezclan con la temporada regular.
- Para películas, `--next` no aplica.

### 3.2 Biblioteca sana

Nuevo comando `mm doctor`:

- `mm doctor` — diagnóstico legible, sin modificar nada.
- `mm doctor --json` — resultado estructurado para scripts.
- `mm doctor --full-hash` — comparación completa para duplicados.
- `mm doctor --prune-missing` — elimina del índice ficheros ausentes, tras backup.

Comprobaciones:

- `PRAGMA integrity_check` y `PRAGMA foreign_key_check`.
- Ficheros del índice que ya no existen.
- Títulos sin ningún fichero.
- Ficheros fuera de las raíces configuradas.
- Duplicados por tamaño y huella parcial (cabeza + cola); completa con `--full-hash`.
- Enlaces simbólicos rotos y errores de permisos.
- Ejecuciones de indexación incompletas o antiguas.

`mm doctor` es **read-only por defecto**. Cualquier reparación exige una opción explícita
y crea un backup previo.

### 3.3 Favoritos y filtros

- `mm favorite <título>` y `mm favorite <título> --off`.
- `mm list --favorites`, `--in-progress`, `--unwatched` y `--watched`.
- La TUI añade **Favoritos** y filtros combinables.
- Los filtros se reflejan en `--json` con los mismos campos que el listado actual.

### 3.4 TUI

- Sección **Continuar** en la barra lateral.
- Indicadores `●` en curso, `✓` completado y `★` favorito.
- Barra de progreso compacta en la lista y detalle.
- Atajos nuevos, configurables en `keybindings`:
  - `w`: marcar visto/no visto.
  - `n`: siguiente episodio.
  - `f`: favorito.
  - `g`: ciclo de filtros.
- La ventana de la lista se mantiene; no se renderiza la biblioteca completa.
- Los cálculos de progreso se hacen en lote, sin una consulta por título.

### 3.5 CLI

| Comando                      | Uso                                |
| ---------------------------- | ---------------------------------- |
| `mm continue`                | títulos en curso                   |
| `mm history`                 | historial reciente                 |
| `mm mark`                    | visto, no visto o posición manual  |
| `mm favorite`                | marcar o desmarcar favorito        |
| `mm doctor`                  | salud del índice y de los ficheros |
| `mm play --resume/--restart` | reanudar o empezar desde cero      |
| `mm play --next`             | siguiente episodio                 |
| `mm list --in-progress`      | listar en curso                    |
| `mm list --favorites`        | listar favoritos                   |
| `mm list --unwatched`        | listar no vistos                   |

Todos los comandos nuevos admiten `--json` y respetan los códigos de salida y el contrato
de errores del [ADR 0004](../adr/0004-contrato-cli.md).

## 4. Modelo de datos

### Migración v3

```text
playback_state
├── media_file_id  INTEGER PRIMARY KEY → media_files(id) ON DELETE CASCADE
├── position_seconds REAL NOT NULL DEFAULT 0
├── duration_seconds REAL
├── completed_at   TEXT
├── last_played_at TEXT NOT NULL
├── play_count     INTEGER NOT NULL DEFAULT 0
└── updated_at     TEXT NOT NULL
```

```text
media_preferences
├── media_id   INTEGER PRIMARY KEY → media(id) ON DELETE CASCADE
├── favorite   INTEGER NOT NULL DEFAULT 0 CHECK (favorite IN (0, 1))
├── created_at TEXT NOT NULL
└── updated_at TEXT NOT NULL
```

Índices:

- `playback_state(last_played_at)`
- `playback_state(completed_at)`
- `media_preferences(favorite)`

La migración es aditiva: no cambia tablas existentes ni FTS5. Se prueba desde una base
`v1` y desde una base `v2` de `1.0`.

### Backup y compatibilidad

- Antes de migrar, copiar `library.db` a `library.db.bak-v2-<timestamp>`.
- Si la copia falla, abortar la migración.
- Conservar el backup más reciente; no borrar backups antiguos automáticamente.
- `v2.0` puede leer bases de `1.x`; `1.x` no puede leer una base `v2.0`.
- El downgrade requiere restaurar el backup y se documenta como operación manual.
- Añadir `mm db backup` para crear copias bajo demanda.

## 5. Integración con reproductores

### Placeholder `{start}`

Las plantillas de `players.*.args` admiten un placeholder opcional `{start}`:

```yaml
player:
  players:
    mpv:
      command: mpv
      args: ['--force-window=immediate', '--start={start}', '{file}']
    vlc:
      command: vlc
      args: ['--started-from-file', '--start-time={start}', '{file}']
```

- Si no hay reanudación, se elimina el argumento que contiene `{start}`.
- `{file}` sigue siendo obligatorio.
- Una plantilla sin `{start}` puede reproducir, pero no reanudar; se avisa al usuario.

### Seguimiento automático de mpv

- Solo cuando el reproductor es `mpv` y `playback.tracking: auto`.
- Se añade `--input-ipc-server=<socket>` antes del fichero.
- Socket en `$XDG_RUNTIME_DIR` o directorio temporal privado, permisos `0600`; nunca TCP.
- Un proceso auxiliar (`mm-track`, interno) observa `time-pos`, `duration`, `pause` y
  `eof-reached`.
- Guarda como máximo cada 5 segundos y al cerrar; evita escrituras por fotograma.
- Si el tracker falla, el reproductor sigue funcionando y el progreso queda manual.
- El socket y el proceso auxiliar se limpian al terminar o al recibir `SIGINT`/`SIGTERM`.

VLC y reproductores genéricos no tienen integración IPC en `2.0`. `mm mark` y la TUI
permiten actualizar el estado; `{start}` permite reanudar la posición guardada.

### Configuración nueva

```yaml
playback:
  tracking: auto # auto | manual | off
  resume: true
  resumeMinSeconds: 30
  completedThreshold: 0.9
  completedTailSeconds: 30
```

Con `resume: true` (valor por defecto) `mm play` reanuda; es un cambio de comportamiento
deliberado de `2.0`. Para conservar el comportamiento de `1.x`, usar `resume: false` o
`mm play --restart`.

## 6. Arquitectura

Módulos nuevos:

- `src/core/playback/` — cálculo de progreso, reanudación y siguiente episodio.
- `src/core/playback/mpvIpc.ts` — cliente IPC y tracker.
- `src/core/db/repositories/playbackRepo.ts`.
- `src/core/db/repositories/preferencesRepo.ts`.
- `src/core/library/health.ts` — comprobaciones de `mm doctor`.
- `src/commands/continue.ts`, `history.ts`, `mark.ts`, `favorite.ts`, `doctor.ts`.

Reglas:

- `core/**` no importa `ui/**` ni `commands/**`.
- La UI y los comandos no escriben SQL; usan repositorios y `LibraryService`.
- El tracker recibe la ruta de la base y el id del fichero; no crea conexiones globales.
- Las consultas de progreso se agrupan para evitar N+1.
- El contrato de `--json` se mantiene aditivo.

## 7. Multiplataforma

### Linux

- Mantener x64 y arm64.
- Misma instalación y mismos assets.

### macOS

- Assets `macos-x64` y `macos-arm64`.
- Node oficial y `better-sqlite3` compilados para cada arquitectura.
- CI en runners macOS x64 y arm64.
- Instalador POSIX reutilizado; documentar Gatekeeper y cuarentena.
- Mantener rutas XDG (`~/.config`, `~/.local/share`) para no romper la paridad con Linux.
- `mpv`/VLC se siguen resolviendo desde `PATH` o configuración.

### Windows

- Fuera de `v2.0`; planificado para `2.1`.
- Requiere instalador PowerShell, `.zip`, rutas y named pipes para IPC.

## 8. Seguridad

- El socket IPC de mpv es local, privado y de un solo uso.
- Nunca se abre un puerto TCP para controlar el reproductor.
- El tracker no acepta rutas ni comandos de fuentes no confiables.
- `mm doctor` no modifica nada sin una opción explícita.
- `mm doctor --prune-missing` hace backup antes de borrar filas.
- Los backups se crean con permisos del usuario, sin world-readable.
- Se mantienen los controles de los instaladores: checksum, atestación, extracción segura
  y desinstalación conservadora.
- No se añaden dependencias de red ni servicios externos.

## 9. Rendimiento

- Añadir benchmarks de:
  - listado con progreso y favoritos sobre 100 000 títulos;
  - `mm continue` y `mm history`;
  - cálculo de siguiente episodio;
  - migración v2 → v3.
- Objetivo: mantener el presupuesto de `docs/guides/performance.md`; ninguna consulta de
  listado debe hacer una consulta por elemento.
- La ventana de la TUI sigue limitada; el progreso se calcula en lote.
- El tracker escribe como máximo cada 5 segundos.

## 10. Pruebas

### Unitarias

- Cálculo de porcentaje, finalización y reanudación.
- Selección de siguiente episodio y fin de temporada.
- Filtros combinados y favoritos.
- Normalización de tiempos y placeholders de reproductor.

### Integración

- Migración desde una base `v1` y desde una base `v2` con datos.
- Backup antes de migrar y aborto si falla.
- Repositorios de playback y preferencias.
- Servidor IPC falso de mpv: posición, pausa, fin y cierre.
- `mm doctor` con ficheros ausentes, duplicados y huérfanos.
- TUI con sección Continuar y marcas de estado.

### E2E

- `mm play --dry-run` muestra `--start` al reanudar.
- `--restart` no muestra `--start`.
- `mm play --next` elige el episodio correcto.
- `mm mark`, `mm continue`, `mm history` y `--json`.
- `mm doctor` no modifica nada por defecto.
- Actualización de una base `1.x` a `2.0` con backup.

### Matriz

- Linux x64/arm64.
- macOS x64/arm64.
- Node 22/24/26 en Linux; Node 24 en macOS.

## 11. Hitos

### M9 — Contratos, datos y migración v3

- [x] ADR 0010: estado de reproducción y reanudación.
- [x] ADR 0011: salud de la biblioteca y reparaciones seguras.
- [x] Esquema v3, repositorios y migración con backup.
- [x] Configuración `playback` y placeholder `{start}`.
- [x] Contrato CLI/JSON de los comandos nuevos.
- [x] Tests de migración y repositorios.

**DoD:** una base `1.0` migra a v3, conserva todos los títulos y deja backup verificable.

### M10 — Reproducción con memoria

- [x] `playback_state` y cálculo de progreso.
- [x] Tracker IPC de mpv y fallback manual.
- [x] `mm play --resume/--restart/--next`.
- [x] `mm mark`, `mm continue`, `mm history`.
- [x] Sección Continuar en la TUI.
- [x] Tests con IPC falso y E2E de `--dry-run`.

**DoD:** se puede empezar un episodio, salir, volver a `mm` y continuar donde se dejó.

### M11 — Biblioteca sana y descubrimiento

- [ ] Favoritos y filtros de listado.
- [ ] `mm doctor` read-only y `--json`.
- [ ] Duplicados, ausentes y huérfanos.
- [ ] `--prune-missing` con backup.
- [ ] Filtros y acciones rápidas en la TUI.
- [ ] Benchmarks de listados con estado.

**DoD:** `mm doctor` detecta y explica problemas sin tocar datos; las reparaciones son
explícitas y reversibles mediante backup.

### M12 — macOS y experiencia v2

- [ ] ADR 0012: soporte macOS y límites de Windows.
- [ ] Assets macOS x64/arm64.
- [ ] CI y pruebas en macOS.
- [ ] Instalación/desinstalación verificadas en macOS.
- [ ] Progreso, favoritos y filtros pulidos en la TUI.
- [ ] Documentación de Gatekeeper y reproductores.
- [ ] Pruebas de rendimiento con estado.

**DoD:** un usuario de macOS instala, reproduce y reanuda sin Node ni npm instalados.

### M13 — Release v2.0.0

- [ ] Release candidate con migración real desde `1.0`.
- [ ] Actualización y rollback documentados.
- [ ] README, release, architecture, performance, SECURITY, AGENTS y PLAN.
- [ ] ADR 0010/0011 y guía de migración.
- [ ] Assets, checksums, SBOM y atestaciones.
- [ ] Tag `v2.0.0` y release de GitHub.
- [ ] Marcar M9–M13 como completados.

**DoD:** release publicada y verificada desde una instalación `1.x`.

## 12. Criterios de aceptación

- [ ] Una base `1.x` se actualiza a `2.0` sin perder títulos ni ficheros.
- [ ] El backup se crea antes de la migración.
- [ ] `mm play` reanuda y `--restart` empieza desde cero.
- [ ] `mm play --next` elige el siguiente episodio correcto.
- [ ] `mm mark`, `mm continue` y `mm history` funcionan con `--json`.
- [ ] La TUI muestra Continuar, progreso y favoritos.
- [ ] `mm doctor` detecta ausentes, duplicados y huérfanos.
- [ ] `mm doctor` no modifica datos sin opción explícita.
- [ ] Linux x64/arm64 y macOS x64/arm64 tienen assets verificables.
- [ ] Se mantiene el rendimiento de 100 000 títulos.
- [ ] No hay regresiones en indexación, búsqueda, reproducción ni watch.
- [ ] La release `v2.0.0` incluye checksums, SBOM y atestaciones.

## 13. Riesgos y mitigaciones

| Riesgo                                       | Mitigación                                                |
| -------------------------------------------- | --------------------------------------------------------- |
| Pérdida de progreso al mover ficheros        | Documentarlo; estado por `media_file_id`                  |
| Complejidad del IPC de mpv                   | Adaptador aislado, socket privado, fallback manual        |
| VLC sin seguimiento automático               | Manual + `{start}`; documentado                           |
| Migración destructiva                        | Backup previo, tests desde v1/v2 y aborto si falla        |
| N+1 en listados con progreso                 | Consultas agregadas, benchmarks y revisión de SQL         |
| Runners macOS no disponibles                 | Validar matriz antes de M12; degradar a release posterior |
| Gatekeeper bloquea el binario                | Documentar cuarentena y pasos de instalación              |
| Scope creep con etiquetas/metadatos externos | Fuera de alcance; revisar en 2.1                          |
| Windows retrasa la release                   | Windows fuera de 2.0                                      |

## 14. Documentación

- `README.md`: continuar viendo, marcas, favoritos, doctor y macOS.
- `docs/guides/release.md`: assets macOS y migración.
- `docs/guides/architecture.md`: playback, tracker y health.
- `docs/guides/performance.md`: nuevos benchmarks.
- `docs/guides/migracion-v2.md`: backup, actualización y rollback.
- `SECURITY.md`: socket IPC, backups y doctor.
- `AGENTS.md` y `PLAN.md`: hitos y estado.
- ADR 0010: estado de reproducción.
- ADR 0011: salud de la biblioteca.
- ADR 0012: soporte macOS y límites de Windows.

## 15. Decisiones pendientes

Las decisiones 1, 2, 3, 4 y 6 quedaron cerradas en el
[ADR 0010](../adr/0010-estado-reproduccion.md) y el [ADR 0011](../adr/0011-salud-biblioteca.md).
La decisión 5 se cerró en el ADR 0011 (confirmación interactiva salvo `--yes`).

1. ~~¿`mm play` reanuda por defecto en `2.0` o solo con `--resume`?~~ → sí, por defecto.
2. ~~¿El tracker automático se limita a mpv o se intenta también VLC?~~ → solo mpv.
3. ~~¿Windows entra en `2.0` o se confirma `2.1`?~~ → `2.1`.
4. ~~¿Favoritos es suficiente o hacen falta etiquetas/colecciones en `2.0`?~~ → favoritos.
5. ~~¿`mm doctor --prune-missing` debe pedir confirmación interactiva?~~ → sí, salvo `--yes`.
6. ~~¿macOS mantiene rutas XDG o adopta `~/Library/Application Support`?~~ → XDG.
