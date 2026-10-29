#!/bin/sh
# Desinstalador de media-manager-cli.
#
# Conserva la configuración y la base de datos salvo que se use --purge.
set -eu
umask 022

APP_NAME="media-manager-cli"

PREFIX="${HOME}/.local"
BINDIR=""
PURGE=0
DRY_RUN=0
ASSUME_YES=0

usage() {
  cat <<'EOF'
Uso: uninstall.sh [opciones]

Desinstala media-manager-cli y el comando `mm`.

Opciones:
  --prefix <dir>   prefijo usado en la instalación (por defecto, ~/.local)
  --bindir <dir>   directorio del wrapper (por defecto, <prefix>/bin)
  --purge          elimina también configuración y base de datos
  --dry-run        muestra lo que haría sin borrar nada
  --yes            no pide confirmación
  --help           muestra esta ayuda

Por defecto se conservan:
  ~/.config/media-manager/   (o $XDG_CONFIG_HOME/media-manager)
  ~/.local/share/media-manager/ (o $XDG_DATA_HOME/media-manager)

La biblioteca multimedia nunca se toca.
EOF
}

say() { printf '%s\n' "$*"; }
step() { printf '· %s\n' "$*"; }
warn() { printf 'aviso: %s\n' "$*" >&2; }
die() {
  _code=$1
  shift
  printf 'error: %s\n' "$*" >&2
  exit "$_code"
}

while [ $# -gt 0 ]; do
  case "$1" in
    --prefix)
      [ $# -ge 2 ] || die 2 "falta el valor de --prefix"
      PREFIX=$2
      shift 2
      ;;
    --bindir)
      [ $# -ge 2 ] || die 2 "falta el valor de --bindir"
      BINDIR=$2
      shift 2
      ;;
    --purge) PURGE=1; shift ;;
    --dry-run) DRY_RUN=1; shift ;;
    --yes | -y) ASSUME_YES=1; shift ;;
    --help | -h) usage; exit 0 ;;
    --) shift; break ;;
    *) die 2 "opción desconocida: $1 (usa --help)" ;;
  esac
done
[ $# -eq 0 ] || die 2 "argumentos inesperados: $*"

case "$PREFIX" in
  \~) PREFIX="${HOME}" ;;
  \~/*) PREFIX="${HOME}/${PREFIX#??}" ;;
  /*) ;;
  *) die 2 "--prefix debe ser una ruta absoluta" ;;
esac
while [ "${PREFIX%/}" != "$PREFIX" ]; do PREFIX="${PREFIX%/}"; done
case "$PREFIX" in
  "" | "/") die 2 "--prefix no puede ser la raíz del sistema" ;;
esac

if [ -z "$BINDIR" ]; then
  BINDIR="${PREFIX}/bin"
fi
case "$BINDIR" in
  \~) BINDIR="${HOME}" ;;
  \~/*) BINDIR="${HOME}/${BINDIR#??}" ;;
  /*) ;;
  *) die 2 "--bindir debe ser una ruta absoluta" ;;
esac
while [ "${BINDIR%/}" != "$BINDIR" ]; do BINDIR="${BINDIR%/}"; done
case "$BINDIR" in
  "" | "/") die 2 "--bindir no puede ser la raíz del sistema" ;;
esac

# Resolver rutas reales para validaciones de seguridad
resolve_path() { (unset CDPATH; cd -- "$1" 2>/dev/null && pwd -P); }

HOME_REAL=$(resolve_path "$HOME") || die 1 "no se pudo resolver HOME"

if [ -e "$PREFIX" ] || [ -L "$PREFIX" ]; then
  PREFIX_REAL=$(resolve_path "$PREFIX") || die 1 "no se pudo resolver --prefix"
else
  PREFIX_REAL=$PREFIX
fi

case "$PREFIX_REAL" in
  "" | "/") die 2 "--prefix no puede ser la raíz del sistema" ;;
esac

if [ "$PREFIX_REAL" = "$HOME_REAL" ]; then
  die 2 "--prefix no puede ser el directorio personal"
fi

case "$HOME_REAL" in
  "$PREFIX_REAL"/*) die 2 "--prefix no puede ser un directorio padre de $HOME_REAL" ;;
esac

APP_DIR="${PREFIX_REAL}/share/${APP_NAME}"
BINARY="${BINDIR}/mm"

if [ -n "${XDG_CONFIG_HOME:-}" ]; then
  case "$XDG_CONFIG_HOME" in
    /*) CONFIG_DIR="${XDG_CONFIG_HOME}/media-manager" ;;
    *) CONFIG_DIR="${HOME}/.config/media-manager" ;;
  esac
else
  CONFIG_DIR="${HOME}/.config/media-manager"
fi
if [ -n "${XDG_DATA_HOME:-}" ]; then
  case "$XDG_DATA_HOME" in
    /*) DATA_DIR="${XDG_DATA_HOME}/media-manager" ;;
    *) DATA_DIR="${HOME}/.local/share/media-manager" ;;
  esac
else
  DATA_DIR="${HOME}/.local/share/media-manager"
fi

VERSION=""
if [ -f "${APP_DIR}/install.json" ]; then
  VERSION=$(sed -n 's/^[[:space:]]*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*$/\1/p' \
    "${APP_DIR}/install.json" | head -n 1)
fi

if ! { [ -e "$APP_DIR" ] || [ -L "$APP_DIR" ]; }; then
  say "No hay ninguna instalación de ${APP_NAME} en ${APP_DIR}."
  exit 0
fi

say ""
[ -n "$VERSION" ] && say "Se desinstalará ${APP_NAME} ${VERSION}" || say "Se desinstalará ${APP_NAME}"
say "  aplicación: ${APP_DIR}"
say "  comando:    ${BINARY}"
if [ "$PURGE" -eq 1 ]; then
  say "  configuración: ${CONFIG_DIR} (se eliminará)"
  say "  base de datos: ${DATA_DIR} (se eliminará)"
else
  say "  configuración: ${CONFIG_DIR} (se conserva)"
  say "  base de datos: ${DATA_DIR} (se conserva)"
fi
say ""

confirm() {
  [ "$ASSUME_YES" -eq 1 ] && return 0
  [ -t 0 ] || die 2 "no hay terminal interactiva; usa --yes para confirmar"
  printf '%s [y/N] ' "$1"
  read -r _answer || _answer=""
  case "$_answer" in
    y | Y | yes | YES | Yes) return 0 ;;
    *) return 1 ;;
  esac
}

if [ "$DRY_RUN" -eq 1 ]; then
  say "Simulación: no se ha borrado nada."
  exit 0
fi

confirm "¿Continuar?" || {
  say "Cancelado."
  exit 0
}

if [ "$PURGE" -eq 1 ]; then
  confirm "Esto eliminará también la configuración y la base de datos. ¿Seguro?" || {
    say "Cancelado."
    exit 0
  }
fi

if [ -e "$BINARY" ] || [ -L "$BINARY" ]; then
  step "eliminando ${BINARY}"
  rm -f "$BINARY" || die 1 "no se pudo eliminar ${BINARY}"
fi

if [ -e "$APP_DIR" ] || [ -L "$APP_DIR" ]; then
  step "eliminando ${APP_DIR}"
  rm -rf "$APP_DIR" || die 1 "no se pudo eliminar ${APP_DIR}"
fi

if [ "$PURGE" -eq 1 ]; then
  if [ -e "$CONFIG_DIR" ] || [ -L "$CONFIG_DIR" ]; then
    step "eliminando ${CONFIG_DIR}"
    rm -rf "$CONFIG_DIR" || die 1 "no se pudo eliminar ${CONFIG_DIR}"
  fi
  if [ -e "$DATA_DIR" ] || [ -L "$DATA_DIR" ]; then
    step "eliminando ${DATA_DIR}"
    rm -rf "$DATA_DIR" || die 1 "no se pudo eliminar ${DATA_DIR}"
  fi
fi

say ""
say "✔ ${APP_NAME} desinstalado"
if [ "$PURGE" -ne 1 ]; then
  say "  Se conservan ${CONFIG_DIR} y ${DATA_DIR}."
  say "  Usa --purge para eliminarlos."
fi