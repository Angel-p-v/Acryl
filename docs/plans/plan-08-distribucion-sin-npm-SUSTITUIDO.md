# Plan: distribución sin npm mediante instaladores — 🔄 SUSTITUIDO

> **Estado:** sustituido por [`instaladores-y-seguridad-COMPLETADO.md`](./instaladores-y-seguridad-COMPLETADO.md).
> **Índice de planes:** [`docs/plans/README.md`](./README.md)

- **Estado:** sustituido por [`instaladores-y-seguridad-COMPLETADO.md`](./instaladores-y-seguridad-COMPLETADO.md)
- **Fecha:** 2026-10-22
- **Sustituye:** la publicación en npm descrita en el
  [ADR 0008](../adr/0008-publicacion-y-distribucion.md)

## 1. Objetivo

Distribuir `media-manager-cli` sin publicar en el registro de npm:

- GitHub Releases como canal de distribución.
- Artefactos portables que incluyan Node y `better-sqlite3`.
- `install.sh` y `uninstall.sh` para Linux y macOS.
- Sin `NPM_TOKEN` ni credenciales de npm.
- npm queda limitado al desarrollo y a la construcción de artefactos.

El comando instalado seguirá siendo `mm`.

## 2. Alcance inicial

| Elemento                      | Primera versión                                   |
| ----------------------------- | ------------------------------------------------- |
| Plataformas                   | Linux x64 y Linux arm64                           |
| macOS                         | Después de validar Linux; mismos scripts POSIX    |
| Windows                       | Fuera de alcance; requeriría `install.ps1`        |
| Runtime                       | Node incluido en el artefacto portable            |
| Dependencia nativa            | `better-sqlite3` incluida en el artefacto         |
| Configuración y base de datos | En rutas XDG, fuera del directorio de instalación |

No se tocarán las carpetas de la biblioteca multimedia durante la instalación ni la
desinstalación.

## 3. Decisiones propuestas

1. **Eliminar npm como canal de publicación.** No se mantiene `publish.yml`, no se exige
   `NPM_TOKEN` y no se documenta `npm install --global media-manager-cli`.
2. **Instalación portable por defecto.** El usuario no necesita Node ni npm.
3. **Instalación por usuario.** Sin `sudo` por defecto; se instala en `~/.local`.
4. **Desinstalación conservadora.** Configuración y base de datos se conservan salvo que
   se use `--purge`.
5. **Versiones inmutables.** Cada release tiene su tag y sus artefactos; el instalador
   permite fijar una versión con `--version`.

## 4. Estructura de instalación

```text
~/.local/share/media-manager-cli/
├── 1.0.0/
│   ├── mm
│   ├── node
│   ├── node_modules/
│   │   └── media-manager-cli/
│   ├── config/
│   ├── docs/
│   ├── install.json
│   └── ...
└── current -> 1.0.0

~/.local/bin/
└── mm                 # wrapper gestionado por el instalador
```

`current` es un enlace simbólico al directorio de la versión activa. El wrapper de
`~/.local/bin/mm` ejecuta `current/mm` mediante una ruta absoluta:

```sh
#!/bin/sh
# managed by media-manager-cli installer
exec "/home/usuario/.local/share/media-manager-cli/current/mm" "$@"
```

No se enlaza directamente el `mm` del bundle porque su lanzador calcula `HERE` a partir
de `$0`; un symlink desde otro directorio rompería la resolución de `node` y
`node_modules`.

`install.json` guarda, como mínimo:

```json
{
  "name": "media-manager-cli",
  "version": "1.0.0",
  "prefix": "/home/usuario/.local",
  "appDir": "/home/usuario/.local/share/media-manager-cli",
  "binary": "/home/usuario/.local/bin/mm",
  "installedAt": "2026-10-22T00:00:00.000Z"
}
```

## 5. `install.sh`

Script POSIX `sh`, compatible con Linux y macOS. Debe funcionar tanto desde un archivo
local como desde una descarga de GitHub.

### Opciones

```text
--version <tag>     versión a instalar; por defecto, la última release
--prefix <dir>      prefijo de instalación; por defecto, ~/.local
--bindir <dir>      directorio del comando mm; por defecto, <prefix>/bin
--file <tar.gz>     instala desde un archivo local
--sha256 <file>     checksum del archivo local
--force             reinstala aunque la versión ya exista
--dry-run           muestra lo que haría sin modificar el sistema
--help              ayuda
```

### Flujo

1. Comprobar herramientas: `tar`, `curl` o `wget`, y `sha256sum` o `shasum`.
2. Detectar sistema y arquitectura:
   - `Linux` + `x86_64` → `linux-x64`
   - `Linux` + `aarch64`/`arm64` → `linux-arm64`
   - `Darwin` + `arm64` → `macos-arm64` (cuando exista artefacto)
3. Resolver la versión:
   - `--version` explícito, o
   - redirección de `releases/latest` para obtener el tag.
4. Construir la URL del artefacto:
   `https://github.com/Angel-p-v/media-manager/releases/download/<tag>/media-manager-cli-<version>-<plataforma>.tar.gz`
5. Descargar el artefacto y su `.sha256` a un directorio temporal creado con `mktemp`.
6. Verificar el checksum; abortar si falta o no coincide.
7. Extraer en el directorio temporal y comprobar:
   - existe `mm`;
   - existe `node`;
   - existe `node_modules/media-manager-cli/dist/cli.js`;
   - la versión de `install.json` coincide con la solicitada.
8. Instalar en `<prefix>/share/media-manager-cli/<version>/`.
9. Actualizar el enlace `current` de forma atómica (crear el nuevo y renombrarlo).
10. Crear o actualizar el wrapper `<bindir>/mm` con la marca del instalador.
11. Ejecutar `<bindir>/mm --version` y comprobar que devuelve la versión esperada.
12. Avisar si `<bindir>` no está en `PATH`; no modificar `.bashrc`, `.zshrc` ni similares.
13. Limpiar el directorio temporal incluso si hay error.

### Seguridad y robustez

- Solo URLs HTTPS.
- Checksum obligatorio; sin checksum no se instala.
- No usar `sudo`, `eval` ni construir comandos con rutas sin citar.
- Extraer siempre dentro de un directorio temporal.
- No sobrescribir una instalación existente sin `--force`.
- No eliminar versiones anteriores automáticamente durante una actualización.
- Rechazar archivos que no tengan la estructura esperada.
- Avisar si se ejecuta como `root`, aunque se permite un `--prefix` explícito.
- Documentar que el bundle es específico de plataforma; en Alpine/musl puede no funcionar
  si no se publica un artefacto compatible.

### Configuración

El instalador **no crea ni modifica** la configuración por defecto. Al terminar muestra:

```text
Copia config/config.example.yaml a ~/.config/media-manager/config.yaml
y ajusta tus carpetas antes de ejecutar mm index.
```

Se puede añadir más adelante `--init-config`, pero no forma parte del primer alcance.

## 6. `uninstall.sh`

Script POSIX con estas opciones:

```text
--prefix <dir>   prefijo usado en la instalación
--bindir <dir>   directorio del wrapper
--purge          elimina también configuración y base de datos
--dry-run        muestra lo que haría sin borrar nada
--yes            no pide confirmación
--help           ayuda
```

### Comportamiento

1. Localizar `install.json` en la instalación.
2. Mostrar versión, rutas y elementos que se van a eliminar.
3. Pedir confirmación, salvo `--yes` o `--dry-run`.
4. Eliminar `<bindir>/mm` solo si contiene la marca del instalador.
5. Eliminar `<prefix>/share/media-manager-cli`.
6. Conservar por defecto:
   - `~/.config/media-manager/`
   - `~/.local/share/media-manager/` (incluye `library.db`)
7. Con `--purge`, eliminar también esas dos rutas tras una segunda confirmación.
8. Nunca eliminar rutas de bibliotecas configuradas por el usuario.
9. Si no hay instalación, informar y salir con código 0.

## 7. Artefactos de release

Se reutiliza `npm run standalone`, que ya genera:

```text
build/standalone/media-manager-cli-<versión>-<plataforma>.tar.gz
```

### Nombres

```text
media-manager-cli-1.0.0-linux-x64.tar.gz
media-manager-cli-1.0.0-linux-x64.tar.gz.sha256
media-manager-cli-1.0.0-linux-arm64.tar.gz
media-manager-cli-1.0.0-linux-arm64.tar.gz.sha256
```

Cada `.sha256` contiene el hash y el nombre del archivo, en formato compatible con
`sha256sum -c`.

### Workflow

- Crear un workflow reutilizable `release-artifacts.yml` con `workflow_call` y un input
  `tag`.
- `release.yml` se dispara con `release: published` y llama a `release-artifacts.yml`.
- `release-please.yml` llama al mismo workflow cuando `release_created == 'true'`, igual
  que hace hoy con `publish.yml`. Esto evita depender de eventos creados con
  `GITHUB_TOKEN`.
- Matriz inicial:
  - `ubuntu-24.04` → `linux-x64`
  - `ubuntu-24.04-arm` → `linux-arm64` (si el runner está disponible en el plan)
- Pasos por plataforma:
  1. checkout del tag;
  2. `npm ci`;
  3. `npm run verify`;
  4. `npm run standalone`;
  5. generar `.sha256`;
  6. `gh release upload <tag> <archivo> <archivo>.sha256 --clobber`.
- Permisos: `contents: write` para subir assets. Sin `id-token` ni `NPM_TOKEN`.
- Añadir un modo `workflow_dispatch` para construir y probar artefactos sin publicar una
  release.

### Licencias

Al redistribuir Node y las dependencias hay que incluir los avisos de licencia:

- `LICENSE` del proyecto.
- Licencia del runtime Node incluido.
- Licencias de las dependencias de producción dentro del bundle.

Añadir un `THIRD_PARTY_NOTICES.md` al artefacto y verificar que se genera en el build.

## 8. Cambios en el repositorio

### Añadir

- `install.sh`
- `uninstall.sh`
- `.github/workflows/release-artifacts.yml`
- `tests/integration/installer.integration.test.ts` (o equivalente)
- `docs/adr/0009-distribucion-github-releases.md`
- `THIRD_PARTY_NOTICES.md` o el mecanismo equivalente de avisos

### Modificar

- `.github/workflows/release.yml`: delegar en `release-artifacts.yml`.
- `.github/workflows/release-please.yml`: llamar a `release-artifacts.yml`.
- `README.md`: instalación y desinstalación desde GitHub Releases.
- `docs/guides/release.md`: eliminar requisitos de npm y documentar assets/checksums.
- `AGENTS.md`: nuevo flujo de release y gotchas de los instaladores.
- `PLAN.md`: cambiar el alcance de M8 y marcar la publicación npm como sustituida.
- `docs/adr/0008-publicacion-y-distribucion.md`: marcar la parte de npm como
  reemplazada por el ADR 0009.
- `package.json`: quitar `publishConfig` y scripts relacionados con la publicación npm;
  conservar `prepack`, `files` y `smoke` mientras sigan siendo útiles para construir y
  validar el tarball.

### Eliminar

- `.github/workflows/publish.yml`
- Referencias a `NPM_TOKEN` en workflows y documentación.
- Instrucciones de `npm install --global media-manager-cli`.

## 9. Pruebas

### Unitarias/integración

Usar un `HOME` y unas rutas XDG temporales. Crear un artefacto de prueba pequeño con la
estructura esperada y un `mm` simulado que responda a `--version`.

Casos mínimos:

- instalación desde archivo local con checksum correcto;
- creación del wrapper y del enlace `current`;
- `mm --version` devuelve la versión esperada;
- instalación repetida sin `--force` falla de forma controlada;
- checksum ausente o incorrecto aborta;
- plataforma no soportada aborta con un mensaje claro;
- `--dry-run` no modifica nada;
- desinstalación elimina la aplicación y el wrapper;
- desinstalación conserva configuración y base de datos;
- `--purge` elimina también configuración y base de datos;
- el wrapper ajeno no se borra si no tiene la marca del instalador.

### CI

- `shellcheck` sobre `install.sh` y `uninstall.sh`.
- `sh -n` como comprobación rápida de sintaxis.
- Job de integración en Ubuntu x64.
- Smoke real: en la release, descargar el asset publicado, verificar checksum, instalar en
  un `HOME` temporal y ejecutar `mm --version`.
- La matriz de artefactos no necesita ejecutarse en cada PR; sí en tags/releases y bajo
  `workflow_dispatch`.

### Prueba manual antes de `v1.0.0`

1. Construir el artefacto Linux x64.
2. Subirlo a una release de prueba o servirlo localmente.
3. Ejecutar `install.sh` en un contenedor o usuario limpio sin Node.
4. Comprobar `mm --version`, `mm config --validate` y una indexación de prueba.
5. Ejecutar `uninstall.sh` y verificar que la configuración y la base siguen ahí.
6. Ejecutar `uninstall.sh --purge` y verificar que desaparecen.

## 10. Documentación de usuario

README:

```bash
# Instalar la última versión
curl -fsSL https://raw.githubusercontent.com/Angel-p-v/media-manager/main/install.sh \
  -o /tmp/mm-install.sh
sh /tmp/mm-install.sh

# Instalar una versión concreta
sh /tmp/mm-install.sh --version v1.0.0

# Desinstalar conservando configuración y base de datos
curl -fsSL https://raw.githubusercontent.com/Angel-p-v/media-manager/main/uninstall.sh \
  -o /tmp/mm-uninstall.sh
sh /tmp/mm-uninstall.sh

# Desinstalar y borrar también configuración y base de datos
sh /tmp/mm-uninstall.sh --purge
```

También se documentará la descarga manual del `.tar.gz` y la instalación desde el código
fuente para desarrollo.

## 11. Fases de implementación

1. **Scripts y pruebas locales**
   - `install.sh`, `uninstall.sh`.
   - Artefacto de prueba y tests de integración.
   - `shellcheck` y `sh -n`.
2. **CI y release**
   - `release-artifacts.yml`.
   - Integración con `release.yml` y `release-please.yml`.
   - Checksums y avisos de licencias.
3. **Documentación y ADR**
   - README, `docs/guides/release.md`, `AGENTS.md`, `PLAN.md`.
   - ADR 0009 y actualización del ADR 0008.
4. **Release candidate**
   - Construir artefactos Linux x64/arm64.
   - Probar instalación/desinstalación desde la release.
5. **`v1.0.0`**
   - Crear tag y GitHub Release.
   - Adjuntar artefactos y checksums.
   - Verificar la instalación desde cero.
   - Marcar M8 como completado.

## 12. Criterios de aceptación

- [ ] `install.sh` instala `mm` en Linux x64 sin Node ni npm instalados.
- [ ] El checksum se verifica antes de instalar.
- [ ] `uninstall.sh` elimina la aplicación y el wrapper, pero conserva configuración y
      base de datos por defecto.
- [ ] `uninstall.sh --purge` elimina también configuración y base de datos.
- [ ] El wrapper `mm` funciona desde cualquier directorio y no depende de un symlink
      frágil.
- [ ] La release publica `.tar.gz` y `.sha256` para cada plataforma soportada.
- [ ] `release.yml` y release-please suben los artefactos sin `NPM_TOKEN`.
- [ ] CI valida sintaxis, shellcheck y el flujo de instalación/desinstalación.
- [ ] README, `docs/guides/release.md`, `PLAN.md`, `AGENTS.md` y ADR están actualizados.
- [ ] La instalación desde la GitHub Release se prueba antes de cerrar M8.

## 13. Decisiones pendientes de confirmar

1. ¿Eliminar npm por completo o mantenerlo como canal secundario? La propuesta recomienda
   eliminarlo.
2. ¿Linux x64 y arm64 en la primera release, o solo Linux x64?
3. ¿Se añade macOS en esta misma release o se pospone?
4. ¿Se incluye `--init-config` en `install.sh` o se deja para una versión posterior?
5. ¿Se mantiene `npm run standalone` como generador del artefacto o se crea un script
   independiente de npm?

## 14. Riesgos

- **Tamaño del bundle:** incluye Node y dependencias; puede superar los 50 MB.
- **Compatibilidad glibc/musl:** un binario Linux no funciona en todas las
  distribuciones. Hay que documentar la plataforma soportada.
- **Actualizaciones:** no hay autoactualización; el usuario debe volver a ejecutar
  `install.sh`.
- **Confianza en el instalador:** se mitiga con HTTPS, checksum y descarga inspeccionable,
  pero no sustituye a una firma criptográfica.
- **Mantenimiento de la matriz:** cada plataforma añade tiempo de CI y un artefacto que
  probar.
- **Licencias:** redistribuir Node exige incluir sus avisos legales.
