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
# Además pregunta si quieres desinstalar las dependencias de sistema que
# instaló el installer (GTK4, Libadwaita, mpv, etc.) y si quieres eliminar
# Rust / Cargo (solo si no lo usas para otros proyectos).
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
REMOVE_DEPS=0
REMOVE_RUST=0

# Paquetes que el installer sabe instalar (por gestor de paquetes)
DEPS_APT="libgtk-4-dev libadwaita-1-dev librsvg2-dev libgdk-pixbuf2.0-dev mpv pkg-config"
DEPS_PACMAN="gtk4 libadwaita librsvg gdk-pixbuf2 mpv pkgconf"
DEPS_DNF="gtk4-devel libadwaita-devel librsvg2-devel gdk-pixbuf2-devel mpv pkgconf"
DEPS_ZYPPER="gtk4-devel libadwaita-devel librsvg2-devel gdk-pixbuf2-devel mpv pkgconf"
DEPS_APK="gtk4-dev libadwaita-dev librsvg-dev gdk-pixbuf-dev mpv pkgconf"

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
    command -v "$1" >/dev/null 2>&1 || return 1
    return 0
}

detect_package_manager() {
    if require_cmd apt-get; then echo "apt"; return 0; fi
    if require_cmd pacman; then echo "pacman"; return 0; fi
    if require_cmd dnf; then echo "dnf"; return 0; fi
    if require_cmd zypper; then echo "zypper"; return 0; fi
    if require_cmd apk; then echo "apk"; return 0; fi
    return 1
}

confirm() {
    if [[ "$FORCE" -eq 1 ]]; then
        return 0
    fi
    read -rp "$1 [s/N] " resp
    case "$resp" in
        s|S|y|Y) return 0 ;;
        *) return 1 ;;
    esac
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

    # Rutas de datos (mismo cálculo que el código Rust)
    if [[ -n "${XDG_DATA_HOME:-}" ]]; then
        data_dir_path="${XDG_DATA_HOME}/acryl"
    elif [[ -n "${HOME:-}" ]]; then
        data_dir_path="${HOME}/.local/share/acryl"
    else
        data_dir_path="acryl-data"
    fi

    if [[ -n "${ACRYL_CONFIG:-}" ]]; then
        config_file="${ACRYL_CONFIG}"
    elif [[ -n "${HOME:-}" ]]; then
        config_file="${HOME}/.local/share/acryl/config.yaml"
    else
        config_file="config.yaml"
    fi

    poster_dir="${data_dir_path}/cache/posters"

    # db_path: leer del config.yaml si existe, fallback a data_dir/acryl.db
    db_file="${data_dir_path}/acryl.db"
    if [[ -f "$config_file" ]]; then
        local val
        val=$(grep -E '^[[:space:]]*db_path:' "$config_file" 2>/dev/null | head -1 | sed -E 's/^[[:space:]]*db_path:[[:space:]]*//; s/^["'"'"']//; s/["'"'"']$//' || true)
        if [[ -n "$val" ]]; then
            db_file="$val"
        fi
    fi

    log "Purgando datos de usuario de $APP_NAME..."

    if [[ -f "$db_file" ]]; then
        log "Eliminando base de datos: $db_file"
        rm -f "$db_file"
    else
        warn "No se encontró la base de datos: $db_file (se omite)"
    fi

    if [[ -f "$config_file" ]]; then
        log "Eliminando configuración: $config_file"
        rm -f "$config_file"
    else
        warn "No se encontró la configuración: $config_file (se omite)"
    fi

    if [[ -d "$poster_dir" ]]; then
        log "Eliminando caché de pósters: $poster_dir"
        rm -rf "$poster_dir"
    else
        warn "No se encontró el caché de pósters: $poster_dir (se omite)"
    fi

    if [[ -d "$data_dir_path" ]]; then
        find "$data_dir_path" -type d -empty -delete 2>/dev/null || true
        if [[ -d "$data_dir_path" ]] && [[ -z "$(ls -A "$data_dir_path" 2>/dev/null)" ]]; then
            log "Eliminando directorio de datos vacío: $data_dir_path"
            rmdir "$data_dir_path" 2>/dev/null || true
        fi
    fi

    log "Datos de usuario purgados."
}

remove_system_deps() {
    local pm
    if ! pm=$(detect_package_manager); then
        warn "No se detectó ningún gestor de paquetes soportado. No se pueden eliminar dependencias de sistema."
        return 0
    fi

    local deps=""
    case "$pm" in
        apt)    deps="$DEPS_APT" ;;
        pacman) deps="$DEPS_PACMAN" ;;
        dnf)    deps="$DEPS_DNF" ;;
        zypper) deps="$DEPS_ZYPPER" ;;
        apk)    deps="$DEPS_APK" ;;
    esac

    log "Dependencias de sistema que podría eliminar: $deps"
    warn "AVISO: esto también podría afectar a otras aplicaciones que las usen."

    if ! confirm "¿Desinstalar estas dependencias de sistema?"; then
        log "Se conservan las dependencias de sistema."
        return 0
    fi

    case "$pm" in
        apt)
            sudo apt-get remove -y $deps || warn "Falló la eliminación de algunas dependencias."
            sudo apt-get autoremove -y || true
            ;;
        pacman)
            sudo pacman -Rs --noconfirm $deps || warn "Falló la eliminación de algunas dependencias."
            ;;
        dnf)
            sudo dnf remove -y $deps || warn "Falló la eliminación de algunas dependencias."
            sudo dnf autoremove -y || true
            ;;
        zypper)
            sudo zypper remove -y $deps || warn "Falló la eliminación de algunas dependencias."
            ;;
        apk)
            sudo apk del $deps || warn "Falló la eliminación de algunas dependencias."
            ;;
    esac

    log "Dependencias de sistema eliminadas (o se omitieron las que no se encontraron)."
}

remove_rust() {
    if ! require_cmd cargo; then
        warn "Rust/Cargo no está instalado (no se encontró 'cargo')."
        return 0
    fi

    warn "ADVERTENCIA: si usas Rust/Cargo para otros proyectos, eliminarlo los dejará sin funcionar."
    log "Rust se instaló mediante rustup en ~/.cargo y ~/.rustup."

    if ! confirm "¿Desinstalar Rust y Cargo completamente?"; then
        log "Se conserva Rust/Cargo."
        return 0
    fi

    if [[ -f "$HOME/.cargo/bin/rustup" ]]; then
        "$HOME/.cargo/bin/rustup" self uninstall -y || warn "Falló rustup self uninstall."
    else
        warn "No se encontró rustup en ~/.cargo/bin/rustup."
    fi

    # Eliminar carpetas de rustup y cargo si existen
    local removed_any=0
    for dir in "$HOME/.cargo" "$HOME/.rustup"; do
        if [[ -d "$dir" ]]; then
            log "Eliminando $dir..."
            rm -rf "$dir"
            removed_any=1
        fi
    done

    if [[ "$removed_any" -eq 1 ]]; then
        log "Rust/Cargo eliminado. Recuerda quitar ~/.cargo/bin de tu PATH si lo habías añadido."
    else
        warn "No se encontraron carpetas de Rust/Cargo para eliminar."
    fi
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

    # Confirmación general
    if [[ "$PURGE" -eq 1 ]]; then
        printf '\n'
        warn "ATENCIÓN: --purge borrará TODOS los datos de la aplicación, incluyendo:"
        warn "  - Base de datos SQLite (acryl.db)"
        warn "  - Configuración YAML (config.yaml)"
        warn "  - Caché de pósters descargados"
        warn "  Esta acción es IRREVERSIBLE."
        printf '\n'
        if ! confirm "¿Estás seguro de que quieres desinstalar y borrar todo de $APP_NAME?"; then
            log "Desinstalación cancelada."
            exit 0
        fi
    else
        if ! confirm "¿Estás seguro de que quieres desinstalar $APP_NAME?"; then
            log "Desinstalación cancelada."
            exit 0
        fi
    fi

    # 1. Archivos de la app
    remove_binary
    remove_desktop
    remove_icon

    # 2. Datos de usuario (--purge)
    if [[ "$PURGE" -eq 1 ]]; then
        purge_user_data
    fi

    # 3. Preguntar por dependencias de sistema
    printf '\n'
    log "--- Dependencias de sistema ---"
    warn "El installer pudo haber instalado: GTK4, Libadwaita, librsvg2, gdk-pixbuf2, mpv, pkg-config."
    remove_system_deps

    # 4. Preguntar por Rust / Cargo
    printf '\n'
    log "--- Rust / Cargo ---"
    remove_rust

    # 5. Actualizar cachés
    update_desktop_database
    update_icon_cache

    log "=== Desinstalación completada ==="
}

main "$@"
