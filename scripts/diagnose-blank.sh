#!/usr/bin/env bash
#
# diagnose-blank.sh — Diagnóstico cuando Acryl se abre pero no muestra nada
#
# Comprueba:
#   1. Si la base de datos SQLite tiene medios indexados
#   2. Si hay logs de la aplicación con errores
#   3. Si el WebView/GTK funciona correctamente
#   4. Si la ventana se está renderizando (proceso activo)
#
# Uso:
#   ./scripts/diagnose-blank.sh
#
set -euo pipefail

readonly APP_NAME="acryl"
readonly DB_PATHS=(
  "$HOME/.local/share/acryl/acryl.db"
  "$HOME/acryl/acryl.db"
  "$HOME/$APP_NAME/acryl.db"
  "$PWD/acryl.db"
)
readonly LOG_DIRS=(
  "$HOME/.local/share/acryl/logs"
  "$HOME/.cache/acryl"
  "$HOME/.config/acryl/logs"
)

info()  { printf '\033[1;34m[INFO]\033[0m %s\n' "$*"; }
ok()    { printf '\033[1;32m[OK]\033[0m  %s\n' "$*"; }
warn()  { printf '\033[1;33m[WARN]\033[0m %s\n' "$*"; }
error() { printf '\033[1;31m[ERROR]\033[0m %s\n' "$*" >&2; }

have() { command -v "$1" >/dev/null 2>&1; }

echo "============================================================"
echo "  Diagnóstico: Acryl se abre pero no muestra nada"
echo "============================================================"
echo

# -----------------------------------------------------------------------------
# 1. Proceso en ejecución
# -----------------------------------------------------------------------------
info "Comprobando si el proceso de Acryl está en ejecución..."
if pgrep -f "$APP_NAME" >/dev/null 2>&1; then
  ok "El proceso de Acryl está en ejecución."
  pgrep -af "$APP_NAME"
else
  warn "No hay ningún proceso de Acryl en ejecución."
  echo "  (La aplicación podría haberse cerrado tras abrirse)"
fi
echo

# -----------------------------------------------------------------------------
# 2. Base de datos SQLite
# -----------------------------------------------------------------------------
info "Buscando base de datos SQLite de Acryl..."
FOUND_DB=false
for db in "${DB_PATHS[@]}"; do
  if [[ -f "$db" ]]; then
    ok "Base de datos encontrada: $db"
    FOUND_DB=true

    if have sqlite3; then
      echo
      info "Contenido de la base de datos:"

      TABLES=$(sqlite3 "$db" ".tables" 2>/dev/null || true)
      echo "  Tablas: $TABLES"

      if echo "$TABLES" | grep -q "media"; then
        COUNT=$(sqlite3 "$db" "SELECT COUNT(*) FROM media;" 2>/dev/null || echo "0")
        echo "  Medios indexados: $COUNT"
        if [[ "$COUNT" -eq 0 ]]; then
          warn "La base de datos está vacía. No hay medios indexados para mostrar."
          echo "  → Indexa una carpeta de medios desde la aplicación o usa la línea de comandos."
        fi
      fi

      if echo "$TABLES" | grep -q "media_files"; then
        FILES=$(sqlite3 "$db" "SELECT COUNT(*) FROM media_files;" 2>/dev/null || echo "0")
        echo "  Archivos indexados: $FILES"
      fi
    else
      warn "sqlite3 no instalado; no se puede inspeccionar la base de datos."
    fi
  fi
done

if [[ "$FOUND_DB" == "false" ]]; then
  warn "No se encontró ninguna base de datos de Acryl."
  echo "  Rutas buscadas:"
  for db in "${DB_PATHS[@]}"; do
    echo "    - $db"
  done
fi
echo

# -----------------------------------------------------------------------------
# 3. Logs de la aplicación
# -----------------------------------------------------------------------------
info "Buscando archivos de log de Acryl..."
FOUND_LOG=false
for dir in "${LOG_DIRS[@]}"; do
  if [[ -d "$dir" ]]; then
    ok "Directorio de logs: $dir"
    find "$dir" -type f \( -name '*.log' -o -name '*.txt' \) 2>/dev/null | while read -r f; do
      ok "Log: $f"
      echo "  Últimas 30 líneas:"
      tail -30 "$f" 2>/dev/null || true
      echo
    done
    FOUND_LOG=true
  fi
done

if [[ "$FOUND_LOG" == "false" ]]; then
  warn "No se encontraron archivos de log de Acryl."
fi
echo

# -----------------------------------------------------------------------------
# 4. Logs del sistema (journalctl)
# -----------------------------------------------------------------------------
info "Buscando errores recientes en journalctl..."
if have journalctl; then
  echo "  Logs de usuario:"
  journalctl --user -g "acryl" --no-pager -n 20 2>/dev/null || true
  echo "  Logs del sistema:"
  journalctl -g "acryl" --no-pager -n 20 2>/dev/null || true
else
  warn "journalctl no disponible."
fi
echo

# -----------------------------------------------------------------------------
# 5. WebView / GTK
# -----------------------------------------------------------------------------
info "Comprobando WebView (GTK/WebKit)..."
if have ldd && command -v "$APP_NAME" >/dev/null 2>&1; then
  BIN=$(command -v "$APP_NAME")
  MISSING=$(ldd "$BIN" 2>/dev/null | grep "not found" || true)
  if [[ -n "$MISSING" ]]; then
    error "Librerías faltantes en el ejecutable:"
    echo "$MISSING"
  else
    ok "Todas las librerías del ejecutable están presentes."
  fi
fi

if have gtk-launch; then
  ok "gtk-launch disponible."
else
  warn "gtk-launch no disponible; el lanzador de aplicaciones podría no funcionar."
fi

echo
echo "  Variables de entorno relevantes:"
echo "    DISPLAY=${DISPLAY:-no definida}"
echo "    WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-no definida}"
echo "    XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-no definida}"
echo "    GDK_BACKEND=${GDK_BACKEND:-no definida}"
echo "    WEBKIT_DISABLE_COMPOSITING_MODE=${WEBKIT_DISABLE_COMPOSITING_MODE:-no definida}"
echo

# -----------------------------------------------------------------------------
# 6. Cómo ejecutar con depuración
# -----------------------------------------------------------------------------
echo "============================================================"
echo "  Cómo obtener más información"
echo "============================================================"
echo
echo "Ejecuta la aplicación desde la terminal para ver errores en tiempo real:"
echo "  RUST_BACKTRACE=1 $HOME/.local/bin/$APP_NAME"
echo
echo "Si usas el modo desarrollo:"
echo "  cd /home/coronel/Projects/Acryl"
echo "  npm run tauri:dev"
echo
echo "Si la base de datos está vacía, indexa medios desde la aplicación o:"
echo "  sqlite3 ~/.local/share/acryl/acryl.db '.tables'"
echo
echo "Si no aparece nada aunque haya medios, puede ser un problema de"
echo "WebView (webkit2gtk) con Wayland. Intenta:"
echo "  GDK_BACKEND=x11 $HOME/.local/bin/$APP_NAME"
echo
