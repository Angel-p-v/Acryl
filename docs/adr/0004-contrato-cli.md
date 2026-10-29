# ADR 0004 — Contrato de la CLI _headless_

- **Estado:** aceptada
- **Fecha:** 2026-10-22

## Contexto

La CLI (`mm`) es la otra cara de la TUI y también una herramienta para scripts. Eso
obliga a fijar qué es estable y qué no: un humano necesita mensajes legibles, pero un
script necesita JSON parseable, códigos de salida predecibles y ausencia de colores.
Además, Commander toma por defecto decisiones (código 1 para errores de uso) que no
distinguen «has escrito mal el comando» de «la base de datos está corrupta».

## Decisión

- **Códigos de salida:** `0` éxito · `1` error de ejecución (configuración inválida,
  base de datos, título inexistente…) · `2` uso incorrecto (comando u opción
  desconocida, argumento obligatorio ausente, valor inválido).
- **`--json` es el contrato estable.** Cada comando imprime un único objeto JSON por
  `stdout`, sin ANSI y con claves en español/inglés fijas; los cambios incompatibles
  requieren un `BREAKING CHANGE`.
- **La salida humana no es estable.** Se puede mejorar el formato, los colores o los
  mensajes sin considerarlo ruptura.
- **Los errores van a `stderr`** y nunca contaminan el JSON de `stdout`.
- **La configuración se pasa con `-c/--config`** en cada subcomando, o con `MM_CONFIG`.
  No hay opción global porque `mm` sin argumentos abre la TUI y no debe cargar la
  configuración dos veces.
- **Los filtros se validan en el _parser_ de Commander**, de modo que un `--type`
  inválido termina con código 2 antes de abrir la base de datos.
- **La búsqueda usa FTS5** (`media_fts MATCH`), no `LIKE`: el plan de consulta usa el
  índice virtual y escala con la biblioteca. Un `LIKE '%texto%'` se descarta porque
  obliga a un escaneo completo.

## Alternativas consideradas

- **Todo en JSON también para humanos**: incómodo de leer y obliga a un formateador
  externo (`jq`) para el uso diario.
- **Código 1 para todo**: los scripts no pueden distinguir un error de uso de un fallo
  real; los tests E2E tampoco.
- **Salida con colores siempre**: rompe tuberías y `--json`.
- **Búsqueda con `LIKE`**: simple, pero sin índice de texto y con peor comportamiento
  cuanto más crece la biblioteca.

## Consecuencias

- Los tests E2E comprueban `stdout`, `stderr` y `exitCode` del binario compilado.
- Los tests de integración fijan la forma del JSON con _snapshots_.
- Añadir un campo a un objeto JSON es compatible; renombrarlo o quitarlo no.
- La TUI puede reutilizar `LibraryService` sin pasar por Commander ni por la capa CLI.
