# ADR-0019: Detección de pistas de audio/subtítulos con ffprobe y selección en mpv

## 1. Contexto

Los archivos de vídeo pueden contener múltiples pistas de audio y subtítulos en diferentes idiomas y códecs. Los usuarios de Acryl necesitan poder seleccionar qué pista de audio y subtítulos usar al reproducir un archivo.

Durante la indexación de medios, la aplicación necesita:
- Detectar las pistas disponibles en cada archivo de vídeo
- Almacenar esa información en la base de datos
- Permitir al usuario seleccionar pistas en la vista de detalles
- Pasar las selecciones a mpv mediante `--aid` y `--sid` al lanzar el reproductor

## 2. Opciones consideradas

| Opción | Descripción | Ventajas | Desventajas |
|--------|-------------|----------|-------------|
| **A. ffprobe (JSON)** | Ejecutar `ffprobe -print_format json -show_streams -select_streams a,s` al indexar | Herramienta estándar de FFmpeg, salida JSON estructurada, disponible en la mayoría de distribuciones Linux | Requiere ffmpeg/ffprobe instalado, ejecución de proceso externo durante indexación |
| **B. libavformat (FFmpeg bindings Rust)** | Usar bindings Rust directos a libavformat | Sin proceso externo, integración más estrecha | Bindings Rust para FFmpeg menos maduros, mayor complejidad, dependencias adicionales |
| **C. mpv --list-properties** | Consultar pistas a través del socket IPC de mpv | No requiere ffprobe separado | Solo disponible tras lanzar mpv, no útil para mostrar antes de reproducir |
| **D. Detección basada en extensión** | Infiernir pistas del nombre de archivo | Simple, sin dependencias externas | No fiable, no detecta pistas reales del contenedor |

## 3. Decisión

**Se adopta la opción A: ffprobe con salida JSON.**

Razones:
1. **Estándar de la industria**: ffprobe es parte de FFmpeg, la herramienta más usada para inspeccionar archivos multimedia en Linux.
2. **Salida JSON estructurada**: Fácil de parsear en Rust con `serde_json`, menos propenso a errores que parsear texto plano.
3. **Selección por tipo**: `-select_streams a,s` filtra solo streams de audio y subtítulos, reduciendo el volumen de datos.
4. **Disponibilidad**: ffmpeg/ffprobe está disponible en los repositorios de todas las distribuciones Linux principales.
5. **Ya disponible en el entorno**: El plan de la Fase G indica que `ffmpeg`/`ffprobe` ya están disponibles en el entorno.

## 4. Consecuencias

### Positivas
- Detección precisa de pistas de audio y subtítulos por archivo
- Información almacenada en BD (`media_streams` con idioma, códec, título)
- Usuario puede seleccionar pistas antes de reproducir
- mpv recibe `--aid` y `--sid` correctos (índices 1-based por tipo)

### Negativas
- Requiere ffprobe instalado en el sistema (dependencia externa)
- Ejecución de proceso externo durante indexación (puede ser lento para muchos archivos)
- Los índices de ffprobe (globales) difieren de los de mpv (por tipo), requiere mapeo correcto

## 5. Alternativas descartadas

- **libavformat bindings**: Bindings Rust para FFmpeg menos maduros, añadiría complejidad y dependencias.
- **mpv IPC**: Solo funciona tras lanzar mpv, no permite mostrar pistas antes de reproducir.
- **Detección por extensión**: No fiable para determinar pistas reales.

## 6. Mapeo de índices ffprobe → mpv

ffprobe numera streams globalmente (0, 1, 2, ...), mientras que mpv numera audio y subtítulos por separado empezando en 1 (`--aid=1`, `--sid=1`). La aplicación filtra streams por tipo (`audio` / `subtitle`) y numera secuencialmente por tipo antes de pasar a mpv.

---

**Estado**: Aprobado  
**Fecha**: 2026-11-01  
**Autor**: Acryl development team
