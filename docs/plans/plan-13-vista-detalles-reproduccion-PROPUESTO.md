# Plan: Nueva vista de detalles de reproducción tipo Jellyfin

## Problemas reportados

### 1. Al darle a 'p' sobre una serie, siempre empieza desde el episodio 1

Cuando el usuario selecciona una serie y presiona 'p', la reproducción inicia desde el primer episodio en lugar de respetar el episodio seleccionado o el estado de reproducción guardado.

### 2. Enter debería abrir una vista de detalles (tipo Jellyfin), no reproducir directamente

Actualmente Enter no tiene acción asignada en la biblioteca (solo 'p' reproduce). El usuario pide que:

- **Enter**: abra una nueva vista de detalles de reproducción (poster a la izquierda, descripción/actores/pistas de audio/subtítulos a la derecha)
- **'p'**: siga reproduciendo directamente desde el principio (comportamiento actual)

## Archivos implicados

| Archivo                                         | Rol                                                                           |
| ----------------------------------------------- | ----------------------------------------------------------------------------- |
| `src/ui/App.tsx`                                | Handlers de teclado (Enter, 'p'), estado de vista de detalles de reproducción |
| `src/ui/views/Library.tsx`                      | Vista de biblioteca, renderizado de items                                     |
| `src/ui/components/MediaDetails.tsx`            | Panel de detalles existente (referencia para diseño)                          |
| `src/ui/views/PlayDetails.tsx` (nuevo)          | Nueva vista de detalles de reproducción tipo Jellyfin                         |
| `src/ui/types.ts`                               | Nuevo `ViewName` 'play-details'                                               |
| `src/ui/theme.ts`                               | Colores/tema para la nueva vista                                              |
| `tests/integration/ui/App.integration.test.tsx` | Tests de integración para Enter y 'p'                                         |

## Plan de trabajo (hitos y commits atómicos)

### Hito 1: Añadir nuevo ViewName 'play-details' y estado en App.tsx

**Archivos:** `src/ui/types.ts`, `src/ui/App.tsx`

- Añadir `'play-details'` al tipo `ViewName`
- Añadir estado `playDetailsVisible` y `selectedMediaForPlay` en App
- Handler para Enter que abre la vista de detalles de reproducción
- Handler para 'p' que reproduce directamente (ya existe, mantener)

**Criterio de aceptación:** Presionar Enter sobre un título abre la nueva vista; presionar 'p' reproduce directamente.

---

### Hito 2: Crear componente PlayDetails (vista tipo Jellyfin)

**Archivos:** `src/ui/views/PlayDetails.tsx` (nuevo)

Vista con:

- Poster/título a la izquierda
- Descripción, sinopsis, actores a la derecha
- Selección de pista de audio
- Selección de pista de subtítulos
- Botón/ataque para reproducir
- Tecla 'q' o Escape para volver

**Criterio de aceptación:** La vista se renderiza correctamente con poster, sinopsis y selecciones de audio/subtítulos.

---

### Hito 3: Navegación entre biblioteca y vista de detalles de reproducción

**Archivos:** `src/ui/App.tsx`

- 'q' / Escape en PlayDetails vuelve a la biblioteca
- Enter en un título de la biblioteca abre PlayDetails
- 'p' en PlayDetails reproduce el título seleccionado

**Criterio de aceptación:** Se puede navegar biblioteca → play-details → biblioteca, y reproducir desde PlayDetails con 'p'.

---

### Hito 4: Pruebas y verificación

**Archivos:** `tests/integration/ui/App.integration.test.tsx`, `tests/unit/`

- Test: Enter abre vista de detalles de reproducción
- Test: 'p' reproduce directamente
- Test: Escape/q vuelve desde play-details a biblioteca
- Ejecutar `npm run verify` (lint, format, typecheck, test) y que pase en verde

---

## Reglas del proyecto a respetar

- No usar `any`, no usar `as` sin justificación.
- Tipado estricto absoluto.
- Manejo de errores con clases tipadas o `Result<T, E>`, no `throw new Error(...)` genéricos.
- Commits Conventional Commits con scope (`ui`, `cli`, `config`, etc.).
- `npm run verify` debe pasar antes de push.
- Una rama por feature, PR a `main`, merge con squash.
- Los cambios en scripts de instalación y README van en la misma PR si es necesario.

## Pendiente de confirmación con el usuario

- ¿Qué información mostrar exactamente en la vista de detalles (actores, año, género, etc.)?
- ¿Las pistas de audio/subtítulos se obtienen del archivo de vídeo o son estáticas por ahora?
