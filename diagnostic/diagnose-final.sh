#!/usr/bin/env bash
#
# diagnose-final.sh — Diagnóstico final para Wayland/Hyprland
#
# Prueba el binario directamente con diferentes configuraciones
# y muestra cualquier error en tiempo real.
#
# Uso:
#   ./scripts/diagnose-final.sh
#
set -euo pipefail

readonly APP_BIN_REAL="$HOME/.local/bin/acryl-real"

info()  { printf '\033[1;34m[INFO]\033[0m %s\n' "$*"; }
ok()    { printf '\033[1;32m[OK]\033[0m  %s\n' "$*"; }
warn()  { printf '\033[1;33m[WARN]\033[0m %s\n' "$*"; }
error() { printf '\033[1;31m[ERROR]\033[0m %s\n' "$*" >&2; }

echo "============================================================"
echo "  Diagnóstico final: Acryl en Wayland/Hyprland"
echo "============================================================"
echo

if [[ ! -f "$APP_BIN_REAL" || ! -x "$APP_BIN_REAL" ]]; then
  error "Binario real no encontrado: $APP_BIN_REAL"
  exit 1
fi

ok "Binario encontrado: $APP_BIN_REAL"
echo

echo "============================================================"
echo "  Prueba 1: Ejecutar directamente con GDK_BACKEND=x11"
echo "============================================================"
echo
echo "Comando: GDK_BACKEND=x11 timeout 10 $APP_BIN_REAL"
echo "(Ventana debería aparecer en 3 segundos. Si no, presiona Ctrl+C)"
echo
read -p "¿Probar? (s/N): " resp
if [[ "$resp" =~ ^[sS]$ ]]; then
  GDK_BACKEND=x11 timeout 10 "$APP_BIN_REAL" 2>&1 || true
fi
echo

echo "============================================================"
echo "  Prueba 2: Con WEBKIT_DISABLE_DMABUF_RENDERER=1"
echo "============================================================"
echo
echo "Comando: GDK_BACKEND=x11 WEBKIT_DISABLE_DMABUF_RENDERER=1 timeout 10 $APP_BIN_REAL"
echo
read -p "¿Probar? (s/N): " resp
if [[ "$resp" =~ ^[sS]$ ]]; then
  GDK_BACKEND=x11 WEBKIT_DISABLE_DMABUF_RENDERER=1 timeout 10 "$APP_BIN_REAL" 2>&1 || true
fi
echo

echo "============================================================"
echo "  Prueba 3: Con LIBGL_ALWAYS_SOFTWARE=1"
echo "============================================================"
echo
echo "Comando: LIBGL_ALWAYS_SOFTWARE=1 GDK_BACKEND=x11 timeout 10 $APP_BIN_REAL"
echo
read -p "¿Probar? (s/N): " resp
if [[ "$resp" =~ ^[sS]$ ]]; then
  LIBGL_ALWAYS_SOFTWARE=1 GDK_BACKEND=x11 timeout 10 "$APP_BIN_REAL" 2>&1 || true
fi
echo

echo "============================================================"
echo "  Conclusión"
echo "============================================================"
echo
echo "Si ninguna prueba mostró la ventana de Acryl, el problema es"
echo "que WebKit2GTK no puede renderizar en Wayland/Hyprland."
echo
echo "SOLUCIÓN DEFINITIVA:"
echo "  Cerrar sesión y seleccionar 'X11' o 'Xorg' en el gestor"
echo "  de inicio (GDM, SDDM, LightDM), luego ejecutar:"
echo "    ~/.local/bin/acryl"
echo
echo "No es necesario reinstalar nada. El problema es el entorno"
echo "gráfico (Wayland), no la aplicación."
