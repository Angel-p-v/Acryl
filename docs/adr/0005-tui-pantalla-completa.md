# ADR 0005 — TUI de pantalla completa con estado local

- **Estado:** aceptada
- **Fecha:** 2026-10-22

## Contexto

La TUI debe navegar una biblioteca que puede tener miles de títulos, responder al teclado
sin parpadeos y compartir con la CLI la misma capa de datos. Ink renderiza React en la
terminal, pero no impone ni el modelo de estado ni el ciclo de vida de la pantalla. Había
que decidir cómo ocupar la terminal, dónde vivir el estado de navegación y cómo evitar
renderizar listas enormes.

## Decisión

- **Pantalla alternativa** (`\u001b[?1049h` / `\u001b[?1049l`) al entrar y salir de la TUI.
  La terminal recupera su contenido anterior al cerrar y no se mezclan los logs con la
  interfaz.
- **Estado local en `App`** con `useState`: vista (`library`, `search`, `settings`), sección,
  índice seleccionado, consulta y visibilidad de la ayuda. No hay store global ni
  singleton: la TUI se monta con un `LibraryService` inyectado y se puede testear con un
  doble.
- **Consultas síncronas memoizadas.** SQLite es síncrono; `useLibrary` y `useMediaDetails`
  recalculan con `useMemo` al cambiar sección o consulta, sin estados de carga.
- **Ventana de filas.** El servicio devuelve como máximo 200 títulos y `MediaList` renderiza
  solo 12 alrededor de la selección. Así el primer frame no depende del tamaño de la
  biblioteca; el techo de 300 ms con 10 000 títulos está cubierto por un test.
- **Atajos desde YAML.** `useConfig` fusiona `defaultConfig.keybindings` con el archivo del
  usuario. Las acciones que no aplican a la vista actual se ignoran en el handler (por
  ejemplo, `q` dentro del buscador es texto).
- **Ayuda modal que sustituye el contenido principal**, no un panel apilado: en una
  terminal de 80×24 el panel inferior se recortaba.
- **La ayuda y los ajustes son de solo lectura**; editar YAML desde la TUI queda fuera de
  este hito.
- **La reproducción sigue siendo de la CLI/M6.** La tecla `p` muestra un aviso honesto en
  lugar de fingir que lanza un reproductor.

## Alternativas consideradas

- **Renderizar en la pantalla normal**: deja restos de la interfaz y compite con los logs;
  la pantalla alternativa es el comportamiento esperado en una TUI.
- **Un store global (Zustand/Redux)**: añade dependencia y estado compartido para un árbol
  de componentes pequeño; el estado cabe en `App`.
- **Renderizar todos los títulos**: sencillo, pero el primer frame y cada pulsación crecen
  con la biblioteca.
- **Panel de ayuda apilado debajo del contenido**: se recorta en terminales pequeñas.
- **Implementar `mm play` en M5**: mezcla hitos y obliga a decidir el contrato del
  reproductor antes de M6.

## Consecuencias

- La TUI se puede probar sin terminal real con `ink-testing-library` y un `LibraryService`
  falso.
- El estado se pierde al cerrar: no hay sesión persistente ni “último título visto” todavía.
- La lista muestra 200 títulos como máximo por sección; la búsqueda FTS5 es la forma de
  llegar a colecciones mayores.
- `run.tsx` es el único punto que abre y cierra la base de datos para la TUI.
- Las decisiones de reproducción se aplazan a M6, donde vive el launcher.
