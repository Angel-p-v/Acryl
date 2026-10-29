# Plan: Nuevas mejoras futuras (Plan 9)

**Estado:** ✅ Completado — todos los ítems implementados y mergeados en `main` (D1 PR #171, C1 PR #173, T5 PR #174, M2 PR #175, T1 PR #176, R3 PR #178, T2 PR #179, C2 PR #180).  
**Índice de planes:** [`docs/plans/README.md`](./README.md)

## 1. Objetivo

Recopilar mejoras de valor para el proyecto que no entraron en planes anteriores, ordenadas por impacto y complejidad. Cada ítem es independiente y se puede aceptar o rechazar individualmente.

---

## 2. TUI y experiencia de usuario

| #   | Área | Descripción                                                                           | Prioridad | Esfuerzo | Rama sugerida                    |
| --- | ---- | ------------------------------------------------------------------------------------- | --------- | -------- | -------------------------------- |
| T1  | TUI  | Temas de color personalizables definidos por el usuario en el YAML                    | Baja      | Bajo     | `feat/tui-temas-personalizados`  |
| T2  | TUI  | Atajos de teclado configurables desde la propia TUI (no solo en YAML)                 | Media     | Medio    | `feat/tui-atachos-configurables` |
| T3  | TUI  | Vista de calendario: ver lanzamientos por fecha (útil para series/anime)              | Media     | Media    | `feat/tui-vista-calendario`      |
| T4  | TUI  | Página de "próximos episodios" basada en metadatos de TMDB/TVDB                       | Media     | Media    | `feat/tui-proximos-episodios`    |
| T5  | TUI  | Soporte para subtítulos: listar y seleccionar archivos `.srt` asociados al reproducir | Media     | Media    | `feat/tui-subtitulos`            |

---

## 3. Indexador y watcher

| #   | Área      | Descripción                                                                                | Prioridad | Esfuerzo | Rama sugerida                       |
| --- | --------- | ------------------------------------------------------------------------------------------ | --------- | -------- | ----------------------------------- |
| I1  | Indexador | Watching selectivo por raíz: vigilar solo una carpeta específica sin reiniciar todo        | Baja      | Medio    | `feat/indexer-watching-selectivo`   |
| I2  | Indexador | Detección de duplicados: alertar si el mismo archivo está indexado bajo rutas distintas    | Media     | Media    | `feat/indexer-detectar-duplicados`  |
| I3  | Indexador | Importación desde otros gestores (Plex, Jellyfin, Kodi) para migrar bibliotecas existentes | Media     | Alta     | `feat/importar-desde-otras-fuentes` |
| I4  | Indexador | Índice de puertos: si el archivo es un disco/ISO, indexar su contenido interno             | Alta      | Alta     | `feat/indexer-discos-iso`           |

---

## 4. Metadatos y enriquecimiento

| #   | Área      | Descripción                                                                      | Prioridad | Esfuerzo | Rama sugerida                  |
| --- | --------- | -------------------------------------------------------------------------------- | --------- | -------- | ------------------------------ |
| M1  | Metadatos | Proveedor fanart.tv para obtener arte de fondo y portadas alternativas           | Media     | Medio    | `feat/metadatos-fanarttv`      |
| M2  | Metadatos | Proveedor AniList para mejores datos de anime                                    | Media     | Medio    | `feat/metadatos-anilist`       |
| M3  | Metadatos | Cache de metadatos offline: funcionar sin red usando solo lo cacheado            | Baja      | Bajo     | `feat/metadatos-cache-offline` |
| M4  | Metadatos | Búsqueda difusa (fuzzy search): encontrar títulos aunque se escriban mal         | Media     | Media    | `feat/busqueda-difusa`         |
| M5  | Metadatos | Calificación de usuario propia (sistema de estrellas/like interno, no solo TMDB) | Baja      | Medio    | `feat/calificacion-usuario`    |

---

## 5. Reproductor y playback

| #   | Área        | Descripción                                                                              | Prioridad | Esfuerzo | Rama sugerida                    |
| --- | ----------- | ---------------------------------------------------------------------------------------- | --------- | -------- | -------------------------------- |
| R1  | Reproductor | Soporte para más reproductores (IINA, QuickTime, etc.) en macOS/Windows                  | Baja      | Bajo     | `feat/reproductores-adicionales` |
| R2  | Reproductor | Resumir desde el minuto exacto con soporte para capítulos                                | Media     | Media    | `feat/resumir-con-capitulos`     |
| R3  | Reproductor | Lista de reproducción (playlist): reproducir varios episodios seguidos automáticamente   | Media     | Media    | `feat/lista-reproduccion`        |
| R4  | Reproductor | Control remoto desde otra terminal: enviar comandos a una instancia `mm` ya ejecutándose | Alta      | Alta     | `feat/control-remoto`            |

---

## 6. Distribución y despliegue

| #   | Área         | Descripción                                                                      | Prioridad | Esfuerzo | Rama sugerida                  | Estado        |
| --- | ------------ | -------------------------------------------------------------------------------- | --------- | -------- | ------------------------------ | ------------- |
| D1  | Distribución | Docker Compose completo: base de datos persistente + config + volúmenes de media | Media     | Bajo     | `feat/docker-compose-completo` | ✅ completado |
| D2  | Distribución | Paquetes .deb y .rpm para instalación nativa en Debian/RedHat sin script         | Media     | Medio    | `feat/paquetes-deb-rpm`        |               |
| D3  | Distribución | Actualización delta: descargar solo lo que cambió, no el bundle entero           | Alta      | Alta     | `feat/actualizacion-delta`     |               |

---

## 7. Calidad y testing

| #   | Área    | Descripción                                                                        | Prioridad | Esfuerzo | Rama sugerida                  |
| --- | ------- | ---------------------------------------------------------------------------------- | --------- | -------- | ------------------------------ |
| Q1  | Testing | Tests de propiedad (property-based) para el parser de nombres de episodios y rutas | Media     | Medio    | `test/property-based-parser`   |
| Q2  | Testing | Benchmark de indexación masiva para detectar regresiones de rendimiento            | Baja      | Bajo     | `test/bench-indexacion-masiva` |
| Q3  | Testing | E2E en Windows y macOS (ya hay runners, ampliar cobertura)                         | Media     | Medio    | `test/e2e-windows-macos`       |

---

## 8. Documentación y comunidad (open source)

| #   | Área          | Descripción                                                                      | Prioridad | Esfuerzo | Rama sugerida                       |
| --- | ------------- | -------------------------------------------------------------------------------- | --------- | -------- | ----------------------------------- |
| C1  | Documentación | Guía de migración desde Plex/Jellyfin para atraer usuarios                       | Baja      | Bajo     | `docs/guia-migracion-plex-jellyfin` |
| C2  | Documentación | Ejemplos de configuración YAML para distintos escenarios (servidor, laptop, NAS) | Baja      | Bajo     | `docs/ejemplos-config`              |
| C3  | Comunidad     | Vídeos tutoriales para ayudar a la adopción del proyecto                         | Baja      | Bajo     | (no es código)                      |

---

## 3. Orden recomendado de implementación

Todos los ítems del plan 9 están implementados y mergeados en `main`:

1. **D1** — Docker Compose completo ✅ completado (PR #171)
2. **C1** — Guía de migración desde Plex/Jellyfin ✅ completado (PR #173)
3. **T5** — Soporte para subtítulos (.srt) en `mm play` ✅ completado (PR #174)
4. **M2** — Proveedor AniList para metadatos de anime ✅ completado (PR #175)
5. **T1** — Temas de color personalizables desde YAML ✅ completado (PR #176)
6. **R3** — Lista de reproducción (`mm playlist`) ✅ completado (PR #178)
7. **T2** — Atajos de teclado configurables desde la TUI ✅ completado (PR #179)
8. **C2** — Ejemplos de configuración YAML ✅ completado (PR #180)

---

## 4. Notas

- Cada ítem se ejecuta en su propia rama con commit atómico, PR y merge squash.
- `npm run verify` debe pasar antes de cada push.
- No es necesario implementar todos los ítems; se pueden seleccionar por prioridad.
