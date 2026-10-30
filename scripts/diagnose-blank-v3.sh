#!/usr/bin/env bash
#
# diagnose-blank-v3.sh — Diagnóstico rápido sin ejecutar la app
#
set -euo pipefail

readonly APP_BIN="$HOME/.local/bin/acryl"
readonly RELEASE_DIR="$HOME/Projects/Acryl/target/release"

info()  { printf '\033[1;34m[INFO]\033[0m %s\n' "$*"; }
ok()    { printf '\033[1;32m[OK]\033[0m  %s\n' "$*"; }
warn()  { printf '\033[1;33m[WARN]\033[0m %s\n' "$*"; }
error() { printf '\033[1;31m[ERROR]\033[0m %s\n' "$*" >&2; }

echo "============================================================"
echo "  Diagnóstico rápido: Acryl pantalla en blanco"
echo "============================================================"
echo

# 1. Verificar archivos del build
info "1. Archivos del build frontend:"
if [[ -f "$BUILD_DIR/index.html" ]]; then
  ok "index.html EXISTE: $BUILD_DIR/index.html"
  echo "   Tamaño: $(du -h "$BUILD_DIR/index.html" | cut -f1)"
  echo "   Contenido:"
  cat "$BUILD_DIR/index.html"
else
  error "index.html NO EXISTE en $BUILD_DIR"
fi
echo

# 2. Verificar ejecutable
info "2. Ejecutable:"
if [[ -f "$APP_BIN" && -x "$APP_BIN" ]]; then
  ok "Ejecutable EXISTE: $APP_BIN"
else
  error "Ejecutable NO EXISTE: $APP_BIN"
fi
echo

# 3. Verificar GTK4 y Libadwaita
info "3. Paquetes GTK4 y Libadwaita instalados:"
if command -v pacman >/dev/null 2>&1; then
  pacman -Qs gtk4 libadwaita 2>/dev/null || echo "   No se encontraron paquetes gtk4/libadwaita"
else
  warn "pacman no disponible"
fi
echo

# 4. Verificar si se puede ejecutar el binario directamente (sin timeout)
info "4. Ejecutable directo (solo verificar que inicia):"
if [[ -f "$APP_BIN" ]]; then
  # Ejecutar solo 3 segundos y capturar salida
  timeout 3 env GDK_BACKEND=x11 "$APP_BIN" 2>&1 || true
  echo "   (Salida capturada arriba)"
fi
echo

echo "============================================================"
echo "  Solución recomendada"
echo "============================================================"
echo
echo "Si la aplicación se abre pero muestra pantalla en blanco,"
echo "prueba estas opciones en orden:"
echo
echo "Opción 1 - Forzar X11 (si usas Wayland/Hyprland):"
echo "  GDK_BACKEND=x11 ~/.local/bin/acryl"
echo
echo "Opción 2 - Ejecutar con backtrace para ver errores de GTK4:"
echo "  RUST_BACKTRACE=1 ~/.local/bin/acryl"
echo
echo "Opción 3 - Verificar que el binario existe y es ejecutable:"
echo "  ls -la ~/.local/bin/acryl"
echo
echo "Opción 4 - Reinstalar completamente:"
echo "  ./scripts/install.sh"
