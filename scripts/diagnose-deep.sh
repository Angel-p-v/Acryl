#!/usr/bin/env bash
#
# diagnose-deep.sh — Diagnóstico profundo de Acryl en Wayland
#
# Verifica:
#   1. Si el binario real existe y es ejecutable
#   2. Si se puede iniciar el proceso sin errores
#   3. Qué configuraciones de entorno permiten iniciar el proceso
#   4. Si hay errores de WebKit2GTK/GTK en los logs
#
# Uso:
#   ./scripts/diagnose-deep.sh
#
set -euo pipefail

readonly APP_BIN_REAL="$HOME/.local/bin/acryl-real"
readonly APP_BIN="$HOME/.local/bin/acryl"
readonly LOG_FILE="$HOME/.local/share/acryl/acryl-launch.log"

info()  { printf '\033[1;34m[INFO]\033[0m %s\n' "$*"; }
ok()    { printf '\033[1;32m[OK]\033[0m  %s\n' "$*"; }
warn()  { printf '\033[1;33m[WARN]\033[0m %s\n' "$*"; }
error() { printf '\033[1;31m[ERROR]\033[0m %s\n' "$*" >&2; }

echo "============================================================"
echo "  Diagnóstico profundo: Acryl en Wayland/Hyprland"
echo "============================================================"
echo

# 1. Verificar binario real
info "1. Verificando binario real..."
if [[ -f "$APP_BIN_REAL" && -x "$APP_BIN_REAL" ]]; then
  ok "Binario real encontrado y ejecutable: $APP_BIN_REAL"
  echo "   Tamaño: $(du -h "$APP_BIN_REAL" | cut -f1)"
else
  error "Binario real NO encontrado: $APP_BIN_REAL"
  echo "   Buscando alternativas..."
  for c in "$HOME/.local/bin/acryl" "/usr/bin/acryl" "/usr/local/bin/acryl"; do
    if [[ -x "$c" ]]; then
      ok "  Encontrado: $c"
    else
      echo "  No encontrado: $c"
    fi
  done
fi
echo

# 2. Verificar wrapper
info "2. Verificando wrapper..."
if [[ -f "$APP_BIN" && -x "$APP_BIN" ]]; then
  ok "Wrapper encontrado y ejecutable: $APP_BIN"
else
  error "Wrapper NO encontrado: $APP_BIN"
fi
echo

# 3. Verificar log de lanzador
info "3. Verificando log de lanzador..."
if [[ -f "$LOG_FILE" ]]; then
  ok "Log encontrado: $LOG_FILE"
  echo "   Últimas 10 líneas:"
  tail -10 "$LOG_FILE"
else
  warn "No existe el log: $LOG_FILE"
fi
echo

# 4. Probar ejecutar el binario real directamente con X11 (solo 5 segundos)
info "4. Probando ejecución directa con GDK_BACKEND=x11 (5 segundos)..."
if [[ -x "$APP_BIN_REAL" ]]; then
  echo "   Ejecutando: GDK_BACKEND=x11 timeout 5 $APP_BIN_REAL"
  GDK_BACKEND=x11 timeout 5 "$APP_BIN_REAL" 2>&1 | head -20 || true
  echo "   (Proceso terminado o timeout)"
else
  warn "No se puede probar: binario real no existe"
fi
echo

# 5. Probar con WEBKIT_DISABLE_DMABUF_RENDERER=1
info "5. Probando con WEBKIT_DISABLE_DMABUF_RENDERER=1 (5 segundos)..."
if [[ -x "$APP_BIN_REAL" ]]; then
  echo "   Ejecutando: GDK_BACKEND=x11 WEBKIT_DISABLE_DMABUF_RENDERER=1 timeout 5 $APP_BIN_REAL"
  GDK_BACKEND=x11 WEBKIT_DISABLE_DMABUF_RENDERER=1 timeout 5 "$APP_BIN_REAL" 2>&1 | head -20 || true
  echo "   (Proceso terminado o timeout)"
else
  warn "No se puede probar: binario real no existe"
fi
echo

# 6. Verificar variables de entorno
info "6. Variables de entorno relevantes:"
echo "   DISPLAY=${DISPLAY:-no definida}"
echo "   WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-no definida}"
echo "   GDK_BACKEND=${GDK_BACKEND:-no definida}"
echo "   XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-no definida}"
echo "   LIBGL_ALWAYS_SOFTWARE=${LIBGL_ALWAYS_SOFTWARE:-no definida}"
echo

# 7. Recomendaciones
echo "============================================================"
echo "  Diagnóstico completado"
echo "============================================================"
echo
echo "Si el proceso se ejecuta pero no muestra ventana, el problema es WebKit2GTK en Wayland."
echo
echo "Soluciones definitivas (probar en este orden):"
echo
echo "1. INSTALAR XWAYLAND:"
echo "   sudo pacman -S xorg-xwayland"
echo "   Luego reiniciar sesión y ejecutar: GDK_BACKEND=x11 ~/.local/bin/acryl"
echo
echo "2. USAR SESIÓN X11 (no Wayland):"
echo "   Cerrar sesión y seleccionar 'X11' o 'Xorg' en el gestor de inicio"
echo
echo "3. VARIABLES DE ENTORNO ADICIONALES:"
echo "   export WEBKIT_DISABLE_DMABUF_RENDERER=1"
echo "   export WEBKIT_DISABLE_COMPOSITING_MODE=1"
echo "   export LIBGL_ALWAYS_SOFTWARE=1"
echo "   GDK_BACKEND=x11 ~/.local/bin/acryl"
