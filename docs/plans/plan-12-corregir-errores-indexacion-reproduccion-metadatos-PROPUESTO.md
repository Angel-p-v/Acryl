# Plan: Corregir errores de indexación y metadatos de mm

## Problemas reportados

### 1. Categorías de series o películas indexadas incorrectamente

Los archivos se indexan con el tipo equivocado (`series` / `anime` / `movie`) o no se indexan en absoluto.

**Causa raíz:** En `src/core/indexer/classifier.ts` (líneas 47-81), el tipo de contenido se determina **exclusivamente por el `type` de la `LibraryRoot` configurada por el usuario**, nunca inspeccionando el contenido real de los archivos:

- Si una raíz está configurada como `series` pero contiene películas, se intentará parsear cada archivo como episodio (`parseEpisode`). Si no encuentra patrón de episodio, el archivo se descarta y **nunca se indexa**.
- Si una raíz está configurada como `movie` pero contiene capítulos de serie, se tratará cada archivo como película independiente mediante `parseMovie`. Si el nombre no tiene año reconocible (ej. solo etiquetas de release como `1080p.mkv`), se descarta.
- Los ficheros sueltos en la raíz de una carpeta `series`/`anime` se descartan completamente (línea 65: `rest.length === 0` → retorna `null`).
- No existe detección de contenido ni reasignación automática de tipo. Si el usuario configura mal una raíz, los archivos se indexan con el tipo equivocado o se pierden por completo.

### 2. Al darle a 'r' en un capítulo sin metadatos, no pasa nada

Hay dos causas independientes:

**Causa 2a — Desajuste de keybinding:** En `src/ui/App.tsx` línea 366, el handler se registra como `refreshMetadata: handleRefreshMetadata`, pero el keybinding por defecto en `src/core/config/defaults.ts` línea 46 define `refresh: 'r'` (clave `refresh`, no `refreshMetadata`). En `useKeymap` se busca `bindings['refreshMetadata']` que es `undefined`, así que presionar 'r' nunca dispara `handleRefreshMetadata`.

**Causa 2b — `getOrFetchMetadata` no obtiene metadatos desde cero:** En `src/core/library/service.ts` (líneas 288-343), cuando un título **no tiene metadatos previos** (`existing === undefined`):

1. Se salta el bloque de TTL porque `if (existing)` es falso.
2. Entra al bloque del proveedor configurado, pero `if (existing?.externalId)` es falso.
3. **Nunca llama a `metadataService.search()` ni a `metadataService.getDetails()`**.
4. Retorna `undefined`.

La UI en `src/ui/components/MediaDetails.tsx` línea 55 le dice al usuario "Pulsa «r» para obtener metadatos externos", creando una expectativa que el código no cumple.

## Archivos implicados

| Archivo                              | Rol                                                                                          |
| ------------------------------------ | -------------------------------------------------------------------------------------------- |
| `src/core/indexer/classifier.ts`     | `classifyFile()` — determina tipo de contenido desde `root.type` sin inspeccionar el archivo |
| `src/ui/App.tsx`                     | Handlers de `useKeymap` (desajuste `refreshMetadata`)                                        |
| `src/ui/hooks/useKeymap.ts`          | Dispatch de teclas a handlers; busca `bindings[action]` por cada clave de handlers           |
| `src/core/config/defaults.ts`        | Keybindings por defecto (`refresh`, `play`)                                                  |
| `src/core/library/service.ts`        | `getOrFetchMetadata()` — no busca metadatos si no hay `existing?.externalId`                 |
| `src/ui/components/MediaDetails.tsx` | UI que anuncia "Pulsa «r» para obtener metadatos externos"                                   |

## Plan de trabajo (hitos y commits atómicos)

### Hito 1: Detectar tipo de contenido por inspección del archivo, no solo por `root.type`

**Archivos:** `src/core/indexer/classifier.ts`, `src/core/indexer/parsers/episode.ts`, `src/core/indexer/parsers/movie.ts`

Cambiar `classifyFile()` para que determine el tipo de contenido inspeccionando el nombre y estructura del archivo, no únicamente el `type` de la raíz:

- Si el nombre del archivo coincide con un patrón de episodio (S01E02, 1x02, etc.) → clasificar como `series` o `anime` según corresponda.
- Si el nombre del archivo tiene patrón de película (título + año) → clasificar como `movie`.
- Si no coincide con ningún patrón conocido, intentar heurística de carpeta padre (nombre de la carpeta contenedora).
- Si tras todos los intentos no se puede determinar, devolver tipo `unknown` en lugar de descartar el archivo.

**Criterio de aceptación:** Un archivo con patrón de episodio se indexa como `series`/`anime` incluso si está en una raíz configurada como `movie`, y viceversa. Si se puede inferir por carpeta padre, se usa ese tipo. Si tras todos los intentos no se puede determinar, se indexa con tipo `unknown` en lugar de descartarse.

---

### Hito 2: Que la tecla 'r' obtenga metadatos desde cero en títulos sin metadatos

**Archivos:** `src/ui/App.tsx`, `src/core/config/defaults.ts`, `src/core/library/service.ts`

1. **Arreglar desajuste de keybinding:** Renombrar el handler de `refreshMetadata` a `refresh` (o viceversa) para que coincida con la clave del keybinding por defecto (`refresh: 'r'`).
2. **Arreglar `getOrFetchMetadata`:** Cuando `existing === undefined`, llamar a `metadataService.search()` con el título del media para obtener un `externalId`, y luego `metadataService.getDetails()` para poblar los metadatos.

**Criterio de aceptación:** Presionar 'r' sobre un capítulo o película sin metadatos previos muestra una notificación de estado ("Buscando metadatos…") mientras se consulta TMDB/TVDB, y al terminar muestra los metadatos obtenidos en el panel de detalles.

---

### Hito 3: Pruebas y verificación

**Archivos:** `tests/unit/`, `tests/integration/`

- Test unitario para `classifyFile` con archivos en raíces de tipo equivocado.
- Test unitario para `getOrFetchMetadata` cuando no hay metadatos previos.
- Ejecutar `npm run verify` (lint, format, typecheck, test) y que pase en verde.

---

## Reglas del proyecto a respetar

- No usar `any`, no usar `as` sin justificación.
- Tipado estricto absoluto.
- Manejo de errores con clases tipadas o `Result<T, E>`, no `throw new Error(...)` genéricos.
- Commits Conventional Commits con scope (`ui`, `cli`, `config`, `db`, `indexer`, `parser`, `player`, etc.).
- `npm run verify` debe pasar antes de push.
- Una rama por feature, PR a `main`, merge con squash.
- Los cambios en scripts de instalación (`install.sh`, `install.ps1`) y README van en la misma PR que los cambios de la aplicación, si es necesario.

## Decisiones tomadas

- **Tipo desconocido:** Se intenta determinar el tipo por todos los medios disponibles (patrón de episodio, patrón de película, heurística de carpeta padre). Si tras todo no se puede determinar, se indexa como `unknown` en lugar de descartar el archivo.
- **Metadatos 'r':** Se muestra una notificación de estado ("Buscando metadatos…") al usuario mientras se consulta TMDB/TVDB, para que sepa que la aplicación está trabajando.
