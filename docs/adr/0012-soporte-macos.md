# ADR 0012 — Soporte macOS y límites de Windows

- **Estado:** aceptada
- **Fecha:** 2026-10-23
- **Plan:** [`version-2.0.md`](../plans/version-2.0.md) (M12)
- **Relacionada con:** [ADR 0009](./0009-distribucion-github-releases.md), [ADR 0011](./0011-salud-biblioteca.md)

## Contexto

`v1.x` y `v2.0` hasta M11 solo publican artefactos para Linux (x64 y arm64). El plan v2.0 establece que macOS entra en `2.0` y Windows queda para `2.1`.

Para soportar macOS necesitamos:

1. Artefactos compilados en runners macOS (x64 y arm64).
2. Node oficial verificado para cada arquitectura.
3. `better-sqlite3` compilado nativamente en cada arquitectura (ya se compila en el bundle).
4. Instalador POSIX que funcione en macOS (ya es compatible, usa `sh`).
5. CI que valide la instalación en macOS.
6. Documentación de Gatekeeper/cuarentena y reproductores recomendados.

## Decisión

### Artefactos macOS

Se publican 4 assets por release:

- `media-manager-cli-<version>-linux-x64.tar.gz`
- `media-manager-cli-<version>-linux-arm64.tar.gz`
- `media-manager-cli-<version>-macos-x64.tar.gz`
- `media-manager-cli-<version>-macos-arm64.tar.gz`

Cada uno con su `.sha256`, SBOM y atestación de procedencia.

### CI/CD

- Job `bundle` en matriz: `ubuntu-24.04` (x64), `ubuntu-24.04-arm` (arm64), `macos-14` (x64), `macos-14-arm64` (arm64).
- Job `install-test` se ejecuta en `ubuntu-24.04` y `macos-14` con contenedor Docker solo para Linux; en macOS se usa el runner directamente (sin Docker) porque el bundle ya incluye Node.
- Los tests de integración/E2E se ejecutan en la matriz completa.

### Instalador

El script `install.sh` ya es POSIX `sh` y funciona en macOS. Cambios:

- `detect_platform` reconoce `Darwin` → `macos`.
- Rutas XDG se mantienen (`~/.config`, `~/.local/share`) para paridad con Linux (decisión del ADR 0011).
- El wrapper `mm` usa `exec "$HERE/node" ...` que funciona en macOS.
- Gatekeeper: al descargar el `.tar.gz` y extraerlo, el binario `node` y el wrapper `mm` pueden quedar en cuarentena. El instalador ejecuta `xattr -d com.apple.quarantine` sobre los binarios extraídos si está en macOS.
- Documentación en README: pasos para `xattr -d com.apple.quarantine ~/.local/share/media-manager/current/node ~/.local/bin/mm` si el sistema lo bloquea.

### Reproductores en macOS

- `mpv`: se instala via Homebrew (`brew install mpv`) o MacPorts. El binario suele estar en `/opt/homebrew/bin/mpv` (arm64) o `/usr/local/bin/mpv` (x64). El PATH del usuario suele incluirlo.
- `vlc`: se instala via Homebrew (`brew install --cask vlc`) o descarga directa. El binario CLI es `/Applications/VLC.app/Contents/MacOS/VLC`.
- La config por defecto detecta `mpv` y `vlc` en PATH; el usuario puede sobrescribir `player.players.*.command` con la ruta absoluta.

### Windows

Queda fuera de `v2.0`. Requiere:

- Instalador PowerShell (`.ps1`) firmado.
- Artefacto `.zip` (no `.tar.gz`).
- Named pipes para IPC de mpv (no sockets Unix).
- Rutas `AppData\Local` y `AppData\Roaming` en lugar de XDG.
- Matriz de CI en `windows-latest`.

Se abordará en `v2.1`.

## Alternativas consideradas

- **Un solo binario multi-arquitectura (Universal Binary)**: No aplica; el bundle incluye Node + addons nativos por arquitectura.
- **Notarización Apple**: Requiere cuenta de desarrollador ($99/año) y `xcrun notarytool`. Se pospone; por ahora se documenta Gatekeeper.
- **Homebrew tap**: Podría añadirse como canal secundario; el canal principal sigue siendo GitHub Releases + instalador.
- **Rutas `~/Library/Application Support`**: Rechazado; se mantienen XDG para paridad Linux/macOS y evitar migración de datos.

## Consecuencias

- La release `v2.0.0` incluirá 4 plataformas.
- El instalador maneja cuarentena en macOS automáticamente.
- El usuario de macOS instala con `sh install.sh` sin Node ni npm.
- Windows se documenta explícitamente como "próximamente en v2.1".
