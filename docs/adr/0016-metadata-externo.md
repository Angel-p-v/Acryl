# ADR 0016: Metadatos externos (TMDB/TVDB) · v2.2

**Fecha:** 2026-10-24
**Estado:** Aceptado
**Autor:** Coronel

## Contexto

media-manager (`mm`) indexa y reproduce contenido multimedia local, pero no tiene información enriquecida como sinopsis, póster, elenco, calificación o año de estreno (más allá de lo que se infere del nombre del fichero).

Los usuarios han solicitado la posibilidad de enriquecer su biblioteca con metadatos externos de fuentes como TheMovieDB (TMDB) para películas y TheTVDB para series/anime.

## Decisión

**Integración opcional de metadatos externos en v2.2** con las siguientes características:

| Aspecto       | Decisión                                                                    |
| ------------- | --------------------------------------------------------------------------- |
| Proveedores   | TMDB (películas), TVDB (series/anime)                                       |
| Autenticación | Claves API configurables en `config.yaml`                                   |
| Caché         | Tabla `media_metadata` en SQLite (aditiva, migración v5)                    |
| Rate-limiting | Respeto a límites de API del proveedor (reintentos con backoff)             |
| Comandos CLI  | `mm metadata fetch <id>`, `mm metadata search <query>`, `mm metadata clear` |
| TUI           | Mostrar metadatos en panel de detalles (opcional, v2.2)                     |
| Offline       | Funciona sin claves API (usa solo datos locales)                            |

### Tablas

```sql
CREATE TABLE IF NOT EXISTS media_metadata (
  media_id    INTEGER PRIMARY KEY REFERENCES media(id) ON DELETE CASCADE,
  provider    TEXT    NOT NULL CHECK (provider IN ('tmdb', 'tvdb')),
  external_id INTEGER NOT NULL,
  title       TEXT,
  overview    TEXT,
  poster_path TEXT,
  backdrop_path TEXT,
  vote_average REAL,
  first_air_date TEXT,
  release_date TEXT,
  original_language TEXT,
  fetched_at   TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at   TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_metadata_provider ON media_metadata(provider, external_id);
```

### Repositorio `metadataRepo`

```ts
export type MetadataRepository = {
  get(mediaId: number): MediaMetadataRow | undefined;
  set(input: SetMetadataInput): MediaMetadataRow;
  remove(mediaId: number): boolean;
  searchByExternalId(provider: string, externalId: number): MediaMetadataRow | undefined;
};
```

### Configuración

```yaml
metadata:
  tmdb_api_key: 'tu-clave-api'
  tvdb_api_key: 'tu-clave-api'
  cache_ttl_hours: 24 # cuánto considerar válidos los metadatos cacheados
```

### Flujo de uso

1. Usuario configura claves API en `config.yaml` (opcional)
2. Usuario ejecuta `mm metadata fetch 42` para obtener metadatos del título #43
3. La CLI consulta la API externa, verifica rate-limiting, guarda en caché
4. `mm show 43` o la TUI muestran los metadatos enriquecidos

## Alternativas consideradas

| Alternativa                                  | Por qué se descarta                                                                     |
| -------------------------------------------- | --------------------------------------------------------------------------------------- |
| Obtener metadatos automáticamente al indexar | Requiere claves API y hace llamadas de red en cada indexación; demasiado lento y frágil |
| Usar solo TMDB (no TVDB)                     | TVDB es mejor para series/anime; limitarse a TMDB reduciría utilidad                    |
| Caché en archivo JSON separado               | Menos consistente con el resto de la BD; requiere gestión de archivos adicionales       |
| Sincronización automática periódica          | Complejidad innecesaria para v2.2; se puede añadir en v2.3                              |

## Consecuencias

### Positivas

- Biblioteca más útil con información enriquecida
- Opcional: no requiere claves API para funcionar básicamente
- Caché local evita llamadas repetidas a la API

### Negativas / Riesgos

- Las claves API son sensibles; no deben commitearse
- Rate-limiting de APIs externas puede fallar
- Dependencia de servicios externos que pueden cambiar o desaparecer
- Complejidad adicional en el esquema de BD (migración v5)

## Referencias

- ADR 0013 (M14 scope v2.1)
- ADR 0015 (etiquetas y colecciones)
- [`src/core/db/migrations/index.ts`](./src/core/db/migrations/index.ts)
- [`src/core/library/service.ts`](./src/core/library/service.ts)
