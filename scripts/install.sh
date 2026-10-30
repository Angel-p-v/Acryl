#!/usr/bin/env bash
#
# install.sh — Instalador automático de Acryl para Linux
#
# Detecta el gestor de paquetes del sistema, instala las dependencias de
# sistema y de desarrollo, clona el repositorio, compila la aplicación con
# Rust + GTK4 y crea la entrada .desktop para que se pueda buscar desde el
# lanzador de aplicaciones.
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
        libgtk-4-dev \
        libadwaita-1-dev \
        librsvg2-dev \
        libayatana-appindicator3-dev \
        libgdk-pixbuf2.0-dev || {
        warn "Algunos paquetes de Debian/Ubuntu no se pudieron instalar."
        warn "Intenta: sudo apt-get install libgtk-4-dev libadwaita-1-dev librsvg2-dev libgdk-pixbuf2.0-dev"
      }
      ;;
    pacman)
      sudo pacman -Sy --needed --noconfirm \
        base-devel \
        curl \
        git \
        gtk4 \
        libadwaita \
        librsvg \
        libayatana-appindicator \
        gdk-pixbuf2 || {
        warn "Algunos paquetes base de Arch no se pudieron instalar."
      }
      ;;
    dnf)
      sudo dnf install -y \
        gcc gcc-c++ make \
        curl git \
        gtk4-devel \
        libadwaita-devel \
        librsvg2-devel \
        libayatana-appindicator3-devel \
        gdk-pixbuf2-devel || {
        warn "Algunos paquetes base de dnf no se pudieron instalar."
      }
      ;;
    zypper)
      sudo zypper install -y \
        patterns-base-C_C++ \
        curl git \
        gtk4-devel \
        libadwaita1-devel \
        librsvg2-devel \
        libayatana-appindicator3-devel \
        gdk-pixbuf2-devel || {
        warn "Algunos paquetes base de openSUSE no se pudieron instalar."
      }
      ;;
    apk)
      sudo apk add \
        build-base \
        curl git \
        gtk4-dev \
        libadwaita-dev \
        librsvg-dev \
        libayatana-appindicator3-dev \
        gdk-pixbuf2-dev || {
        warn "Algunos paquetes base de Alpine no se pudieron instalar."
      }
      ;;
    *)
      error "Gestor de paquetes no soportado: '$pm'."
      error "Instala manualmente: libgtk-4, libadwaita-1, librsvg2, Rust ≥ $MIN_RUST."
      return 1
      ;;
  esac

  ok "Dependencias de sistema instaladas (o advertencias si faltaron algunas)."
}

# -----------------------------------------------------------------------------
# Rust / Cargo ≥ 1.90
# -----------------------------------------------------------------------------
ensure_rust() {
  local rust_ok=false

  if have cargo && have rustc; then
    local rust_version
    rust_version=$(rustc --version | sed 's/^rustc //')
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
      . "$HOME/.cargo/env"
    fi
    if ! have cargo; then
      curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
      . "$HOME/.cargo/env"
    fi
    rustup default stable >/dev/null 2>&1 || true
  fi

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
# Clonar (o actualizar) el repositorio
# -----------------------------------------------------------------------------
clone_repo() {
  local target_dir="${1:-.}"

  if [[ -d "$target_dir/.git" ]]; then
    info "El repositorio ya existe en $target_dir. Actualizando..."
    git -C "$target_dir" remote set-url origin "$REPO_URL" >/dev/null 2>&1 || true
    if ! git -C "$target_dir" pull --ff-only 2>/dev/null; then
      warn "No se pudo actualizar el repositorio automáticamente."
      warn "Posibles cambios locales sin marcar en $target_dir."
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

  info "Compilando la aplicación con Rust y GTK4..."
  # shellcheck disable=SC1090
  [[ -f "$HOME/.cargo/env" ]] && . "$HOME/.cargo/env"
  export PATH="$HOME/.cargo/bin:$PATH"

  cargo build --release

  local bundle_dir
  bundle_dir="$project_dir/target/release"
  if [[ ! -f "$bundle_dir/$APP_NAME" ]]; then
    error "No se encontró el binario en $bundle_dir/$APP_NAME"
    return 1
  fi

  info "Instalando el binario en $BIN_DIR/$APP_NAME..."
  mkdir -p "$BIN_DIR"
  install -Dm755 "$bundle_dir/$APP_NAME" "$BIN_DIR/$APP_NAME"
  ok "Binario instalado en $BIN_DIR/$APP_NAME"
}

# -----------------------------------------------------------------------------
# Crear entrada .desktop
# -----------------------------------------------------------------------------
create_desktop_entry() {
  local project_dir="$1"

  info "Creando entrada .desktop en $DESKTOP_DIR..."
  mkdir -p "$DESKTOP_DIR"

  # Copiar el icono al tema de iconos de usuario
  mkdir -p "$ICON_DIR"
  if [[ -f "$project_dir/icons/acryl.svg" ]]; then
    install -Dm644 "$project_dir/icons/acryl.svg" "$ICON_DIR/$APP_NAME.svg"
  elif [[ -f "$project_dir/icons/icon.png" ]]; then
    install -Dm644 "$project_dir/icons/icon.png" "$ICON_DIR/$APP_NAME.png"
  fi

  local icon_name="$APP_NAME"
  if [[ -f "$ICON_DIR/$APP_NAME.svg" ]]; then
    icon_name="$ICON_DIR/$APP_NAME.svg"
  elif [[ -f "$ICON_DIR/$APP_NAME.png" ]]; then
    icon_name="$ICON_DIR/$APP_NAME.png"
  fi

  local desktop_file="$DESKTOP_DIR/$APP_NAME.desktop"
  if [[ -f "$project_dir/acryl.desktop" ]]; then
    install -Dm644 "$project_dir/acryl.desktop" "$desktop_file"
  else
    # Fallback si no existe el archivo estático
    cat > "$desktop_file" <<EOF
[Desktop Entry]
Name=Acryl
GenericName=Gestor de contenido multimedia
Comment=Indexa y reproduce tu colección de series, anime y películas
Exec=$APP_NAME %F
Icon=$icon_name
Terminal=false
Type=Application
Categories=AudioVideo;Player;
MimeType=video/mp4;video/mkv;video/avi;video/webm;video/quicktime;
Keywords=media;video;movie;series;anime;library;
EOF
  fi

  chmod +x "$desktop_file"

  # Actualizar la base de datos de escritorios si la herramienta está disponible
  if have update-desktop-database; then
    update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
  fi

  ok "Entrada .desktop creada: $desktop_file"
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

# -----------------------------------------------------------------------------
# Instalar lanzador con captura de errores
# -----------------------------------------------------------------------------
install_launcher_wrapper() {
  local project_dir="$1"
  local wrapper_bin="$HOME/.local/bin/$APP_NAME"

  info "Instalando lanzador con captura de errores..."

  # Instalar el wrapper que busca y ejecuta el binario real
  install -Dm755 "$project_dir/scripts/acryl-launcher.sh" "$wrapper_bin"
  ok "Lanzador instalado en $wrapper_bin"

  # Actualizar el .desktop para usar el wrapper
  local desktop_file="$DESKTOP_DIR/$APP_NAME.desktop"
  if [[ -f "$desktop_file" ]]; then
    sed -i "s|^Exec=.*|Exec=$wrapper_bin %F|" "$desktop_file"
    ok "Entrada .desktop actualizada para usar el lanzador"
  fi
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
  ensure_rust

  clone_repo "$target_dir"

  build_and_install "$target_dir"
  create_desktop_entry "$target_dir"
  create_empty_database
  install_launcher_wrapper "$target_dir"

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

main "$@"