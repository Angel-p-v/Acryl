#!/usr/bin/env bash
#
# acryl-launcher-final.sh — Último intento para Wayland/Hyprland
#
# Prueba TODAS las combinaciones posibles de entorno para WebKit2GTK.
# Si ninguna funciona, guía al usuario sobre qué hacer.
#
set -euo pipefail

readonly APP_BIN_REAL="$HOME/.local/bin/acryl-real"
readonly LOG_FILE="$HOME/.local/share/acryl/acryl-launch.log"

info()  { printf '\033[1;34m[INFO]\033[0m %s\n' "$*"; }
ok()    { printf '\033[1;32m[OK]\033[0m  %s\n' "$*"; }
warn()  { printf '\033[1;33m[WARN]\033[0m %s\n' "$*"; }
error() { printf '\033[1;31m[ERROR]\033[0m %s\n' "$*" >&2; }

mkdir -p "$HOME/.local/share/acryl"

# Función para probar una configuración
try_config() {
  local config_name="$1"
  local env_vars="$2"
  shift 2
  
  echo "" >> "$LOG_FILE"
  echo "Probando: $config_name" >> "$LOG_FILE"
  echo "Variables: $env_vars" >> "$LOG_FILE"
  
  # Ejecutar con timeout de 6 segundos
  timeout 6 env $env_vars "$APP_BIN_REAL" "$@" >> "$LOG_FILE" 2>&1
  local rc=$?
  
  if [[ $rc -eq 0 ]] || [[ $rc -eq 124 ]]; then
    echo "EXITO con: $config_name" >> "$LOG_FILE"
    return 0
  else
    echo "FALLO ($rc) con: $config_name" >> "$LOG_FILE"
    return 1
  fi
}

echo "============================================================"
echo "  Acryl - Último intento para Wayland/Hyprland"
echo "============================================================"
echo

if [[ ! -x "$APP_BIN_REAL" ]]; then
  error "Binario real no encontrado: $APP_BIN_REAL"
  exit 1
fi

echo "Probando todas las configuraciones posibles..."
echo ""

# Lista de configuraciones a probar (de la más probable a la menos probable)
declare -a CONFIGS=(
  "GDK_BACKEND=x11 WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 LIBGL_ALWAYS_SOFTWARE=1"
  "GDK_BACKEND=x11 WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1"
  "GDK_BACKEND=x11 WEBKIT_DISABLE_DMABUF_RENDERER=1"
  "GDK_BACKEND=x11 WEBKIT_DISABLE_COMPOSITING_MODE=1"
  "GDK_BACKEND=x11"
  "XDG_SESSION_TYPE=x11 GDK_BACKEND=x11 WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1"
  "XDG_SESSION_TYPE=x11 GDK_BACKEND=x11 WEBKIT_DISABLE_DMABUF_RENDERER=1"
  "XDG_SESSION_TYPE=x11 GDK_BACKEND=x11"
  "LIBGL_ALWAYS_SOFTWARE=1 GDK_BACKEND=x11 WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1"
  "LIBGL_ALWAYS_SOFTWARE=1 GDK_BACKEND=x11 WEBKIT_DISABLE_DMABUF_RENDERER=1"
  "LIBGL_ALWAYS_SOFTWARE=1 GDK_BACKEND=x11"
  "WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 GDK_BACKEND=x11"
  "WEBKIT_DISABLE_DMABUF_RENDERER=1 GDK_BACKEND=x11"
  "WEBKIT_DISABLE_COMPOSITING_MODE=1 GDK_BACKEND=x11"
  "GDK_BACKEND=wayland WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1"
  "GDK_BACKEND=wayland WEBKIT_DISABLE_DMABUF_RENDERER=1"
  "GDK_BACKEND=wayland"
)

FOUND_WORKING=false

for config in "${CONFIGS[@]}"; do
  echo -n "Probando: $config ... "
  if timeout 5 env $config "$APP_BIN_REAL" > /dev/null 2>&1; then
    ok "FUNCIONÓ"
    FOUND_WORKING=true
    break
  else
    echo "falló"
  fi
done

echo ""

if [[ "$FOUND_WORKING" == "true" ]]; then
  echo "============================================================"
  echo "  ¡ENCONTRADO! Configuración que funciona:"
  echo "============================================================"
  echo
  echo "Agrega esta línea a tu ~/.bashrc o ~/.zshrc:"
  echo "  export GDK_BACKEND=x11"
  echo ""
  echo "O ejecuta la aplicación con:"
  echo "  GDK_BACKEND=x11 ~/.local/bin/acryl"
else
  echo "============================================================"
  echo "  NADA FUNCIONÓ"
  echo "============================================================"
  echo
  echo "WebKit2GTK no puede renderizar en tu entorno Wayland/Hyprland."
  echo ""
  echo "Esto es un problema conocido de Tauri + WebKit2GTK en Wayland."
  echo ""
  echo "Opciones restantes:"
  echo "  1. Usar sesión X11 (cerrar sesión y seleccionar X11 en el login)"
  echo "  2. Usar una distribución con mejor soporte Wayland"
  echo "  3. Esperar a que WebKit2GTK mejore su soporte Wayland"
  echo ""
  echo "El código de Acryl está correcto; el problema es el motor web."
fi
