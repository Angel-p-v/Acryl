# Arquitectura

`media-manager` se organiza en cuatro capas con dependencias que apuntan **hacia
dentro**. La regla principal es que la lógica de negocio no sabe que existe una
terminal: eso permite testearla en milisegundos y reutilizarla desde cualquier
interfaz.

```
┌──────────────────────────────┐
│  ui/        commands/        │   Presentación: Ink (TUI) y CLI headless
├──────────────────────────────┤
│  core/                        │   Dominio: config, db, indexer, library, player
├──────────────────────────────┤
│  core/db/                     │   Persistencia: SQLite + repositorios
├──────────────────────────────┤
│  shared/                      │   Utilidades transversales
└──────────────────────────────┘
```

## Reglas de dependencia

| Regla                                               | Cómo se verifica                                                    |
| --------------------------------------------------- | ------------------------------------------------------------------- |
| `core/**` no importa de `ui/**` ni de `commands/**` | Test de arquitectura (hito M5)                                      |
| `ui/**` y `commands/**` no escriben SQL             | Solo se accede a datos por repositorios                             |
| `shared/**` no depende de nada del proyecto         | Solo librerías estándar                                             |
| El acceso a datos se inyecta (DIP)                  | `openDatabase()` devuelve la instancia; los repositorios la reciben |

## Capas

### `src/shared/`

Errores tipados (`AppError` y derivados), normalización de errores para la CLI
(`describeError` / `formatErrorWithCode`), logger estructurado y utilidades de rutas.
El logger soporta niveles (`silent`…`debug`), formato `text` o `json` (una línea por
evento), campos estructurados y `child()` para añadir contexto sin mutar el padre.

### `src/core/config/`

Esquema Zod (`schema.ts`), valores por defecto (`defaults.ts`), fusión profunda
(`merge.ts`), rutas XDG (`paths.ts`) y carga/validación (`loader.ts`).

Precedencia de la configuración: `defaults → archivo YAML → variables de entorno`.

### `src/core/db/`

Conexión SQLite (WAL, claves foráneas), migraciones versionadas y repositorios
`mediaRepo` / `fileRepo`. Se implementa en el hito M2. La migración v2 (M7) añade
índices de rendimiento para listados y mantenimiento sin tocar los datos.

`maintenance.ts` encapsula `PRAGMA optimize` y `VACUUM`; `size.ts` calcula el tamaño del
fichero principal más WAL/SHM. `closeDatabase()` ejecuta una optimización _best effort_
antes de cerrar, mientras que la compactación solo se lanza con `mm db optimize --vacuum`.

### `src/core/indexer/`

Recorrido del árbol de ficheros, clasificación del contenido y parsers de nombres
(`S01E02`, `1x02`, títulos de película). Hito M3. `watcher.ts` añade el modo _watch_:
filtra eventos de vídeo, agrupa ráfagas con debounce y reindexa de forma incremental para
detectar altas, cambios y borrados (M6).

### `src/core/library/`

Casos de uso de consulta implementados en el hito M4: listar con filtros y paginación,
buscar con FTS5, resolver un id o un título con sus ficheros y agregar estadísticas del
índice. La capa de presentación no escribe SQL: consume `LibraryService`.

### `src/core/player/`

Lanzamiento de reproductores externos implementado en el hito M6:

- `launcher.ts` construye el plan a partir de la plantilla `{file}`, elige el reproductor
  configurado y comprueba que el binario exista antes de lanzarlo. Usa siempre
  `spawn` con `shell: false` y argumentos separados.
- En modo `detach` el hijo se lanza con stdio ignorado y `unref()`, de modo que sobrevive
  al cierre de la TUI; en `foreground` hereda stdio y `wait()` espera su salida.
- `selection.ts` resuelve qué fichero se reproduce: el primero ordenado por temporada y
  episodio, o el id indicado con `--file`.
- La TUI fuerza `detach` para no bloquearse; la CLI respeta `player.onLaunch` y permite
  `--foreground`.

### `src/ui/`

Interfaz de terminal con Ink implementada en el hito M5:

- `App.tsx` mantiene el estado local de navegación (vista, sección, selección y búsqueda)
  y recibe un `LibraryService`; no conoce SQLite.
- `hooks/` resuelve la configuración y los atajos (`useConfig`), consulta la biblioteca
  (`useLibrary`, `useMediaDetails`) y despacha el teclado (`useKeymap`).
- `components/` y `views/` son presentacionales: `Library`, `Search` y `Settings` componen
  la cabecera, la barra lateral, la lista, el detalle, la barra de estado y la ayuda.
- `screen.ts` entra y sale de la pantalla alternativa; `run.tsx` abre la biblioteca, monta
  la TUI y garantiza que la conexión se cierra al salir.

### `src/commands/`

Subcomandos de la CLI que no dependen de Ink: `config`, `index`, `list`, `search`,
`show`, `play`, `watch`, `db stats` y `db optimize`. Los códigos de salida y el contrato
de `--json` están fijados en el [ADR 0004](../adr/0004-contrato-cli.md); los errores van a
`stderr`. Las decisiones de reproducción y _watch_ están en el
[ADR 0006](../adr/0006-reproductor-y-watch.md).

`context.ts` carga la configuración, crea el logger y abre/cierra la biblioteca en un
único punto. `errorHandler.ts` es el gestor global de errores: imprime `[CÓDIGO] mensaje`
y, con `--verbose`, la pila. Las decisiones de logging, códigos y mantenimiento están en
el [ADR 0007](../adr/0007-logging-errores-y-mantenimiento.md).

## Flujo de datos de una indexación

```
config.yaml
   │  loader (YAML → Zod → AppConfig)
   ▼
AppConfig ──▶ indexer/scanner ──▶ parsers ──▶ mediaRepo ──▶ SQLite
```

## Flujo de datos de una consulta

```
mm search "sever" ──▶ commands/search ──▶ LibraryService.search
                                              │
                                              ▼
                                   mediaRepo.search (FTS5)
                                              │
                                              ▼
                                   media_fts MATCH + bm25 → SQLite
```

## Flujo de reproducción

```
mm play "sever" ──▶ LibraryService.find ──▶ fichero (primero o --file)
                                              │
                                              ▼
                           createLaunchPlan (sustituye {file})
                                              │
                                              ▼
                     spawn(command, args, { shell: false })
                         │ detach                    │ foreground
                         ▼                           ▼
              stdio ignore + unref          stdio inherit + await close
```

## Flujo del modo _watch_

```
chokidar (raíces) ──▶ evento add/change/unlink
                              │ debounce 400 ms
                              ▼
                    indexLibrary (incremental)
                              │
                              ▼
                 repositorios → SQLite (altas y borrados)
```

`mm list`, `mm show` y `mm db stats` recorren el mismo camino: la capa de comandos solo
formatea la salida (humana o JSON) y nunca escribe SQL.

## Logging y errores

El nivel y el formato se configuran en `logging` (YAML) o con `MM_LOG_LEVEL` /
`MM_LOG_FORMAT`. `--verbose` es global y fuerza `debug`; Commander lo copia al subcomando
para que el flag funcione antes o después del comando. Los logs van a `stderr` en texto o
en una línea JSON por evento; `stdout` queda reservado para la salida de usuario y
`--json`.

Los errores de ejecución se normalizan con `describeError()` y los imprime un único
manejador global en `stderr` con el formato `[CÓDIGO] mensaje`. La pila solo aparece con
`--verbose`. Los códigos de salida siguen siendo `0`/`1`/`2`.

El presupuesto de rendimiento y la metodología de los benchmarks están en
[`performance.md`](./performance.md).

## Decisiones registradas

Las decisiones con más de un camino válido se documentan en [`adr/`](../adr).
