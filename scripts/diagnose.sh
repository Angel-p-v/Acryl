#!/usr/bin/env bash
#
# diagnose.sh — Diagnóstico de por qué Acryl no se abre
#
# Ejecuta comprobaciones y recopila logs para identificar el error
# cuando la aplicación se instala pero no lanza.
#
# Uso:
#   ./scripts/diagnose.sh
#
set -euo pipefail

readonly APP_NAME="acryl"
readonly BIN_PATHS=(
  "$HOME/.local/bin/$APP_NAME"
  "/usr/bin/$APP_NAME"
  "/usr/local/bin/$APP_NAME"
  "$HOME/acryl/target/release/$APP_NAME"
)
readonly LOG_DIRS=(
  "$HOME/.local/share/acryl/logs"
  "$HOME/.cache/acryl"
  "$HOME/.config/acryl/logs"
  "/var/log/acryl"
)

info()  { printf '\033[1;34m[INFO]\033[0m %s\n' "$*"; }
ok()    { printf '\033[1;32m[OK]\033[0m  %s\n' "$*"; }
warn()  { printf '\033[1;33m[WARN]\033[0m %s\n' "$*"; }
error() { printf '\033[1;31m[ERROR]\033[0m %s\n' "$*" >&2; }

have() { command -v "$1" >/dev/null 2>&1; }

echo "============================================================"
echo "  Diagnóstico de Acryl — por qué no se abre"
echo "============================================================"
echo

# -----------------------------------------------------------------------------
# 1. ¿Dónde está el ejecutable?
# -----------------------------------------------------------------------------
info "Buscando el ejecutable de Acryl..."
FOUND_BIN=""
for path in "${BIN_PATHS[@]}"; do
  if [[ -f "$path" && -x "$path" ]]; then
    ok "Ejecutable encontrado: $path"
    FOUND_BIN="$path"
    break
  fi
done

if [[ -z "$FOUND_BIN" ]]; then
  error "No se encontró el ejecutable de Acryl en ninguna ubicación conocida."
  echo "  Rutas buscadas:"
  for path in "${BIN_PATHS[@]}"; do
    echo "    - $path"
  done
  echo
  exit 1
fi

# -----------------------------------------------------------------------------
# 2. ¿Es un binario válido? ¿Faltan librerías?
# -----------------------------------------------------------------------------
echo
info "Verificando dependencias del binario..."
if have ldd; then
  MISSING=$(ldd "$FOUND_BIN" 2>/dev/null | grep "not found" || true)
  if [[ -n "$MISSING" ]]; then
    error "Librerías faltantes en el ejecutable:"
    echo "$MISSING"
  else
    ok "Todas las librerías del binario están presentes."
  fi
else
  warn "ldd no disponible; saltando verificación de librerías."
fi

# -----------------------------------------------------------------------------
# 3. ¿Qué hace al ejecutarlo directamente?
# -----------------------------------------------------------------------------
echo
info "Probando ejecución directa (5 segundos de espera)..."
echo "Comando: timeout 5 $FOUND_BIN"
echo "------------------------------------------------------------"
timeout 5 "$FOUND_BIN" 2>&1 || true
echo "------------------------------------------------------------"
echo
if [[ $? -eq 124 ]]; then
  ok "La aplicación se ejecutó durante 5 segundos sin cerrarse (timeout)."
  echo "  Si no apareció ventana, puede ser un problema de WebView/pantalla."
elif [[ $? -eq 139 ]]; then
  error "La aplicación falló con SIGSEGV (segmentation fault)."
elif [[ $? -ne 0 ]]; then
  error "La aplicación terminó con código de error $?."
else
  info "La aplicación terminó normalmente (código 0)."
fi

# -----------------------------------------------------------------------------
# 4. Logs de la aplicación
# -----------------------------------------------------------------------------
echo
info "Buscando archivos de log de Acryl..."
FOUND_LOG=false
for dir in "${LOG_DIRS[@]}"; do
  if [[ -d "$dir" ]]; then
    ok "Directorio de logs encontrado: $dir"
    find "$dir" -type f -name '*.log' -o -name '*.txt' 2>/dev/null | while read -r f; do
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

# -----------------------------------------------------------------------------
# 5. Logs del sistema (journalctl)
# -----------------------------------------------------------------------------
echo
info "Buscando errores recientes en journalctl relacionados con Acryl..."
if have journalctl; then
  journalctl --user -g "acryl" --no-pager -n 30 2>/dev/null || true
  journalctl -g "acryl" --no-pager -n 30 2>/dev/null || true
else
  warn "journalctl no disponible."
fi

# -----------------------------------------------------------------------------
# 6. ¿Está el .desktop bien configurado?
# -----------------------------------------------------------------------------
echo
info "Verificando entrada .desktop..."
DESKTOP_FILE="$HOME/.local/share/applications/$APP_NAME.desktop"
if [[ -f "$DESKTOP_FILE" ]]; then
  ok "Archivo .desktop encontrado: $DESKTOP_FILE"
  echo "  Contenido:"
  cat "$DESKTOP_FILE"
else
  warn "No se encontró .desktop en $DESKTOP_FILE"
fi

# -----------------------------------------------------------------------------
# 7. Variables de entorno relevantes
# -----------------------------------------------------------------------------
echo
info "Variables de entorno relevantes:"
echo "  DISPLAY=${DISPLAY:-no definida}"
echo "  WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-no definida}"
echo "  XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-no definida}"
echo "  GDK_BACKEND=${GDK_BACKEND:-no definida}"

# -----------------------------------------------------------------------------
# Resumen
# -----------------------------------------------------------------------------
echo
echo "============================================================"
echo "  Próximos pasos"
echo "============================================================"
echo
echo "1. Si hay librerías faltantes (paso 2): instálalas con tu gestor."
echo "2. Si la ejecución directa falla (paso 3): copia el mensaje de error."
echo "3. Si no aparece ventana pero no hay error: puede ser un problema"
echo "   de WebView (webkit2gtk) o de servidor de pantalla (X11/Wayland)."
echo
echo "Por favor, comparte la salida de este script (especialmente los pasos"
echo "2 y 3) para poder identificar el error exacto."
echo
