# ADR 0017: Sincronización automática y TUI con metadatos visuales · v2.2

**Fecha:** 2026-10-24
**Estado:** Aceptado
**Autor:** Coronel

## Contexto

media-manager v2.1 ya permite obtener metadatos externos de TMDB/TVDB vía comandos CLI (`mm metadata fetch/search/clear`). Sin embargo:

1. No hay sincronización automática: el usuario debe ejecutar manualmente `mm metadata fetch` para cada título.
2. La TUI no muestra metadatos obtenidos (sinopsis, póster, calificación) en el panel de detalles.
3. No hay forma visual de ver qué títulos tienen metadatos y cuáles no.

M17 completa v2.2 añadiendo sincronización automática y visualización en la TUI.

## Decisión

**Sincronización automática basada en TTL de caché** + **visualización en TUI**:

| Aspecto         | Decisión                                                                                                                                                               |
| --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Sincronización  | Cuando el usuario abre detalles de un título en TUI, si los metadatos están expirados (según `cacheTtlHours`), se refrescan automáticamente si hay API key configurada |
| TTL             | Configurable vía `metadata.cacheTtlHours` (default: 24 horas)                                                                                                          |
| TUI             | Panel de detalles muestra metadatos si existen; si no, ofrece opción de refrescar                                                                                      |
| Forzar refresco | Tecla `r` en detalles para refrescar metadatos del título actual                                                                                                       |
| Fallback        | Si no hay API key o falla la red, se muestra mensaje informativo sin bloquear                                                                                          |

### Flujo de sincronización

```
Usuario abre detalles de título → ¿Existen metadatos? → Sí: ¿Expirados? → Sí: refrescar automáticamente → Mostrar
                                              ↳ No: mostrar directamente
                                              ↳ No: ofrecer "obtener metadatos" → Usuario confirma → Obtener → Mostrar
```

### TUI

- Panel de detalles muestra: título, año, sinopsis, calificación, fecha de estreno, idioma original
- Si hay póster: se muestra como bloque de texto con información del proveedor
- Tecla `r`: refrescar metadatos del título actual (forzar descarga)
- Tecla `m` en listado: ir a detalles con metadatos si existen

## Alternativas consideradas

| Alternativa                                        | Por qué se descarta                                                 |
| -------------------------------------------------- | ------------------------------------------------------------------- |
| Sincronización automática en segundo plano (watch) | Requiere proceso residente, más complejo, mayor consumo de recursos |
| Sincronización al iniciar la app                   | Ralentiza el arranque si hay muchos títulos y red lenta             |
| Sincronización solo manual (estado actual)         | No cumple el objetivo de M17 de automatizar                         |

## Consecuencias

### Positivas

- Los metadatos se mantienen actualizados automáticamente
- La TUI ofrece experiencia visual completa con información enriquecida
- El usuario tiene control explícito sobre cuándo refrescar

### Negativas / Riesgos

- La primera apertura de detalles puede ser lenta si hay que descargar de la API
- Dependencia de APIs externas que pueden estar caídas o tener rate limits
- Se necesita manejo elegante de errores de red

## Referencias

- ADR 0016 (metadatos externos TMDB/TVDB)
- [`src/core/metadata/service.ts`](./src/core/metadata/service.ts)
- [`src/core/db/repositories/metadataRepo.ts`](./src/core/db/repositories/metadataRepo.ts)
- [`src/core/library/service.ts`](./src/core/library/service.ts)
