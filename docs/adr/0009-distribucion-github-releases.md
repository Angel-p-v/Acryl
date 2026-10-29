# ADR 0009 — Distribución por GitHub Releases

- **Estado:** aceptada
- **Fecha:** 2026-10-23
- **Sustituye parcialmente:** [ADR 0008](./0008-publicacion-y-distribucion.md)

## Contexto

El ADR 0008 dejó la distribución en dos canales: el paquete `media-manager-cli` en npm y
un bundle portable con Node incluido como adjunto de la GitHub Release. La publicación en
npm exige un `NPM_TOKEN` y una cuenta del registro que el mantenedor no quiere mantener
para una herramienta local; además, obliga al usuario a tener Node instalado o a confiar
en los scripts de instalación de una dependencia nativa (`better-sqlite3`).

El plan [`instaladores-y-seguridad.md`](../plans/instaladores-y-seguridad.md) concreta una
alternativa verificable: instalar desde GitHub Releases con un script POSIX que comprueba
checksum y estructura antes de ejecutar nada.

## Decisión

### Canal único

- La distribución oficial pasa a ser **GitHub Releases**. No se publica en npm.
- El paquete `media-manager-cli` seguirá existiendo en `package.json` como nombre interno y
  para construir el tarball que consume el bundle, pero `prepack`, `files` y `smoke` son
  herramientas de construcción, no un canal de instalación.
- Se eliminan `publish.yml`, `NPM_TOKEN`, `publishConfig` y las referencias a npm en la
  documentación de release.

### Artefactos

Cada release publica, por plataforma:

```text
media-manager-cli-<versión>-<plataforma>.tar.gz
media-manager-cli-<versión>-<plataforma>.tar.gz.sha256
sbom.cdx.json
```

- El `.tar.gz` incluye `dist/`, dependencias de producción, un runtime Node oficial
  descargado y verificado desde `nodejs.org`, licencias y `install.json` de construcción.
- El `.sha256` usa el formato que aceptan `sha256sum -c` y `shasum -a 256 -c`.
- La atestación de procedencia se genera con `actions/attest-build-provenance` y se puede
  verificar con `gh attestation verify`. No sustituye al checksum.
- El SBOM se genera en formato CycloneDX.

Plataformas iniciales: **Linux x64** y **Linux arm64**. macOS y Windows quedan para fases
posteriores porque necesitan sus propios scripts y runners.

### Instalación

- `install.sh` y `uninstall.sh` son POSIX `sh`, sin dependencias de Bash, y se pueden
  revisar antes de ejecutarlos.
- La instalación es por usuario en `~/.local` (o `--prefix`), sin `sudo`, sin modificar el
  `PATH` y sin tocar la configuración existente.
- El script descarga por HTTPS estricto, exige checksum, valida el contenido del archivo
  (incluidas rutas `../` y absolutas) y solo entonces extrae e instala de forma atómica.
- Se mantiene una versión por directorio con un enlace `current`; el wrapper de
  `<prefix>/bin/mm` ejecuta la ruta real para que el lanzador del bundle resuelva `node` y
  `node_modules` con `$0`.
- `uninstall.sh` conserva configuración y base de datos salvo `--purge`, y rechaza rutas
  peligrosas o manipuladas.

### Modelo de confianza

- HTTPS, checksum, validación del archivo y atestación son controles complementarios.
- La raíz de confianza sigue siendo la cuenta de GitHub que publica la release; no hay
  firma GPG/minisign en la primera versión.
- Los workflows usan permisos mínimos, acciones fijadas por SHA y `npm ci` con lockfile.
- La instalación no ejecuta ningún binario del bundle antes de verificar la integridad.

## Consecuencias

- Desaparece la dependencia de credenciales de npm y el usuario no necesita Node instalado.
- El bundle pesa más que el paquete npm y es específico de plataforma y de glibc; no se
  promete compatibilidad con Alpine/musl.
- La actualización es manual: se vuelve a ejecutar `install.sh` con la nueva versión.
- La verificación de la atestación es opcional en el instalador y obligatoria en CI.
- El ADR 0008 queda vigente solo en lo relativo al nombre del paquete, el binario `mm` y el
  bundle portable como formato; su parte de publicación en npm queda sustituida.
