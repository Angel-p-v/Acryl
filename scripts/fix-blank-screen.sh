#!/usr/bin/env bash
#
# fix-blank-screen.sh — Solución para pantalla en blanco en Acryl
#
# Prueba diferentes configuraciones para hacer que Acryl funcione
# en Wayland/Hyprland.
#
# Uso:
#   ./scripts/fix-blank-screen.sh
#
set -euo pipefail

readonly APP_BIN="$HOME/.local/bin/acryl"

info()  { printf '\033[1;34m[INFO]\033[0m %s\n' "$*"; }
ok()    { printf '\033[1;32m[OK]\033[0m  %s\n' "$*"; }
warn()  { printf '\033[1;33m[WARN]\033[0m %s\n' "$*"; }
error() { printf '\033[1;31m[ERROR]\033[0m %s\n' "$*" >&2; }

echo "============================================================"
echo "  Solución: Acryl pantalla en blanco (Wayland/Hyprland)"
echo "============================================================"
echo

# Verificar que el ejecutable existe
if [[ ! -f "$APP_BIN" || ! -x "$APP_BIN" ]]; then
  error "Ejecutable no encontrado: $APP_BIN"
  echo "   Ejecuta primero ./scripts/install.sh"
  exit 1
fi

ok "Ejecutable encontrado: $APP_BIN"
echo

echo "============================================================"
echo "  Prueba 1: GDK_BACKEND=x11 + WEBKIT_DISABLE_COMPOSITING_MODE=1"
echo "============================================================"
echo
echo "Ejecutando: GDK_BACKEND=x11 WEBKIT_DISABLE_COMPOSITING_MODE=1 $APP_BIN"
echo "(La ventana debería abrirse. Si se cierra, presiona Ctrl+C)"
echo
read -p "¿Quieres probar esta opción? (s/N): " resp
if [[ "$resp" =~ ^[sS]$ ]]; then
  GDK_BACKEND=x11 WEBKIT_DISABLE_COMPOSITING_MODE=1 "$APP_BIN"
fi
echo

echo "============================================================"
echo "  Prueba 2: Solo GDK_BACKEND=x11"
echo "============================================================"
echo
echo "Ejecutando: GDK_BACKEND=x11 $APP_BIN"
echo "(La ventana debería abrirse. Si se cierra, presiona Ctrl+C)"
echo
read -p "¿Quieres probar esta opción? (s/N): " resp
if [[ "$resp" =~ ^[sS]$ ]]; then
  GDK_BACKEND=x11 "$APP_BIN"
fi
echo

echo "============================================================"
echo "  Prueba 3: Ejecutar desde terminal para ver errores"
echo "============================================================"
echo
echo "Ejecutar: RUST_BACKTRACE=1 $APP_BIN"
echo "(Revisa la salida de la terminal para ver errores de GTK4)"
echo
read -p "¿Quieres probar esta opción? (s/N): " resp
if [[ "$resp" =~ ^[sS]$ ]]; then
  RUST_BACKTRACE=1 "$APP_BIN"
fi
echo

echo "============================================================"
echo "  Si nada funciona"
echo "============================================================"
echo
echo "Si la aplicación no se abre o queda en blanco, prueba:"
echo
echo "1. Usar un servidor X11 (Xorg) en lugar de Wayland"
echo "2. Usar XWayland: GDK_BACKEND=x11 ~/.local/bin/acryl"
echo "3. Instalar xwayland si no está instalado:"
echo "   sudo pacman -S xorg-xwayland"
echo
echo "4. Reportar el issue en GitHub con la salida de:"
echo "   RUST_BACKTRACE=1 GDK_BACKEND=x11 ~/.local/bin/acryl"
