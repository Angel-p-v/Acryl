# ADR 0002 — Configuración en YAML validada con Zod

- **Estado:** aceptada
- **Fecha:** 2026-10-22

## Contexto

La aplicación necesita poder configurarse fuera del código: rutas de la biblioteca,
patrones de exclusión, reproductores, preferencias de interfaz y atajos de teclado.
El archivo se edita a mano, así que los errores de escritura son probables y deben
detectarse con un mensaje útil, no con un `undefined` en mitad de la indexación.

## Decisión

- Formato **YAML** ( legible, con comentarios y anidación natural).
- El archivo vive en `~/.config/media-manager/config.yaml` respetando `XDG_CONFIG_HOME`
  y se puede sobrescribir con `MM_CONFIG`.
- El esquema se declara **una sola vez** en `src/core/config/schema.ts` con **Zod**.
- La carga sigue esta precedencia: `valores por defecto → archivo → variables de entorno`.
- Los arrays se **reemplazan** al fusionar (no se concatenan) para que el usuario pueda
  redefinir `library.exclude` sin arrastrar los valores de fábrica.
- Las rutas relativas se resuelven contra el directorio del archivo de configuración y
  `~` se expande al directorio personal.
- El esquema se puede exportar a JSON Schema (`z.toJSONSchema`) para documentarlo.

## Alternativas consideradas

- **TOML**: sintaxis más estricta, pero menos expresiva para listas de objetos y sin
  ventaja clara aquí.
- **JSON**: no admite comentarios y es unpleasant de editar a mano.
- **Lua**: daría una configuración programable, pero obliga a incrustar un intérprete y
  descarta la validación estática en el arranque.
- **Validación a mano**: más código y más riesgo de inconsistencias que un esquema
  declarativo único.

## Consecuencias

- Un error de configuración se detecta **al arrancar**, con la ruta del campo exacta.
- Los valores por defecto están duplicados conceptualmente entre `defaults.ts` y los
  `.default()` del esquema; `defaults.ts` es la fuente de verdad para la fusión y el
  esquema garantiza la forma final. Semantenerán sincronizados con un test.
- Añadir una opción implica tocar el esquema, los valores por defecto y la plantilla
  `config/config.example.yaml`.
