#!/usr/bin/env bash
#
# install.sh — Instalador automático de Acryl para Linux
#
# Detecta el gestor de paquetes del sistema, instala las dependencias de
# sistema y de desarrollo, clona el repositorio, compila la aplicación e
# instala la entrada .desktop para que se pueda buscar desde el lanzador
# de aplicaciones (menú de inicio / buscador).
#
# Uso:
#   curl -fsSL https://raw.githubusercontent.com/Angel-p-v/app-media-manager/main/install.sh | bash
#
#   o bien:
#   git clone https://github.com/Angel-p-v/app-media-manager.git acryl
#   cd acryl && ./install.sh
#
set -euo pipefail

# -----------------------------------------------------------------------------
# Configuración
# -----------------------------------------------------------------------------
readonly REPO_URL="https://github.com/Angel-p-v/app-media-manager.git"
readonly APP_NAME="acryl"
readonly MIN_NODE_MAJOR=22
readonly MIN_RUST="1.90"

# Rutas de instalación a nivel de usuario (no requieren root para el .desktop)
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
# Intenta instalar una lista de nombres de paquete; usa el primero que exista.
# Devuelve 0 si al menos uno se instaló, 1 si ninguno.
# -----------------------------------------------------------------------------
try_install_pkgs() {
  local pm="$1"; shift
  local pkgs=("$@")
  local installed_any=false

  for pkg in "${pkgs[@]}"; do
    case "$pm" in
      apt)
        if sudo apt-get install -y -qq "$pkg" 2>/dev/null; then
          installed_any=true
          break
        fi
        ;;
      pacman)
        if sudo pacman -Sy --needed --noconfirm "$pkg" 2>/dev/null; then
          installed_any=true
          break
        fi
        ;;
      dnf)
        if sudo dnf install -y "$pkg" 2>/dev/null; then
          installed_any=true
          break
        fi
        ;;
      zypper)
        if sudo zypper install -y "$pkg" 2>/dev/null; then
          installed_any=true
          break
        fi
        ;;
      apk)
        if sudo apk add "$pkg" 2>/dev/null; then
          installed_any=true
          break
        fi
        ;;
    esac
  done

  if [[ "$installed_any" == "true" ]]; then
    return 0
  fi
  return 1
}

# -----------------------------------------------------------------------------
# Dependencias de sistema
# -----------------------------------------------------------------------------
install_system_deps() {
  local pm="$1"

  info "Instalando dependencias de sistema con $pm..."

  case "$pm" in
    apt)
      sudo apt-get update -qq
      sudo apt-get install -y -qq \
        build-essential \
        curl \
        git \
        libwebkit2gtk-4.0-dev \
        libgtk-3-dev \
        librsvg2-dev \
        libayatana-appindicator3-dev \
        libmpv-dev || {
        warn "Algunos paquetes de Debian/Ubuntu no se pudieron instalar."
        warn "Intenta: sudo apt-get install libwebkit2gtk-4.0-dev libgtk-3-dev librsvg2-dev libmpv-dev"
      }
      ;;
    pacman)
      # Arch divide webkit2gtk en paquetes versionados (4.0 / 4.1); Tauri v2
      # funciona con cualquiera de las dos. mpv se llama simplemente 'mpv'.
      # Node.js se gestiona por separado en ensure_node() para evitar conflictos
      # con versiones LTS ya instaladas (p.ej. nodejs-lts-jod).
      sudo pacman -Sy --needed --noconfirm \
        base-devel \
        curl \
        git \
        gtk3 \
        librsvg \
        libayatana-appindicator \
        mpv || {
        warn "Algunos paquetes base de Arch no se pudieron instalar."
      }

      # webkit2gtk: probar 4.1 primero, luego 4.0
      if ! try_install_pkgs pacman webkit2gtk-4.1 webkit2gtk-4.0; then
        warn "No se encontró webkit2gtk-4.1 ni webkit2gtk-4.0 en los repos de Arch."
        warn "Instala manualmente: sudo pacman -S webkit2gtk-4.1 (o webkit2gtk-4.0)"
      fi
      ;;
    dnf)
      # Fedora reciente: webkit2gtk4.0-devel; distros antiguas: webkit2gtk3-devel
      # Node.js se gestiona por separado en ensure_node().
      sudo dnf install -y \
        gcc gcc-c++ make \
        curl git \
        gtk3-devel \
        librsvg2-devel \
        libayatana-appindicator3-devel \
        libmpv-devel || {
        warn "Algunos paquetes base de dnf no se pudieron instalar."
      }

      if ! try_install_pkgs dnf webkit2gtk4.0-devel webkit2gtk3-devel; then
        warn "No se encontró webkit2gtk4.0-devel ni webkit2gtk3-devel."
        warn "Instala manualmente: sudo dnf install webkit2gtk4.0-devel (o webkit2gtk3-devel)"
      fi
      ;;
    zypper)
      sudo zypper install -y \
        patterns-base-C_C++ \
        curl git \
        gtk3-devel \
        librsvg2-devel \
        libayatana-appindicator3-devel \
        libmpv-devel || {
        warn "Algunos paquetes base de openSUSE no se pudieron instalar."
      }

      if ! try_install_pkgs zypper libwebkit2gtk-4_0-devel libwebkit2gtk3-devel; then
        warn "No se encontró libwebkit2gtk-4_0-devel ni libwebkit2gtk3-devel."
        warn "Instala manualmente: sudo zypper install libwebkit2gtk-4_0-devel"
      fi
      ;;
    apk)
      sudo apk add \
        build-base \
        curl git \
        gtk+3-dev \
        librsvg-dev \
        libayatana-appindicator3-dev \
        libmpv-dev || {
        warn "Algunos paquetes base de Alpine no se pudieron instalar."
      }

      if ! try_install_pkgs apk webkit2gtk-dev; then
        warn "No se encontró webkit2gtk-dev en Alpine."
        warn "Instala manualmente: sudo apk add webkit2gtk-dev"
      fi
      ;;
    *)
      error "Gestor de paquetes no soportado: '$pm'."
      error "Instala manualmente: libwebkit2gtk, libgtk-3, librsvg2, libmpv, Node.js ≥ $MIN_NODE_MAJOR, Rust ≥ $MIN_RUST."
      return 1
      ;;
  esac

  ok "Dependencias de sistema instaladas (o advertencias si faltaron algunas)."
}

# -----------------------------------------------------------------------------
# Node.js ≥ 22
# -----------------------------------------------------------------------------
ensure_node() {
  if have node && have npm; then
    local node_version
    node_version=$(node -v | sed 's/^v//')
    local node_major
    node_major="${node_version%%.*}"

    if (( node_major >= MIN_NODE_MAJOR )); then
      ok "Node.js $node_version ya está instalado (>= $MIN_NODE_MAJOR)."
      return 0
    fi
    warn "Node.js $node_version es inferior a la versión requerida ($MIN_NODE_MAJOR)."
  fi

  info "Instalando Node.js ≥ $MIN_NODE_MAJOR..."

  local pm
  pm=$(detect_package_manager)

  # Intento 1: gestor de paquetes de la distro
  case "$pm" in
    apt)
      if ! have node || ! have npm; then
        sudo apt-get install -y -qq nodejs npm || true
      fi
      ;;
  esac

  if have node && have npm; then
    local node_version
    node_version=$(node -v | sed 's/^v//')
    local node_major="${node_version%%.*}"
    if (( node_major >= MIN_NODE_MAJOR )); then
      ok "Node.js $node_version instalado."
      return 0
    fi
  fi

  # Intento 2: NodeSource (solo familias Debian/Ubuntu)
  if [[ "$pm" == "apt" ]] && have curl; then
    info "La versión de Node.js del repositorio es antigua; configurando NodeSource ($MIN_NODE_MAJOR.x)..."
    if curl -fsSL "https://deb.nodesource.com/setup_${MIN_NODE_MAJOR}.x" | sudo -E bash - && \
       sudo apt-get install -y -qq nodejs; then
      ok "Node.js instalado vía NodeSource."
      return 0
    fi
    warn "No se pudo instalar Node.js vía NodeSource."
  fi

  error "No se pudo instalar Node.js ≥ $MIN_NODE_MAJOR automáticamente."
  error "Instálalo manualmente (nvm, nodesource o el paquete de tu distro) y vuelve a ejecutar este script."
  return 1
}

# -----------------------------------------------------------------------------
# Rust / Cargo ≥ 1.90
# -----------------------------------------------------------------------------
ensure_rust() {
  local rust_ok=false

  if have cargo && have rustc; then
    local rust_version
    rust_version=$(rustc --version | sed 's/^rustc //')
    # rustc --version devuelve "1.XX.0"; comparamos major.minor
    local rust_major rust_minor
    rust_major="${rust_version%%.*}"
    rust_minor="${rust_version#*.}"
    rust_minor="${rust_minor%%.*}"

    if (( rust_major > 1 )) || { (( rust_major == 1 )) && (( rust_minor >= 90 )); }; then
      ok "Rust $rust_version ya está instalado (>= $MIN_RUST)."
      rust_ok=true
    else
      warn "Rust $rust_version es inferior a la versión requerida ($MIN_RUST)."
    fi
  fi

  if [[ "$rust_ok" == "false" ]]; then
    info "Instalando Rust vía rustup..."
    if [[ -f "$HOME/.cargo/env" ]]; then
      # shellcheck disable=SC1090
      . "$HOME/.cargo/env"
    fi
    if ! have cargo; then
      curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
      # shellcheck disable=SC1090
      . "$HOME/.cargo/env"
    fi
    rustup default stable >/dev/null 2>&1 || true
  fi

  # shellcheck disable=SC1090
  [[ -f "$HOME/.cargo/env" ]] && . "$HOME/.cargo/env"
  export PATH="$HOME/.cargo/bin:$PATH"

  if have rustc; then
    local rust_version
    rust_version=$(rustc --version | sed 's/^rustc //')
    ok "Rust $rust_version listo."
  else
    error "No se pudo instalar Rust."
    return 1
  fi
}

# -----------------------------------------------------------------------------
# Tauri CLI
# -----------------------------------------------------------------------------
ensure_tauri_cli() {
  # shellcheck disable=SC1090
  [[ -f "$HOME/.cargo/env" ]] && . "$HOME/.cargo/env"
  export PATH="$HOME/.cargo/bin:$PATH"

  if cargo tauri --version >/dev/null 2>&1; then
    ok "Tauri CLI ya está instalado."
    return 0
  fi

  info "Instalando Tauri CLI..."
  cargo install tauri-cli --version 2 --locked
  ok "Tauri CLI instalado."
}

# -----------------------------------------------------------------------------
# Clonar (o actualizar) el repositorio
# -----------------------------------------------------------------------------
clone_repo() {
  local target_dir="${1:-.}"

  if [[ -d "$target_dir/.git" ]]; then
    info "El repositorio ya existe en $target_dir. Actualizando..."
    git -C "$target_dir" remote set-url origin "$REPO_URL" >/dev/null 2>&1 || true
    if ! git -C "$target_dir" pull --rebase --autostash 2>/dev/null; then
      warn "No se pudo actualizar el repositorio automáticamente."
      warn "Posibles cambios locales sin marcar en $target_dir."
      warn "Ejecuta manualmente: cd $target_dir && git pull --rebase --autostash"
    else
      ok "Repositorio actualizado."
    fi
  else
    info "Clonando $REPO_URL en $target_dir..."
    git clone "$REPO_URL" "$target_dir"
    ok "Repositorio clonado."
  fi
}

# -----------------------------------------------------------------------------
# Compilar e instalar la aplicación
# -----------------------------------------------------------------------------
build_and_install() {
  local project_dir="$1"
  cd "$project_dir"

  info "Instalando dependencias de Node.js (frontend)..."
  (cd frontend && npm install)

  info "Compilando la aplicación con Tauri (genera .deb / AppImage / .rpm)..."
  # shellcheck disable=SC1090
  [[ -f "$HOME/.cargo/env" ]] && . "$HOME/.cargo/env"
  export PATH="$HOME/.cargo/bin:$PATH"
  npm run tauri:build

  local bundle_dir
  bundle_dir="$project_dir/src-tauri/target/release/bundle"
  if [[ ! -d "$bundle_dir" ]]; then
    error "No se encontró el directorio de bundles en $bundle_dir"
    return 1
  fi

  info "Instalando el paquete generado..."
  install_package "$bundle_dir" "$project_dir"
}

install_package() {
  local bundle_dir="$1"
  local project_dir="$2"
  local pm
  pm=$(detect_package_manager)

  local deb_file rpm_file appimage_file
  deb_file=$(find "$bundle_dir" -maxdepth 3 -name '*.deb' -print -quit 2>/dev/null || true)
  rpm_file=$(find "$bundle_dir" -maxdepth 3 -name '*.rpm' -print -quit 2>/dev/null || true)
  appimage_file=$(find "$bundle_dir" -maxdepth 3 -name '*.AppImage' -print -quit 2>/dev/null || true)

  case "$pm" in
    apt)
      if [[ -n "$deb_file" ]]; then
        info "Instalando paquete .deb: $(basename "$deb_file")"
        sudo apt-get install -y -qq "$deb_file"
        ok "Aplicación instalada desde .deb."
        return 0
      fi
      ;;
    dnf)
      if [[ -n "$rpm_file" ]]; then
        info "Instalando paquete .rpm: $(basename "$rpm_file")"
        sudo dnf install -y "$rpm_file"
        ok "Aplicación instalada desde .rpm."
        return 0
      fi
      ;;
    zypper)
      if [[ -n "$rpm_file" ]]; then
        info "Instalando paquete .rpm: $(basename "$rpm_file")"
        sudo zypper install -y "$rpm_file"
        ok "Aplicación instalada desde .rpm."
        return 0
      fi
      ;;
  esac

  # Fallback: AppImage a nivel de usuario
  if [[ -n "$appimage_file" ]]; then
    info "No se pudo instalar un paquete nativo; usando AppImage como fallback."
    mkdir -p "$BIN_DIR"
    install -Dm755 "$appimage_file" "$BIN_DIR/$APP_NAME"
    ok "AppImage instalado en $BIN_DIR/$APP_NAME"
    INSTALLED_BIN="$BIN_DIR/$APP_NAME"
    return 0
  fi

  error "No se encontró ningún paquete instalable (.deb, .rpm o .AppImage) en $bundle_dir"
  return 1
}

# -----------------------------------------------------------------------------
# Crear entrada .desktop
# -----------------------------------------------------------------------------
create_desktop_entry() {
  local project_dir="$1"
  local exec_cmd="${INSTALLED_BIN:-$APP_NAME}"

  info "Creando entrada .desktop en $DESKTOP_DIR..."
  mkdir -p "$DESKTOP_DIR"

  # Copiar el icono al tema de iconos de usuario
  mkdir -p "$ICON_DIR"
  if [[ -f "$project_dir/src-tauri/icons/acryl.svg" ]]; then
    install -Dm644 "$project_dir/src-tauri/icons/acryl.svg" "$ICON_DIR/$APP_NAME.svg"
  elif [[ -f "$project_dir/src-tauri/icons/icon.png" ]]; then
    install -Dm644 "$project_dir/src-tauri/icons/icon.png" "$ICON_DIR/$APP_NAME.png"
  fi

  local icon_name="$APP_NAME"
  if [[ -f "$ICON_DIR/$APP_NAME.svg" ]]; then
    icon_name="$ICON_DIR/$APP_NAME.svg"
  elif [[ -f "$ICON_DIR/$APP_NAME.png" ]]; then
    icon_name="$ICON_DIR/$APP_NAME.png"
  fi

  local desktop_file="$DESKTOP_DIR/$APP_NAME.desktop"
  cat > "$desktop_file" <<EOF
[Desktop Entry]
Name=Acryl
GenericName=Gestor de contenido multimedia
Comment=Indexa y reproduce tu colección de series, anime y películas
Exec=$exec_cmd %F
Icon=$icon_name
Terminal=false
Type=Application
Categories=AudioVideo;Player;
MimeType=video/mp4;video/mkv;video/avi;video/webm;video/quicktime;
Keywords=media;video;movie;series;anime;library;
EOF

  chmod +x "$desktop_file"

  # Actualizar la base de datos de escritorios si la herramienta está disponible
  if have update-desktop-database; then
    update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
  fi

  ok "Entrada .desktop creada: $desktop_file"
}

# -----------------------------------------------------------------------------
# Flujo principal
# -----------------------------------------------------------------------------
main() {
  local target_dir="${1:-$HOME/acryl}"

  echo "============================================================"
  echo "  Instalador automático de Acryl"
  echo "============================================================"
  echo

  local pm
  pm=$(detect_package_manager)
  if [[ -z "$pm" ]]; then
    error "No se detectó ningún gestor de paquetes soportado (apt, pacman, dnf, zypper, apk)."
    exit 1
  fi
  info "Gestor de paquetes detectado: $pm"

  install_system_deps "$pm"
  ensure_node
  ensure_rust
  ensure_tauri_cli

  clone_repo "$target_dir"

  build_and_install "$target_dir"
  create_desktop_entry "$target_dir"

  echo
  ok "Instalación completada."
  echo
  echo "Puedes buscar 'Acryl' en el menú de aplicaciones o ejecutar:"
  echo "  $APP_NAME"
  echo
  echo "Código fuente: $target_dir"
  echo "Repositorio:   $REPO_URL"
}

main "$@"
