#!/usr/bin/env bash
#
# acryl-launcher.sh — Lanzador de Acryl con captura de errores
#
# Busca el binario real de Acryl en múltiples ubicaciones y lo ejecuta
# con variables de entorno para Wayland/Hyprland. Guarda cualquier error
# en un archivo de log para diagnóstico.
#
# Uso:
#   ./acryl-launcher.sh [argumentos...]
#
set -euo pipefail

readonly APP_NAME="acryl"

# Posibles ubicaciones del binario real (en orden de preferencia)
readonly REAL_BIN_CANDIDATES=(
  "$HOME/.local/bin/${APP_NAME}-real"
  "/usr/bin/${APP_NAME}"
  "/usr/local/bin/${APP_NAME}"
)

readonly LOG_DIR="$HOME/.local/share/acryl"
readonly LOG_FILE="$LOG_DIR/acryl-launch.log"

# Crear directorio de logs
mkdir -p "$LOG_DIR"

# Registrar inicio
echo "=== Acryl lanzador: $(date) ===" >> "$LOG_FILE"
echo "DISPLAY=${DISPLAY:-no definida}" >> "$LOG_FILE"
echo "WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-no definida}" >> "$LOG_FILE"
echo "GDK_BACKEND=${GDK_BACKEND:-no definida}" >> "$LOG_FILE"
echo "XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-no definida}" >> "$LOG_FILE"
echo "Argumentos: $*" >> "$LOG_FILE"

# Buscar el binario real
REAL_BIN=""
for candidate in "${REAL_BIN_CANDIDATES[@]}"; do
  if [[ -x "$candidate" ]]; then
    REAL_BIN="$candidate"
    break
  fi
done

if [[ -z "$REAL_BIN" ]]; then
  echo "ERROR: No se encontró el binario real de Acryl en ninguna ubicación:" >> "$LOG_FILE"
  for c in "${REAL_BIN_CANDIDATES[@]}"; do
    echo "  - $c" >> "$LOG_FILE"
  done
  echo "" >> "$LOG_FILE"
  exit 1
fi

echo "Ejecutando binario real: $REAL_BIN" >> "$LOG_FILE"

# Ejecutar con X11 (necesario para WebKit2GTK en Wayland/Hyprland)
GDK_BACKEND=x11 "$REAL_BIN" "$@" >> "$LOG_FILE" 2>&1
EXIT_CODE=$?
echo "Código de salida: $EXIT_CODE" >> "$LOG_FILE"
echo "" >> "$LOG_FILE"

exit $EXIT_CODE
