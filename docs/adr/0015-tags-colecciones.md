# ADR 0015: Etiquetas y colecciones (v2.1)

**Fecha:** 2026-10-24
**Estado:** Aceptado
**Autor:** Coronel

## Contexto

Actualmente `media-manager` tiene un concepto de "favoritos" binario (una media es favorita o no) almacenado en `media_preferences.favorite`. Los usuarios han solicitado una forma más flexible de organizar su biblioteca: etiquetas múltiples por título (ej. "por ver", "dropped", "recomendada", "4K").

El siguiente hito (M14d) añade etiquetas/colecciones como extensión del modelo de favoritos existente.

## Decisión

**Sistema de etiquetas múltiples por título** con las siguientes características:

| Aspecto             | Decisión                                                             |
| ------------------- | -------------------------------------------------------------------- |
| Modelo de datos     | Tablas `tags` e `media_tags` (relación muchos-a-muchos)              |
| Migración           | v4 (aditiva: no toca tablas existentes)                              |
| Favoritos           | Se convierten en tag especial `favorite` (opcional, backward-compat) |
| Filtro en `mm list` | `--tag <nombre>` (puede repetirse para AND)                          |
| TUI                 | Vista "Etiquetas" en sidebar, tecla `t` para asignar/quitar          |
| CLI                 | `mm tag add                                                          | remove | list | rename` |

### Tablas

```sql
CREATE TABLE IF NOT EXISTS tags (
  id         INTEGER PRIMARY KEY,
  name       TEXT    NOT NULL UNIQUE,
  color      TEXT,           -- hex opcional para la TUI
  created_at TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS media_tags (
  media_id INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE,
  tag_id   INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  PRIMARY KEY (media_id, tag_id)
);

CREATE INDEX IF NOT EXISTS idx_media_tags_tag ON media_tags(tag_id);
CREATE INDEX IF NOT EXISTS idx_media_tags_media ON media_tags(media_id);
```

### Repositorio `tagRepo`

```ts
export type TagRepository = {
  // CRUD de tags
  create(input: { name: string; color?: string }): TagRow;
  get(id: number): TagRow | undefined;
  getByName(name: string): TagRow | undefined;
  list(): TagRow[];
  update(id: number, input: { name?: string; color?: string }): boolean;
  remove(id: number): boolean;
  // Asignación a medios
  assign(mediaId: number, tagId: number): void;
  unassign(mediaId: number, tagId: number): void;
  unassignAll(mediaId: number): void;
  // Consulta
  listByMedia(mediaId: number): TagRow[];
  listMediaByTag(tagId: number): number[]; // devuelve media IDs
  countByTag(tagId: number): number;
};
```

### Filtro en `mediaRepo.list()`

Añadir opción `tags?: number[]` (IDs de tags). Cuando se proporciona, se hace JOIN con `media_tags` para filtrar solo medios que tengan todos los tags indicados (AND).

### Migración de favoritos

Opcional: al migrar a v4, se puede crear automáticamente un tag `favorite` y asignarlo a todos los medios que tenían `favorite = 1` en `media_preferences`. Esto se deja como opcional para no romper la backward-compat.

## Alternativas consideradas

| Alternativa                                            | Por qué se descarta                                                        |
| ------------------------------------------------------ | -------------------------------------------------------------------------- |
| Columna `tags` con array en `media`                    | SQLite no tiene arrays nativos; requeriría JSON y sería difícil de indexar |
| Tabla `tags` sin `media_tags` (solo nombres en JSON)   | Difícil de consultar y mantener integridad referencial                     |
| Favoritos como columna separada + tags separados       | Duplica concepto; los favoritos son solo un tag más                        |
| Etiquetas jerárquicas (colecciones con subcolecciones) | Scope demasiado grande para v2.1; se puede añadir en v2.2                  |

## Consecuencias

### Positivas

- Los usuarios pueden organizar su biblioteca con múltiples etiquetas
- Los filtros de `mm list` y la TUI se vuelven más potentes
- Backward-compatible: `media_preferences.favorite` sigue funcionando

### Negativas / Riesgos

- Complejidad adicional en el esquema de BD (migración v4)
- El filtro por tag en `mediaRepo.list` requiere JOIN dinámico
- La TUI necesita una nueva vista para gestionar etiquetas

## Referencias

- ADR 0013 (M14 scope v2.1)
- [`src/core/db/migrations/index.ts`](./src/core/db/migrations/index.ts)
- [`src/core/db/repositories/mediaRepo.ts`](./src/core/db/repositories/mediaRepo.ts)
- [`src/core/library/service.ts`](./src/core/library/service.ts)
