# ADR-0020: Caché de pósters y descarga asíncrona

## 1. Contexto

Acryl muestra pósters de medios (películas, series, anime) en la galería principal y en vistas de detalles. Los pósters se obtienen de APIs públicas (TVmaze, AniList) y necesitan ser descargados, almacenados en caché y mostrados sin bloquear la interfaz GTK4.

Los requisitos son:
- Descarga asíncrona para no bloquear el hilo principal de GTK4
- Caché en disco para evitar re-descargas
- Placeholder visual mientras se descarga o si falla
- Compatibilidad con múltiples ubicaciones de caché (XDG, ruta clásica)
- Limpieza automática de archivos antiguos

## 2. Opciones consideradas

| Opción | Descripción | Ventajas | Desventajas |
|--------|-------------|----------|-------------|
| **A. reqwest + tokio + caché en disco** | Descarga HTTP asíncrona con reqwest, almacenamiento en disco bajo `~/.local/share/acryl/cache/posters/` | No bloquea la UI, caché persistente entre reinicios, reqwest ya es dependencia del proyecto | Requiere gestión de concurrencia (tokio), manejo de errores de red |
| **B. Descarga síncrona en hilo secundario** | Usar `std::thread::spawn` con reqwest bloqueante | Simpler, sin tokio | No aprovecha async, más hilos del SO |
| **C. gdk-pixbuf loader con URI remoto** | Usar gdk-pixbuf para cargar imágenes directamente desde URL | Integrado con GTK, sin código de descarga manual | gdk-pixbuf no soporta descarga HTTP nativamente, requeriría glib networking |
| **D. No caché, descarga cada vez** | Descargar el póster cada vez que se muestra | Sin gestión de caché | Muy lento, consumo de ancho de banda, no funciona sin conexión |

## 3. Decisión

**Se adopta la opción A: reqwest + tokio + caché en disco.**

Razones:
1. **reqwest ya es dependencia**: Se usa para consultas a TVmaze/AniList, no añade nuevas dependencias.
2. **tokio ya es dependencia**: Se usa para peticiones HTTP async, no añade nuevas dependencias.
3. **Caché persistente**: Los pósters sobreviven a reinicios de la aplicación.
4. **UI no bloqueada**: Las descargas se hacen en tokio tasks separadas del hilo principal de GTK4.
5. **Placeholder**: Se muestra un widget de placeholder mientras se descarga o si falla.

## 4. Consecuencias

### Positivas
- Descargas no bloquean la interfaz GTK4
- Caché en disco reduce ancho de banda y tiempo de carga
- Compatible con XDG_DATA_HOME y rutas clásicas
- Limpieza automática de archivos antiguos (`clean_cache`)

### Negativas
- Requiere manejo de errores de red (timeouts, HTTP errores, DNS)
- Tokio runtime añade complejidad al proyecto
- Caché puede crecer indefinidamente sin limpieza periódica

## 5. Alternativas descartadas

- **Descarga síncrona en hilo secundario**: Funciona pero no aprovecha el runtime async existente (tokio).
- **gdk-pixbuf con URI remoto**: gdk-pixbuf no soporta descarga HTTP directamente.
- **Sin caché**: Rendimiento inaceptable para galerías con muchos pósters.

---

**Estado**: Aprobado  
**Fecha**: 2026-11-01  
**Autor**: Acryl development team
