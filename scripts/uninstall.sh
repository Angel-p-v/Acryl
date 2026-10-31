#!/usr/bin/env bash
#
# scripts/uninstall.sh — Desinstalador de acryl
#
# Elimina:
#   - el binario `acryl`
#   - el archivo .desktop
#   - el icono
#   - (con --purge) también la base de datos, configuración y caché de pósters
#
# Usa las mismas rutas que install.sh. Si se instaló con --system, usar --system.
#
# Uso:
#   ./scripts/uninstall.sh                 # desinstala rutas de usuario
#   sudo ./scripts/uninstall.sh --system   # desinstala rutas del sistema
#   ./scripts/uninstall.sh --purge         # borra absolutamente todo (datos del usuario)
#   sudo ./scripts/uninstall.sh --system --purge
#   ./scripts/uninstall.sh --help          # muestra esta ayuda
#   ./scripts/uninstall.sh -f              # sin confirmación
set -euo pipefail

###############################################################################
# Variables por defecto
###############################################################################
APP_NAME="acryl"
BIN_DIR="${BIN_DIR:-$HOME/.local/bin}"
DESKTOP_DIR="${DESKTOP_DIR:-$HOME/.local/share/applications}"
ICON_DIR="${ICON_DIR:-$HOME/.local/share/icons/hicolor/scalable/apps}"
SYSTEM_MODE=0
FORCE=0
PURGE=0

###############################################################################
# Funciones de ayuda
###############################################################################
log()   { printf '[uninstall] %s\n' "$*"; }
warn()  { printf '[uninstall][advertencia] %s\n' "$*" >&2; }
error() { printf '[uninstall][error] %s\n' "$*" >&2; exit 1; }

usage() {
    cat <<EOF
Uso: $0 [opciones]

Opciones:
  --system     Desinstala desde rutas del sistema (/usr/local/bin, /usr/share/applications,
               /usr/share/icons/hicolor/scalable/apps). Requiere sudo.
  --bin-dir    Directorio donde está instalado el binario
               (por defecto: $BIN_DIR)
  --desktop-dir Directorio donde está instalado el archivo .desktop
               (por defecto: $DESKTOP_DIR)
  --icon-dir   Directorio donde está instalado el icono
               (por defecto: $ICON_DIR)
  -p, --purge  Borra absolutamente todo de la aplicación: binario, .desktop, icono,
               base de datos SQLite, configuración YAML y caché de pósters.
               Úsalo si quieres eliminar la app por completo, incluidos los datos
               del usuario (~/.local/share/acryl).
  -f, --force  No pide confirmación
  -h, --help   Muestra esta ayuda y sale

Ejemplos:
  $0                      # desinstalación para el usuario actual (sin sudo)
  sudo $0 --system        # desinstalación para todos los usuarios
  $0 --purge              # desinstala y borra todos los datos del usuario
  sudo $0 --system --purge # desinstalación completa en rutas del sistema
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
            -p|--purge)
                PURGE=1; shift ;;
            -f|--force)
                FORCE=1; shift ;;
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

confirm() {
    if [[ "$FORCE" -eq 1 ]]; then
        return 0
    fi
    if [[ "$PURGE" -eq 1 ]]; then
        printf '\n'
        warn "ATENCIÓN: --purge borrará TODOS los datos de la aplicación, incluyendo:"
        warn "  - Base de datos SQLite (acryl.db)"
        warn "  - Configuración YAML (config.yaml)"
        warn "  - Caché de pósters descargados"
        warn "  Esta acción es IRREVERSIBLE."
        printf '\n'
        read -rp "¿Estás seguro de que quieres desinstalar y borrar todo de $APP_NAME? [s/N] " resp
    else
        read -rp "¿Estás seguro de que quieres desinstalar $APP_NAME? [s/N] " resp
    fi
    case "$resp" in
        s|S|y|Y) return 0 ;;
        *) log "Desinstalación cancelada."; exit 0 ;;
    esac
}

# --- Rutas de datos de la app (mismo cálculo que el código Rust) -------------

data_dir() {
    if [[ -n "${XDG_DATA_HOME:-}" ]]; then
        echo "${XDG_DATA_HOME}/acryl"
    elif [[ -n "${HOME:-}" ]]; then
        echo "${HOME}/.local/share/acryl"
    else
        echo "acryl-data"
    fi
}

config_path() {
    if [[ -n "${ACRYL_CONFIG:-}" ]]; then
        echo "${ACRYL_CONFIG}"
    elif [[ -n "${HOME:-}" ]]; then
        echo "${HOME}/.local/share/acryl/config.yaml"
    else
        echo "config.yaml"
    fi
}

poster_cache_dir() {
    echo "$(data_dir)/cache/posters"
}

db_path_from_config() {
    local cfg="$1"
    if [[ -f "$cfg" ]]; then
        # Intenta leer db_path del YAML (formato simple: db_path: "/ruta")
        local val
        val=$(grep -E '^[[:space:]]*db_path:' "$cfg" 2>/dev/null | head -1 | sed -E 's/^[[:space:]]*db_path:[[:space:]]*//; s/^["'"'"']//; s/["'"'"']$//' || true)
        if [[ -n "$val" ]]; then
            echo "$val"
            return 0
        fi
    fi
    # Valor por defecto: data_dir/acryl.db
    echo "$(data_dir)/acryl.db"
}

remove_binary() {
    local target="$BIN_DIR/$APP_NAME"
    if [[ -f "$target" ]]; then
        log "Eliminando binario: $target"
        rm -f "$target"
        log "Binario eliminado."
    else
        warn "No se encontró el binario: $target (se omite)"
    fi
}

remove_desktop() {
    local target="$DESKTOP_DIR/$APP_NAME.desktop"
    if [[ -f "$target" ]]; then
        log "Eliminando entrada de escritorio: $target"
        rm -f "$target"
        log "Entrada de escritorio eliminada."
    else
        warn "No se encontró el archivo .desktop: $target (se omite)"
    fi
}

remove_icon() {
    local removed=0
    for ext in png svg; do
        local target="$ICON_DIR/$APP_NAME.$ext"
        if [[ -f "$target" ]]; then
            log "Eliminando icono: $target"
            rm -f "$target"
            removed=1
        fi
    done
    if [[ "$removed" -eq 0 ]]; then
        warn "No se encontró ningún icono en $ICON_DIR (se omite)"
    else
        log "Icono eliminado."
    fi
}

purge_user_data() {
    local data_dir_path config_file db_file poster_dir

    data_dir_path="$(data_dir)"
    config_file="$(config_path)"
    db_file="$(db_path_from_config "$config_file")"
    poster_dir="$(poster_cache_dir)"

    log "Purgando datos de usuario de $APP_NAME..."

    # Base de datos SQLite
    if [[ -f "$db_file" ]]; then
        log "Eliminando base de datos: $db_file"
        rm -f "$db_file"
    else
        warn "No se encontró la base de datos: $db_file (se omite)"
    fi

    # Configuración YAML
    if [[ -f "$config_file" ]]; then
        log "Eliminando configuración: $config_file"
        rm -f "$config_file"
    else
        warn "No se encontró la configuración: $config_file (se omite)"
    fi

    # Caché de pósters
    if [[ -d "$poster_dir" ]]; then
        log "Eliminando caché de pósters: $poster_dir"
        rm -rf "$poster_dir"
    else
        warn "No se encontró el caché de pósters: $poster_dir (se omite)"
    fi

    # Si el data dir quedó vacío (o solo tiene carpetas vacías), intentar limpiarlo
    if [[ -d "$data_dir_path" ]]; then
        # Elimina carpetas vacías recursivamente hacia arriba hasta data_dir
        find "$data_dir_path" -type d -empty -delete 2>/dev/null || true
        # Si data_dir quedó vacío, lo eliminamos también
        if [[ -d "$data_dir_path" ]] && [[ -z "$(ls -A "$data_dir_path" 2>/dev/null)" ]]; then
            log "Eliminando directorio de datos vacío: $data_dir_path"
            rmdir "$data_dir_path" 2>/dev/null || true
        fi
    fi

    log "Datos de usuario purgados."
}

update_desktop_database() {
    if command -v update-desktop-database >/dev/null 2>&1; then
        if [[ "$SYSTEM_MODE" -eq 1 ]]; then
            sudo update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
        else
            update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
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

    log "=== Desinstalador de $APP_NAME ==="
    confirm
    remove_binary
    remove_desktop
    remove_icon

    if [[ "$PURGE" -eq 1 ]]; then
        purge_user_data
    fi

    update_desktop_database
    update_icon_cache

    log "=== Desinstalación completada ==="
}

main "$@"
