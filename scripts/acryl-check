#!/usr/bin/env bash
#
# scripts/acryl-check — Comprobación general de Acryl
#
# Único script de diagnóstico. Responde a:
#   - "¿Acryl está instalado y listo?"          (salud general)
#   - "¿Por qué no se ve la ventana / pantalla en blanco?" (gráficos)
#
# Comprueba de forma completa:
#   1. Entorno de sesión (DISPLAY, WAYLAND_DISPLAY, XDG_SESSION_TYPE, etc.)
#   2. Ejecutable de Acryl (existencia, permisos, librerías)
#   3. Ejecución directa (timeout)
#   4. Base de datos SQLite (medios indexados)
#   5. Archivos de log de la aplicación
#   6. Configuración YAML
#   7. Entrada de menú .desktop
#   8. Entorno gráfico (Wayland / Hyprland / X11)
#   9. Pruebas de ejecución con diferentes backends GTK y WebKit
#  10. Resumen y recomendaciones concretas para cada caso
#
# Uso:
#   ./scripts/acryl-check                 # rutas por defecto (usuario)
#   ./scripts/acryl-check --bin /ruta/a/acryl
#   ./scripts/acryl-check --db  /ruta/a/acryl.db
#   ./scripts/acryl-check --help
#
set -euo pipefail

###############################################################################
# Variables por defecto
###############################################################################
APP_NAME="acryl"
BIN_PATHS=(
  "$HOME/.local/bin/$APP_NAME"
  "/usr/local/bin/$APP_NAME"
  "/usr/bin/$APP_NAME"
)
DB_PATHS=(
  "${XDG_DATA_HOME:-$HOME/.local/share}/acryl/acryl.db"
  "$HOME/.local/share/acryl/acryl.db"
  "$PWD/acryl.db"
)
LOG_DIRS=(
  "${XDG_DATA_HOME:-$HOME/.local/share}/acryl/logs"
  "$HOME/.local/share/acryl/logs"
  "$HOME/.cache/acryl"
)
CONFIG_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/acryl/config.yaml"
DESKTOP_FILE="$HOME/.local/share/applications/$APP_NAME.desktop"

BIN_ARG=""
DB_ARG=""

info()  { printf '\033[1;34m[INFO]\033[0m %s\n' "$*"; }
ok()    { printf '\033[1;32m[OK]\033[0m  %s\n' "$*"; }
warn()  { printf '\033[1;33m[WARN]\033[0m %s\n' "$*"; }
error() { printf '\033[1;31m[ERROR]\033[0m %s\n' "$*" >&2; }

have() { command -v "$1" >/dev/null 2>&1; }

usage() {
    cat <<EOF
Uso: $0 [opciones]

Comprobación general de Acryl: salud de la instalación y diagnóstico gráfico.

Opciones:
  --bin RUTA   Ruta al ejecutable de Acryl (por defecto: busca en rutas habituales)
  --db  RUTA   Ruta a la base de datos SQLite (por defecto: busca en rutas habituales)
  -h, --help   Muestra esta ayuda y sale

Ejemplos:
  $0                           # chequeo completo (salud + gráficos)
  $0 --bin ~/.local/bin/acryl  # usa este ejecutable
  $0 --db ~/.local/share/acryl/acryl.db  # usa esta base de datos
EOF
}

###############################################################################
# Argumentos
###############################################################################
while [[ $# -gt 0 ]]; do
    case "$1" in
        --bin)  [[ $# -lt 2 ]] && error "--bin requiere una ruta"; BIN_ARG="$2"; shift 2 ;;
        --db)   [[ $# -lt 2 ]] && error "--db requiere una ruta"; DB_ARG="$2"; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) error "Opción desconocida: $1. Ejecuta '$0 --help'." ;;
    esac
done

###############################################################################
# Cabecera
###############################################################################
echo "============================================================"
echo "  Acryl Check — Comprobación general de Acryl"
echo "============================================================"
echo

###############################################################################
# 1. Entorno de sesión
###############################################################################
info "1. Entorno de sesión gráfica"
echo "  DISPLAY=${DISPLAY:-no definida}"
echo "  WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-no definida}"
echo "  XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-no definida}"
echo "  XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-no definida}"
echo "  GDK_BACKEND=${GDK_BACKEND:-no definida}"
echo "  LIBGL_ALWAYS_SOFTWARE=${LIBGL_ALWAYS_SOFTWARE:-no definida}"
echo "  WEBKIT_DISABLE_DMABUF_RENDERER=${WEBKIT_DISABLE_DMABUF_RENDERER:-no definida}"
echo "  WEBKIT_DISABLE_COMPOSITING_MODE=${WEBKIT_DISABLE_COMPOSITING_MODE:-no definida}"
echo

###############################################################################
# 2. Ejecutable de Acryl
###############################################################################
info "2. Ejecutable de Acryl"
FOUND_BIN=""
if [[ -n "$BIN_ARG" ]]; then
    if [[ -f "$BIN_ARG" && -x "$BIN_ARG" ]]; then
        ok "Ejecutable (indicado): $BIN_ARG"
        FOUND_BIN="$BIN_ARG"
    else
        error "El ejecutable indicado no existe o no es ejecutable: $BIN_ARG"
    fi
else
    for path in "${BIN_PATHS[@]}"; do
        if [[ -f "$path" && -x "$path" ]]; then
            ok "Ejecutable encontrado: $path"
            FOUND_BIN="$path"
            break
        fi
    done
    if [[ -z "$FOUND_BIN" ]]; then
        error "No se encontró el ejecutable de Acryl en ninguna ruta conocida."
        echo "  Rutas buscadas:"
        for path in "${BIN_PATHS[@]}"; do
            echo "    - $path"
        done
        echo
        echo "  Solución: instala Acryl con ./scripts/install.sh"
        exit 1
    fi
fi

if have ldd; then
    MISSING=$(ldd "$FOUND_BIN" 2>/dev/null | grep "not found" || true)
    if [[ -n "$MISSING" ]]; then
        error "Librerías del sistema faltantes:"
        echo "$MISSING"
        echo
        echo "  Solución: instala las dependencias de GTK4, Libadwaita y gdk-pixbuf2."
    else
        ok "Todas las librerías del binario están presentes."
    fi
else
    warn "ldd no disponible; se omite la verificación de librerías."
fi
echo

###############################################################################
# 3. Ejecución directa
###############################################################################
info "3. Ejecución directa (timeout 5 segundos)"
echo "  Comando: timeout 5 $FOUND_BIN"
echo "------------------------------------------------------------"
timeout 5 "$FOUND_BIN" 2>&1 || true
echo "------------------------------------------------------------"
RC=$?
if [[ $RC -eq 124 ]]; then
    ok "La aplicación se ejecutó sin cerrarse durante 5 segundos."
elif [[ $RC -eq 139 ]]; then
    error "La aplicación falló con SIGSEGV (segmentation fault)."
elif [[ $RC -ne 0 ]]; then
    error "La aplicación terminó con código de error $RC."
else
    info "La aplicación terminó normalmente (código 0)."
fi
echo

###############################################################################
# 4. Base de datos SQLite
###############################################################################
info "4. Base de datos SQLite y medios indexados"
FOUND_DB=false
if [[ -n "$DB_ARG" ]]; then
    if [[ -f "$DB_ARG" ]]; then
        ok "Base de datos (indicada): $DB_ARG"
        FOUND_DB=true
    else
        warn "La base de datos indicada no existe: $DB_ARG"
    fi
else
    for db in "${DB_PATHS[@]}"; do
        if [[ -f "$db" ]]; then
            ok "Base de datos encontrada: $db"
            FOUND_DB=true
            if have sqlite3; then
                TABLES=$(sqlite3 "$db" ".tables" 2>/dev/null || true)
                echo "  Tablas: $TABLES"
                if echo "$TABLES" | grep -q "media"; then
                    COUNT=$(sqlite3 "$db" "SELECT COUNT(*) FROM media;" 2>/dev/null || echo "0")
                    echo "  Medios indexados: $COUNT"
                    if [[ "$COUNT" -eq 0 ]]; then
                        warn "La base de datos está vacía: no hay medios para mostrar."
                        echo "  Solución: indexa una carpeta de medios desde Ajustes."
                    fi
                fi
                if echo "$TABLES" | grep -q "media_files"; then
                    FILES=$(sqlite3 "$db" "SELECT COUNT(*) FROM media_files;" 2>/dev/null || echo "0")
                    echo "  Archivos indexados: $FILES"
                fi
            else
                warn "sqlite3 no instalado; no se puede inspeccionar la base de datos."
            fi
            break
        fi
    done
fi
if [[ "$FOUND_DB" == "false" ]]; then
    warn "No se encontró ninguna base de datos de Acryl."
    echo "  Rutas buscadas:"
    for db in "${DB_PATHS[@]}"; do
        echo "    - $db"
    done
fi
echo

###############################################################################
# 5. Logs de la aplicación
###############################################################################
info "5. Archivos de log de Acryl"
FOUND_LOG=false
for dir in "${LOG_DIRS[@]}"; do
    if [[ -d "$dir" ]]; then
        ok "Directorio de logs: $dir"
        find "$dir" -type f \( -name '*.log' -o -name '*.txt' \) 2>/dev/null | while read -r f; do
            ok "Log: $f"
            echo "  Últimas 20 líneas:"
            tail -20 "$f" 2>/dev/null || true
            echo
        done
        FOUND_LOG=true
    fi
done
if [[ "$FOUND_LOG" == "false" ]]; then
    warn "No se encontraron archivos de log de Acryl."
fi
echo

###############################################################################
# 6. Configuración YAML
###############################################################################
info "6. Configuración YAML ($CONFIG_FILE)"
if [[ -f "$CONFIG_FILE" ]]; then
    ok "Archivo de configuración encontrado."
    if have grep; then
        echo "  Contenido relevante:"
        grep -E '^(language|db_path|media_folders)' "$CONFIG_FILE" 2>/dev/null || true
    fi
else
    warn "No se encontró el archivo de configuración."
    echo "  Ruta esperada: $CONFIG_FILE"
    echo "  Solución: abre Acryl y guarda los ajustes para crearlo."
fi
echo

###############################################################################
# 7. Entrada .desktop
###############################################################################
info "7. Entrada de menú .desktop"
if [[ -f "$DESKTOP_FILE" ]]; then
    ok "Archivo .desktop encontrado: $DESKTOP_FILE"
else
    warn "No se encontró .desktop en $DESKTOP_FILE"
    echo "  Solución: ejecuta ./scripts/install.sh para crear la entrada de menú."
fi
echo

###############################################################################
# 8. Diagnóstico gráfico (Wayland / Hyprland / X11)
###############################################################################
info "8. Diagnóstico gráfico y de pantalla"
echo
if [[ -z "$FOUND_BIN" ]]; then
    error "No se puede diagnosticar gráficos sin el ejecutable de Acryl."
else
    echo "  Entorno gráfico detectado:"
    echo "    DISPLAY=${DISPLAY:-no definida}"
    echo "    WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-no definida}"
    echo "    XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-no definida}"
    echo "    GDK_BACKEND=${GDK_BACKEND:-no definida}"
    echo
    echo "  Probando ejecución con diferentes configuraciones gráficas:"
    echo "------------------------------------------------------------"

    run_test() {
        local label="$1"
        shift
        echo "  $label"
        echo "  Comando: $*"
        timeout 8 env "$@" "$FOUND_BIN" 2>&1 | head -20 || true
        echo
    }

    run_test "Prueba 1: GDK_BACKEND=x11 (forzar X11 sobre Wayland)" GDK_BACKEND=x11
    run_test "Prueba 2: WebKit sin renderizado DMABUF" GDK_BACKEND=x11 WEBKIT_DISABLE_DMABUF_RENDERER=1
    run_test "Prueba 3: Renderizado OpenGL por software" LIBGL_ALWAYS_SOFTWARE=1 GDK_BACKEND=x11
    run_test "Prueba 4: Combinación completa (X11 + software + sin DMABUF + sin composición)" GDK_BACKEND=x11 WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 LIBGL_ALWAYS_SOFTWARE=1

    echo "------------------------------------------------------------"
    echo
fi

###############################################################################
# 9. Resumen y recomendaciones
###############################################################################
echo "============================================================"
echo "  Resumen y recomendaciones"
echo "============================================================"
echo

if [[ -z "$FOUND_BIN" ]]; then
    echo "CRÍTICO: Acryl no está instalado."
    echo "  Ejecuta: ./scripts/install.sh"
elif [[ "$FOUND_DB" == "false" ]]; then
    echo "AVISO: Acryl está instalado pero no hay medios indexados."
    echo "  Abre Acryl y configura al menos una carpeta de medios en Ajustes."
else
    echo "Salud general: todo parece correcto."
    echo
    echo "Si la aplicación no se ve o muestra pantalla en blanco, el problema"
    echo "es de gráficos (Wayland / Hyprland). Prueba en este orden:"
    echo
    echo "  1. Instalar XWayland:"
    echo "     Arch:       sudo pacman -S xorg-xwayland"
    echo "     Debian/Ub:  sudo apt install xserver-xorg-core"
    echo
    echo "  2. Usar sesión X11 en lugar de Wayland:"
    echo "     Cierra sesión y selecciona 'X11' o 'Xorg' en el gestor de inicio."
    echo
    echo "  3. Variables de entorno (exportar antes de lanzar):"
    echo "     export GDK_BACKEND=x11"
    echo "     export WEBKIT_DISABLE_DMABUF_RENDERER=1"
    echo "     export WEBKIT_DISABLE_COMPOSITING_MODE=1"
    echo "     export LIBGL_ALWAYS_SOFTWARE=1"
    echo "     ~/.local/bin/acryl"
    echo
    echo "El problema suele ser el entorno gráfico, no la aplicación."
    echo "No es necesario reinstalar nada."
fi
echo
