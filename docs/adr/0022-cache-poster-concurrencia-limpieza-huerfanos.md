# ADR-0022: Límite de concurrencia, progreso y limpieza de huérfanos en la caché de pósters

## 1. Contexto

Acryl descarga y almacena en caché los pósters y banners de los medios (películas, series, anime) bajo `~/.local/share/acryl/cache/posters/`. La estrategia base (ADR-0020) define descarga asíncrona con `reqwest` + `tokio` y caché en disco, con limpieza por antigüedad (`clean_cache`).

Con bibliotecas grandes, pueden solicitarse decenas de pósters al abrir la galería. Sin límite de concurrencia, múltiples descargas HTTP compiten por ancho de banda y disco, saturando la red y ralentizando la UI. Además, cuando un medio se elimina de la biblioteca, su póster/banner queda huérfano en el caché y nunca se reutiliza, haciendo que el caché crezca indefinidamente.

También falta visibilidad del estado de descarga en la UI: el spinner por tarjeta existe, pero no hay un reporte de progreso centralizado que permita un spinner global o indicador de descarga activa.

## 2. Opciones consideradas

| Opción | Descripción | Ventajas | Desventajas |
|--------|-------------|----------|-------------|
| **A. Semáforo de concurrencia + progreso + limpieza de huérfanos** | `tokio::sync::Semaphore` en `PosterService` para limitar descargas concurrentes (4 por defecto, configurable); reporte de `ProgressEvent` vía callback opcional; `clean_orphaned(dir, referenced_urls)` elimina imágenes ya no referenciadas por ningún medio. | Controla red/disco, reporte de progreso para la UI, caché no crece indefinidamente, no añade dependencias nuevas (tokio `sync` ya es uso interno). | Complejidad moderada en el servicio; el llamador debe recopilar URLs referenciadas desde el repositorio. |
| **B. Sin límite de concurrencia, solo limpieza por antigüedad** | Mantener `clean_cache` por edad y sin semáforo. | Más simple. | Saturación de red/disco en galerías grandes; caché crece con huérfanos. |
| **C. Descarga serial (una a la vez)** | Semáforo de 1 permiso. | Saturación mínima. | Muy lento para muchas imágenes. |

## 3. Decisión

**Se adopta la opción A.**

Razones:
1. **Saturación de red y disco controlada**: el semáforo limita las descargas concurrentes (4 por defecto), evitando que varias tarjetas de la galería descarguen a la vez.
2. **Caché acotado**: `clean_orphaned` elimina pósters/banners de medios borrados de la biblioteca, evitando crecimiento indefinido.
3. **Progreso visible**: `ProgressEvent` permite a la UI mostrar un spinner global o por tarjeta durante las descargas.
4. **Sin nuevas dependencias**: `tokio` con feature `sync` ya es dependencia del proyecto.
5. **Servicio independiente de la BD**: `clean_orphaned` recibe las URLs referenciadas como parámetro, por lo que el servicio de pósters no depende de la capa de persistencia.

## 4. Consecuencias

### Positivas
- Descargas concurrentes acotadas; la galería no satura la red ni el disco.
- El caché se limpia de imágenes huérfanos al eliminar medios.
- Posibilidad de mostrar progreso de descarga en la UI.
- Retrocompatible: `download_async`, `download_async_force`, `clean_cache` y `remove_cached` conservan su firma.

### Negativas
- El llamador de `clean_orphaned` debe recopilar las URLs de imagen aún en uso desde `media_repo`/`file_repo`.
- Un semáforo compartido por todas las instancias de `PosterService` limita globalmente (deseable para proteger red/disco).

## 5. Alternativas descartadas

- **Opción B**: no protege contra saturación ni crecimiento indefinido del caché.
- **Opción C**: demasiado lenta para galerías con muchos pósters.

---

**Estado**: Aprobado  
**Fecha**: 2026-11-04  
**Autor**: Acryl development team
