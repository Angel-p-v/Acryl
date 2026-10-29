# ADR 0014: Soporte Windows (v2.1)

**Fecha:** 2026-10-24
**Estado:** Aceptado
**Autor:** Coronel

## Contexto

media-manager (`mm`) está soportado en Linux x64/arm64 con bundles portables e instalador POSIX (`install.sh`/`uninstall.sh`). El siguiente hito (M14) incluye soporte para Windows como plataforma oficial de v2.1.

El autor no tiene acceso a hardware macOS, por lo que macOS queda como "won't fix" en v2.1 (ADR 0013). Windows sí es viable mediante runners de GitHub Actions (`windows-latest`).

## Decisión

**Soportar Windows nativamente en v2.1** con los siguientes componentes:

| Componente          | Implementación Windows             | Equivalente POSIX                   |
| ------------------- | ---------------------------------- | ----------------------------------- |
| Instalador          | `install.ps1` (PowerShell)         | `install.sh`                        |
| Desinstalador       | `uninstall.ps1` (PowerShell)       | `uninstall.sh`                      |
| Asset de release    | `.zip` (equivalente a `.tar.gz`)   | `.tar.gz`                           |
| Wrapper del binario | `.bat` + shim de PowerShell        | shell script `mm`                   |
| IPC mpv             | Named pipes (`\\.\pipe\mpv-<pid>`) | Unix socket                         |
| Build CI            | runner `windows-latest`            | `ubuntu-24.04` / `ubuntu-24.04-arm` |
| Smoke test          | PowerShell en Windows limpio       | Docker Ubuntu                       |

## Detalles por área

### 1. Build y empaquetado (`scripts/build/`)

- `platform.mjs`: añadir `win32: 'windows'` a `OS_NAMES` y `NODE_OS_NAMES`. Node publica binarios oficiales para `win32-x64`.
- `standalone.mjs`: generar launcher `.bat` para Windows en lugar de shell script POSIX. Empaquetar como `.zip` en lugar de `.tar.gz` (PowerShell `Expand-Archive` es nativo en Windows; `tar` también está disponible desde Windows 10 1803+).
- `fetch-node.mjs`: ya soporta `win32-x64` vía `nodePlatform()`; solo falta registrar el nombre en `platform.mjs`.

### 2. IPC mpv — Named pipes (`src/core/playback/mpvTracker.ts`)

mpv en Windows **no soporta sockets Unix**. Usa named pipes con formato `\\.\pipe\mpv-<pid>` o similar. La implementación debe:

- En Windows (`process.platform === 'win32'`): crear un named pipe server con `node:net` (Net soporta named pipes en Windows con `\\.\pipe\...`).
- En POSIX: mantener el comportamiento actual (socket Unix en filesystem).
- `createSocketPath()`: en Windows devolver `\\.\pipe\mm-mpv-<random>`; en POSIX, ruta de archivo temporal.

Node.js `net.createServer()` y `net.createConnection()` soportan named pipes en Windows nativamente desde la v0.12+, por lo que la lógica del tracker (`MpvTracker`) no necesita cambios sustanciales: solo cambia la ruta del pipe.

### 3. Instalador PowerShell (`install.ps1` / `uninstall.ps1`)

Equivalente funcional a `install.sh`/`uninstall.sh`:

- Descargar `.zip` desde GitHub Releases.
- Verificar checksum SHA-256 (`.sha256`).
- Extraer con `Expand-Archive`.
- Verificar `install.json` del bundle.
- Crear directorio de instalación bajo `$HOME\AppData\Local\media-manager-cli` (o `--prefix`).
- Crear wrapper `.bat` o shim de PowerShell en `$bindir`.
- Eliminar zona de seguridad (Mark-of-the-Web) de los archivos extraídos con `Unblock-File`.
- Códigos de salida: 0 éxito, 1 error, 2 uso incorrecto, 3 plataforma no soportada, 4 instalación existente.

### 4. CI/CD (`.github/workflows/release-artifacts.yml`)

Añadir matriz Windows:

- `runs-on: windows-latest`
- `platform: windows-x64`
- Build del bundle (`.zip` + `.sha256` + atestación).
- Install-test con PowerShell en Windows (sin Node instalado; verificar que el bundle es autocontenido).

### 5. Documentación (`README.md`)

- Sección de instalación en Windows (`.\install.ps1`).
- Reproductores soportados en Windows (mpv, VLC).
- Limitaciones conocidas (macOS sin assets oficiales).

## Alternativas consideradas

| Alternativa                               | Por qué se descarta                                                                                  |
| ----------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| WSL2 como "soporte Windows"               | No es Windows nativo; requiere WSL instalado y configurado. El usuario espera `.exe`/`.msi`/`.zip`.  |
| Named pipes con `\\.\pipe\` vs socket TCP | TCP requiere puerto libre y gestión de conflictos; named pipes es lo nativo de mpv en Windows.       |
| `.msi` / `.exe` installer                 | Scope demasiado grande para v2.1; PowerShell + `.zip` cubre el 90% de casos con menos mantenimiento. |

## Consecuencias

### Positivas

- Usuarios Windows pueden instalar y usar `mm` sin Node ni npm.
- Paridad de características: tracking de reproducción vía named pipes funciona igual que en Linux/macOS.
- CI valida build y instalación en Windows automáticamente.

### Negativas / Riesgos

- El autor no puede probar Windows localmente; depende de CI de GitHub Actions.
- Named pipes en Node.js requieren rutas `\\.\pipe\...`; comportamiento en contenedores/CI puede diferir de Windows nativo.
- `install.ps1` requiere PowerShell 5.1+ o PowerShell 7; Windows 10 trae 5.1 por defecto.

## Referencias

- ADR 0009 (distribución GitHub Releases)
- ADR 0010 (estado reproducción / IPC mpv)
- ADR 0013 (M14 scope v2.1)
- [`scripts/build/platform.mjs`](./scripts/build/platform.mjs)
- [`src/core/playback/mpvTracker.ts`](./src/core/playback/mpvTracker.ts)
- [`install.sh`](./install.sh) (referencia POSIX para `install.ps1`)
