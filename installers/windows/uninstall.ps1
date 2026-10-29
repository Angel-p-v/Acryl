#!/usr/bin/env pwsh
# Desinstalador de media-manager-cli para Windows.
#
# Conserva la configuración y la base de datos salvo que se use --purge.
#
# Uso: .\uninstall.ps1 [opciones]
#   .\uninstall.ps1
#   .\uninstall.ps1 --purge --yes

$ErrorActionPreference = 'Stop'
$PSDefaultParameterValues['*:Encoding'] = 'utf8'

$APP_NAME = "media-manager-cli"
$MARKER = "# managed by media-manager-cli installer"

$PREFIX = "$env:LOCALAPPDATA\${APP_NAME}"
$BINDIR = ""
$PURGE = $false
$DRY_RUN = $false
$ASSUME_YES = $false

function usage() {
@"
Uso: .\uninstall.ps1 [opciones]

Desinstala media-manager-cli y el comando `mm`.

Opciones:
  --prefix <dir>   prefijo usado en la instalación (por defecto, %LOCALAPPDATA%\media-manager-cli)
  --bindir <dir>   directorio del wrapper (por defecto, <prefix>\bin)
  --purge          elimina también configuración y base de datos
  --dry-run        muestra lo que haría sin borrar nada
  --yes            no pide confirmación
  --help           muestra esta ayuda

Por defecto se conservan:
  %APPDATA%\media-manager\        (configuración)
  %LOCALAPPDATA%\media-manager\   (base de datos)

La biblioteca multimedia nunca se toca.
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

# --- Parseo de argumentos ---
$argsCopy = $args
$i = 0
while ($i -lt $argsCopy.Count) {
    $arg = $argsCopy[$i]
    switch ($arg) {
        "--prefix" {
            if ($i + 1 -ge $argsCopy.Count) { die 2 "falta el valor de --prefix" }
            $PREFIX = $argsCopy[$i + 1]
            $i += 2
        }
        "--bindir" {
            if ($i + 1 -ge $argsCopy.Count) { die 2 "falta el valor de --bindir" }
            $BINDIR = $argsCopy[$i + 1]
            $i += 2
        }
        "--purge" { $PURGE = $true; $i++ }
        "--dry-run" { $DRY_RUN = $true; $i++ }
        "--yes" { $ASSUME_YES = $true; $i++ }
        "-y" { $ASSUME_YES = $true; $i++ }
        "--help" { usage }
        "-h" { usage }
        "--" { $i++; break }
        default { die 2 "opción desconocida: $arg (usa --help)" }
    }
}
if ($argsCopy.Count -gt $i) { die 2 "argumentos inesperados: $($argsCopy[$i..($argsCopy.Count-1)])" }

if ($PREFIX -like "~\*") { $PREFIX = $PREFIX.Replace("~", $env:USERPROFILE) }
if (-not [System.IO.Path]::IsPathRooted($PREFIX)) { die 2 "--prefix debe ser una ruta absoluta" }
$PREFIX = $PREFIX.TrimEnd('\')

if (-not $BINDIR) { $BINDIR = "$PREFIX\bin" }
if ($BINDIR -like "~\*") { $BINDIR = $BINDIR.Replace("~", $env:USERPROFILE) }
if (-not [System.IO.Path]::IsPathRooted($BINDIR)) { die 2 "--bindir debe ser una ruta absoluta" }
$BINDIR = $BINDIR.TrimEnd('\')

if ($PREFIX -eq "" -or $PREFIX -eq "\") { die 2 "--prefix no puede ser la raíz del sistema" }

$APP_DIR = Join-Path $PREFIX "share" $APP_NAME
$CURRENT_LINK = Join-Path $APP_DIR "current"
$BINARY = Join-Path $BINDIR "mm.bat"

if (-not (Test-Path $APP_DIR)) {
    say "No hay ninguna instalación de ${APP_NAME} en ${APP_DIR}."
    exit 0
}
if (-not (Test-Path $CURRENT_LINK -PathType Container)) {
    die 1 "${APP_DIR} existe pero no tiene un enlace 'current' válido; revísalo manualmente"
}
$CURRENT_REAL = (Get-Item $CURRENT_LINK).Target
if (-not [System.IO.Path]::IsPathRooted($CURRENT_REAL)) {
    $CURRENT_REAL = Join-Path $APP_DIR $CURRENT_REAL
}
if (-not (Test-Path $CURRENT_REAL -PathType Container)) {
    die 1 "el enlace 'current' apunta a una ruta inexistente; revísalo manualmente"
}
$MANIFEST = Join-Path $CURRENT_REAL "install.json"
if (-not (Test-Path $MANIFEST)) {
    die 1 "${CURRENT_REAL} no contiene un install.json válido; revísalo manualmente"
}

$manifestContent = Get-Content $MANIFEST -Raw | ConvertFrom-Json
$VERSION = $manifestContent.version
if (-not (($VERSION -match '^[0-9]+\.[0-9]+\.[0-9]+'))) { die 1 "install.json contiene una versión inválida" }
$INSTALLED_AT = $manifestContent.installedAt

$CONFIG_DIR = Join-Path $env:APPDATA "media-manager"
$DATA_DIR = Join-Path $env:LOCALAPPDATA "media-manager"

say ""
say "Se desinstalará $APP_NAME $VERSION"
if ($INSTALLED_AT) { say "  instalado: $INSTALLED_AT" }
say "  aplicación: $APP_DIR"
say "  comando:    $BINARY"
if ($PURGE) {
    say "  configuración: $CONFIG_DIR (se eliminará)"
    say "  base de datos: $DATA_DIR (se eliminará)"
} else {
    say "  configuración: $CONFIG_DIR (se conserva)"
    say "  base de datos: $DATA_DIR (se conserva)"
}
say ""

function confirm($msg) {
    if ($ASSUME_YES) { return $true }
    if (-not [Console]::IsInputRedirected) {
        Write-Host "$msg [y/N] " -NoNewline
        $answer = Read-Host
        return ($answer -in @('y', 'Y', 'yes', 'YES', 'Yes'))
    }
    die 2 "no hay terminal interactiva; usa --yes para confirmar"
}

if ($DRY_RUN) {
    say "Simulación: no se ha borrado nada."
    exit 0
}

confirm "¿Continuar?" || {
    say "Cancelado."
    exit 0
}

if ($PURGE) {
    confirm "Esto eliminará también la configuración y la base de datos. ¿Seguro?" || {
        say "Cancelado."
        exit 0
    }
}

function remove_within($path, $parent) {
    if (-not (Test-Path $path)) { return 0 }
    $resolved = (Resolve-Path $path -ErrorAction SilentlyContinue)
    if (-not $resolved) {
        # Puede ser un enlace roto
        if (Test-Path $path -PathType Leaf) {
            step "eliminando el archivo $path"
            Remove-Item $path -Force
        }
        return 0
    }
    $resolvedPath = $resolved.Path
    $parentResolved = (Resolve-Path $parent -ErrorAction SilentlyContinue)
    if (-not $parentResolved) { return 0 }
    $parentPath = $parentResolved.Path
    if (-not $resolvedPath.StartsWith($parentPath)) {
        die 1 "$path queda fuera de $parent; no se elimina"
    }
    if ($resolvedPath -eq $parentPath) { die 1 "no se elimina $parent directamente" }
    if ($resolvedPath -eq "" -or $resolvedPath -eq "\" -or $resolvedPath -eq $env:USERPROFILE) {
        die 1 "ruta peligrosa: $resolvedPath"
    }
    step "eliminando $path"
    Remove-Item $path -Recurse -Force
}

if (Test-Path $BINARY) {
    $content = Get-Content $BINARY -Raw -ErrorAction SilentlyContinue
    if (-not $content.Contains($MARKER)) {
        warn "$BINARY no lo gestiona este instalador; no se elimina"
    } else {
        step "eliminando $BINARY"
        Remove-Item $BINARY -Force
    }
}

remove_within $APP_DIR (Join-Path $PREFIX "share")

if ($PURGE) {
    remove_within $CONFIG_DIR (Split-Path $CONFIG_DIR)
    remove_within $DATA_DIR (Split-Path $DATA_DIR)
}

say ""
say "✔ $APP_NAME desinstalado"
if ($PURGE -ne $true) {
    say "  Se conservan $CONFIG_DIR y $DATA_DIR."
    say "  Usa --purge para eliminarlos."
}
