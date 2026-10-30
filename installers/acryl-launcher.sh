#!/usr/bin/env bash
#
# acryl-launcher.sh — Lanzador de Acryl con múltiples configuraciones Wayland/X11
#
# Prueba diferentes configuraciones de entorno para hacer que GTK4/Libadwaita
# renderice correctamente en Wayland/Hyprland. Guarda los errores en un log.
#
# Uso:
#   ./acryl-launcher.sh [argumentos...]
#
set -euo pipefail

readonly APP_NAME="acryl"

# Posibles ubicaciones del binario real (instalado por install.sh como acryl-real)
readonly REAL_BIN_CANDIDATES=(
  "$HOME/.local/bin/${APP_NAME}-real"
  "/usr/bin/${APP_NAME}-real"
  "/usr/local/bin/${APP_NAME}-real"
)

readonly LOG_DIR="$HOME/.local/share/acryl"
readonly LOG_FILE="$LOG_DIR/acryl-launch.log"

# Configuraciones de entorno a probar (en orden de preferencia)
# Si estamos en Wayland, probamos wayland primero; si en X11, x11 primero.
readonly CONFIGS=(
  "GDK_BACKEND=wayland"
  "GDK_BACKEND=x11"
  "XDG_SESSION_TYPE=x11 GDK_BACKEND=x11"
  "LIBGL_ALWAYS_SOFTWARE=1 GDK_BACKEND=x11"
)

# Crear directorio de logs
mkdir -p "$LOG_DIR"

# Registrar inicio
{
  echo "=== Acryl lanzador: $(date) ==="
  echo "DISPLAY=${DISPLAY:-no definida}"
  echo "WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-no definida}"
  echo "GDK_BACKEND=${GDK_BACKEND:-no definida}"
  echo "XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-no definida}"
  echo "Argumentos: $*"
} >> "$LOG_FILE"

# Buscar el binario real
REAL_BIN=""
for candidate in "${REAL_BIN_CANDIDATES[@]}"; do
  if [[ -x "$candidate" ]]; then
    REAL_BIN="$candidate"
    break
  fi
done

if [[ -z "$REAL_BIN" ]]; then
  {
    echo "ERROR: No se encontró el binario real de Acryl en ninguna ubicación:"
    for c in "${REAL_BIN_CANDIDATES[@]}"; do
      echo "  - $c"
    done
    echo ""
  } >> "$LOG_FILE"
  exit 1
fi

echo "Ejecutando binario real: $REAL_BIN" >> "$LOG_FILE"

# Probar cada configuración (ejecutar en foreground, sin timeout que mate la app)
for config in "${CONFIGS[@]}"; do
  echo "" >> "$LOG_FILE"
  echo "Probando configuración: $config" >> "$LOG_FILE"

  # Ejecutar en foreground: el launcher espera a que el binario termine
  if env $config "$REAL_BIN" "$@" >> "$LOG_FILE" 2>&1; then
    echo "EXITO: Configuración funcionó: $config" >> "$LOG_FILE"
    echo "" >> "$LOG_FILE"
    exit 0
  else
    exit_code=$?
    echo "FALLÓ con código $exit_code: $config" >> "$LOG_FILE"
  fi
done

echo "" >> "$LOG_FILE"
echo "TODAS LAS CONFIGURACIONES FALLARON" >> "$LOG_FILE"
echo "" >> "$LOG_FILE"

# Pausa para que el usuario pueda leer los errores si se ejecuta desde .desktop
sleep 5

exit 1
