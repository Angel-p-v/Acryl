//! Tema visual al estilo Crunchyroll.
//!
//! Define la paleta de colores, tipografía y CSS compartido por todos los
//! componentes de la interfaz. Crunchyroll utiliza:
//! - Fondo muy oscuro (#141414)
//! - Superficies ligeramente más claras para paneles (#1e1e1e, #252525)
//! - Color de acento naranja (#f75c03)
//! - Texto blanco / gris claro con buen contraste
//! - Bordes sutiles y esquinas redondeadas (8px)
//! - Transiciones suaves (150-200ms)

use gtk4::CssProvider;
use std::sync::Once;

/// Modo de tema de la aplicación.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    /// Tema oscuro (Crunchyroll, por defecto).
    #[default]
    Dark,
    /// Tema claro con paleta adaptada.
    Light,
}

static INSTALL_THEME_CSS: Once = Once::new();

/// Colores de la paleta Crunchyroll (tema oscuro).
pub mod colors {
    /// Fondo principal de la aplicación.
    pub const BG: &str = "#141414";
    /// Superficie de paneles y contenedores.
    pub const SURFACE: &str = "#1e1e1e";
    /// Superficie elevada (tarjetas, modales).
    pub const CARD: &str = "#252525";
    /// Borde sutil entre superficies.
    pub const BORDER: &str = "#333333";
    /// Borde de hover / seleccionado.
    pub const BORDER_HOVER: &str = "#f75c03";
    /// Texto primario (títulos, etiquetas importantes).
    pub const TEXT_PRIMARY: &str = "#ffffff";
    /// Texto secundario (descripciones, metadatos).
    pub const TEXT_SECONDARY: &str = "#b3b3b3";
    /// Texto muted (etiquetas pequeñas, ayudas).
    pub const TEXT_MUTED: &str = "#737373";
    /// Color de acento principal (naranja Crunchyroll).
    pub const ACCENT: &str = "#f75c03";
    /// Color de acento en hover.
    pub const ACCENT_HOVER: &str = "#e85a2c";
    /// Color de acento en estado activo/presionado.
    pub const ACCENT_ACTIVE: &str = "#d44f00";
    /// Fondo de botón de acción principal.
    pub const BTN_PRIMARY_BG: &str = "#f75c03";
    /// Texto de botón primario.
    pub const BTN_PRIMARY_TEXT: &str = "#ffffff";
    /// Fondo de botón secundario (plano).
    pub const BTN_SECONDARY_BG: &str = "transparent";
    /// Texto de botón secundario.
    pub const BTN_SECONDARY_TEXT: &str = "#b3b3b3";
    /// Overlay oscuro sobre imágenes para legibilidad de texto.
    pub const OVERLAY_DARK: &str = "rgba(0, 0, 0, 0.65)";
    /// Overlay más oscuro en la parte inferior de banners.
    pub const OVERLAY_BOTTOM: &str =
        "linear-gradient(to top, rgba(0,0,0,0.85) 0%, rgba(0,0,0,0) 40%)";
    /// Fondo de la barra superior.
    pub const TOP_BAR: &str = "#141414";
    /// Color de esqueleto de carga (skeleton).
    pub const SKELETON_BG: &str = "#2a2a2a";
    /// Color de brillo del esqueleto de carga.
    pub const SKELETON_HIGHLIGHT: &str = "#3a3a3a";
}

/// Colores de la paleta Crunchyroll adaptada a tema claro.
pub mod light_colors {
    /// Fondo principal de la aplicación.
    pub const BG: &str = "#ffffff";
    /// Superficie de paneles y contenedores.
    pub const SURFACE: &str = "#f5f5f5";
    /// Superficie elevada (tarjetas, modales).
    pub const CARD: &str = "#ffffff";
    /// Borde sutil entre superficies.
    pub const BORDER: &str = "#dddddd";
    /// Borde de hover / seleccionado.
    pub const BORDER_HOVER: &str = "#f75c03";
    /// Texto primario (títulos, etiquetas importantes).
    pub const TEXT_PRIMARY: &str = "#1a1a1a";
    /// Texto secundario (descripciones, metadatos).
    pub const TEXT_SECONDARY: &str = "#666666";
    /// Texto muted (etiquetas pequeñas, ayudas).
    pub const TEXT_MUTED: &str = "#999999";
    /// Color de acento principal (naranja Crunchyroll).
    pub const ACCENT: &str = "#f75c03";
    /// Color de acento en hover.
    pub const ACCENT_HOVER: &str = "#e85a2c";
    /// Color de acento en estado activo/presionado.
    pub const ACCENT_ACTIVE: &str = "#d44f00";
    /// Fondo de botón de acción principal.
    pub const BTN_PRIMARY_BG: &str = "#f75c03";
    /// Texto de botón primario.
    pub const BTN_PRIMARY_TEXT: &str = "#ffffff";
    /// Fondo de botón secundario (plano).
    pub const BTN_SECONDARY_BG: &str = "transparent";
    /// Texto de botón secundario.
    pub const BTN_SECONDARY_TEXT: &str = "#666666";
    /// Overlay oscuro sobre imágenes para legibilidad de texto.
    pub const OVERLAY_DARK: &str = "rgba(0, 0, 0, 0.5)";
    /// Overlay más oscuro en la parte inferior de banners.
    pub const OVERLAY_BOTTOM: &str =
        "linear-gradient(to top, rgba(0,0,0,0.6) 0%, rgba(0,0,0,0) 40%)";
    /// Fondo de la barra superior.
    pub const TOP_BAR: &str = "#ffffff";
    /// Color de esqueleto de carga (skeleton).
    pub const SKELETON_BG: &str = "#eaeaea";
    /// Color de brillo del esqueleto de carga.
    pub const SKELETON_HIGHLIGHT: &str = "#f5f5f5";
}

/// Conjunto de colores para un tema concreto.
pub struct ThemeColors {
    pub bg: &'static str,
    pub surface: &'static str,
    pub card: &'static str,
    pub border: &'static str,
    pub border_hover: &'static str,
    pub text_primary: &'static str,
    pub text_secondary: &'static str,
    pub text_muted: &'static str,
    pub accent: &'static str,
    pub accent_hover: &'static str,
    pub accent_active: &'static str,
    pub btn_primary_bg: &'static str,
    pub btn_primary_text: &'static str,
    pub btn_secondary_bg: &'static str,
    pub btn_secondary_text: &'static str,
    pub overlay_dark: &'static str,
    pub overlay_bottom: &'static str,
    pub top_bar: &'static str,
    pub skeleton_bg: &'static str,
    pub skeleton_highlight: &'static str,
}

impl ThemeColors {
    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self {
                bg: colors::BG,
                surface: colors::SURFACE,
                card: colors::CARD,
                border: colors::BORDER,
                border_hover: colors::BORDER_HOVER,
                text_primary: colors::TEXT_PRIMARY,
                text_secondary: colors::TEXT_SECONDARY,
                text_muted: colors::TEXT_MUTED,
                accent: colors::ACCENT,
                accent_hover: colors::ACCENT_HOVER,
                accent_active: colors::ACCENT_ACTIVE,
                btn_primary_bg: colors::BTN_PRIMARY_BG,
                btn_primary_text: colors::BTN_PRIMARY_TEXT,
                btn_secondary_bg: colors::BTN_SECONDARY_BG,
                btn_secondary_text: colors::BTN_SECONDARY_TEXT,
                overlay_dark: colors::OVERLAY_DARK,
                overlay_bottom: colors::OVERLAY_BOTTOM,
                top_bar: colors::TOP_BAR,
                skeleton_bg: colors::SKELETON_BG,
                skeleton_highlight: colors::SKELETON_HIGHLIGHT,
            },
            ThemeMode::Light => Self {
                bg: light_colors::BG,
                surface: light_colors::SURFACE,
                card: light_colors::CARD,
                border: light_colors::BORDER,
                border_hover: light_colors::BORDER_HOVER,
                text_primary: light_colors::TEXT_PRIMARY,
                text_secondary: light_colors::TEXT_SECONDARY,
                text_muted: light_colors::TEXT_MUTED,
                accent: light_colors::ACCENT,
                accent_hover: light_colors::ACCENT_HOVER,
                accent_active: light_colors::ACCENT_ACTIVE,
                btn_primary_bg: light_colors::BTN_PRIMARY_BG,
                btn_primary_text: light_colors::BTN_PRIMARY_TEXT,
                btn_secondary_bg: light_colors::BTN_SECONDARY_BG,
                btn_secondary_text: light_colors::BTN_SECONDARY_TEXT,
                overlay_dark: light_colors::OVERLAY_DARK,
                overlay_bottom: light_colors::OVERLAY_BOTTOM,
                top_bar: light_colors::TOP_BAR,
                skeleton_bg: light_colors::SKELETON_BG,
                skeleton_highlight: light_colors::SKELETON_HIGHLIGHT,
            },
        }
    }
}

/// Radios de esquina utilizados en la interfaz.
pub mod radius {
    pub const SMALL: &str = "6px";
    pub const MEDIUM: &str = "8px";
    pub const LARGE: &str = "16px";
}

/// Tiempos de transición.
pub mod transition {
    pub const FAST: &str = "150ms ease";
    pub const NORMAL: &str = "200ms ease";
}

/// Instala los estilos globales del tema Crunchyroll para el modo dado.
///
/// Es idempotente: si la primera llamada falla (por ejemplo, porque GTK4 no
/// está inicializado), no envenena el `Once` y una llamada posterior podrá
/// reintentar la instalación.
pub fn install_theme_css(mode: ThemeMode) {
    let c = ThemeColors::for_mode(mode);
    INSTALL_THEME_CSS.call_once(|| {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let provider = CssProvider::new();
            let mut css = String::new();

            css.push_str(&format!(
                "window {{ background-color: {}; color: {}; font-family: 'Inter', 'Roboto', 'Helvetica', 'Arial', sans-serif; }}\n",
                c.bg, c.text_primary,
            ));
            css.push_str("box { background-color: transparent; }\n");
            css.push_str(&format!("label {{ color: {}; }}\n", c.text_primary));
            css.push_str(&format!("label.dim-label {{ color: {}; }}\n", c.text_muted));
            css.push_str(&format!(
                "entry {{ background-color: {}; color: {}; border: 1px solid {}; border-radius: {}; padding: 8px 12px; }}\n",
                c.card, c.text_primary, c.border, radius::MEDIUM,
            ));
            css.push_str(&format!("entry:focus {{ border-color: {}; }}\n", c.accent));
            css.push_str(&format!(
                "button {{ background-color: {}; color: {}; border: 1px solid {}; border-radius: {}; padding: 8px 16px; font-weight: 600; min-height: 36px; transition: all {}; }}\n",
                c.card, c.text_primary, c.border, radius::MEDIUM, transition::FAST,
            ));
            css.push_str(&format!(
                "button:hover {{ background-color: {}; border-color: {}; }}\n",
                c.surface, c.border_hover,
            ));
            css.push_str(&format!(
                "button:active {{ background-color: {}; border-color: {}; color: {}; }}\n",
                c.accent_active, c.accent, c.btn_primary_text,
            ));
            css.push_str(&format!(
                "button.flat {{ background-color: transparent; border: none; color: {}; padding: 6px 12px; }}\n",
                c.text_secondary,
            ));
            css.push_str(&format!(
                "button.flat:hover {{ background-color: {}; color: {}; border-radius: {}; }}\n",
                c.surface, c.text_primary, radius::SMALL,
            ));
            css.push_str(&format!(
                "button.flat:active {{ color: {}; }}\n",
                c.accent,
            ));
            css.push_str(&format!(
                "button.primary {{ background-color: {}; color: {}; border: none; }}\n",
                c.btn_primary_bg, c.btn_primary_text,
            ));
            css.push_str(&format!(
                "button.primary:hover {{ background-color: {}; }}\n",
                c.accent_hover,
            ));
            css.push_str(&format!(
                "button.primary:active {{ background-color: {}; }}\n",
                c.accent_active,
            ));
            css.push_str("button.danger { background-color: #dc2626; color: #ffffff; border: none; }\n");
            css.push_str("button.danger:hover { background-color: #b91c1c; }\n");
            css.push_str("scrolledwindow { background-color: transparent; }\n");
            css.push_str("viewport { background-color: transparent; }\n");
            css.push_str("flowbox { background-color: transparent; }\n");
            css.push_str("overlay { background-color: transparent; }\n");
            css.push_str("stack { background-color: transparent; }\n");

            /* Historial: eliminar cualquier fondo oscuro del contenedor y asegurar
             * que las tarjetas se vean sobre el fondo de la aplicación. */
            css.push_str(".history-grid { background-color: transparent; }\n");
            css.push_str(&format!(
                ".history-grid .media-card {{ background-color: {}; }}\n",
                c.card,
            ));
            css.push_str(&format!(
                ".history-grid .media-card:hover {{ background-color: {}; }}\n",
                c.surface,
            ));

            /* Cabecera de detalles. */
            css.push_str(&format!(
                ".details-header {{ background-color: {}; border-radius: {}; padding: 0; margin-bottom: 24px; }}\n",
                c.surface, radius::LARGE,
            ));

            /* Barra superior. */
            css.push_str(&format!(
                ".top-bar {{ background-color: {}; border-bottom: 1px solid {}; padding: 10px 20px; }}\n",
                c.top_bar, c.border,
            ));
            css.push_str(".top-bar .app-title { font-weight: 800; font-size: 1.35rem; }\n");
            css.push_str(
                ".logo-button { background-color: transparent; border: none; padding: 4px 8px; min-height: 0; }\n",
            );
            css.push_str(
                ".logo-button:hover { background-color: transparent; border: none; transform: none; }\n",
            );

            /* Barra secundaria (nivel 2). */
            css.push_str(&format!(
                ".top-bar-level2 {{ background-color: {}; border-bottom: 1px solid {}; padding: 8px 20px; font-size: 0.88rem; }}\n",
                c.surface, c.border,
            ));

            /* Chip removible de filtro activo */
            css.push_str(&format!(
                ".filter-chip {{ background-color: {}; border: 1px solid {}; color: {}; border-radius: 999px; padding: 6px 16px; min-height: 32px; font-weight: 600; }}\n",
                c.card, c.border, c.text_secondary,
            ));
            css.push_str(&format!(
                ".filter-chip:hover {{ border-color: {}; color: {}; background-color: {}; }}\n",
                c.border_hover, c.text_primary, c.surface,
            ));

            /* Píldora "Continuar viendo" */
            css.push_str(&format!(
                ".continue-button {{ background-color: {}; border: 1px solid {}; color: {}; border-radius: 999px; padding: 6px 16px; min-height: 32px; font-weight: 600; }}\n",
                c.accent, c.accent, c.btn_primary_text,
            ));
            css.push_str(&format!(
                ".continue-button:hover {{ background-color: {}; border-color: {}; }}\n",
                c.accent_hover, c.accent,
            ));

            /* Botones de modo de vista (Grid/Lista) */
            css.push_str(&format!(
                ".view-mode-button {{ background-color: {}; border: 1px solid {}; color: {}; border-radius: {}; padding: 6px 12px; min-height: 32px; font-weight: 600; }}\n",
                c.card, c.border, c.text_secondary, radius::SMALL,
            ));
            css.push_str(&format!(
                ".view-mode-button:hover {{ border-color: {}; color: {}; background-color: {}; }}\n",
                c.border_hover, c.text_primary, c.surface,
            ));

            /* Navegación horizontal (Categorías) */
            css.push_str(&format!(
                ".nav-button {{ background-color: transparent; border: none; color: {}; font-size: 0.9rem; font-weight: 600; padding: 6px 12px; margin: 0 4px; border-radius: {}; min-height: 32px; }}\n",
                c.text_secondary, radius::SMALL,
            ));
            css.push_str(&format!(
                ".nav-button:hover {{ background-color: {}; color: {}; }}\n",
                c.surface, c.text_primary,
            ));
            css.push_str(&format!(
                ".nav-button.active {{ background-color: {}; color: {}; }}\n",
                c.accent, c.btn_primary_text,
            ));

            /* Título de página */
            css.push_str(&format!(
                ".page-title {{ font-size: 1.6rem; font-weight: 800; color: {}; }}\n",
                c.text_primary,
            ));

            /* Barra de filtro activo sobre la galería */
            css.push_str(&format!(
                ".filter-bar {{ background-color: {}; border-bottom: 1px solid {}; padding: 10px 24px; }}\n",
                c.surface, c.border,
            ));
            css.push_str(&format!(
                ".filter-label {{ color: {}; font-weight: 600; font-size: 0.92rem; }}\n",
                c.text_secondary,
            ));

            /* Chips de categoría */
            css.push_str(&format!(
                ".category-chip {{ background-color: {}; border: 1px solid {}; color: {}; border-radius: 999px; padding: 8px 18px; min-height: 36px; }}\n",
                c.card, c.border, c.text_secondary,
            ));
            css.push_str(&format!(
                ".category-chip:hover {{ border-color: {}; color: {}; }}\n",
                c.border_hover, c.text_primary,
            ));
            css.push_str(&format!(
                ".category-chip.active {{ background-color: {}; border-color: {}; color: {}; }}\n",
                c.accent, c.accent, c.btn_primary_text,
            ));

            /* Tarjeta de medio en galería */
            css.push_str(&format!(
                ".media-card {{ background-color: {}; border: 1px solid {}; border-radius: {}; padding: 8px; transition: all {}; }}\n",
                c.card, c.border, radius::LARGE, transition::NORMAL,
            ));
            css.push_str(&format!(
                ".media-card:hover {{ background-color: {}; border-color: {}; transform: scale(1.03); }}\n",
                c.surface, c.border_hover,
            ));
            /* Anular el fondo de acento que GTK4 aplica por defecto al ítem
             * seleccionado de un GtkGridView/GtkListView. */
            css.push_str(&format!(
                ".media-card:selected {{ background-color: {}; border-color: {}; }}\n",
                c.card, c.border,
            ));

            /* Póster de medio */
            css.push_str(&format!(
                ".media-poster {{ border-radius: {}; }}\n",
                radius::MEDIUM,
            ));

            /* Etiqueta de tipo en tarjeta de galería */
            css.push_str(&format!(
                ".card-type-label {{ color: {}; font-size: 0.75rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.5px; margin-top: 4px; }}\n",
                c.accent,
            ));

            /* Badge de continuación */
            css.push_str(&format!(
                ".continue-badge {{ background-color: {}; color: {}; border-radius: {}; font-size: 0.68rem; font-weight: 700; padding: 2px 8px; }}\n",
                c.accent, c.btn_primary_text, radius::SMALL,
            ));

            /* Título de sección */
            css.push_str(&format!(
                ".section-title {{ color: {}; font-weight: 700; font-size: 1.25rem; }}\n",
                c.text_primary,
            ));

            /* Texto sutil */
            css.push_str(&format!(".text-muted {{ color: {}; }}\n", c.text_muted));
            css.push_str(&format!(".text-secondary {{ color: {}; }}\n", c.text_secondary));

            /* Botón de reproducción de episodio */
            css.push_str(&format!(
                ".episode-play-btn {{ background-color: {}; color: {}; border: none; border-radius: {}; font-weight: 700; min-width: 72px; min-height: 30px; }}\n",
                c.accent, c.btn_primary_text, radius::SMALL,
            ));
            css.push_str(&format!(
                ".episode-play-btn:hover {{ background-color: {}; }}\n",
                c.accent_hover,
            ));

            /* Botón de marcar episodio como visto/no visto */
            css.push_str(&format!(
                ".episode-watched-btn {{ background-color: transparent; color: {}; border: 1px solid {}; border-radius: {}; font-weight: 600; min-width: 72px; min-height: 30px; }}\n",
                c.text_secondary, c.border, radius::SMALL,
            ));
            css.push_str(&format!(
                ".episode-watched-btn:hover {{ color: {}; border-color: {}; }}\n",
                c.text_primary, c.border_hover,
            ));
            css.push_str(&format!(
                ".episode-watched-btn.watched {{ color: {}; border-color: {}; background-color: {}; }}\n",
                c.btn_primary_text, c.accent, c.accent,
            ));

            /* Tarjeta de episodio marcada como visto */
            css.push_str(".episode-card.episode-watched { opacity: 0.55; }\n");
            css.push_str(".episode-card.episode-watched:hover { opacity: 0.75; }\n");

            /* Estado visto/no visto en tarjeta de episodio */
            css.push_str(".episode-status { font-size: 0.75em; font-weight: 600; margin-top: 4px; }\n");
            css.push_str(&format!(
                ".episode-status-watched {{ color: {}; }}\n",
                c.accent,
            ));
            css.push_str(&format!(
                ".episode-status-unwatched {{ color: {}; }}\n",
                c.text_muted,
            ));

            /* Tarjeta de episodio en rejilla */
            css.push_str(&format!(
                ".episode-card {{ background-color: {}; border: 1px solid {}; border-radius: {}; transition: all {}; }}\n",
                c.card, c.border, radius::MEDIUM, transition::FAST,
            ));
            css.push_str(&format!(
                ".episode-card:hover {{ background-color: {}; border-color: {}; transform: scale(1.02); }}\n",
                c.surface, c.border_hover,
            ));
            css.push_str(&format!(
                ".episode-card:selected {{ background-color: {}; border-color: {}; }}\n",
                c.surface, c.accent,
            ));

            /* Portada de episodio */
            css.push_str(&format!(
                ".episode-poster-img {{ border-radius: {}; }}\n",
                radius::SMALL,
            ));

            /* Sobrelay de estado visto en episodio */
            css.push_str(".watched-overlay { background-color: rgba(0, 0, 0, 0.5); border-radius: 6px; }\n");

            /* Estrellas de calificación */
            css.push_str(&format!(".star {{ color: {}; }}\n", c.text_muted));
            css.push_str(&format!(".star.filled {{ color: {}; }}\n", c.accent));

        /* Banner hero de detalles */
        css.push_str(&format!(".banner {{ border-radius: {}; }}\n", radius::LARGE));
        css.push_str(&format!(
            ".banner-title {{ font-size: 2.5em; font-weight: 800; color: {}; }}\n",
            c.text_primary,
        ));
        css.push_str(&format!(
            ".banner-subtitle {{ font-size: 1.1em; color: {}; }}\n",
            c.text_secondary,
        ));

        /* Fila de acciones hero */
        css.push_str(".hero-actions { margin-top: 14px; }\n");
        css.push_str(&format!(
            ".hero-action-btn {{ min-height: 40px; padding: 0 16px; font-weight: 700; border-radius: {}; }}\n",
            radius::SMALL,
        ));
        css.push_str(&format!(
            ".hero-action-btn.primary {{ background-color: {}; color: {}; border: none; }}\n",
            c.btn_primary_bg, c.btn_primary_text,
        ));
        css.push_str(&format!(
            ".hero-action-btn.primary:hover {{ background-color: {}; }}\n",
            c.accent_hover,
        ));
        css.push_str(&format!(
            ".hero-action-btn.secondary {{ background-color: {}; color: {}; border: 1px solid {}; }}\n",
            c.surface, c.text_secondary, c.border,
        ));
        css.push_str(&format!(
            ".hero-action-btn.secondary:hover {{ border-color: {}; background-color: {}; }}\n",
            c.border_hover, c.card,
        ));

        /* Dropdown de temporada */
        css.push_str(".season-dropdown { min-width: 180px; }\n");
        css.push_str(".season-watched-dropdown { min-height: 30px; min-width: 150px; }\n");

        /* Esqueleto de carga (skeleton) */
        css.push_str(&format!(
            ".skeleton {{ background-color: {}; border-radius: {}; animation: pulse 1.5s infinite ease-in-out; }}\n",
            c.skeleton_bg, radius::MEDIUM,
        ));
        css.push_str("@keyframes pulse { 0% { opacity: 0.6; } 50% { opacity: 1; } 100% { opacity: 0.6; } }\n");
        css.push_str(&format!(
            ".skeleton-text {{ min-height: 1em; border-radius: {}; }}\n",
            radius::SMALL,
        ));
        css.push_str(&format!(
            ".skeleton-image {{ border-radius: {}; }}\n",
            radius::MEDIUM,
        ));

        /* Sinopsis */
        css.push_str(&format!(
            ".synopsis-text {{ color: {}; line-height: 1.6; }}\n",
            c.text_secondary,
        ));

        /* Botón plano (flat) */
        css.push_str(&format!(
            ".flat-button {{ background-color: transparent; border: none; color: {}; font-size: 0.9em; padding: 6px 12px; }}\n",
            c.btn_secondary_text,
        ));
        css.push_str(&format!(
            ".flat-button:hover {{ color: {}; background-color: {}; border-radius: {}; }}\n",
            c.text_primary, c.surface, radius::SMALL,
        ));

        /* Rejilla de episodios */
        css.push_str(".episode-grid { margin-top: 8px; }\n");
        css.push_str(".season-grid-section { margin-top: 28px; }\n");
        css.push_str(&format!(
            ".season-grid-title {{ font-weight: 700; font-size: 1.2em; color: {}; margin-bottom: 12px; margin-left: 6px; }}\n",
            c.text_primary,
        ));

        /* Nota: GTK4 CSS no soporta @media queries de ancho de forma fiable.
         * El responsive se gestiona desde Rust en media_list.rs (update_columns)
         * calculando columnas dinámicamente según el ancho del contenedor.
         * Los tamaños base de las tarjetas están optimizados para pantallas
         * de portátil (1366px+) y grandes (1920px+). */

        provider.load_from_data(&css);

        if let Some(display) = gtk4::gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }));
});
}
