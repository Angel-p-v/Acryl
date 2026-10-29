# Plan: Edición de configuración desde la TUI

## Objetivo

Permitir al usuario modificar los ajustes de `media-manager` directamente desde la interfaz de terminal (TUI), sin tener que editar el archivo YAML manualmente ni usar la CLI.

## Contexto actual

- El instalador (PR #190, mergeado en `main`) ya crea `~/.config/media-manager/config.yaml` por defecto tras la instalación.
- El comando `mm config --init` crea el archivo de configuración si falta.
- Actualmente la TUI puede **leer** y **mostrar** la configuración, pero no permite **modificarla** y guardarla de vuelta a disco.

## Alcance (in-scope)

1. Capa de dominio (`core/config`): función para escribir la configuración serializada a YAML en disco, con validación de esquema.
2. Capa de comandos (`commands/config.ts`): opcionalmente, una opción `--set` para establecer clave/valor desde CLI (reutilizable por la TUI).
3. Capa de UI (`ui/`): nueva pantalla "Ajustes" en la TUI con campos editables y botón de guardar.
4. Navegación: entrada en el menú principal/barra lateral para acceder a "Ajustes".
5. Tests unitarios e integración para la nueva funcionalidad.

## Fuera de alcance (out-of-scope)

- Sincronización remota de configuración.
- Perfiles de configuración múltiples.
- Edición de metadatos de biblioteca desde ajustes (eso es otra pantalla).

## Arquitectura y reglas del proyecto

- `core/**` nunca importa desde `ui/**` ni `commands/**`.
- `ui/**` y `commands/**` nunca escriben SQL; acceden a datos por repositorios/funciones de dominio.
- Tipado estricto absoluto: nada de `any`, nada de `as` sin justificación.
- Modelado con `interface` para contratos expansibles, `type` para uniones.
- Manejo de errores con tipos personalizados o `Result<T, E>`, no `throw new Error(...)` genérico.
- Cada función nueva lleva su test; nombres `describe > it` en Given/When/Then.
- Commits pequeños y atómicos (Conventional Commits, scope `ui`, `config`, `cli`).
- `npm run verify` debe pasar antes de push.

## Pasos de implementación

### Paso 1: Función de escritura de configuración en `core/config/loader.ts`

- Añadir `writeConfig(configPath: string, config: AppConfig): Promise<void>`.
- Serializar `AppConfig` a YAML usando la librería `yaml` (ya disponible como dependencia).
- Validar el objeto resultante contra el esquema Zod antes de escribir.
- Crear el directorio padre si no existe (`mkdirSync(recursive: true)`).
- Devolver un tipo `Result<void, ConfigError>` o lanzar `ConfigError` tipado.
- Test unitario: `tests/unit/core/config/write-config.test.ts`.

### Paso 2: Opción `--set` en `mm config` (opcional pero recomendado)

- Añadir `--set <clave=valor>` a `ConfigCommandOptions` y registrarlo en `cli.tsx`.
- Permitir establecer una clave anidada (p. ej. `player.default=vlc`).
- Reutilizar `writeConfig` del Paso 1.
- Test unitario: `tests/unit/commands/config-set.test.ts`.

### Paso 3: Pantalla de Ajustes en la TUI (`ui/`)

- Crear componente `SettingsScreen` (Ink + React).
- Campos editables mínimos:
  - `player.default` (select: mpv, vlc)
  - `player.onLaunch` (select: detach, foreground)
  - `database.path` (input de texto)
  - `library.roots[]` (lista de rutas, añadir/eliminar)
  - `ui.theme` (select)
  - `ui.layout` (select: split, single)
  - `logging.level` (select: warn, info, debug, error)
  - keybindings principales (input de texto)
- Estado local controlado con `useState`.
- Botón "Guardar" que llama a la función de escritura de configuración (inyectada vía props, no singleton global).
- Mensaje de confirmación/error tras guardar.
- Test de render (Ink testing library si existe, o snapshot).

### Paso 4: Navegación desde la TUI

- Añadir entrada "Ajustes" en el menú principal o barra lateral.
- Al seleccionar, renderizar `SettingsScreen`.
- Al guardar, volver a la pantalla anterior con la configuración recargada.

### Paso 5: Integración y tests

- Test de integración: abrir TUI, navegar a Ajustes, cambiar un valor, guardar, verificar que `config.yaml` se actualizó en disco.
- Test de integración: verificar que `mm config --path` refleja los cambios tras guardar desde TUI.
- Actualizar `PLAN.md` y `README.md` si es necesario.

### Paso 6: Verificación y merge

- Ejecutar `npm run verify` (lint, format, typecheck, test).
- Crear rama `feat/ui-settings-config-editor`.
- Commit atómico por paso.
- PR a `main`, revisar CI, merge con squash.

## Criterios de aceptación

1. El usuario puede abrir una pantalla de "Ajustes" desde la TUI.
2. Puede modificar al menos: jugador predeterminado, ruta de base de datos, nivel de logging y tema.
3. Al pulsar "Guardar", los cambios se persisten en `config.yaml`.
4. `mm config --path` apunta al archivo modificado y `mm config --validate` lo considera válido.
5. Los cambios se reflejan al reiniciar la TUI (la configuración se recarga desde disco).
6. `npm run verify` pasa en verde.
7. Al menos un test de integración cubre el flujo completo TUI → guardar → verificar en disco.

## Estimación

- Paso 1: 1-2 horas
- Paso 2: 1 hora
- Paso 3: 4-6 horas
- Paso 4: 1-2 horas
- Paso 5: 2-3 horas
- Paso 6: 1 hora
  Total estimado: ~12-15 horas
