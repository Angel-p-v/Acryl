# ADR 0006 — Reproductor desacoplado y modo _watch_

- **Estado:** aceptada
- **Fecha:** 2026-10-22

## Contexto

Reproducir un vídeo implica lanzar un proceso externo (`mpv`, `vlc` u otro) que puede
durar horas. La TUI no puede quedarse esperando ni perder el control de la terminal, y el
reproductor debe seguir vivo cuando el usuario cierre `mm`. Además, una serie tiene varios
ficheros y el índice puede cambiar en disco mientras la aplicación está abierta.

Había que decidir cómo construir el comando sin una shell, cómo desacoplar el proceso,
qué fichero reproducir por defecto y cómo reaccionar a los cambios del sistema de ficheros.

## Decisión

### Launcher

- La configuración define `command` y una lista de `args`. Cada argumento se pasa a
  `spawn` por separado, con `shell: false`. No existe una cadena de shell ni interpolación
  de variables del sistema.
- La plantilla debe contener `{file}` en algún argumento. Se sustituye en **todos** los
  argumentos que lo contengan, preservando una ruta con espacios como un único argv.
  Si falta el marcador se lanza `PlayerError`: abrir el reproductor sin fichero sería un
  fallo silencioso.
- Antes de lanzar se comprueba que el binario exista y sea ejecutable, buscándolo en
  `PATH` cuando no es una ruta. Así el error se devuelve aunque el proceso se desacople
  inmediatamente.
- `detach`: `detached: true`, `stdio: 'ignore'` y `unref()`. El hijo no comparte la
  terminal, no bloquea y sobrevive al cierre de la TUI.
- `foreground`: `stdio: 'inherit'` y `wait()` espera el cierre. Un código distinto de 0 o
  una señal se convierten en `PlayerError`.

### Selección del fichero

- `LibraryService.find` devuelve los ficheros ordenados por temporada y episodio.
- `mm play` usa el primero por defecto y permite `--file <id>`. No se inventa un estado de
  "visto" que la base de datos todavía no guarda.
- La TUI reproduce el primer fichero del título seleccionado. El panel de detalle muestra
  los ids de fichero para poder usar `--file` desde la CLI.

### Integración con la TUI

- La TUI siempre lanza en `detach`, independientemente de `player.onLaunch`. Un
  `foreground` en la TUI congelaría la interfaz y mezclaría la salida del reproductor con
  la pantalla alternativa.
- El lanzamiento es asíncrono: la barra de estado muestra "Abriendo…" y la interfaz sigue
  respondiendo. Si el binario falla, el error se muestra en la misma barra.
- No se espera el proceso hijo; el desacople es responsabilidad del launcher.

### Modo _watch_

- chokidar v6 observa las raíces. Solo los eventos de ficheros de vídeo (`add`, `change`,
  `unlink`) y `unlinkDir` programan una reindexación.
- Los eventos se agrupan con un debounce de 400 ms para no indexar durante una copia.
- Se ejecuta `indexLibrary` completo, que ya es incremental. Filtrar por raíz desactivaría
  la poda de huérfanos del indexador; la reindexación completa mantiene los borrados
  correctos sin duplicar lógica en el watcher.
- `mm watch` hace una indexación inicial, espera a que chokidar esté listo y se cierra
  ordenadamente con `SIGINT`/`SIGTERM`. Los errores de una reindexación se muestran y el
  watcher continúa.
- `--json` no se ofrece en `watch`: es un flujo continuo y rompería el contrato de "un
  objeto JSON por ejecución" del ADR 0004.

## Alternativas consideradas

- **Construir una línea de comandos y usar `shell: true`**: cómodo para plantillas, pero
  interpreta metacaracteres y rompe rutas con espacios o `$`. Se descarta por seguridad.
- **Lanzar siempre en foreground y esperar**: bloquea la TUI y el cierre de `mm` mataría el
  reproductor.
- **Lanzar siempre en detach también en CLI**: impide usar el reproductor desde scripts que
  necesitan saber cuándo termina. Se mantienen ambos modos.
- **Elegir el "siguiente episodio sin ver"**: requiere una tabla de progreso que no existe;
  prometerlo sería incorrecto. Queda para un hito futuro.
- **Watcher que reindexa solo la raíz afectada**: el indexador no poda borrados en modo
  filtrado. Se prefiere una pasada completa incremental, más simple y correcta.
- **Un plugin de reproducción o integración con IPC de mpv**: fuera del alcance de M6.

## Consecuencias

- El reproductor es un proceso independiente: cerrar `mm` no lo detiene en modo `detach`.
- La configuración puede apuntar a cualquier binario, no solo `mpv`/`vlc`.
- `mm play --dry-run` es seguro y testeable sin instalar el reproductor.
- El modo _watch_ puede reindexar de más si cambian muchos ficheros a la vez, pero el
  debounce y la indexación incremental limitan el coste.
- La TUI no refleja automáticamente los cambios detectados por `mm watch` en otra
  instancia; habría que refrescar o añadir un watcher compartido en un hito posterior.
