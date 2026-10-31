#!/usr/bin/env bash
#
# scripts/update.sh — Actualizador de acryl
#
# Actualiza la aplicación en un solo paso:
#   1. Ejecuta `git pull` para traer los últimos cambios del repositorio.
#   2. Ejecuta `scripts/install.sh`, que instala/actualiza todas las
#      dependencias necesarias (sistema, Rust/Cargo) y recompila la app.
#
# Uso:
#   ./scripts/update.sh                 # actualiza e instala en rutas de usuario
#   sudo ./scripts/update.sh --system   # actualiza e instala en rutas del sistema
#   ./scripts/update.sh --help          # muestra esta ayuda
set -euo pipefail

###############################################################################
# Variables por defecto
###############################################################################
SYSTEM_MODE=0

###############################################################################
# Funciones de ayuda
###############################################################################
log()   { printf '[update] %s\n' "$*"; }
warn()  { printf '[update][advertencia] %s\n' "$*" >&2; }
error() { printf '[update][error] %s\n' "$*" >&2; exit 1; }

usage() {
    cat <<EOF
Uso: $0 [opciones]

Opciones:
  --system     Actualiza e instala en rutas del sistema. Pasa --system al
               instalador al finalizar. Requiere sudo.
  -h, --help   Muestra esta ayuda y sale

Ejemplos:
  $0                      # git pull + install (usuario)
  sudo $0 --system        # git pull + install --system (todos los usuarios)
EOF
}

parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --system)
                SYSTEM_MODE=1; shift ;;
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

main() {
    parse_args "$@"

    if [[ "$SYSTEM_MODE" -eq 1 ]] && [[ "$EUID" -ne 0 ]]; then
        error "--system requiere privilegios de root. Ejecuta con sudo."
    fi

    # Verificar que estamos dentro del repositorio de acryl
    if [[ ! -d .git ]]; then
        error "No se encontró el directorio .git. Ejecuta este script desde la raíz del repositorio de acryl."
    fi

    require_cmd git || error "Falta 'git', necesario para actualizar."

    log "=== Actualizador de acryl ==="

    # Obtener la rama actual para hacer pull sobre ella
    local current_branch
    current_branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null) || error "No se pudo obtener la rama actual."

    log "Rama actual: $current_branch"

    # Verificar que no haya cambios sin confirmar que se podrían perder
    if [[ -n "$(git status --porcelain 2>/dev/null)" ]]; then
        warn "Hay cambios sin confirmar en el working tree."
        read -rp "¿Continuar con el git pull de todos modos? Los cambios locales podrían entrar en conflicto. [s/N] " resp
        case "$resp" in
            s|S|y|Y) ;;
            *) log "Actualización cancelada."; exit 0 ;;
        esac
    fi

    log "Ejecutando git pull..."
    git pull || error "Falló el git pull. Resuelve los conflictos manualmente y vuelve a intentar."

    log "Cambios obtenidos correctamente desde el remoto."

    # Ejecutar el instalador (instala/actualiza dependencias y compila)
    log "Ejecutando scripts/install.sh..."
    if [[ "$SYSTEM_MODE" -eq 1 ]]; then
        sudo ./scripts/install.sh --system
    else
        ./scripts/install.sh
    fi

    log "=== Actualización completada ==="
    log "Acryl está actualizado. Puedes lanzarlo con: acryl"
}

main "$@"
