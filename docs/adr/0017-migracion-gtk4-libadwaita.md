# ADR-0017: Migración de Tauri/WebView a GTK4/Libadwaita (Sin motor Web)

## 1. Contexto

La aplicación Acryl utiliza actualmente Tauri v2 con WebKit2GTK como motor de renderizado para la interfaz de usuario (React + TypeScript dentro de un WebView). Esto ha generado problemas críticos en entornos Wayland/Hyprland, donde WebKit2GTK no renderiza correctamente la interfaz, resultando en pantallas en blanco o aplicaciones que no arrancan.

Se han agotado todas las configuraciones de entorno conocidas:
- `GDK_BACKEND=x11`
- `WEBKIT_DISABLE_DMABUF_RENDERER=1`
- `WEBKIT_DISABLE_COMPOSITING_MODE=1`
- `LIBGL_ALWAYS_SOFTWARE=1`
- `XDG_SESSION_TYPE=x11`

Además, la arquitectura actual con Tauri + frontend web introduce complejidad innecesaria:
- Dos ecosistemas de programación (Rust + TypeScript/React)
- Comunicación vía IPC serializada (JSON)
- Dependencia de Node.js y npm para el frontend
- Tamaño de binario mayor por incluir el WebView
- Mayor consumo de memoria por el motor Chromium/WebKit embebido

## 2. Opciones consideradas

| Opción | Descripción | Ventajas | Desventajas |
|--------|-------------|----------|-------------|
| **A. CEF / Chromium Embedded Framework** | Incrustar Chromium (Blink) directamente | Máxima compatibilidad web, sin problemas de WebKit2GTK | Aumenta tamaño (100-200 MB), consumo RAM (400-800 MB), requiere bindings Rust para CEF (menos maduros), salida de Tauri |
| **B. GTK4 / Libadwaita nativo** | Reescribir UI en Rust + GTK4/Libadwaita | Rendimiento óptimo, sin motor de navegador, soporte nativo Wayland, menor RAM, binario más pequeño | Requiere reescribir toda la UI, curva de aprendizaje GTK4 |
| **C. Mantener Tauri + WebKit2GTK** | Seguir con la arquitectura actual | No requiere reescritura | No funciona en Wayland/Hyprland, problema no resuelto |

## 3. Decisión

**Se adopta la opción B: Migración a GTK4 / Libadwaita nativo.**

Razones:
1. **Funcionalidad en Wayland/Hyprland**: GTK4 tiene soporte nativo y maduro para Wayland, resolviendo el problema crítico de pantalla en blanco.
2. **Rendimiento**: Sin motor de navegador embebido, menor consumo de RAM y CPU.
3. **Simplicidad**: Un solo lenguaje (Rust), un solo ecosistema, sin IPC serializado entre frontend y backend.
4. **Mantenibilidad**: Menos dependencias, menos capas, código más fácil de mantener.
5. **Tamaño de distribución**: Binario más pequeño al no incluir WebView.

## 4. Consecuencias

### Positivas
- Aplicación funciona correctamente en Wayland/Hyprland
- Menor consumo de memoria y CPU
- Binario más pequeño (sin WebView embebido)
- Código unificado en Rust
- Mejor integración con el escritorio Linux (notificaciones, atajos de teclado, etc.)
- No requiere Node.js ni npm para desarrollo ni ejecución

### Negativas
- Requiere reescribir toda la interfaz de usuario de React/TypeScript a Rust + GTK4
- Curva de aprendizaje para GTK4/Libadwaita
- Pérdida del ecosistema de componentes web (CSS, librerías UI web)
- Tiempo estimado de migración: 10-12 semanas adicionales

## 5. Alternativas descartadas

- **CEF/Chromium**: Demasiado pesado (100-200 MB adicional), alto consumo de memoria, bindings Rust para CEF menos maduros que Tauri.
- **Mantener Tauri**: No resuelve el problema fundamental en Wayland/Hyprland.

## 6. Plan de migración (Hitos E.1 a E.5)

| Hito | Contenido | Tiempo estimado |
|------|-----------|-----------------|
| E.1 | Prototipo mínimo GTK4 (ventana, lista, póster, búsqueda) | 2 semanas |
| E.2 | UI completa GTK4 (detalles, favoritos, visto, navegación) | 2-3 semanas |
| E.3 | Reproductor integrado con GtkMediaFile en GTK4 | 1-2 semanas |
| E.4 | Empaquetado nativo Linux sin Tauri | 1 semana |
| E.5 | Limpieza y documentación (eliminar frontend/Tauri) | 1 semana |

**Total estimado**: 10-12 semanas

## 7. Tecnologías a eliminar

| Tecnología | Motivo |
|------------|--------|
| Tauri v2 | Depende de WebView del sistema (WebKit2GTK) |
| WebKit2GTK | No renderiza en Wayland/Hyprland |
| Node.js | Solo se usaba para el frontend React |
| npm / yarn | Solo para dependencias del frontend |
| Vite | Herramienta de construcción del frontend |
| React | Framework de UI web |
| TypeScript | Lenguaje del frontend |
| @tauri-apps/api | API de comandos Tauri |
| better-sqlite3 | SQLite para desarrollo frontend |
| Tauri CLI | Herramienta de construcción Tauri |

## 8. Tecnologías a agregar

| Tecnología | Propósito |
|------------|-----------|
| gtk4 | Interfaz gráfica nativa |
| libadwaita | Componentes de UI modernos (GNOME style) |
| gdk-pixbuf | Decodificación de imágenes (pósters) |
| cairo | Gráficos 2D |

---

**Estado**: Aprobado  
**Fecha**: 2026-10-29  
**Autor**: Acryl development team
