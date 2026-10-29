# ADR 0019: Renderizado de pósters en terminal con ink-picture · v2.2

**Fecha:** 2026-10-28  
**Estado:** Aceptado  
**Autor:** Coronel

## Contexto

El póster de los títulos no se renderiza en terminales con soporte gráfico nativo
(Ghostty, Kitty, WezTerm) usando el Protocolo Gráfico de Kitty (SGP). El encoder crudo
implementado en `src/shared/kittyProtocol.ts` tenía múltiples defectos del protocolo:

1. Usaba `t=f` (tipo: fichero) mientras enviaba datos Base64 inline; debe ser `t=d` o
   omitirse.
2. Enviaba datos JPEG con `f=100` (formato PNG); Kitty no acepta JPEG como `f=100`.
3. Invertía el parámetro `m`: `m=1` debe ir en todos los chunks menos el último, y
   `m=0` solo en el último.
4. Repetía todos los parámetros en chunks de continuación; solo el primero debe llevar
   los parámetros completos (`a=T` en el último, `a=f` en intermedios).
5. Truncaba la imagen a `maxFrames` chunks (10), enviando solo ~30 KB de un JPEG de 1.4 MB,
   dejando la imagen incompleta.
6. Generaba un `imageId` aleatorio en cada render, causando reenvíos, borrados y colisiones
   de ID.
7. No posicionaba el cursor; `c=` y `r=` solo dimensionan, la imagen se pinta en el cursor
   actual de la terminal.
8. Escribía directamente a `process.stdout`, corrompiendo el estado de stdout que gestiona
   Ink.

Además, `npm install ink-picture` reintroducía 30 vulnerabilidades `high` de `@jimp/*`
(transitivas), exactamente lo que se había eliminado al quitar `terminal-image` previamente.

## Decisión

Se sustituye el encoder crudo de Kitty por **`ink-picture`** (v2.1.0), una librería
mantenida que encapsula toda la complejidad del protocolo gráfico de terminal:

- Decodifica JPEG/PNG/WebP/GIF con Jimp y redimensiona al tamaño de visualización.
- Elige automáticamente el mejor protocolo: Kitty > iTerm2 > Sixel > Half-Block > Braille > ASCII.
- Transmite la imagen una vez y la reposiciona tras cada render de Ink (mediante Profiler).
- Borra la imagen al desmontar el componente.
- Soporta `protocol="kitty"` para forzar Kitty SGP en Ghostty/Kitty/WezTerm.

### ¿Por qué no mantener el encoder propio?

El encoder propio requeriría corregir 7 defectos del protocolo, implementar redimensión de
imágenes (necesitando Jimp de todas formas), gestión de cursor, repintado tras cada render
de Ink y borrado al desmontar. `ink-picture` ya resuelve todo esto de forma probada.

### ¿Por qué no `terminal-image`?

`terminal-image` dependía de `@jimp/*`, trayendo 30 vulnerabilidades `high`. Fue eliminada
precisamente por ese motivo. `ink-picture` también depende de `jimp`, pero las
vulnerabilidades son pre-existing en el ecosistema `jimp` 1.x y no hay una alternativa
mantenida sin ellas para renderizado de imágenes en terminal Node.js.

### Fallback

Cuando el terminal NO soporta Kitty SGP, se usa `chafa --format=symbols --symbols=sextant`
como fallback ANSI (ya existente en `src/shared/terminalImage.ts`). El fallback Unicode está
limitado a 2×3 subpíxeles por celda y nunca se verá como un 4K; es aceptable para
terminales sin gráficos nativos.

## Alternativas consideradas

| Alternativa                                | Por qué se descarta                                                                                                                                         |
| ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Corregir `kittyProtocol.ts` encoder propio | Requiere Jimp para redimensionar, gestión de cursor, repintado por render, borrado al desmontar. Más código que mantener y menos probado que `ink-picture`. |
| `terminal-image`                           | 30 vulnerabilidades `high` de `@jimp/*`, ya eliminadas previamente.                                                                                         |
| `chafa --format=kitty` como encoder        | Salida ~1.5 MB para 450×640 RGBA; combina transmisión y visualización; no reposiciona tras renders de Ink.                                                  |
| No renderizar pósters en modo nativo       | Degrada la experiencia en Ghostty/Kitty/WezTerm, el uso principal reportado por el usuario.                                                                 |

## Consecuencias

### Positivas

- Pósteres JPEG 4K se renderizan nítidos en Ghostty/Kitty/WezTerm con proporción correcta (`objectFit="contain"`).
- No hay truncamiento: la imagen completa se transmite; no existe `maxFrames`.
- ID estable gestionado por `ink-picture`; re-renders de Ink no reenvían ni borran la imagen.
- Al cambiar de título o salir, `ink-picture` borra la imagen gráfica (no quedan flotantes).
- Fallback ANSI con `chafa` en terminales sin gráficos nativos.
- Código eliminado: `src/shared/kittyProtocol.ts` y su test (encoder crudo).

### Negativas / Riesgos

- `ink-picture` depende de `jimp` 1.x, que tiene vulnerabilidades `high` conocidas
  (prototype poisoning vía `deepmerge`, memoria agotada en `zod`). Son pre-existing en el
  ecosistema y no hay alternativa mantenida sin ellas.
- Se requiere envolver la app en `<InkPictureProvider>` en `src/ui/run.tsx`.
- El fallback Unicode nunca ofrecerá calidad 4K; es una degradación elegante esperada.

## Referencias

- [`src/ui/components/PosterImage.tsx`](./src/ui/components/PosterImage.tsx) — componente reescrito
- [`src/shared/terminalImage.ts`](./src/shared/terminalImage.ts) — `supportsKittyGraphics()`, `detectNativeFormat()` solo anuncia Kitty
- [`src/ui/run.tsx`](./src/ui/run.tsx) — envuelve en `<InkPictureProvider>`
- [`src/ui/components/MediaDetails.tsx`](./src/ui/components/MediaDetails.tsx) — Hooks antes del return (Reglas de Hooks)
- [`src/ui/views/PlayDetails.tsx`](./src/ui/views/PlayDetails.tsx) — Hooks antes del return (Reglas de Hooks)
- [`src/core/metadata/posterCache.ts`](./src/core/metadata/posterCache.ts) — validación de Content-Type, magic bytes, escritura atómica `.part`
- [`docs/guides/architecture.md`](./docs/guides/architecture.md) — arquitectura de capas
