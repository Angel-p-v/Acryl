# Plan: Corregir errores de ajustes (settings) de mm

## Problemas reportados

1. **No se puede cambiar nada en los ajustes**: Al entrar en modo edición de un campo (select o texto), aparecen las opciones/el cursor, pero al guardar no se persiste nada. En `Settings.tsx`, `saveEdit()` verifica `if (!item || !configPath)` y devuelve temprano con _"No se puede guardar: no hay ruta de configuración."_ cuando `configPath` es `undefined` (p. ej. primera ejecución sin archivo de config YAML real, solo valores por defecto).

2. **El reproductor se llama "jugador"**: En `Settings.tsx` línea 74, el label del select del reproductor predeterminado está hardcodeado como `'Jugador'`. Debería ser `'Reproductor'` o `'Reproductor predeterminado'` para ser consistente con el resto de la UI y la documentación.

3. **`mm config --path` devuelve una ruta que no existe**: `resolveConfigPath()` resuelve la ruta del archivo de configuración pero no verifica (ni crea) el archivo. `mm config --path` imprime la ruta aunque el archivo YAML no exista, y al intentar editarla desde los ajustes falla porque no hay nada que guardar (relacionado con el problema 1).

## Archivos implicados

- `src/ui/views/Settings.tsx` — vista de ajustes de la TUI, `buildItems`, `saveEdit`, label `'Jugador'`.
- `src/commands/config.ts` — comando `mm config`, `--path`, `resolveConfigPath`.
- `src/core/config/loader.ts` — `resolveConfigPath`, `saveConfig`.
- `src/ui/App.tsx` — pasa `configPath` a `SettingsView`.
- `src/shared/locales/es.json` — (opcional) si se decide usar i18n para el label.

## Plan de trabajo (hitos y commits atómicos)

### Hito 1: Corregir etiqueta "Jugador" → "Reproductor predeterminado"

**Archivos:** `src/ui/views/Settings.tsx`

Cambiar el label del ítem `player-default` de `'Jugador'` a `'Reproductor predeterminado'` (o `'Reproductor'`). Verificar que no haya otras referencias a "jugador" en la UI que deban ajustarse.

**Criterio de aceptación:** En la TUI, el campo del reproductor predeterminado muestra "Reproductor predeterminado" en lugar de "Jugador".

---

### Hito 2: Permitir guardar cambios en ajustes aunque no exista archivo de config

**Archivos:** `src/ui/views/Settings.tsx`, `src/core/config/loader.ts`, `src/ui/App.tsx`

El problema es que `saveEdit()` abandona si `configPath` es `undefined`. Cuando no hay archivo de configuración YAML (primera ejecución, valores por defecto), `configPath` puede no estar definido o apuntar a un archivo inexistente.

Opciones a evaluar (elegir una con el usuario):

- **Opción A:** En `App.tsx`, asegurar que `configPath` siempre tenga un valor válido (la ruta por defecto de `defaultConfigPath`) antes de crear `App`, aunque el archivo aún no exista. Luego `saveConfig` crearía el archivo al guardar.
- **Opción B:** En `Settings.tsx`, si `configPath` es `undefined`, usar `resolveConfigPath()` para obtener la ruta por defecto y permitir el guardado, creando el archivo si no existe.

Además, revisar el `useEffect` de sincronización de ítems (línea 175) para asegurar que no resetee el estado de edición innecesariamente.

**Criterio de aceptación:** Desde la TUI, se puede entrar en modo edición de cualquier campo de ajustes, cambiar el valor, presionar Enter y el cambio se persiste en el archivo YAML (se crea si no existía).

---

### Hito 3: `mm config --path` debe indicar si el archivo existe o crearlo

**Archivos:** `src/commands/config.ts`

Actualmente `mm config --path` imprime la ruta resuelta aunque el archivo no exista. Mejorar para:

- Mostrar una advertencia si el archivo no existe (p. ej. `"ruta/al/config.yaml (no existe, se usará por defecto)"`).
- O, opcionalmente, añadir una bandera `--init` ya existente que cree el archivo. Documentar que `--path` solo informa la ruta, y `--init` crea el archivo.

**Criterio de aceptación:** `mm config --path` deja claro si el archivo de configuración existe o no, y el usuario sabe que debe usar `mm config --init` para crearlo antes de editarlo desde los ajustes.

---

### Hito 4: Verificación y pruebas

**Archivos:** `tests/unit/`, `tests/integration/`

- Añadir tests unitarios para `saveEdit` / guardado de config sin archivo previo.
- Añadir test de integración para `mm config --path` cuando el archivo no existe.
- Ejecutar `npm run verify` (lint, format, typecheck, test) y que pase en verde.

---

## Reglas del proyecto a respetar

- No usar `any`, no usar `as` sin justificación.
- Tipado estricto absoluto.
- Manejo de errores con clases tipadas o `Result<T, E>`, no `throw new Error(...)` genéricos.
- Commits Conventional Commits con scope (`ui`, `cli`, `config`, etc.).
- `npm run verify` debe pasar antes de push.
- Una rama por feature, PR a `main`, merge con squash.

## Pendiente de confirmación con el usuario

- Confirmar cuál de las Opción A / Opción B prefiere para el Hito 2.
- Confirmar si el label debe ser exactamente "Reproductor predeterminado" o "Reproductor".
