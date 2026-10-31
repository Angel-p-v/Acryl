#!/usr/bin/env bash
#
# scripts/install.sh — Instalador de acryl (Gestor de contenido multimedia)
#
# Instala absolutamente todo lo necesario para que Acryl funcione:
#   1. Detecta el gestor de paquetes de tu distro e instala las dependencias
#      de sistema (GTK4, Libadwaita, librsvg2, gdk-pixbuf2, mpv, pkg-config).
#   2. Instala Rust / Cargo (≥ 1.90) si falta o está desactualizado.
#   3. Compila la aplicación con cargo build --release.
#   4. Instala el binario, el archivo .desktop y el icono.
#
# Por defecto los archivos de la app se instalan a nivel de usuario
# (~/.local), sin necesidad de root para ellos. Solo se usa sudo para
# instalar dependencias de sistema.
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
MIN_RUST_VERSION="1.90"

# Paquetes de sistema por gestor de paquetes
DEPS_APT="libgtk-4-dev libadwaita-1-dev librsvg2-dev libgdk-pixbuf2.0-dev mpv pkg-config"
DEPS_PACMAN="gtk4 libadwaita librsvg gdk-pixbuf2 mpv pkgconf base-devel"
DEPS_DNF="gtk4-devel libadwaita-devel librsvg2-devel gdk-pixbuf2-devel mpv pkgconf"
DEPS_ZYPPER="gtk4-devel libadwaita-devel librsvg2-devel gdk-pixbuf2-devel mpv pkgconf"
DEPS_APK="gtk4-dev libadwaita-dev librsvg-dev gdk-pixbuf-dev mpv pkgconf"

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
  $0                      # instala todo y pone la app en ~/.local (usuario)
  sudo $0 --system        # instala todo y pone la app en /usr/local (sistema)
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
    command -v "$1" >/dev/null 2>&1 || return 1
    return 0
}

# Devuelve 0 si $1 >= $2 (versiones semánticas simples X.Y)
version_ge() {
    printf '%s\n%s\n' "$2" "$1" | sort -V -C 2>/dev/null
}

detect_package_manager() {
    if require_cmd apt-get; then echo "apt"; return 0; fi
    if require_cmd pacman; then echo "pacman"; return 0; fi
    if require_cmd dnf; then echo "dnf"; return 0; fi
    if require_cmd zypper; then echo "zypper"; return 0; fi
    if require_cmd apk; then echo "apk"; return 0; fi
    return 1
}

install_system_deps() {
    local pm="$1"

    log "Instalando dependencias de sistema con $pm..."
    case "$pm" in
        apt)
            sudo apt-get update
            sudo apt-get install -y $DEPS_APT
            ;;
        pacman)
            sudo pacman -Sy --needed $DEPS_PACMAN
            ;;
        dnf)
            sudo dnf install -y $DEPS_DNF
            ;;
        zypper)
            sudo zypper install -y $DEPS_ZYPPER
            ;;
        apk)
            sudo apk add $DEPS_APK
            ;;
        *)
            error "Gestor de paquetes '$pm' no soportado. Instala manualmente: GTK4, Libadwaita, librsvg2, gdk-pixbuf2, mpv, pkg-config."
            ;;
    esac
    log "Dependencias de sistema instaladas."
}

ensure_rust() {
    log "Verificando Rust / Cargo..."

    if require_cmd cargo; then
        local cargo_version
        cargo_version=$(cargo --version | grep -oE '[0-9]+\.[0-9]+' | head -1)
        if [[ -n "$cargo_version" ]] && version_ge "$cargo_version" "$MIN_RUST_VERSION"; then
            log "Cargo $cargo_version ya está instalado (>= $MIN_RUST_VERSION)."
            return 0
        fi
        warn "Cargo $cargo_version está instalado pero se requiere >= $MIN_RUST_VERSION."
    else
        warn "Cargo no está instalado."
    fi

    read -rp "¿Instalar/actualizar Rust y Cargo con rustup? [s/N] " resp
    case "$resp" in
        s|S|y|Y) ;;
        *) error "Rust/Cargo es requerido para compilar acryl." ;;
    esac

    if ! require_cmd curl; then
        error "Falta 'curl', necesario para descargar rustup."
    fi

    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

    # Cargar el entorno de rustup para la sesión actual
    if [[ -f "$HOME/.cargo/env" ]]; then
        # shellcheck source=/dev/null
        . "$HOME/.cargo/env"
    elif [[ -f "$HOME/.env" ]]; then
        . "$HOME/.env"
    fi

    if ! require_cmd cargo; then
        error "Rust/Cargo se instaló pero no se encontró 'cargo' en el PATH. Añade ~/.cargo/bin a tu PATH y vuelve a ejecutar."
    fi

    local new_version
    new_version=$(cargo --version | grep -oE '[0-9]+\.[0-9]+' | head -1)
    if [[ -n "$new_version" ]] && ! version_ge "$new_version" "$MIN_RUST_VERSION"; then
        error "La versión instalada de Cargo ($new_version) es menor que la requerida ($MIN_RUST_VERSION)."
    fi

    log "Rust/Cargo listo (Cargo $new_version)."
}

check_build_deps() {
    log "Verificando dependencias de compilación..."
    require_cmd pkg-config || error "Falta 'pkg-config'. Instálalo primero."
    pkg-config --exists gtk4 || error "Falta gtk4 (librerías de desarrollo de GTK4)."
    pkg-config --exists libadwaita-1 || error "Falta libadwaita-1 (librerías de desarrollo de Libadwaita)."
    log "Dependencias de compilación OK."
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
    local src_icon=""
    if [[ -f "icons/acryl.svg" ]]; then
        src_icon="icons/acryl.svg"
    elif [[ -f "icons/acryl.png" ]]; then
        src_icon="icons/acryl.png"
    fi

    if [[ -z "$src_icon" ]]; then
        warn "No se encontró ningún icono en icons/ (se omite la instalación del icono)"
        return 0
    fi

    local ext="${src_icon##*.}"
    log "Instalando icono en $ICON_DIR..."
    mkdir -p "$ICON_DIR"
    install -m 0644 "$src_icon" "$ICON_DIR/$APP_NAME.$ext"
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

    # 1. Dependencias de sistema
    local pm
    if pm=$(detect_package_manager); then
        install_system_deps "$pm"
    else
        warn "No se detectó ningún gestor de paquetes soportado (apt, pacman, dnf, zypper, apk)."
        warn "Asegúrate de tener instaladas manualmente: GTK4, Libadwaita, librsvg2, gdk-pixbuf2, mpv, pkg-config."
    fi

    # 2. Rust / Cargo
    ensure_rust

    # 3. Dependencias de compilación (pkg-config, gtk4, libadwaita)
    check_build_deps

    # 4. Compilar e instalar
    build_release
    install_binary
    install_desktop
    install_icon
    update_desktop_database
    update_icon_cache

    log "=== Instalación completada ==="
    log "Puedes lanzar la aplicación con: $APP_NAME"
    log "O desde el menú de aplicaciones si está disponible."
    if [[ "$SYSTEM_MODE" -eq 0 ]]; then
        log "Si el comando no se encuentra, añade ~/.local/bin a tu PATH."
    fi
}

main "$@"
