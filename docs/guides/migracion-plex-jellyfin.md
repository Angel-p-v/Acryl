# Guía de migración desde Plex / Jellyfin a media-manager

> media-manager es un gestor de contenido multimedia 100 % terminal: indexa tu
> colección en una base de datos SQLite local y lanza los archivos en `mpv` o
> `vlc` sin salir de la terminal. No hay servidor, no hay web, no hay streaming:
> solo índice local + reproductor local.

Esta guía te ayuda a pasar de Plex o Jellyfin a media-manager reutilizando las
carpetas de medios que ya tienes y sin perder tus archivos de vídeo.

---

## 1. Qué cambia respecto a Plex / Jellyfin

| Concepto      | Plex / Jellyfin                 | media-manager                                            |
| ------------- | ------------------------------- | -------------------------------------------------------- |
| Servidor      | Siempre corriendo en background | No hay servidor; ejecutas `mm` cuando lo necesitas       |
| Interfaz      | Web / apps móviles              | Terminal (CLI headless + TUI con Ink)                    |
| Base de datos | SQLite/MySQL remota             | SQLite local (`~/.local/share/media-manager/library.db`) |
| Reproducción  | Transcodificación integrada     | Lanza el archivo directamente en `mpv` / `vlc`           |
| Metadatos     | Descargados automáticamente     | Opcionales (TMDB/TVDB) bajo demanda                      |
| Configuración | Interfaz web                    | `config.yaml` validado con Zod                           |

media-manager no transcodifica ni sirve contenido por red: lee tus archivos
directamente del disco y los abre con tu reproductor local.

---

## 2. Requisitos previos

- Node.js `>=22` (o el bundle portable descargado desde GitHub Releases)
- `mpv` o `vlc` instalado en el sistema
- Tus archivos de multimedia organizados en carpetas (igual que en Plex/Jellyfin)

Comprueba que tienes Node y un reproductor:

```bash
node --version
which mpv vlc
```

---

## 3. Instalar media-manager

### Opción A: instalador oficial (recomendado)

```bash
curl -fsSL https://raw.githubusercontent.com/Angel-p-v/media-manager/main/install.sh | sh
```

El instalador descarga el bundle portable, verifica el checksum SHA-256 y deja el
comando `mm` disponible en `~/.local/bin/mm`.

### Opción B: Docker Compose

Si ya usas Docker, puedes levantar media-manager con base de datos y config
persistentes:

```bash
git clone https://github.com/Angel-p-v/media-manager.git
cd media-manager
docker compose up -d
docker compose run --mm index --json
```

Edita `config/docker-config.yaml` para apuntar `library.roots` a tus carpetas de
medios antes de indexar.

---

## 4. Mapear tus carpetas de media

En Plex/Jellyfin probablemente tienes carpetas como:

```
/media/series/Serie X/Temporada 1/...
/media/peliculas/Pelicula Y.mkv
/media/anime/Anime Z/...
```

media-manager indexa por **raíces de biblioteca**, cada una con un tipo de
contenido (`series`, `anime`, `movie`). Edita `~/.config/media-manager/config.yaml`
(creado tras la primera ejecución) o usa `config/docker-config.yaml`:

```yaml
version: 1
library:
  roots:
    - path: /media/series
      type: series
      label: Series
    - path: /media/peliculas
      type: movie
      label: Películas
    - path: /media/anime
      type: anime
      label: Anime
  exclude:
    - '**/*.sample.*'
    - '**/sample/**'
    - '**/.hidden/**'
    - '**/@eaDir/**'
  scan:
    followSymlinks: false
    maxDepth: 4
    minSizeBytes: 0
database:
  path: ~/.local/share/media-manager/library.db
```

Las rutas pueden ser absolutas o relativas al archivo de configuración. Usa `~`
para referirte al directorio personal.

---

## 5. Indexar la biblioteca

Ejecuta la indexación para leer todos los archivos y poblar la base de datos
SQLite:

```bash
mm index
```

Opciones útiles:

| Opción            | Descripción                                |
| ----------------- | ------------------------------------------ |
| `--dry-run`       | Simula sin escribir en la base de datos    |
| `--root <ruta>`   | Indexa solo una raíz específica            |
| `--folder <ruta>` | Indexa solo una carpeta dentro de una raíz |
| `--json`          | Salida en formato JSON                     |

Ejemplo: indexar solo series

```bash
mm index --root /media/series --json
```

La indexación es **incremental**: usa la ruta + `mtime` del archivo, por lo que
reindexar no crea duplicados. Si borras un archivo del disco, su entrada queda
huérfana y se elimina automáticamente en la siguiente indexación.

---

## 6. Configurar el reproductor

Por defecto media-manager usa `mpv`. Si prefieres `vlc`, edita `config.yaml`:

```yaml
player:
  default: vlc
  onLaunch: detach
```

`onLaunch: detach` lanza el reproductor en segundo plano y no bloquea la
terminal; `foreground` espera a que termine.

Prueba la reproducción:

```bash
mm list
mm play "Nombre de la serie"
```

O desde la TUI (sin argumentos abre la interfaz interactiva):

```bash
mm
```

---

## 7. Comandos útiles durante la migración

```bash
# Ver la configuración efectiva
mm config --json

# Listar la biblioteca indexada
mm list --json

# Buscar un título
mm search "nombre" --json

# Ver detalle de un título
mm show "nombre" --json

# Diagnosticar problemas de la biblioteca
mm doctor --json

# Estadísticas de la base de datos
mm db stats --json
```

---

## 8. Diferencias clave que debes conocer

- **No hay transcodificación:** media-manager abre el archivo tal cual en
  `mpv`/`vlc`. Si un códec no se reproduce en tu reproductor, no se transcodiza.
- **No hay servidor remoto:** todo corre en tu máquina local. Para acceder desde
  otro dispositivo, necesitas montar las carpetas de medios allí.
- **Los metadatos son opcionales:** sin clave de API de TMDB/TVDB, media-manager
  solo usa los datos locales (título, año, carpeta). Puedes obtener metadatos
  externos con `mm metadata fetch <id>`.
- **La TUI se controla con teclas:** flechas para navegar, `p` para reproducir,
  `q` para salir, `?` para ayuda, `/` para buscar.

---

## 9. Solución de problemas

| Problema                         | Solución                                                                                                                |
| -------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `mm: command not found`          | Añade `~/.local/bin` a tu `PATH` o usa la ruta absoluta                                                                 |
| La base de datos no se abre      | Comprueba permisos de escritura en `database.path`                                                                      |
| No aparecen títulos tras indexar | Verifica que `library.roots` apunte a las carpetas correctas y que los archivos tengan extensiones de vídeo reconocidas |
| `mpv` no se encuentra            | Instala `mpv` o cambia `player.default` a `vlc`                                                                         |
| Metadatos no se descargan        | Configura `TMDB_API_KEY` o `TVDB_API_KEY` en el entorno o en `config.yaml`                                              |

Para diagnóstico completo:

```bash
mm doctor --full --json
```

---

## 10. Desinstalar Plex / Jellyfin (opcional)

Cuando hayas verificado que media-manager indexa y reproduce correctamente tu
colección, puedes detener y desinstalar Plex/Jellyfin. Tus archivos de vídeo no
se tocan en ningún momento: media-manager solo lee la base de datos y los
archivos originales.

---

## Siguientes pasos

- Lee [`docs/guides/architecture.md`](./architecture.md) para entender la
  arquitectura interna.
- Consulta [`docs/adr/`](../adr/) para las decisiones de diseño registradas.
- Si encuentras un problema, abre un issue en
  [GitHub](https://github.com/Angel-p-v/media-manager/issues).
