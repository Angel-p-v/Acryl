# ADR 0011 — Salud de la biblioteca y reparaciones seguras

- **Estado:** aceptada
- **Fecha:** 2026-10-23
- **Plan:** [`version-2.0.md`](../plans/version-2.0.md) (M11)
- **Relacionada con:** [ADR 0004](./0004-contrato-cli.md), [ADR 0007](./0007-logging-errores-y-mantenimiento.md)

## Contexto

La biblioteca es un índice de ficheros que el usuario mueve, renombra y borra por su
cuenta. Con el tiempo aparecen entradas que apuntan a rutas inexistentes, títulos sin
ficheros, duplicados y ejecuciones de indexación interrumpidas. `v1.x` no ofrece forma de
diagnosticar ese estado sin abrir SQLite a mano.

Un comando de reparación mal diseñado es más peligroso que el problema: borrar la fila
equivocada pierde el progreso (ADR 0010) y borrar ficheros del disco sería irreversible.
Por eso la reparación debe ser explícita, explicable y reversible.

## Decisión

### `mm doctor` es de solo lectura por defecto

- Sin opciones, `mm doctor` **no modifica la biblioteca ni el sistema de ficheros**.
- Solo escribe si se pasa `--prune-missing`.
- Abrir la base puede aplicar migraciones pendientes; cuando lo hace, se crea el backup
  descrito en el ADR 0010 y el informe lo indica.

### Comprobaciones

| Comprobación          | Cómo se hace                                                              | Gravedad |
| --------------------- | ------------------------------------------------------------------------- | -------- |
| Integridad SQLite     | `PRAGMA integrity_check` y `PRAGMA foreign_key_check`                     | error    |
| Ficheros ausentes     | `stat` de cada `media_files.file_path`                                    | warning  |
| Enlaces simbólicos    | `lstat`/`stat`: enlace roto o destino inaccesible                         | warning  |
| Permisos              | `access(R_OK)` sobre ficheros existentes                                  | warning  |
| Títulos vacíos        | `media` sin filas en `media_files`                                        | warning  |
| Fuera de las raíces   | Ruta que no cae bajo ninguna `library.roots[].path`                       | warning  |
| Duplicados            | Agrupar por tamaño y confirmar con huella parcial (cabeza + cola)         | info     |
| Duplicados exactos    | `--full-hash`: SHA-256 completo del fichero                               | info     |
| Indexaciones anómalas | `index_runs` en `running` durante más de 24 h o última ejecución `failed` | warning  |

- La huella parcial lee como máximo **64 KiB de la cabeza y 64 KiB de la cola** para no
  recorrer varios terabytes en un diagnóstico normal.
- `--full-hash` solo se aplica a grupos que ya coinciden en tamaño; el hash completo
  confirma o descarta el duplicado.
- Los duplicados son **informativos**: `doctor` nunca borra ficheros del disco.
- Un error de permiso al leer un fichero no aborta el diagnóstico; se acumula como issue.

### Reparación: `--prune-missing`

- Elimina del índice las filas de `media_files` cuyo fichero ya no existe y, después, los
  títulos que se quedan sin ficheros (`mediaRepo.removeEmpty`).
- **Nunca** borra ficheros del disco.
- Antes de borrar crea un backup de la base con el formato del ADR 0010:
  `<db>.bak-doctor-<timestamp>`, permisos `0600`. Si el backup falla, no se borra nada.
- Pide confirmación interactiva cuando hay TTY. `--yes` la omite para scripts.
- Con `--json` y sin `--yes` se rechaza con código `2`: un script no debe quedarse
  esperando una respuesta.
- Todo el borrado ocurre en una única transacción.

### Salida y códigos

- Humana: resumen por secciones y una lista de issues con ruta y motivo.
- `--json` devuelve un único objeto aditivo:

```json
{
  "status": "ok",
  "generatedAt": "2026-10-23T10:00:00.000Z",
  "database": { "integrity": "ok", "foreignKeys": "ok", "migrated": false },
  "counts": {
    "media": 120,
    "files": 980,
    "missing": 0,
    "emptyMedia": 0,
    "outsideRoots": 0,
    "permissionErrors": 0,
    "duplicateGroups": 0,
    "staleRuns": 0
  },
  "issues": [
    {
      "kind": "missing-file",
      "severity": "warning",
      "mediaId": 7,
      "fileId": 42,
      "path": "/library/Show/S01E01.mkv",
      "detail": "el fichero no existe"
    }
  ]
}
```

- `status`: `ok` (sin issues), `warning` (solo avisos/info) o `error` (fallo de integridad
  o de base).
- Códigos de salida: `0` sin problemas · `1` problemas encontrados o error de ejecución ·
  `2` uso incorrecto. El `1` por problemas es una excepción documentada al
  [ADR 0004](./0004-contrato-cli.md); el JSON permite distinguir el motivo y `stderr`
  sigue reservado a errores reales.
- `kind` es estable y aditivo: `integrity`, `foreign-key`, `missing-file`, `broken-symlink`,
  `permission`, `empty-media`, `outside-roots`, `duplicate`, `stale-run`.
- El orden de `issues` es determinista (por `kind`, luego ruta/id) para que los tests y
  los diffs no dependan del orden de recorrido del sistema de ficheros.

### Rendimiento

- El diagnóstico recorre el índice en lote y hace `stat` una vez por fichero; no abre
  conexiones nuevas ni consulta por título.
- Los duplicados se agrupan en SQL por tamaño y la huella se calcula solo dentro de los
  grupos candidatos.
- `--prune-missing` reutiliza la lista de ausentes del diagnóstico; no vuelve a recorrer
  el disco.

### Decisiones del plan resueltas

5. `--prune-missing` **pide confirmación** en TTY y acepta `--yes` para automatizar.
6. macOS **mantiene rutas XDG** (`~/.config`, `~/.local/share`) para no romper la paridad
   con Linux ni los scripts existentes.

## Alternativas consideradas

- **Reparar automáticamente al indexar:** silencioso y destructivo; el usuario no sabe por
  qué han desaparecido entradas.
- **`doctor --fix` genérico:** mezcla reparaciones de distinto riesgo. Se prefieren opciones
  explícitas por tipo (`--prune-missing`).
- **Borrar también los ficheros duplicados:** irreversible y fuera del alcance de un
  indexador; el usuario decide con su herramienta de ficheros.
- **Hash completo siempre:** demasiado lento para bibliotecas grandes.
- **Código de salida distinto para «hay problemas»:** rompería el contrato 0/1/2 del
  ADR 0004 sin una ganancia clara.

## Consecuencias

- `doctor` es seguro de ejecutar en cualquier momento y sirve como _smoke_ de la
  biblioteca.
- Las reparaciones quedan auditables: backup + lista de issues + transacción.
- Un `doctor` sobre una base recién migrada reporta el backup creado y no toca datos.
