#!/usr/bin/env pwsh
# Instalador de media-manager-cli para Windows.
#
# Clona el repositorio, instala dependencias, compila y deja el comando `mm`
# disponible por usuario sin privilegios de administrador.
# Revisa el script antes de ejecutarlo.
#
# Uso: .\install.ps1 [opciones]
#   .\install.ps1
#   .\install.ps1 --prefix "$env:LOCALAPPDATA\media-manager-cli"
#   .\install.ps1 --dry-run            # muestra los cambios sin aplicarlos

$ErrorActionPreference = 'Stop'
$PSDefaultParameterValues['*:Encoding'] = 'utf8'

$REPOSITORY = "https://github.com/Angel-p-v/media-manager.git"
$APP_NAME = "media-manager-cli"

$PREFIX = "$env:LOCALAPPDATA\${APP_NAME}"
$PREFIX_EXPLICIT = $false
$BINDIR = ""
$FORCE = $false
$DRY_RUN = $false

function usage() {
    @"
Uso: .\install.ps1 [opciones]

Instala media-manager-cli clonando el repositorio y compilando desde fuente.

Opciones:
  --prefix <dir>         prefijo de instalación (por defecto, %LOCALAPPDATA%\media-manager-cli)
  --bindir <dir>         directorio del wrapper (por defecto, <prefix>\bin)
  --force                reinstala aunque ya exista
  --dry-run              muestra los cambios sin aplicarlos
  --help                 muestra esta ayuda

Ejemplos:
  .\install.ps1
  .\install.ps1 --prefix "$env:LOCALAPPDATA\media-manager-cli"
  .\install.ps1 --dry-run            # muestra los cambios sin aplicarlos

Códigos de salida:
  0 éxito · 1 error · 2 uso incorrecto · 4 instalación existente sin --force
"@
    exit 0
}

function say($msg) { Write-Output $msg }
function step($msg) { Write-Output "· $msg" }
function warn($msg) { Write-Warning $msg }
function die([int]$code, $msg) {
    Write-Error "error: $msg"
    exit $code
}

function have($cmd) {
    return (Get-Command $cmd -ErrorAction SilentlyContinue) -ne $null
}

# ---------------------------------------------------------------------------
# chafa: renderizado de imágenes en terminal (pósters de metadatos)
# ---------------------------------------------------------------------------
function ensure_chafa() {
    if (have chafa) {
        step "chafa ya está instalado (renderizado de pósters habilitado)"
        return
    }

    warn "chafa no está instalado; es necesario para mostrar pósters de metadatos en la TUI"
    step "intentando instalar chafa con el gestor de paquetes de Windows"

    # Intentar winget primero (estándar en Windows 10/11)
    if (have winget) {
        step "instalando chafa mediante winget"
        winget install --id chafa.chafa --silent --accept-package-agreements --accept-source-agreements 2>$null
        if (have chafa) {
            step "chafa instalado correctamente mediante winget (renderizado de pósters habilitado)"
            return
        }
        warn "winget no pudo instalar chafa"
    }

    # Intentar chocolatey como fallback
    if (have choco) {
        step "instalando chafa mediante chocolatey"
        choco install chafa -y 2>$null
        if (have chafa) {
            step "chafa instalado correctamente mediante chocolatey (renderizado de pósters habilitado)"
            return
        }
        warn "chocolatey no pudo instalar chafa"
    }

    warn "no se pudo instalar chafa automáticamente"
    warn "instálalo manualmente (ej. 'winget install chafa.chafa') para ver los pósters"
}

# --- Parseo de argumentos ---
$argsCopy = $args
$i = 0
while ($i -lt $argsCopy.Count) {
    $arg = $argsCopy[$i]
    switch ($arg) {
        "--prefix" {
            if ($i + 1 -ge $argsCopy.Count) { die 2 "falta el valor de --prefix" }
            $PREFIX = $argsCopy[$i + 1]
            $PREFIX_EXPLICIT = $true
            $i += 2
        }
        "--bindir" {
            if ($i + 1 -ge $argsCopy.Count) { die 2 "falta el valor de --bindir" }
            $BINDIR = $argsCopy[$i + 1]
            $i += 2
        }
        "--force" { $FORCE = $true; $i++ }
        "--dry-run" { $DRY_RUN = $true; $i++ }
        "--help" { usage }
        "-h" { usage }
        "--" { $i++; break }
        default { die 2 "opción desconocida: $arg (usa --help)" }
    }
}
if ($argsCopy.Count -gt $i) { die 2 "argumentos inesperados: $($argsCopy[$i..($argsCopy.Count-1)])" }

if (([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator) -and -not $PREFIX_EXPLICIT) {
    die 2 "no ejecutes como administrador; si es necesario, pasa --prefix explícito"
}

# Resolver rutas absolutas
if ($PREFIX -like "~\*") { $PREFIX = $PREFIX.Replace("~", $env:USERPROFILE) }
if (-not [System.IO.Path]::IsPathRooted($PREFIX)) { die 2 "--prefix debe ser una ruta absoluta" }
$PREFIX = $PREFIX.TrimEnd('\')

if (-not $BINDIR) { $BINDIR = "$PREFIX\bin" }
if ($BINDIR -like "~\*") { $BINDIR = $BINDIR.Replace("~", $env:USERPROFILE) }
if (-not [System.IO.Path]::IsPathRooted($BINDIR)) { die 2 "--bindir debe ser una ruta absoluta" }
$BINDIR = $BINDIR.TrimEnd('\')

if ($PREFIX -eq "" -or $PREFIX -eq "\") { die 2 "--prefix no puede ser la raíz del sistema" }
if ($BINDIR -eq "" -or $BINDIR -eq "\") { die 2 "--bindir no puede ser la raíz del sistema" }

$APP_DIR = Join-Path $PREFIX "share" $APP_NAME
$BINARY = Join-Path $BINDIR "mm.bat"
$CONFIG_DIR = Join-Path $env:USERPROFILE ".config\media-manager"
$CONFIG_FILE = Join-Path $CONFIG_DIR "config.yaml"

# Verificar herramientas necesarias
if (-not (have git)) { die 1 "se necesita 'git' para clonar el repositorio" }
if (-not (have node)) { die 1 "se necesita 'node' (Node.js >= 22) para compilar" }
if (-not (have npm)) { die 1 "se necesita 'npm' para instalar dependencias" }

$nodeVersion = (node --version 2>$null).TrimStart('v')
$nodeMajor = ($nodeVersion -split '\.')[0]
if ([int]$nodeMajor -lt 22) {
    die 1 "Node.js debe ser >= 22 (detectado: $nodeVersion)"
}

if (Test-Path $APP_DIR) {
    if (-not $FORCE) { die 4 "la versión ya está instalada en ${APP_DIR}; usa --force" }
}
if (Test-Path $BINARY) {
    if (-not $FORCE) { die 4 "${BINARY} existe y no lo gestiona este instalador; usa --force para reemplazarlo" }
}

if ($DRY_RUN) {
    say ""
    say "Simulación (no se modifica nada):"
    say "  repositorio: $REPOSITORY"
    say "  destino:     $APP_DIR"
    say "  comando:     $BINARY"
    say "  config:      $CONFIG_FILE"
    exit 0
}

if ((Test-Path $PREFIX) -and -not ((Get-Item $PREFIX).PSIsContainer)) { die 1 "$PREFIX existe y no es un directorio" }
New-Item -ItemType Directory -Path $APP_DIR -Force | Out-Null
New-Item -ItemType Directory -Path $BINDIR -Force | Out-Null
New-Item -ItemType Directory -Path $CONFIG_DIR -Force | Out-Null

# Instalar chafa para renderizado de pósters en terminal (si no está disponible)
ensure_chafa

# Clonar el repositorio
step "clonando el repositorio"
$TMP_DIR = Join-Path $env:TEMP ("mm-install-" + [System.IO.Path]::GetRandomFileName())
New-Item -ItemType Directory -Path $TMP_DIR -Force | Out-Null
$CLONE_DIR = Join-Path $TMP_DIR "repo"
git clone --depth 1 $REPOSITORY $CLONE_DIR | Out-Null
if ($LASTEXITCODE -ne 0) { die 1 "no se pudo clonar el repositorio" }

# Instalar dependencias
step "instalando dependencias"
Set-Location $CLONE_DIR
npm install --omit=dev | Out-Null
if ($LASTEXITCODE -ne 0) { die 1 "falló 'npm install'" }

# Compilar
step "compilando"
npm run build | Out-Null
if ($LASTEXITCODE -ne 0) { die 1 "falló 'npm run build'" }

# Instalar en APP_DIR
step "instalando en $APP_DIR"
if (Test-Path $APP_DIR) {
    Remove-Item $APP_DIR -Recurse -Force
}
Copy-Item -Path "${CLONE_DIR}/dist" -Destination "${APP_DIR}/dist" -Recurse -Force
Copy-Item -Path "${CLONE_DIR}/node_modules" -Destination "${APP_DIR}/node_modules" -Recurse -Force
Copy-Item -Path "${CLONE_DIR}/package.json" -Destination "${APP_DIR}/package.json" -Force

# Crear script de entrada mm.bat que ejecute el CLI compilado
$entryScript = @"
@echo off
DIR=%~dp0
node "%DIR%\dist\cli.js" %*
"@
Set-Content -Path "${APP_DIR}/mm.bat" -Value $entryScript -Encoding utf8

step "creando $BINARY"
$wrapper = @"
@echo off
call "%HOMEPATH%\.local\share\media-manager-cli\mm.bat" %*
"@
Set-Content -Path $BINARY -Value $wrapper -Encoding utf8

# Crear archivo de configuración por defecto
step "creando archivo de configuración por defecto"
if (-not (Test-Path $CONFIG_FILE)) {
    $configYaml = @"
version: 1

library:
  roots: []
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

player:
  default: mpv
  onLaunch: detach
  players:
    mpv:
      command: mpv
      args: ['--force-window=immediate', '{file}']
    vlc:
      command: vlc
      args: ['--started-from-file', '{file}']

playback:
  tracking: auto
  resume: true
  resumeMinSeconds: 30
  completedThreshold: 0.9
  completedTailSeconds: 30

ui:
  theme: default
  layout: split
  sort: title
  showDetails: true
  compact: false

logging:
  level: warn
  format: text

keybindings:
  quit: q
  help: '?'
  search: /
  play: p
  nextSection: tab
  settings: s
  up: up
  down: down
  left: left
  right: right
  enter: return
  escape: escape
  configureKeybindings: k
"@
    Set-Content -Path $CONFIG_FILE -Value $configYaml -Encoding utf8
    say "  creado: $CONFIG_FILE"
} else {
    say "  ya existe: $CONFIG_FILE (se conserva)"
}

# Comprobar la instalación
step "comprobando la instalación"
$installedVersion = & $BINARY --version 2>$null | Select-Object -First 1
if (-not $installedVersion) { die 1 "el comando instalado no se pudo ejecutar" }
say ""
say "✔ $APP_NAME instalado"
say "  comando: $BINARY"
say "  versión: $installedVersion.Trim()"
say "  configuración: $CONFIG_FILE"
$pathEnv = [Environment]::GetEnvironmentVariable("PATH", [EnvironmentVariableTarget]::User)
if ($pathEnv -notlike "*$BINDIR*") {
    warn "$BINDIR no está en PATH; añádelo para ejecutar 'mm' sin ruta absoluta"
}

# Limpiar temporal
Remove-Item $TMP_DIR -Recurse -Force -ErrorAction SilentlyContinue

exit 0