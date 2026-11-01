# ADR-0018: Virtualización de galería de medios con GtkGridView

## 1. Contexto

La galería de medios de Acryl necesita renderizar miles de ítems (pósters, títulos, años) sin bloquear la interfaz GTK4. El plan inicial (Fase G, Hito G.2) contemplaba usar `GtkListView` o `GtkColumnView` para virtualización. Sin embargo, durante la implementación se optó por `GtkGridView` con `GListStore` + `SignalListItemFactory`.

La galería debe cumplir estos requisitos:
- Solo renderizar los ítems visibles en pantalla (virtualización)
- Organizar los ítems en múltiples columnas (rejilla), no un ítem por fila
- Adaptarse al ancho de la ventana (reflow dinámico de columnas)
- Tarjetas uniformes con dimensiones fijas (216×404 px)
- Reutilizar widgets de tarjetas para evitar recargas innecesarias de pósters

## 2. Opciones consideradas

| Opción | Descripción | Ventajas | Desventajas |
|--------|-------------|----------|-------------|
| **A. GtkGridView** | Rejilla con múltiples columnas, virtualización nativa | Organización en columnas, adaptación al ancho, renderiza solo visibles, reutiliza widgets | API más reciente (GTK 4.10+), menos ejemplos en ecosistema Rust |
| **B. GtkListView** | Lista vertical con un ítem por fila | Virtualización madura, amplia documentación | Un ítem por fila no es adecuado para galería de pósters |
| **C. GtkColumnView** | Tabla con columnas definidas | Virtualización, estructura tabular | No es una galería fluida, requiere definir columnas fijas |
| **D. GtkGrid (sin virtualización)** | Contenedor simple sin virtualización | Simple de implementar | Carga todos los ítems en memoria, no escala para colecciones grandes |

## 3. Decisión

**Se adopta la opción A: GtkGridView con GListStore + SignalListItemFactory.**

Razones:
1. **Organización en columnas**: `GtkGridView` organiza los ítems en una rejilla con `min_columns` y `max_columns`, ideal para una galería de pósters.
2. **Adaptación al ancho**: Se adapta automáticamente al ancho de la ventana cambiando el número de columnas por fila.
3. **Virtualización**: Renderiza solo los ítems visibles, eliminando advertencias de GTK al medir miles de ítems.
4. **Reutilización de widgets**: `SignalListItemFactory` crea widgets una vez y los vincula (`bind`) a nuevos datos, evitando recargas de pósters.
5. **Cumple GTK 4.10+**: Acryl ya requiere GTK 4.10+ para `GtkFileDialog`, por lo que no hay restricción adicional.

## 4. Consecuencias

### Positivas
- Scroll fluido con bibliotecas grandes (miles de medios)
- No hay advertencias de GTK por medir `GtkGrid` con miles de ítems
- Tarjetas se reutilizan, reduciendo allocations y descargas de pósters
- Galería responsiva que se adapta al ancho de la ventana

### Negativas
- `GtkGridView` es más reciente que `GtkListView`, menos ejemplos disponibles en el ecosistema Rust/GTK
- Se requiere `gtk4` con feature `v4_10` (ya presente en `Cargo.toml`)

## 5. Alternativas descartadas

- **GtkListView**: Un ítem por fila no es adecuado para una galería visual de pósters.
- **GtkColumnView**: Estructura tabular rígida, no una galería fluida.
- **GtkGrid sin virtualización**: No escala para colecciones grandes, bloquea la UI al cargar miles de ítems.

---

**Estado**: Aprobado  
**Fecha**: 2026-11-01  
**Autor**: Acryl development team
