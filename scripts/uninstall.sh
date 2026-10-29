#!/usr/bin/env bash
#
# uninstall.sh — Desinstalador de Acryl para Linux
#
# Elimina la entrada .desktop, el icono, el binario (AppImage fallback) y
# el paquete nativo (.deb / .rpm) si fue instalado con install.sh.
#
# No borra el código fuente clonado ni la base de datos de usuario
# (acryl.db), para no perder tu biblioteca indexada.
#
# Uso:
#   ./scripts/uninstall.sh
#
set -euo pipefail

# -----------------------------------------------------------------------------
# Configuración
# -----------------------------------------------------------------------------
readonly APP_NAME="acryl"

# Rutas a nivel de usuario (mismas que usa install.sh)
readonly DESKTOP_DIR="$HOME/.local/share/applications"
readonly ICON_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"
readonly BIN_DIR="$HOME/.local/bin"

# -----------------------------------------------------------------------------
# Utilidades
# -----------------------------------------------------------------------------
info()  { printf '\033[1;34m[INFO]\033[0m %s\n' "$*"; }
ok()    { printf '\033[1;32m[OK]\033[0m  %s\n' "$*"; }
warn()  { printf '\033[1;33m[WARN]\033[0m %s\n' "$*"; }
error() { printf '\033[1;31m[ERROR]\033[0m %s\n' "$*" >&2; }

have() { command -v "$1" >/dev/null 2>&1; }

# -----------------------------------------------------------------------------
# Detección del gestor de paquetes
# -----------------------------------------------------------------------------
detect_package_manager() {
  if have apt-get; then echo "apt"; return; fi
  if have pacman; then echo "pacman"; return; fi
  if have dnf; then echo "dnf"; return; fi
  if have zypper; then echo "zypper"; return; fi
  if have apk; then echo "apk"; return; fi
  echo ""
}

# -----------------------------------------------------------------------------
# Eliminar entrada .desktop
# -----------------------------------------------------------------------------
remove_desktop_entry() {
  local desktop_file="$DESKTOP_DIR/$APP_NAME.desktop"

  if [[ -f "$desktop_file" ]]; then
    info "Eliminando entrada .desktop: $desktop_file"
    rm -f "$desktop_file"
    ok "Entrada .desktop eliminada."
  else
    warn "No se encontró entrada .desktop en $DESKTOP_DIR."
  fi

  # Eliminar iconos del tema de usuario (varias extensiones posibles)
  local icon_removed=false
  for ext in svg png; do
    local icon_file="$ICON_DIR/$APP_NAME.$ext"
    if [[ -f "$icon_file" ]]; then
      rm -f "$icon_file"
      ok "Icono eliminado: $icon_file"
      icon_removed=true
    fi
  done

  if [[ "$icon_removed" == "false" ]]; then
    warn "No se encontró icono de $APP_NAME en $ICON_DIR."
  fi

  # Actualizar base de datos de escritorios si la herramienta está disponible
  if have update-desktop-database; then
    update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
  fi
}

# -----------------------------------------------------------------------------
# Eliminar binario AppImage fallback
# -----------------------------------------------------------------------------
remove_appimage_bin() {
  local bin_file="$BIN_DIR/$APP_NAME"

  if [[ -f "$bin_file" ]]; then
    info "Eliminando binario AppImage: $bin_file"
    rm -f "$bin_file"
    ok "Binario eliminado."
  else
    warn "No se encontró binario AppImage en $BIN_DIR."
  fi
}

# -----------------------------------------------------------------------------
# Desinstalar paquete nativo (.deb / .rpm)
# -----------------------------------------------------------------------------
remove_native_package() {
  local pm
  pm=$(detect_package_manager)

  case "$pm" in
    apt)
      if dpkg -s "$APP_NAME" >/dev/null 2>&1; then
        info "Desinstalando paquete .deb: $APP_NAME"
        sudo apt-get remove -y -qq "$APP_NAME"
        ok "Paquete .deb desinstalado."
      else
        warn "El paquete .deb $APP_NAME no está instalado."
      fi
      ;;
    pacman)
      if pacman -Q "$APP_NAME" >/dev/null 2>&1; then
        info "Desinstalando paquete: $APP_NAME"
        sudo pacman -R --noconfirm "$APP_NAME"
        ok "Paquete desinstalado."
      else
        warn "El paquete $APP_NAME no está instalado."
      fi
      ;;
    dnf)
      if rpm -q "$APP_NAME" >/dev/null 2>&1; then
        info "Desinstalando paquete .rpm: $APP_NAME"
        sudo dnf remove -y "$APP_NAME"
        ok "Paquete .rpm desinstalado."
      else
        warn "El paquete .rpm $APP_NAME no está instalado."
      fi
      ;;
    zypper)
      if rpm -q "$APP_NAME" >/dev/null 2>&1; then
        info "Desinstalando paquete .rpm: $APP_NAME"
        sudo zypper remove -y "$APP_NAME"
        ok "Paquete .rpm desinstalado."
      else
        warn "El paquete .rpm $APP_NAME no está instalado."
      fi
      ;;
    apk)
      if apk info -e "$APP_NAME" >/dev/null 2>&1; then
        info "Desinstalando paquete: $APP_NAME"
        sudo apk del "$APP_NAME"
        ok "Paquete desinstalado."
      else
        warn "El paquete $APP_NAME no está instalado."
      fi
      ;;
    *)
      warn "Gestor de paquetes no soportado; omite desinstalación nativa."
      ;;
  esac
}

# -----------------------------------------------------------------------------
# Flujo principal
# -----------------------------------------------------------------------------
main() {
  echo "============================================================"
  echo "  Desinstalador de Acryl"
  echo "============================================================"
  echo

  remove_desktop_entry
  remove_appimage_bin
  remove_native_package

  echo
  ok "Desinstalación completada."
  echo
  echo "Se conservaron:"
  echo "  - El código fuente clonado (por ejemplo, ~/acryl)"
  echo "  - La base de datos de tu biblioteca (acril.db)"
  echo
  echo "Si quieres eliminar todo manualmente, borra la carpeta del repositorio"
  echo "y el archivo acril.db de tu carpeta de datos."
}

main "$@"
