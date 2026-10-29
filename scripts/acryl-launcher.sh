#!/usr/bin/env bash
#
# acryl-launcher.sh — Lanzador de Acryl con captura de errores
#
# Ejecuta la aplicación con variables de entorno para Wayland/Hyprland
# y guarda cualquier salida de error en un archivo de log para diagnóstico.
#
# Uso:
#   ./acryl-launcher.sh
#
set -euo pipefail

readonly APP_NAME="acryl"
readonly APP_BIN="$HOME/.local/bin/${APP_NAME}-real"
readonly LOG_FILE="$HOME/.local/share/acryl/acryl-launch.log"

# Crear directorio de logs
mkdir -p "$HOME/.local/share/acryl"

# Registrar inicio
echo "=== Acryl lanzador: $(date) ===" >> "$LOG_FILE"
echo "DISPLAY=${DISPLAY:-no definida}" >> "$LOG_FILE"
echo "WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-no definida}" >> "$LOG_FILE"
echo "GDK_BACKEND=${GDK_BACKEND:-no definida}" >> "$LOG_FILE"
echo "XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-no definida}" >> "$LOG_FILE"

# Intentar ejecutar con diferentes configuraciones
# Primero intentamos con X11 (funciona mejor con WebKit2GTK)
if [[ -x "$APP_BIN" ]]; then
  echo "Ejecutando con GDK_BACKEND=x11..." >> "$LOG_FILE"
  GDK_BACKEND=x11 "$APP_BIN" >> "$LOG_FILE" 2>&1
  EXIT_CODE=$?
  echo "Código de salida: $EXIT_CODE" >> "$LOG_FILE"
  echo "" >> "$LOG_FILE"
else
  echo "ERROR: Ejecutable no encontrado en $APP_BIN" >> "$LOG_FILE"
  exit 1
fi

exit $EXIT_CODE
