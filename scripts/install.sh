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

  info "Compilando frontend con Vite..."
  (cd frontend && npm run build)

  info "Compilando la aplicación con Tauri (genera .deb / AppImage / .rpm)..."
  # shellcheck disable=SC1090
  [[ -f "$HOME/.cargo/env" ]] && . "$HOME/.cargo/env"
  export PATH="$HOME/.cargo/bin:$PATH"
  (cd src-tauri && cargo tauri build)

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
  create_empty_database

  echo
  ok "Instalación completada."
  echo
  echo "========================================================================"
  echo "  Acryl está instalado. Sigue estos pasos para empezar a usarlo:"
  echo "========================================================================"
  echo
  echo "1. EJECUTAR LA APLICACIÓN:"
  echo "   Si usas Wayland (Hyprland, Sway, etc.), la pantalla puede quedar en"
  echo "   blanco. En ese caso, ejecuta con X11:"
  echo ""
  echo "     GDK_BACKEND=x11 $APP_NAME"
  echo ""
  echo "   Si usas X11, ejecuta directamente:"
  echo ""
  echo "     $APP_NAME"
  echo ""
  echo "2. CONFIGURAR LA RUTA DE TUS MEDIOS:"
  echo "   Una vez abierta la aplicación, ve a Configuración y establece la"
  echo "   ruta de tu carpeta de películas/series/anime, o usa el comando:"
  echo ""
  echo "     GDK_BACKEND=x11 $APP_NAME --index-folder /ruta/a/tus/medios"
  echo ""
  echo "   (o desde la UI: Índicar carpeta)"
  echo ""
  echo "3. LA BASE DE DATOS:"
  echo "   Se ha creado vacía en ~/.local/share/acryl/acryl.db"
  echo "   Se llenará cuando indexes tus primeros medios."
  echo ""
  echo "4. SOLUCIÓN DE PROBLEMAS:"
  echo "   Si la aplicación no se abre o se queda en blanco, ejecuta el diagnóstico:"
  echo ""
  echo "     ./scripts/diagnose-blank.sh"
  echo ""
  echo "   O ejecuta directamente para ver errores:"
  echo "     RUST_BACKTRACE=1 GDK_BACKEND=x11 $APP_NAME"
  echo ""
  echo "========================================================================"
  echo
  echo "Código fuente: $target_dir"
  echo "Repositorio:   $REPO_URL"
}

# -----------------------------------------------------------------------------
# Crear base de datos vacía
# -----------------------------------------------------------------------------
create_empty_database() {
  local db_dir="$HOME/.local/share/acryl"
  local db_file="$db_dir/acryl.db"

  info "Creando base de datos vacía en $db_file..."
  mkdir -p "$db_dir"

  if [[ ! -f "$db_file" ]]; then
    # Crear una base de datos SQLite vacía con el esquema
    sqlite3 "$db_file" \
      "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, name TEXT NOT NULL, applied_at TEXT NOT NULL DEFAULT (datetime('now')));" \
      "CREATE TABLE IF NOT EXISTS media (id INTEGER PRIMARY KEY, type TEXT NOT NULL CHECK (type IN ('series', 'anime', 'movie')), title TEXT NOT NULL, sort_title TEXT NOT NULL, year INTEGER, folder_path TEXT NOT NULL UNIQUE, created_at TEXT NOT NULL DEFAULT (datetime('now')), updated_at TEXT NOT NULL DEFAULT (datetime('now')), poster_path TEXT, synopsis TEXT, metadata_fetched_at TEXT, is_favorite INTEGER NOT NULL DEFAULT 0 CHECK (is_favorite IN (0, 1)), is_watched INTEGER NOT NULL DEFAULT 0 CHECK (is_watched IN (0, 1)));" \
      "CREATE TABLE IF NOT EXISTS media_files (id INTEGER PRIMARY KEY, media_id INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE, season INTEGER, episode INTEGER, title TEXT, file_path TEXT NOT NULL UNIQUE, size_bytes INTEGER, mtime_ms INTEGER, indexed_at TEXT NOT NULL DEFAULT (datetime('now')));" \
      "CREATE TABLE IF NOT EXISTS index_runs (id INTEGER PRIMARY KEY, started_at TEXT NOT NULL DEFAULT (datetime('now')), finished_at TEXT, files_seen INTEGER NOT NULL DEFAULT 0, files_added INTEGER NOT NULL DEFAULT 0, files_removed INTEGER NOT NULL DEFAULT 0, status TEXT NOT NULL DEFAULT 'running' CHECK (status IN ('running', 'completed', 'failed')));" \
      "CREATE TABLE IF NOT EXISTS playback_history (id INTEGER PRIMARY KEY, media_id INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE, played_at TEXT NOT NULL DEFAULT (datetime('now')), position REAL NOT NULL DEFAULT 0, duration REAL NOT NULL DEFAULT 0);" \
      "CREATE INDEX IF NOT EXISTS idx_media_sort_title ON media(sort_title);" \
      "CREATE INDEX IF NOT EXISTS idx_media_type ON media(type);" \
      "CREATE INDEX IF NOT EXISTS idx_files_media ON media_files(media_id);" \
      "CREATE INDEX IF NOT EXISTS idx_files_season_episode ON media_files(media_id, season, episode);" \
      "CREATE INDEX IF NOT EXISTS idx_media_type_sort_title ON media(type, sort_title);" \
      "CREATE INDEX IF NOT EXISTS idx_media_created_at ON media(created_at);" \
      "CREATE INDEX IF NOT EXISTS idx_media_updated_at ON media(updated_at);" \
      "CREATE INDEX IF NOT EXISTS idx_index_runs_started_at ON index_runs(started_at);" \
      "CREATE INDEX IF NOT EXISTS idx_playback_history_media ON playback_history(media_id);" \
      "CREATE INDEX IF NOT EXISTS idx_playback_history_played_at ON playback_history(played_at);" \
      2>/dev/null || {
      warn "No se pudo crear la base de datos vacía (¿falta sqlite3?)."
      warn "La aplicación la creará automáticamente al ejecutarse por primera vez."
      return 0
    }

    # Aplicar FTS5 y triggers si sqlite3 los soporta
    sqlite3 "$db_file" \
      "CREATE VIRTUAL TABLE IF NOT EXISTS media_fts USING fts5(title, content='media', content_rowid='id');" \
      "CREATE TRIGGER IF NOT EXISTS media_fts_insert AFTER INSERT ON media BEGIN INSERT INTO media_fts(rowid, title) VALUES (new.id, new.title); END;" \
      "CREATE TRIGGER IF NOT EXISTS media_fts_delete AFTER DELETE ON media BEGIN INSERT INTO media_fts(media_fts, rowid, title) VALUES ('delete', old.id, old.title); END;" \
      "CREATE TRIGGER IF NOT EXISTS media_fts_update AFTER UPDATE ON media BEGIN INSERT INTO media_fts(media_fts, rowid, title) VALUES ('delete', old.id, old.title); INSERT INTO media_fts(rowid, title) VALUES (new.id, new.title); END;" \
      2>/dev/null || true

    ok "Base de datos vacía creada en $db_file"
  else
    info "La base de datos ya existe en $db_file, no se sobrescribe."
  fi
}

main "$@"
