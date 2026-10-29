# ADR 0007 — Logging, errores con código y mantenimiento de SQLite

- **Estado:** aceptada
- **Fecha:** 2026-10-22

## Contexto

El hito M7 debía hacer la aplicación diagnosticable y mantenible sin romper el contrato
de la CLI. Hasta ahora los errores solo mostraban un mensaje, no había forma de activar
diagnóstico detallado y la base de datos no tenía una operación explícita de
mantenimiento. Además, el esquema v1 necesitaba una segunda migración para los listados
ordenados y las consultas de mantenimiento.

Las decisiones afectan a scripts y a la experiencia de usuario, así que no bastaba con
elegir la primera opción que funcionara.

## Decisión

### Logging estructurado

- La configuración incorpora `logging.level` (`silent`, `error`, `warn`, `info`,
  `debug`) y `logging.format` (`text`, `json`). Los valores por defecto son `warn` y
  `text`: la CLI normal no ensucia la salida y los diagnósticos se activan a demanda.
- `MM_LOG_LEVEL` y `MM_LOG_FORMAT` sobrescriben el archivo YAML. Un valor inválido es un
  `CONFIG_ERROR`, no se ignora en silencio.
- `--verbose` (`-v`) es una opción global y fuerza `debug`, independientemente de la
  configuración. Commander la copia al subcomando en `preAction`, de modo que funciona
  antes o después del comando y los casos de uso reciben el mismo flag sin variables
  globales.
- Los logs se escriben **siempre en `stderr`**. En formato `json` cada evento es una línea
  con `time`, `level`, `event` y campos estructurados. `stdout` queda reservado para la
  salida de usuario y para `--json`, que sigue siendo parseable.
- El logger se inyecta en los casos de uso (`indexLibrary`, watcher) en lugar de
  importarlo desde el dominio. `logger.child()` permite añadir contexto sin mutar el
  logger padre.

### Errores con código

- `AppError` ya tenía códigos; `describeError()` normaliza también errores inesperados a
  `APP_ERROR`.
- Un único `reportFatalError()` escribe en `stderr` el formato `[CÓDIGO] mensaje`. Con
  `--verbose` añade la pila. Los códigos de salida no cambian: `0` éxito, `1` error de
  ejecución y `2` uso incorrecto.

### Mantenimiento de la base de datos

- `closeDatabase()` ejecuta `PRAGMA optimize` como _best effort_. Un fallo al optimizar
  nunca impide cerrar ni oculta el error original.
- `mm db optimize` ejecuta la optimización de estadísticas de forma explícita.
  `--vacuum` añade una compactación real; antes hace `wal_checkpoint(TRUNCATE)` para que
  el tamaño reportado incluya el WAL consolidado.
- `VACUUM` no se ejecuta automáticamente porque bloquea, reescribe el fichero completo y
  necesita espacio temporal. Es una operación deliberada del usuario o de un script.

### Migración v1 → v2

La migración v2 solo añade índices de rendimiento; no cambia tablas ni datos:

- `idx_media_type_sort_title` para listados filtrados por tipo y ordenados por título.
- `idx_media_created_at` e `idx_media_updated_at` para los órdenes `added` y `recent`.
- `idx_index_runs_started_at` para las consultas de historial.

La migración se prueba partiendo de una base v1 con datos y FTS5, aplicando v2 y
comprobando que los datos siguen ahí y que los índices existen.

## Consecuencias

- Los scripts pueden consumir logs JSON de `stderr` sin mezclarlos con `stdout`.
- `--verbose` es seguro en cualquier posición y no requiere una segunda configuración.
- Los errores son identificables por código estable, lo que facilita soporte y tests E2E.
- El cierre normal paga un `PRAGMA optimize` barato; la compactación costosa sigue siendo
  opt-in.
- La migración v2 es idempotente y segura para bases v1 existentes.
