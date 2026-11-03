# ADR-0021: Identidad estable de pistas e historial activo único

## 1. Contexto

Acryl guarda por archivo la pista de audio y subtítulos elegida y registra la
reproducción en `playback_history`. Dos problemas aparecieron al usar la
aplicación con series:

1. La preferencia de pistas se guardaba solo por índice (`audio_stream_index`,
   `subtitle_stream_index`). Ese índice no identifica una pista de forma
   duradera: dos episodios pueden compartir el mismo número y un contenedor
   puede reordenar sus streams. Además, los subtítulos externos junto al vídeo
   no se indexaban ni se pasaban a mpv.
2. Cada salida y entrada al mismo episodio insertaba otra fila en el historial.
   El historial acumulaba duplicados pendientes y la reanudación podía leer una
   posición antigua.

También se detectó que lanzar mpv con `--fullscreen` desde una ventana
maximizada hacía que Hyprland reubicara y encogiera Acryl. El modo de ventana de
mpv debe derivarse del estado real de la ventana de Acryl, no asumirse.

## 2. Opciones consideradas

### Identidad de pistas

| Opción | Ventajas | Desventajas |
|--------|----------|-------------|
| **A. Solo índice** | Simple, sin migración | No identifica la pista si cambia el orden; puede reutilizar la pista equivocada en otro episodio |
| **B. Solo metadatos** | Estable ante reordenamientos | Puede haber pistas con metadatos idénticos; no cubre subtítulos externos |
| **C. Clave estable + índice de respaldo** | Conserva bases antiguas, resuelve sidecars por ruta y evita reutilizar pistas ausentes | Requiere migración y más lógica de resolución |

### Subtítulos externos

| Opción | Ventajas | Desventajas |
|--------|----------|-------------|
| **A. Dejar que mpv auto-cargue sidecars** | Sin cambios | `--sid` no es determinista; no se puede guardar una preferencia por archivo |
| **B. Indexar sidecars y pasarlos con `--sub-file` + `--sub-auto=no`** | Identidad por ruta absoluta y `--sid` estable | Hay que descubrir y persistir las rutas |

### Historial

| Opción | Ventajas | Desventajas |
|--------|----------|-------------|
| **A. Borrar duplicados antiguos** | Tabla limpia | Pierde historial legítimo |
| **B. Archivar duplicados y usar índice único parcial** | Conserva historial, garantiza una entrada activa y resiste guardados concurrentes | Requiere migración y consultas algo más complejas |

## 3. Decisión

Se adopta la opción C para pistas, la opción B para subtítulos externos y la
opción B para el historial.

- La preferencia guarda una clave estable:
  - `external:<ruta absoluta>` para subtítulos externos.
  - `embedded:<tipo>:<idioma>:<códec>:<título>` para pistas embebidas.
  - El índice se conserva como respaldo para bases de datos antiguas.
- Los sidecars se indexan en `media_streams` con `external_path` y se pasan a
  mpv con `--sub-file` y `--sub-auto=no`.
- El historial usa `completed_at IS NULL` como entrada activa, un índice único
  parcial sobre `file_path` y UPSERT para el guardado. Los duplicados antiguos
  se archivan con `completed_at = played_at`, sin eliminarlos.
- El modo de ventana de mpv se deriva de `is_fullscreen()` / `is_maximized()` y
  se reafirma al terminar la reproducción.

## 4. Consecuencias

### Positivas

- Cada episodio conserva su propia pista y no hereda la de otro.
- Un reordenamiento de streams no selecciona la pista equivocada.
- Los subtítulos externos funcionan con una identidad estable y `--sid`
  determinista.
- El historial no crece con duplicados pendientes y conserva el historial
  completado.
- La ventana de Acryl no cambia de estado al abrir o cerrar mpv.

### Negativas

- Tres migraciones nuevas (`13`, `14`, `15`) y más lógica de resolución.
- Los metadatos idénticos de dos pistas embebidas siguen siendo ambiguos; en
  ese caso se elige la primera coincidencia.
- Una base de datos indexada antes de este cambio descubre los sidecars al
  reproducir o al abrir el selector, pero no los persiste hasta reindexar.

## 5. Alternativas descartadas

- **Solo índice:** es la causa del problema; no se puede distinguir un episodio
  de otro ni un reordenamiento.
- **Borrar duplicados del historial:** destruye datos legítimos del usuario.
- **Forzar siempre `--fullscreen` en mpv:** provoca el cambio de estado de la
  ventana en Hyprland y no respeta la elección del usuario.

---

**Estado**: Aprobado
**Fecha**: 2026-11-03
**Autor**: Acryl development team
