#!/usr/bin/env bash
#
# scripts/install.sh — Instalador de acryl (Gestor de contenido multimedia)
#
# Compila la aplicación en modo release e instala:
#   - el binario `acryl` en /usr/local/bin (o BIN_DIR si se indica)
#   - el archivo .desktop en ~/.local/share/applications (o DESKTOP_DIR si se indica)
#   - el icono en ~/.local/share/icons/hicolor/scalable/apps/ (o ICON_DIR si se indica)
#
# Requiere: cargo, rustc y las librerías de desarrollo de GTK4 / Libadwaita.
# No requiere privilegios de root si se instalan en directorios del usuario.
#
# Uso:
#   ./scripts/install.sh                 # rutas por defecto (usuario)
#   sudo ./scripts/install.sh --system   # rutas del sistema (/usr/local, /usr/share)
#   ./scripts/install.sh --help          # muestra esta ayuda
set -euo pipefail

###############################################################################
# Variables por defecto
###############################################################################
APP_NAME="acryl"
BIN_DIR="${BIN_DIR:-$HOME/.local/bin}"
DESKTOP_DIR="${DESKTOP_DIR:-$HOME/.local/share/applications}"
ICON_DIR="${ICON_DIR:-$HOME/.local/share/icons/hicolor/scalable/apps}"
SYSTEM_MODE=0

###############################################################################
# Funciones de ayuda
###############################################################################
log()   { printf '[install] %s\n' "$*"; }
warn()  { printf '[install][advertencia] %s\n' "$*" >&2; }
error() { printf '[install][error] %s\n' "$*" >&2; exit 1; }

usage() {
    cat <<EOF
Uso: $0 [opciones]

Opciones:
  --system     Instala en rutas del sistema (/usr/local/bin, /usr/share/applications,
               /usr/share/icons/hicolor/scalable/apps). Requiere sudo.
  --bin-dir    Directorio donde se instalará el binario (por defecto: $BIN_DIR)
  --desktop-dir Directorio donde se instalará el archivo .desktop
               (por defecto: $DESKTOP_DIR)
  --icon-dir   Directorio donde se instalará el icono
               (por defecto: $ICON_DIR)
  -h, --help   Muestra esta ayuda y sale

Ejemplos:
  $0                      # instalación para el usuario actual (sin sudo)
  sudo $0 --system        # instalación para todos los usuarios
EOF
}

parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --system)
                SYSTEM_MODE=1
                BIN_DIR="/usr/local/bin"
                DESKTOP_DIR="/usr/share/applications"
                ICON_DIR="/usr/share/icons/hicolor/scalable/apps"
                shift
                ;;
            --bin-dir)
                [[ $# -lt 2 ]] && error "--bin-dir requiere un valor"
                BIN_DIR="$2"; shift 2 ;;
            --desktop-dir)
                [[ $# -lt 2 ]] && error "--desktop-dir requiere un valor"
                DESKTOP_DIR="$2"; shift 2 ;;
            --icon-dir)
                [[ $# -lt 2 ]] && error "--icon-dir requiere un valor"
                ICON_DIR="$2"; shift 2 ;;
            -h|--help)
                usage; exit 0 ;;
            *)
                error "Opción desconocida: $1. Ejecuta '$0 --help' para ver las opciones." ;;
        esac
    done
}

require_cmd() {
    command -v "$1" >/dev/null 2>&1 || error "Falta el comando requerido: $1"
}

check_dependencies() {
    log "Verificando dependencias de compilación..."
    require_cmd cargo
    require_cmd rustc
    require_cmd pkg-config

    # Verificar GTK4 y Libadwaita (necesarias para compilar)
    pkg-config --exists gtk4 || error "Falta gtk4 (librerías de desarrollo de GTK4)"
    pkg-config --exists libadwaita-1 || error "Falta libadwaita-1 (librerías de desarrollo de Libadwaita)"

    log "Dependencias OK."
}

build_release() {
    log "Compilando $APP_NAME en modo release..."
    cargo build --release --bin "$APP_NAME" || error "Falló la compilación de $APP_NAME"
    log "Compilación completada."
}

install_binary() {
    local src_bin="target/release/$APP_NAME"
    [[ -f "$src_bin" ]] || error "No se encontró el binario compilado: $src_bin"

    log "Instalando binario en $BIN_DIR..."
    mkdir -p "$BIN_DIR"
    install -m 0755 "$src_bin" "$BIN_DIR/$APP_NAME"
    log "Binario instalado: $BIN_DIR/$APP_NAME"
}

install_desktop() {
    local src_desktop="$APP_NAME.desktop"
    [[ -f "$src_desktop" ]] || error "No se encontró el archivo: $src_desktop"

    log "Instalando entrada de escritorio en $DESKTOP_DIR..."
    mkdir -p "$DESKTOP_DIR"
    install -m 0644 "$src_desktop" "$DESKTOP_DIR/$APP_NAME.desktop"
    log "Entrada de escritorio instalada: $DESKTOP_DIR/$APP_NAME.desktop"
}

install_icon() {
    local src_icon="icons/acryl.png"
    # También aceptamos svg si existe
    if [[ -f "icons/acryl.svg" ]]; then
        src_icon="icons/acryl.svg"
    elif [[ ! -f "$src_icon" ]]; then
        warn "No se encontró ningún icono en icons/ (se omite la instalación del icono)"
        return 0
    fi

    log "Instalando icono en $ICON_DIR..."
    mkdir -p "$ICON_DIR"
    install -m 0644 "$src_icon" "$ICON_DIR/$APP_NAME.$( [[ "$src_icon" == *.svg ]] && echo svg || echo png )"
    log "Icono instalado."
}

update_desktop_database() {
    if command -v update-desktop-database >/dev/null 2>&1; then
        local target_dir="$DESKTOP_DIR"
        if [[ "$SYSTEM_MODE" -eq 1 ]]; then
            sudo update-desktop-database "$target_dir" 2>/dev/null || true
        else
            update-desktop-database "$target_dir" 2>/dev/null || true
        fi
    fi
}

update_icon_cache() {
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        local icon_parent
        icon_parent="$(dirname "$(dirname "$ICON_DIR")")"
        if [[ "$SYSTEM_MODE" -eq 1 ]]; then
            sudo gtk-update-icon-cache -qt "$icon_parent" 2>/dev/null || true
        else
            gtk-update-icon-cache -qt "$icon_parent" 2>/dev/null || true
        fi
    fi
}

main() {
    parse_args "$@"

    if [[ "$SYSTEM_MODE" -eq 1 ]] && [[ "$EUID" -ne 0 ]]; then
        error "--system requiere privilegios de root. Ejecuta con sudo."
    fi

    log "=== Instalador de $APP_NAME ==="
    check_dependencies
    build_release
    install_binary
    install_desktop
    install_icon
    update_desktop_database
    update_icon_cache

    log "=== Instalación completada ==="
    log "Puedes lanzar la aplicación con: $APP_NAME"
    log "O desde el menú de aplicaciones si está disponible."
}

main "$@"
