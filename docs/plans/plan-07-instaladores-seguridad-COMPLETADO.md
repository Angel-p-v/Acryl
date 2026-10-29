# Plan: instaladores, distribución y seguridad — ✅ COMPLETADO

> **Estado:** `v1.0.0` y `v1.0.1` publicadas y verificadas (2026-10-23).
> **Índice de planes:** [`docs/plans/README.md`](./README.md)

- **Estado:** completado — `v1.0.0` y `v1.0.1` publicadas y verificadas (2026-10-23)
- **Fecha:** 2026-10-22
- **Sustituye y concreta:** [`distribucion-sin-npm-SUSTITUIDO.md`](./distribucion-sin-npm-SUSTITUIDO.md)
- **Decisión de fondo:** dejar de publicar en npm y distribuir artefactos portables desde
  GitHub Releases.

## 1. Objetivo

Cerrar la release `v1.0.0` con un canal de distribución propio:

- `install.sh` y `uninstall.sh` para Linux y macOS.
- Artefactos portables con Node y `better-sqlite3` incluidos.
- Publicación de assets, checksums, SBOM y atestaciones de procedencia en GitHub Releases.
- Sin `NPM_TOKEN` ni credenciales del registro de npm.
- Seguridad verificable: descarga HTTPS, checksum obligatorio, validación del archivo,
  instalación atómica y desinstalación sin borrados peligrosos.

`npm` seguirá usándose para desarrollar, testear y construir los artefactos, pero no será
el mecanismo de instalación para el usuario final.

## 2. Decisiones de partida

1. **Canal:** GitHub Releases. No se publica en npm.
2. **Formato:** bundle portable con Node y dependencias de producción.
3. **Plataformas iniciales:** Linux x64 y Linux arm64.
4. **macOS:** fase posterior, reutilizando los mismos scripts POSIX.
5. **Windows:** fuera de alcance; requerirá scripts PowerShell.
6. **Instalación por usuario:** `~/.local` por defecto, sin `sudo`.
7. **Desinstalación conservadora:** configuración y base de datos se conservan salvo
   `--purge`.
8. **Integridad:** checksum SHA-256 obligatorio; atestación de GitHub recomendada y
   verificable.
9. **Actualizaciones:** se vuelve a ejecutar `install.sh`; no hay autoactualización.

## 3. Entregables

### Scripts

- `install.sh`
- `uninstall.sh`

### Construcción

- `scripts/build/standalone.mjs` — adaptado para metadatos, licencias y plataforma.
- `scripts/build/fetch-node.mjs` — descarga y verifica el runtime oficial de Node.
- `scripts/build/notices.mjs` — genera avisos de terceros.
- `scripts/build/platform.mjs` — normaliza sistema/arquitectura y nombres de assets.

### Release

- `.github/workflows/release-artifacts.yml` — workflow reutilizable de construcción y
  publicación de assets.
- `.github/workflows/release.yml` — wrapper para `release: published`.
- `.github/workflows/release-please.yml` — llamada al workflow reutilizable cuando
  release-please crea una release.
- `THIRD_PARTY_NOTICES.md` generado en el artefacto.
- SBOM en formato CycloneDX.
- Checksums `.sha256` por artefacto.

### Documentación

- `README.md` — instalación, actualización y desinstalación.
- `docs/guides/release.md` — nuevo flujo sin npm.
- `docs/adr/0009-distribucion-github-releases.md`.
- `SECURITY.md` — modelo de confianza y verificación de artefactos.
- `AGENTS.md`, `PLAN.md` y `CONTRIBUTING.md` — nuevo flujo y comprobaciones.

## 4. Estructura de instalación

```text
~/.local/
├── bin/
│   └── mm                         # wrapper gestionado por el instalador
└── share/
    └── media-manager-cli/
        ├── 1.0.0/                 # versión instalada
        │   ├── mm                 # lanzador interno del bundle
        │   ├── node               # runtime incluido
        │   ├── node_modules/
        │   ├── config/
        │   ├── docs/
        │   ├── install.json       # manifiesto de instalación
        │   ├── THIRD_PARTY_NOTICES.md
        │   └── ...
        └── current -> 1.0.0
```

Datos del usuario, fuera del directorio de instalación (rutas por defecto; la aplicación
respeta `XDG_CONFIG_HOME` y `XDG_DATA_HOME`):

```text
$XDG_CONFIG_HOME/media-manager/config.yaml   # por defecto ~/.config/media-manager/
$XDG_DATA_HOME/media-manager/library.db     # por defecto ~/.local/share/media-manager/
```

### Wrapper

```sh
#!/bin/sh
# managed by media-manager-cli installer
exec "/home/usuario/.local/share/media-manager-cli/current/mm" "$@"
```

No se enlaza directamente el `mm` del bundle porque su lanzador usa `$0` para localizar
`node` y `node_modules`. El wrapper ejecuta la ruta real y evita que un symlink rompa la
resolución.

### `install.json`

```json
{
  "schemaVersion": 1,
  "name": "media-manager-cli",
  "version": "1.0.0",
  "platform": "linux-x64",
  "prefix": "/home/usuario/.local",
  "appDir": "/home/usuario/.local/share/media-manager-cli",
  "binary": "/home/usuario/.local/bin/mm",
  "artifact": "media-manager-cli-1.0.0-linux-x64.tar.gz",
  "sha256": "…",
  "installedAt": "2026-10-22T00:00:00.000Z"
}
```

El `install.json` que viaja dentro del archivo contiene solo metadatos de construcción
(nombre, versión, plataforma y esquema). El instalador escribe una copia ampliada en el
directorio destino con `prefix`, `appDir`, `binary`, `artifact`, `sha256` e
`installedAt`; el hash del propio `.tar.gz` no puede viajar dentro de sí mismo.

## 5. `install.sh`

Script POSIX `sh`, probado con `dash`, `bash --posix` y el `/bin/sh` de macOS.

### Opciones

```text
--version <tag>       instala una versión concreta; por defecto, la última release
--prefix <dir>        prefijo de instalación; por defecto, ~/.local
--bindir <dir>        directorio del wrapper; por defecto, <prefix>/bin
--file <tar.gz>       instala desde un archivo local
--sha256 <archivo>    checksum del archivo local
--verify-attestation  exige verificar la atestación de GitHub con gh
--force               reinstala la misma versión
--dry-run             muestra los cambios sin aplicarlos
--no-path-check       no avisa si <bindir> no está en PATH
--help                ayuda
```

### Flujo

1. Comprobar herramientas: `tar`, `curl` o `wget`, `sha256sum` o `shasum`.
2. Detectar plataforma:
   - Linux `x86_64` → `linux-x64`
   - Linux `aarch64`/`arm64` → `linux-arm64`
   - Darwin `x86_64` → `macos-x64` (cuando exista)
   - Darwin `arm64` → `macos-arm64` (cuando exista)
3. Detectar musl frente a glibc en Linux y abortar con un mensaje claro si no hay bundle
   compatible.
4. Resolver la versión:
   - `--version` explícito, aceptando `1.0.0` y `v1.0.0`;
   - o redirección de `releases/latest` para obtener el tag;
   - rechazar versiones inexistentes antes de descargar.
5. Crear un directorio temporal con `mktemp -d`.
6. Descargar el `.tar.gz` y su `.sha256` por HTTPS.
7. Verificar el checksum **antes de extraer**.
8. Si se usa `--verify-attestation`, ejecutar:
   `gh attestation verify <archivo> --repo Angel-p-v/media-manager`.
9. Validar el nombre y la estructura del archivo antes de extraer:
   - `install.json` con `schemaVersion`, nombre, versión y plataforma correctos;
   - `mm`, `node` y `node_modules/media-manager-cli/dist/cli.js`;
   - ninguna entrada con ruta absoluta o `../`.
10. Extraer en el directorio temporal:
    `tar -xzf <archivo> -C <tmp> --strip-components=1`.
11. Crear el directorio `<prefix>/share/media-manager-cli/<versión>/`.
12. Instalar en una ruta temporal hermana y renombrar de forma atómica.
13. Escribir en el destino el `install.json` ampliado con las rutas y el checksum.
14. Actualizar `current` con un enlace temporal + `mv`.
15. Crear o actualizar `<bindir>/mm` con la marca del instalador.
16. Ejecutar `<bindir>/mm --version` y comprobar la versión.
17. Avisar si `<bindir>` no está en `PATH`.
18. Limpiar temporales y liberar el bloqueo incluso en error o `Ctrl+C`.

### Seguridad del instalador

- `set -eu`; sin `eval`; rutas siempre entre comillas.
- `umask 022`.
- Bloqueo de instalación con `mkdir` para evitar ejecuciones concurrentes.
- `trap` para limpiar temporales con `EXIT`, `INT` y `TERM`.
- No usar `sudo`; si se ejecuta como root, avisar y exigir un `--prefix` explícito.
- No sobrescribir una instalación existente sin `--force`.
- No borrar versiones anteriores automáticamente.
- Checksum obligatorio, incluso con `--file`.
- No ejecutar ningún binario del bundle antes de verificar checksum y, si se solicita,
  atestación.
- No modificar `.bashrc`, `.zshrc` ni el `PATH`.
- No crear ni sobrescribir la configuración del usuario.
- Mensajes de error a `stderr` y códigos de salida estables:
  - `0` éxito;
  - `1` error de red o checksum;
  - `2` uso incorrecto;
  - `3` plataforma no soportada;
  - `4` instalación existente sin `--force`.

## 6. `uninstall.sh`

### Opciones

```text
--prefix <dir>   prefijo usado en la instalación
--bindir <dir>   directorio del wrapper
--purge          elimina también configuración y base de datos
--dry-run        muestra lo que haría
--yes            no pide confirmación
--help           ayuda
```

### Flujo

1. Leer `install.json` de la instalación.
2. Mostrar versión, rutas y elementos afectados.
3. Pedir confirmación salvo `--yes` o `--dry-run`.
4. Verificar que `appDir` resuelto está dentro de `<prefix>/share/media-manager-cli` y no
   es `/`, `$HOME`, un directorio padre del usuario ni una ruta vacía.
5. Eliminar `<bindir>/mm` solo si contiene la marca del instalador.
6. Eliminar `<prefix>/share/media-manager-cli`.
7. Conservar por defecto:
   - `${XDG_CONFIG_HOME:-$HOME/.config}/media-manager/`
   - `${XDG_DATA_HOME:-$HOME/.local/share}/media-manager/`
8. Con `--purge`, pedir una segunda confirmación y eliminar también esas dos rutas.
   Si el usuario configuró una base de datos fuera de la ruta por defecto, no se toca.
9. No seguir symlinks ni borrar rutas fuera de las conocidas.
10. Nunca tocar las carpetas de la biblioteca multimedia.
11. Si no existe instalación, informar y salir con `0`.

### Pruebas de seguridad del desinstalador

- `--prefix /` debe rechazarse.
- `--prefix "$HOME"` debe rechazarse.
- Un wrapper sin la marca del instalador no debe borrarse.
- Un `appDir` manipulado en `install.json` no debe permitir borrar rutas arbitrarias.
- `--purge --dry-run` no debe borrar nada.
- La desinstalación normal no debe tocar configuración, base de datos ni biblioteca.

## 7. Construcción del artefacto

### Runtime de Node

El artefacto de release no debe copiar el Node del runner sin más. Hay que:

1. Fijar la versión exacta del runtime (por ejemplo, la versión LTS usada en CI).
2. Descargar el tarball oficial desde `nodejs.org`.
3. Verificar el SHA-256 contra `SHASUMS256.txt`.
4. Extraer `bin/node` y `LICENSE`.
5. Incluir la licencia de Node en `THIRD_PARTY_NOTICES.md`.

`npm run standalone` puede ofrecer dos modos:

- `--node-source official`: descarga y verifica el runtime oficial; obligatorio en CI.
- `--node-source system`: usa `process.execPath` para desarrollo local.

### Contenido del bundle

- `dist/` compilado.
- `config/config.example.yaml`.
- `docs/` y README.
- `node_modules` de producción.
- `install.json`.
- `THIRD_PARTY_NOTICES.md`.
- `LICENSE`.
- Runtime Node y su licencia.
- Checksums fuera del `.tar.gz` (como assets hermanos).

### Nombres de assets

```text
media-manager-cli-1.0.0-linux-x64.tar.gz
media-manager-cli-1.0.0-linux-x64.tar.gz.sha256
media-manager-cli-1.0.0-linux-arm64.tar.gz
media-manager-cli-1.0.0-linux-arm64.tar.gz.sha256
sbom.cdx.json
```

Cada `.sha256` incluye el hash y el nombre del archivo para que
`sha256sum -c`/`shasum -a 256 -c` funcione.

## 8. Release y CI

### `release-artifacts.yml`

Workflow reutilizable con input `tag` y permisos mínimos por job:

- `contents: write` para subir assets.
- `id-token: write` y `attestations: write` para la atestación.
- `packages: read` no es necesario.

Matriz inicial:

| Runner             | Asset         |
| ------------------ | ------------- |
| `ubuntu-24.04`     | `linux-x64`   |
| `ubuntu-24.04-arm` | `linux-arm64` |

Pasos por plataforma:

1. Checkout del tag con `persist-credentials: false`.
2. `npm ci`.
3. `npm run verify`.
4. `npm run standalone -- --node-source official`.
5. Generar el `install.json` de construcción y `THIRD_PARTY_NOTICES.md`.
6. Generar `.sha256`.
7. `actions/attest-build-provenance` sobre el `.tar.gz` y el `.sha256`.
8. Subir assets con `gh release upload` sin `--clobber`.
9. Generar el SBOM y subirlo.

### Integración con release-please

- `release.yml` se dispara con `release: published` y llama a `release-artifacts.yml`.
- `release-please.yml` llama al mismo workflow cuando `release_created == 'true'`.
- No depender de que el evento `release` vuelva a disparar workflows creados con
  `GITHUB_TOKEN`.

### Eliminación de npm

- Eliminar `.github/workflows/publish.yml`.
- Eliminar `NPM_TOKEN` de documentación y workflows.
- Eliminar `publishConfig` de `package.json`.
- Mantener `prepack`, `files` y `smoke` mientras sigan sirviendo para construir y validar
  el tarball interno.
- Reescribir `docs/guides/release.md` para GitHub Releases.
- Crear el ADR 0009 y marcar el ADR 0008 como parcialmente sustituido.

## 9. Seguridad de la cadena de suministro

### Controles obligatorios

- HTTPS con `curl --proto '=https' --tlsv1.2`.
- Checksum SHA-256 obligatorio.
- Atestación de procedencia generada en cada release; su verificación es opcional en el
  instalador y obligatoria en CI.
- `npm ci` con `package-lock.json` bloqueado.
- Scripts de instalación de dependencias limitados por `allowScripts`.
- Workflows con permisos mínimos y sin secretos en PRs.
- `npm audit --omit=dev --audit-level=high` en CI, bloqueante para vulnerabilidades
  altas o críticas de dependencias de producción.
- SBOM CycloneDX adjunto a cada release.
- Acciones de terceros fijadas por SHA de commit, no por tag flotante.
- Dependabot para npm y GitHub Actions.
- `shellcheck` y `sh -n` para los instaladores.

### Atestaciones

- Generar atestación con `actions/attest-build-provenance`.
- Documentar la verificación manual:

  ```bash
  gh attestation verify media-manager-cli-1.0.0-linux-x64.tar.gz \
    --repo Angel-p-v/media-manager
  ```

- `install.sh --verify-attestation` debe fallar si `gh` no está o la verificación no es
  correcta.
- La atestación no sustituye al checksum: son controles complementarios.

### Riesgos aceptados

- El checksum y el asset viven en el mismo GitHub Release; la atestación mejora la
  integridad, pero la cuenta de GitHub sigue siendo la raíz de confianza.
- El bundle Linux es dependiente de glibc; no se promete compatibilidad con Alpine/musl.
- No hay firma GPG/minisign en la primera versión; se puede añadir después.
- No hay actualización automática; el usuario controla cuándo reinstala.

## 10. Pruebas

### Integración local

Crear un artefacto de prueba pequeño con la estructura esperada y un `mm` simulado.

Casos obligatorios:

- [x] Instalación desde archivo local con checksum correcto.
- [x] Instalación desde URL de release (job `install-test` en un contenedor limpio).
- [x] `current` apunta a la versión instalada.
- [x] Wrapper creado y `mm --version` correcto.
- [x] Instalación en rutas con espacios.
- [x] `--prefix` y `--bindir` personalizados.
- [x] Reinstalación sin `--force` falla.
- [x] Checksum ausente, inválido o de otro archivo aborta.
- [x] Archivo con `../` se rechaza (la comprobación de rutas absolutas está implementada).
- [x] Plataforma no soportada aborta con código 3.
- [x] `--dry-run` no modifica nada.
- [x] Desinstalación normal elimina app y wrapper.
- [x] Desinstalación conserva configuración y base de datos.
- [x] `--purge` elimina configuración y base de datos.
- [x] `--prefix /` y `--prefix "$HOME"` se rechazan.
- [x] Wrapper ajeno sin marca no se borra.
- [x] `install.json` manipulado no permite borrar rutas arbitrarias.
- [x] `--verify-attestation` falla si la atestación no es válida.

### CI

- `shellcheck install.sh uninstall.sh`. ✅
- `sh -n install.sh uninstall.sh` y `dash -n`. ✅
- Job de integración de instalación/desinstalación en Ubuntu x64. ✅
- Prueba en contenedor `ubuntu:24.04` sin Node instalado (job `install-test`). ✅
  1. descargar el asset de la release;
  2. verificar checksum;
  3. ejecutar `install.sh`;
  4. comprobar `mm --version`;
  5. desinstalar y comprobar que la configuración sigue.
- Validar que los assets de la release existen y que el `.sha256` coincide. ✅
- Validar la atestación en CI con `gh attestation verify`. ✅

### Prueba manual antes de `v1.0.0`

1. Construir Linux x64 y, si está disponible, Linux arm64.
2. Crear una release candidata.
3. Instalar en un usuario o contenedor limpio sin Node.
4. Ejecutar `mm --version`, `mm config --validate` y una indexación pequeña.
5. Actualizar a otra versión y comprobar que `current` cambia.
6. Desinstalar y comprobar que configuración y base de datos siguen.
7. Ejecutar `--purge` y comprobar que desaparecen.

## 11. Documentación de usuario

README:

```bash
# Descargar el instalador y revisarlo antes de ejecutarlo
curl -fsSL https://raw.githubusercontent.com/Angel-p-v/media-manager/main/install.sh \
  -o /tmp/mm-install.sh
less /tmp/mm-install.sh
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

También se documentará:

- descarga manual del `.tar.gz`;
- verificación de checksum y atestación;
- instalación con `--prefix` personalizado;
- comportamiento si `~/.local/bin` no está en `PATH`;
- plataformas soportadas;
- instalación desde código fuente para desarrollo.

## 12. Fases de ejecución

### Fase 1 — Contratos y esqueleto

- [x] Crear ADR 0009 con la decisión de GitHub Releases.
- [x] Definir `install.json` y el nombre de assets.
- [x] Crear `install.sh` y `uninstall.sh` con `--help`.
- [x] Añadir `shellcheck` y `sh -n` al CI.

### Fase 2 — Instalación segura

- [x] Implementar detección de plataforma y versión.
- [x] Implementar descarga, checksum y validación de archivo.
- [x] Implementar instalación atómica, `current` y wrapper.
- [x] Implementar `--dry-run`, `--force` y bloqueo.
- [x] Implementar desinstalación conservadora y `--purge`.
- [x] Añadir tests de integración de los casos obligatorios.

### Fase 3 — Artefactos y release

- [x] Adaptar `standalone.mjs` para `install.json` y avisos.
- [x] Implementar descarga verificada del Node oficial.
- [x] Generar `.sha256` y SBOM.
- [x] Crear `release-artifacts.yml` con matriz Linux.
- [x] Integrar `release.yml` y `release-please.yml`.
- [x] Eliminar `publish.yml` y las referencias a npm.
- [x] Añadir atestaciones de procedencia.

### Fase 4 — Seguridad y documentación

- [x] Fijar acciones de GitHub por SHA.
- [x] Añadir `npm audit` y revisar la configuración de Dependabot.
- [x] Actualizar README, release, SECURITY, AGENTS y PLAN.
- [x] Documentar checksum y atestación.
- [x] Probar instalación en contenedor sin Node (job `install-test`, al publicar).

### Fase 5 — Release

- [x] Construir y probar artefactos Linux x64/arm64.
- [x] Publicar `v1.0.0` con los assets de ambas plataformas.
- [x] Instalar desde la release y desinstalar en un entorno limpio.
- [x] Crear `v1.0.0`.
- [x] Adjuntar assets, checksums, SBOM y atestaciones.
- [x] Corregir la expansión de `~` en los instaladores y publicar `v1.0.1`.
- [x] Marcar M8 como completado.

## 13. Criterios de aceptación

- [x] `install.sh` instala `mm` en Linux x64 sin Node ni npm instalados.
- [x] `install.sh` verifica checksum antes de extraer y ejecutar.
- [x] `install.sh` rechaza archivos manipulados o plataformas no soportadas.
- [x] `install.sh` no requiere `sudo` ni modifica el `PATH`.
- [x] `uninstall.sh` conserva configuración y base de datos por defecto.
- [x] `uninstall.sh --purge` elimina también configuración y base de datos.
- [x] El wrapper funciona desde cualquier directorio.
- [x] La release publica `.tar.gz`, `.sha256`, SBOM y atestación.
- [x] El flujo de release no usa `NPM_TOKEN`.
- [x] CI ejecuta `shellcheck`, pruebas de instalación y validación de assets.
- [x] La instalación desde la release se prueba en un entorno limpio.
- [x] README, release, SECURITY, AGENTS, PLAN y ADR están actualizados.
- [x] M8 se cierra solo después de publicar y verificar la release.

## 14. Riesgos y mitigaciones

| Riesgo                                     | Mitigación                                                    |
| ------------------------------------------ | ------------------------------------------------------------- |
| Asset comprometido                         | Checksum + atestación + revisión del workflow                 |
| MITM en la descarga                        | HTTPS estricto y checksum                                     |
| Archivo malicioso con rutas `../`          | Validación de entradas antes de extraer                       |
| Borrado accidental en la desinstalación    | Rutas conocidas, marcas, comprobaciones y `--dry-run`         |
| Ejecución como root                        | Aviso y rechazo salvo prefijo explícito                       |
| Incompatibilidad glibc/musl                | Detección y mensaje claro; assets separados si se soporta     |
| Bundle demasiado grande                    | Documentar tamaño y ofrecer descarga manual                   |
| Node incluido desactualizado               | Fijar versión por release, Dependabot y calendario de parches |
| Acciones de GitHub comprometidas           | Pinning por SHA, permisos mínimos y revisión de Dependabot    |
| Usuario sin `~/.local/bin` en PATH         | Aviso claro y alternativa de `--bindir`                       |
| Windows no soportado en la primera versión | Documentarlo y planificar PowerShell más adelante             |

## 15. Decisiones pendientes

1. **Resuelta:** sí, Linux x64 y arm64 se publicaron juntas en `v1.0.0` y `v1.0.1`.
2. ¿macOS se incluye en esta release o en una siguiente?
3. ¿Se exige `--verify-attestation` por defecto cuando `gh` está disponible?
4. ¿Se añade `--init-config` al instalador o se mantiene la configuración manual?
5. ¿Se firman también los checksums con minisign/GPG además de la atestación de GitHub?
6. ¿Cuántas versiones anteriores se conservan al actualizar?

## 16. Siguiente plan

`v1.0.0` y su corrección `v1.0.1` están publicadas y verificadas, así que este plan y M8
están cerrados. El siguiente paso es
[`version-2.0.md`](./version-2.0.md): estado de reproducción, continuar viendo,
biblioteca sana y soporte macOS.
