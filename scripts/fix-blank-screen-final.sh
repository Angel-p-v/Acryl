#!/usr/bin/env bash
#
# fix-blank-screen-final.sh — Solución definitiva para pantalla en blanco
#
# Prueba múltiples configuraciones para hacer que Acryl funcione
# en Wayland/Hyprland con WebKit2GTK.
#
# Uso:
#   ./scripts/fix-blank-screen-final.sh
#
set -euo pipefail

readonly APP_BIN="$HOME/.local/bin/acryl"
readonly PROJECT_DIR="$HOME/Projects/Acryl"

info()  { printf '\033[1;34m[INFO]\033[0m %s\n' "$*"; }
ok()    { printf '\033[1;32m[OK]\033[0m  %s\n' "$*"; }
warn()  { printf '\033[1;33m[WARN]\033[0m %s\n' "$*"; }
error() { printf '\033[1;31m[ERROR]\033[0m %s\n' "$*" >&2; }

echo "============================================================"
echo "  Solución definitiva: Acryl pantalla en blanco"
echo "============================================================"
echo

if [[ ! -f "$APP_BIN" || ! -x "$APP_BIN" ]]; then
  error "Ejecutable no encontrado: $APP_BIN"
  exit 1
fi

ok "Ejecutable encontrado: $APP_BIN"
echo

echo "============================================================"
echo "  Prueba 1: GDK_BACKEND=x11 (sin composición WebKit)"
echo "============================================================"
echo
echo "Comando: GDK_BACKEND=x11 WEBKIT_DISABLE_COMPOSITING_MODE=1 $APP_BIN"
echo
read -p "¿Probar ahora? (s/N): " resp
if [[ "$resp" =~ ^[sS]$ ]]; then
  GDK_BACKEND=x11 WEBKIT_DISABLE_COMPOSITING_MODE=1 "$APP_BIN" &
  APP_PID=$!
  sleep 5
  if kill -0 $APP_PID 2>/dev/null; then
    ok "La aplicación se ejecutó durante 5 segundos sin cerrarse"
    kill $APP_PID 2>/dev/null || true
  else
    warn "La aplicación se cerró inmediatamente"
  fi
fi
echo

echo "============================================================"
echo "  Prueba 2: Ejecutar en modo desarrollo (ver errores JS)"
echo "============================================================"
echo
echo "Comando: cd $PROJECT_DIR && npm run tauri:dev"
echo
read -p "¿Abrir modo desarrollo? (s/N): " resp
if [[ "$resp" =~ ^[sS]$ ]]; then
  cd "$PROJECT_DIR" && npm run tauri:dev
fi
echo

echo "============================================================"
echo "  Prueba 3: Usar XWayland explícitamente"
echo "============================================================"
echo
echo "Comando: XDG_SESSION_TYPE=x11 GDK_BACKEND=x11 $APP_BIN"
echo
read -p "¿Probar con XDG_SESSION_TYPE=x11? (s/N): " resp
if [[ "$resp" =~ ^[sS]$ ]]; then
  XDG_SESSION_TYPE=x11 GDK_BACKEND=x11 "$APP_BIN"
fi
echo

echo "============================================================"
echo "  Si nada funciona - problema de WebKit2GTK en Wayland"
echo "============================================================"
echo
echo "WebKit2GTK (usado por Tauri) tiene problemas conocidos en"
echo "Wayland/Hyprland. Soluciones definitivas:"
echo
echo "1. USAR X11 EN LUGAR DE WAYLAND:"
echo "   Cierra la sesión y selecciona 'X11' en el gestor de inicio"
echo
echo "2. INSTALAR Y CONFIGURAR XWAYLAND:"
echo "   sudo pacman -S xorg-xwayland"
echo "   Luego: GDK_BACKEND=x11 ~/.local/bin/acryl"
echo
echo "3. EJECUTAR DESDE TERMINAL PARA VER ERRORES:"
echo "   RUST_BACKTRACE=1 GDK_BACKEND=x11 ~/.local/bin/acryl"
echo "   (Mira la salida en la terminal para ver errores de JS)"
echo
echo "4. REINSTALAR COMPLETAMENTE:"
echo "   ./scripts/install.sh"
