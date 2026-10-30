#!/usr/bin/env bash
#
# diagnose-blank-v2.sh — Diagnóstico avanzado para pantalla en blanco
#
# Comprueba:
#   1. Si el binario compilado existe
#   2. Si GTK puede renderizar la ventana
#   3. Variables de entorno de GTK para depuración
#   4. Ejecutar con diferentes configuraciones de GTK
#
# Uso:
#   ./scripts/diagnose-blank-v2.sh
#
set -euo pipefail

readonly APP_NAME="acryl"
readonly APP_BIN="$HOME/.local/bin/$APP_NAME"
readonly RELEASE_DIR="$HOME/Projects/Acryl/target/release"
readonly DESKTOP_FILE="$HOME/.local/share/applications/$APP_NAME.desktop"

info()  { printf '\033[1;34m[INFO]\033[0m %s\n' "$*"; }
ok()    { printf '\033[1;32m[OK]\033[0m  %s\n' "$*"; }
warn()  { printf '\033[1;33m[WARN]\033[0m %s\n' "$*"; }
error() { printf '\033[1;31m[ERROR]\033[0m %s\n' "$*" >&2; }

have() { command -v "$1" >/dev/null 2>&1; }

echo "============================================================"
echo "  Diagnóstico avanzado: Pantalla en blanco"
echo "============================================================"
echo

# -----------------------------------------------------------------------------
# 1. Verificar archivos del build
# -----------------------------------------------------------------------------
info "Verificando archivos del build frontend..."
if [[ -f "$BUILD_DIR/index.html" ]]; then
  ok "index.html encontrado: $BUILD_DIR/index.html"
  echo "  Contenido (primeras 5 líneas):"
  head -5 "$BUILD_DIR/index.html"
else
  error "index.html NO encontrado en $BUILD_DIR"
  echo "  Contenido del directorio:"
  ls -la "$BUILD_DIR/" 2>/dev/null || echo "  Directorio no existe"
fi
echo

# -----------------------------------------------------------------------------
# 2. Verificar el ejecutable
# -----------------------------------------------------------------------------
info "Verificando ejecutable..."
if [[ -f "$APP_BIN" && -x "$APP_BIN" ]]; then
  ok "Ejecutable encontrado: $APP_BIN"
  echo "  Tamaño: $(du -h "$APP_BIN" | cut -f1)"
else
  error "Ejecutable NO encontrado o no ejecutable: $APP_BIN"
fi
echo

# -----------------------------------------------------------------------------
# 3. Verificar .desktop
# -----------------------------------------------------------------------------
info "Verificando entrada .desktop..."
if [[ -f "$DESKTOP_FILE" ]]; then
  ok "Archivo .desktop encontrado"
  echo "  Exec: $(grep '^Exec=' "$DESKTOP_FILE")"
else
  warn "No se encontró .desktop en $DESKTOP_FILE"
fi
echo

# -----------------------------------------------------------------------------
# 4. Probar ejecutar con diferentes configuraciones
# -----------------------------------------------------------------------------
echo "============================================================"
echo "  Pruebas de ejecución"
echo "============================================================"
echo

echo "Prueba 1: Ejecutar con GDK_BACKEND=x11 (timeout 10s)"
echo "------------------------------------------------------------"
timeout 10 env GDK_BACKEND=x11 "$APP_BIN" 2>&1 || true
echo "------------------------------------------------------------"
echo

echo "Prueba 2: Ejecutar con WEBKIT_DISABLE_COMPOSITING_MODE=1"
echo "------------------------------------------------------------"
timeout 10 env GDK_BACKEND=x11 WEBKIT_DISABLE_COMPOSITING_MODE=1 "$APP_BIN" 2>&1 || true
echo "------------------------------------------------------------"
echo

echo "Prueba 3: Ejecutar con GTK_DEBUG=interactive"
echo "------------------------------------------------------------"
timeout 10 env GDK_BACKEND=x11 GTK_DEBUG=interactive "$APP_BIN" 2>&1 || true
echo "------------------------------------------------------------"
echo

# -----------------------------------------------------------------------------
# 5. Verificar si hay archivos de log en /tmp o /var/tmp
# -----------------------------------------------------------------------------
info "Buscando logs temporales de Acryl..."
find /tmp /var/tmp -name '*acryl*' -o -name '*.log' 2>/dev/null | head -10 || true
echo

# -----------------------------------------------------------------------------
# 6. Verificar permisos de los archivos del build
# -----------------------------------------------------------------------------
info "Verificando permisos de archivos del build..."
if [[ -d "$BUILD_DIR" ]]; then
  ls -la "$BUILD_DIR/" 2>/dev/null | head -10
else
  warn "Directorio build no existe: $BUILD_DIR"
fi
echo

# -----------------------------------------------------------------------------
# 7. Recomendaciones
# -----------------------------------------------------------------------------
echo "============================================================"
echo "  Recomendaciones"
echo "============================================================"
echo
echo "Si ninguna prueba mostró la aplicación:"
echo
echo "1. Verificar que GTK4 y Libadwaita están instalados:"
echo "   pacman -Qs gtk4 libadwaita"
echo
echo "2. Probar ejecutar directamente el binario:"
echo "   /home/coronel/Projects/Acryl/target/release/acryl"
echo
echo "3. Verificar si el problema es de permisos en /tmp:"
echo "   ls -la /tmp/ | grep acryl"
echo
echo "4. Probar en un entorno X11 puro (no Wayland):"
echo "   XDG_SESSION_TYPE=x11 GDK_BACKEND=x11 ~/.local/bin/acryl"
echo
