# ADR 0010 — Estado de reproducción, reanudación y contrato de `mm play`

- **Estado:** aceptada
- **Fecha:** 2026-10-23
- **Plan:** [`version-2.0.md`](../plans/version-2.0.md) (M9/M10)
- **Relacionada con:** [ADR 0004](./0004-contrato-cli.md), [ADR 0006](./0006-reproductor-y-watch.md)

## Contexto

`v1.x` indexa y reproduce, pero no recuerda nada. El objetivo de `2.0` es que el usuario
pueda dejar un episodio a medias, cerrar el portátil y retomarlo con `mm play`, además de
consultar qué está viendo y qué ha terminado.

Hay que decidir dónde vive ese estado, cómo se calcula la finalización, qué pasa si el
fichero se mueve y cómo se integra con reproductores que no ofrecen un canal de control.
También hay que fijar el contrato de los comandos nuevos para no romper scripts.

## Decisión

### Modelo de datos (esquema v3)

El estado vive en la misma SQLite que la biblioteca, en dos tablas aditivas:

```text
playback_state
├── media_file_id  INTEGER PRIMARY KEY → media_files(id) ON DELETE CASCADE
├── position_seconds REAL NOT NULL DEFAULT 0
├── duration_seconds REAL
├── completed_at   TEXT
├── last_played_at TEXT NOT NULL
├── play_count     INTEGER NOT NULL DEFAULT 0
└── updated_at     TEXT NOT NULL

media_preferences
├── media_id   INTEGER PRIMARY KEY → media(id) ON DELETE CASCADE
├── favorite   INTEGER NOT NULL DEFAULT 0 CHECK (favorite IN (0, 1))
├── created_at TEXT NOT NULL
└── updated_at TEXT NOT NULL
```

- **Estado por `media_file_id`**, no por título: un mismo título puede tener varios
  ficheros (calidades, episodios) y cada uno tiene su posición.
- Consecuencia aceptada: mover o renombrar un fichero crea una fila nueva y se pierde el
  progreso anterior. Se documenta como limitación conocida; no se intenta reconciliar por
  tamaño o huella.
- Los tiempos en segundos son `REAL`; las fechas son ISO-8601 en UTC (`...Z`) generadas
  por el dominio, no por SQLite, para poder inyectar un reloj en los tests.
- `ON DELETE CASCADE` evita estados huérfanos cuando el indexador poda un fichero o un
  título.
- Índices: `playback_state(last_played_at)`, `playback_state(completed_at)` y
  `media_preferences(favorite)`.

### Migración y backup

- La migración v3 es aditiva: no altera `media`, `media_files`, `index_runs` ni FTS5.
- Antes de aplicar **cualquier** migración pendiente sobre una base existente se crea una
  copia `library.db.bak-v<versión-anterior>-<timestamp>`, con permisos `0600`.
- La copia se hace tras un `wal_checkpoint(TRUNCATE)` para que el fichero principal
  contenga todo lo confirmado. Si la copia falla, la migración **aborta**.
- Solo se conserva la copia más reciente de cada migración; no se borran backups antiguos
  automáticamente.
- Una base `1.x` se puede abrir con `2.0`; una base `2.0` no se puede abrir con `1.x`. El
  downgrade es manual: restaurar el backup y volver a instalar la versión anterior.
- `mm db backup` crea una copia bajo demanda con el mismo formato (`bak-manual-<timestamp>`).

### Reglas de progreso

- **Finalizado** si hay duración y se cumple `position >= duration * completedThreshold`
  (0,9 por defecto) **o** `duration - position <= completedTailSeconds` (30 s).
- **Posición guardada:** se limita al rango `[0, duration]` cuando la duración se conoce.
- **Reanudación:** solo si `playback.resume` es `true`, el estado no está completado y
  `position >= playback.resumeMinSeconds` (30 s). Por debajo se empieza desde cero.
- Sin duración conocida, la finalización solo puede marcarse a mano (`mm mark --watched`
  o `w` en la TUI).
- Reproducir incrementa `play_count` una vez por lanzamiento, no por actualización de
  posición.
- `completed_at` se limpia al marcar como no visto o al reanudar manualmente con
  `mm mark --position`. La posición no se borra salvo `--unwatched`, que la deja en 0.

### Seguimiento automático

- Solo para **mpv** y con `playback.tracking: auto`. Se añade
  `--input-ipc-server=<socket>` antes del fichero.
- El socket vive en `$XDG_RUNTIME_DIR` o en un directorio temporal privado, con permisos
  `0600`; nunca se abre TCP.
- Un proceso auxiliar interno observa `time-pos`, `duration`, `pause` y `eof-reached`.
  Escribe como máximo cada 5 segundos y al cerrar; nunca por fotograma.
- Si el tracker falla, el reproductor sigue funcionando y el progreso queda manual. El
  fallo se registra como aviso, no como error de reproducción.
- VLC y reproductores genéricos no tienen integración IPC en `2.0`: usan `{start}` para
  reanudar y `mm mark`/la TUI para actualizar el estado.

### Placeholder `{start}`

- `players.*.args` admite `{start}` además de `{file}`.
- Al reanudar, cada aparición se sustituye por los segundos enteros de la posición.
- Si no hay reanudación, se elimina el argumento completo que contiene `{start}`.
- `{file}` sigue siendo obligatorio; una plantilla sin `{start}` reproduce, pero no puede
  reanudar. `mm play` avisa por `stderr` cuando hay posición guardada y el reproductor no
  admite reanudación.
- El valor es un número (`--start=90`), nunca texto libre: no se construye una línea de
  shell.

### Configuración nueva

```yaml
playback:
  tracking: auto # auto | manual | off
  resume: true
  resumeMinSeconds: 30
  completedThreshold: 0.9
  completedTailSeconds: 30
```

`resume: true` es un cambio de comportamiento deliberado de `2.0`. Para conservar el
comportamiento de `1.x`: `resume: false` o `mm play --restart`.

### Contrato CLI y JSON

| Comando                                         | Efecto                                                |
| ----------------------------------------------- | ----------------------------------------------------- |
| `mm play <título>`                              | reanuda si procede; si no, empieza desde el principio |
| `mm play <título> --resume`                     | fuerza reanudar (error si no hay posición reanudable) |
| `mm play <título> --restart`                    | ignora la posición guardada                           |
| `mm play <título> --next`                       | siguiente episodio no completado; error en películas  |
| `mm mark <título> --watched`                    | marca completado                                      |
| `mm mark <título> --unwatched`                  | marca no visto y posición 0                           |
| `mm mark <título> --file <id> --position <s>`   | guarda una posición concreta (no completado)          |
| `mm continue [--limit N]`                       | títulos en curso, por última reproducción             |
| `mm history [--limit N]`                        | historial reciente, completados incluidos             |
| `mm favorite <título>` / `--off`                | marca o desmarca favorito                             |
| `mm list --favorites/--in-progress/--unwatched` | filtra el listado                                     |

Reglas del contrato:

- `--resume` y `--restart` son mutuamente excluyentes → código `2`.
- `--next` en una película → código `2` (la opción no aplica).
- `mm mark` exige exactamente una acción; ninguna o varias → código `2`.
- `--position` exige `--file` cuando el título tiene más de un fichero; con un único
  fichero se usa ese. Posición negativa o no numérica → código `2`.
- El siguiente episodio se busca por `(season, episode)` dentro del mismo título. Los
  especiales (`season = 0`) no se mezclan con la temporada regular. Si no hay siguiente,
  se informa y se sale con `1`.
- Los listados nuevos respetan `--json` y el [ADR 0004](./0004-contrato-cli.md): un objeto
  por `stdout`, errores por `stderr`, sin ANSI.
- `mm play --json` conserva los campos actuales y **añade** `playback`:
  `{ "resumedFrom": number | null, "position": number, "duration": number | null,
"completed": boolean }`. Los campos existentes no cambian de tipo ni de nombre.
- `mm continue --json` y `mm history --json` devuelven `{ "items": [...] }`; cada elemento
  incluye `media`, `file`, `position`, `duration`, `completed`, `lastPlayedAt` y
  `playCount`.
- `mm mark --json` devuelve el estado resultante con la misma forma que un elemento de
  historial.

### Decisiones del plan resueltas

1. `mm play` **reanuda por defecto** en `2.0`.
2. El seguimiento automático se limita a **mpv**; VLC queda manual.
3. Windows se pospone a **`2.1`**.
4. Solo **favoritos**; sin etiquetas ni colecciones en `2.0`.

## Alternativas consideradas

- **Estado por título en lugar de por fichero:** más resistente a mover ficheros, pero
  incorrecto con varias calidades o versiones del mismo episodio.
- **Guardar el progreso en un JSON aparte:** rompe la atomicidad con la biblioteca y
  obliga a sincronizar dos fuentes de verdad.
- **Tracker por scraping de la salida de mpv:** frágil y dependiente del idioma; el socket
  IPC es estable y no requiere red.
- **Seguimiento IPC también para VLC:** su interfaz es diferente y no está disponible en
  todas las compilaciones; se pospone.
- **Reanudar siempre, sin umbral:** reanudar a los 5 segundos molesta más que ayuda.

## Consecuencias

- Mover un fichero pierde su progreso; se documenta y se acepta.
- El tracker añade un proceso auxiliar y un socket, con el coste de limpieza asociado.
- Los scripts que ya consumen `mm play --json` siguen funcionando: solo hay campos nuevos.
- El estado manual siempre funciona, aunque mpv no esté instalado o falle el IPC.
