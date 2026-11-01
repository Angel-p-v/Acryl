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

static INSTALL_THEME_CSS: Once = Once::new();

/// Colores de la paleta Crunchyroll.
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

/// Instala una sola vez los estilos globales del tema Crunchyroll.
pub fn install_theme_css() {
    INSTALL_THEME_CSS.call_once(|| {
        let provider = CssProvider::new();
        let mut css = String::new();

        css.push_str(&format!(
            "window {{ background-color: {}; color: {}; font-family: 'Inter', 'Roboto', 'Helvetica', 'Arial', sans-serif; }}\n",
            colors::BG, colors::TEXT_PRIMARY,
        ));
        css.push_str("box { background-color: transparent; }\n");
        css.push_str(&format!("label {{ color: {}; }}\n", colors::TEXT_PRIMARY));
        css.push_str(&format!("label.dim-label {{ color: {}; }}\n", colors::TEXT_MUTED));
        css.push_str(&format!(
            "entry {{ background-color: {}; color: {}; border: 1px solid {}; border-radius: {}; padding: 8px 12px; }}\n",
            colors::CARD, colors::TEXT_PRIMARY, colors::BORDER, radius::MEDIUM,
        ));
        css.push_str(&format!("entry:focus {{ border-color: {}; }}\n", colors::ACCENT));
        css.push_str(&format!(
            "button {{ background-color: {}; color: {}; border: 1px solid {}; border-radius: {}; padding: 8px 16px; font-weight: 600; min-height: 36px; transition: all {}; }}\n",
            colors::CARD, colors::TEXT_PRIMARY, colors::BORDER, radius::MEDIUM, transition::FAST,
        ));
        css.push_str(&format!(
            "button:hover {{ background-color: {}; border-color: {}; }}\n",
            colors::SURFACE, colors::BORDER_HOVER,
        ));
        css.push_str(&format!(
            "button:active {{ background-color: {}; border-color: {}; color: {}; }}\n",
            colors::ACCENT_ACTIVE, colors::ACCENT, colors::BTN_PRIMARY_TEXT,
        ));
        css.push_str(&format!(
            "button.flat {{ background-color: transparent; border: none; color: {}; padding: 6px 12px; }}\n",
            colors::TEXT_SECONDARY,
        ));
        css.push_str(&format!(
            "button.flat:hover {{ background-color: {}; color: {}; border-radius: {}; }}\n",
            colors::SURFACE, colors::TEXT_PRIMARY, radius::SMALL,
        ));
        css.push_str(&format!(
            "button.flat:active {{ color: {}; }}\n",
            colors::ACCENT,
        ));
        css.push_str(&format!(
            "button.primary {{ background-color: {}; color: {}; border: none; }}\n",
            colors::BTN_PRIMARY_BG, colors::BTN_PRIMARY_TEXT,
        ));
        css.push_str(&format!(
            "button.primary:hover {{ background-color: {}; }}\n",
            colors::ACCENT_HOVER,
        ));
        css.push_str(&format!(
            "button.primary:active {{ background-color: {}; }}\n",
            colors::ACCENT_ACTIVE,
        ));
        css.push_str("button.danger { background-color: #dc2626; color: #ffffff; border: none; }\n");
        css.push_str("button.danger:hover { background-color: #b91c1c; }\n");
        css.push_str("scrolledwindow { background-color: transparent; }\n");
        css.push_str("viewport { background-color: transparent; }\n");
        css.push_str("flowbox { background-color: transparent; }\n");
        css.push_str("overlay { background-color: transparent; }\n");
        css.push_str("stack { background-color: transparent; }\n");

        /* Cabecera de detalles */
        css.push_str(&format!(
            ".details-header {{ background-color: {}; border-radius: {}; padding: 24px; margin-bottom: 24px; }}\n",
            colors::SURFACE, radius::LARGE,
        ));

        /* Barra superior. El padding horizontal sustituye los márgenes
         * manuales por elemento y mantiene alineadas las tres zonas. */
        css.push_str(&format!(
            ".top-bar {{ background-color: {}; border-bottom: 1px solid {}; padding: 10px 20px; }}\n",
            colors::TOP_BAR, colors::BORDER,
        ));
        css.push_str(".top-bar .app-title { font-weight: 800; font-size: 1.35rem; }\n");
        css.push_str(
            ".logo-button { background-color: transparent; border: none; padding: 4px 8px; min-height: 0; }\n",
        );
        css.push_str(
            ".logo-button:hover { background-color: transparent; border: none; transform: none; }\n",
        );

        /* Barra secundaria (nivel 2): filtros activos, continuar, vista y orden */
        css.push_str(&format!(
            ".top-bar-level2 {{ background-color: {}; border-bottom: 1px solid {}; padding: 8px 20px; font-size: 0.88rem; }}\n",
            colors::SURFACE, colors::BORDER,
        ));

        /* Chip removible de filtro activo */
        css.push_str(&format!(
            ".filter-chip {{ background-color: {}; border: 1px solid {}; color: {}; border-radius: 999px; padding: 6px 16px; min-height: 32px; font-weight: 600; }}\n",
            colors::CARD, colors::BORDER, colors::TEXT_SECONDARY,
        ));
        css.push_str(&format!(
            ".filter-chip:hover {{ border-color: {}; color: {}; background-color: {}; }}\n",
            colors::BORDER_HOVER, colors::TEXT_PRIMARY, colors::SURFACE,
        ));

        /* Píldora "Continuar viendo" */
        css.push_str(&format!(
            ".continue-button {{ background-color: {}; border: 1px solid {}; color: {}; border-radius: 999px; padding: 6px 16px; min-height: 32px; font-weight: 600; }}\n",
            colors::ACCENT, colors::ACCENT, colors::BTN_PRIMARY_TEXT,
        ));
        css.push_str(&format!(
            ".continue-button:hover {{ background-color: {}; border-color: {}; }}\n",
            colors::ACCENT_HOVER, colors::ACCENT,
        ));

        /* Botones de modo de vista (Grid/Lista) */
        css.push_str(&format!(
            ".view-mode-button {{ background-color: {}; border: 1px solid {}; color: {}; border-radius: {}; padding: 6px 12px; min-height: 32px; font-weight: 600; }}\n",
            colors::CARD, colors::BORDER, colors::TEXT_SECONDARY, radius::SMALL,
        ));
        css.push_str(&format!(
            ".view-mode-button:hover {{ border-color: {}; color: {}; background-color: {}; }}\n",
            colors::BORDER_HOVER, colors::TEXT_PRIMARY, colors::SURFACE,
        ));

        /* Navegación horizontal (Categorías) */
        css.push_str(&format!(
            ".nav-button {{ background-color: transparent; border: none; color: {}; font-size: 0.9rem; font-weight: 600; padding: 6px 12px; margin: 0 4px; border-radius: {}; min-height: 32px; }}\n",
            colors::TEXT_SECONDARY, radius::SMALL,
        ));
        css.push_str(&format!(
            ".nav-button:hover {{ background-color: {}; color: {}; }}\n",
            colors::SURFACE, colors::TEXT_PRIMARY,
        ));
        css.push_str(&format!(
            ".nav-button.active {{ background-color: {}; color: {}; }}\n",
            colors::ACCENT, colors::BTN_PRIMARY_TEXT,
        ));

        /* Título de página */
        css.push_str(&format!(
            ".page-title {{ font-size: 1.6rem; font-weight: 800; color: {}; }}\n",
            colors::TEXT_PRIMARY,
        ));

        /* Barra de filtro activo sobre la galería */
        css.push_str(&format!(
            ".filter-bar {{ background-color: {}; border-bottom: 1px solid {}; padding: 10px 24px; }}\n",
            colors::SURFACE, colors::BORDER,
        ));
        css.push_str(&format!(
            ".filter-label {{ color: {}; font-weight: 600; font-size: 0.92rem; }}\n",
            colors::TEXT_SECONDARY,
        ));

        /* Chips de categoría */
        css.push_str(&format!(
            ".category-chip {{ background-color: {}; border: 1px solid {}; color: {}; border-radius: 999px; padding: 8px 18px; min-height: 36px; }}\n",
            colors::CARD, colors::BORDER, colors::TEXT_SECONDARY,
        ));
        css.push_str(&format!(
            ".category-chip:hover {{ border-color: {}; color: {}; }}\n",
            colors::BORDER_HOVER, colors::TEXT_PRIMARY,
        ));
        css.push_str(&format!(
            ".category-chip.active {{ background-color: {}; border-color: {}; color: {}; }}\n",
            colors::ACCENT, colors::ACCENT, colors::BTN_PRIMARY_TEXT,
        ));

        /* Tarjeta de medio en galería */
        css.push_str(&format!(
            ".media-card {{ background-color: {}; border: 1px solid {}; border-radius: {}; padding: 8px; transition: all {}; }}\n",
            colors::CARD, colors::BORDER, radius::LARGE, transition::NORMAL,
        ));
        css.push_str(&format!(
            ".media-card:hover {{ background-color: {}; border-color: {}; transform: scale(1.03); }}\n",
            colors::SURFACE, colors::BORDER_HOVER,
        ));

        /* Póster de medio */
        css.push_str(&format!(
            ".media-poster {{ border-radius: {}; }}\n",
            radius::MEDIUM,
        ));

        /* Etiqueta de tipo en tarjeta de galería (Anime / Serie / Película) */
        css.push_str(&format!(
            ".card-type-label {{ color: {}; font-size: 0.75rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.5px; margin-top: 4px; }}\n",
            colors::ACCENT,
        ));

        /* Badge de continuación */
        css.push_str(&format!(
            ".continue-badge {{ background-color: {}; color: {}; border-radius: {}; font-size: 0.68rem; font-weight: 700; padding: 2px 8px; }}\n",
            colors::ACCENT, colors::BTN_PRIMARY_TEXT, radius::SMALL,
        ));

        /* Título de sección */
        css.push_str(&format!(
            ".section-title {{ color: {}; font-weight: 700; font-size: 1.25rem; }}\n",
            colors::TEXT_PRIMARY,
        ));

        /* Texto sutil */
        css.push_str(&format!(".text-muted {{ color: {}; }}\n", colors::TEXT_MUTED));
        css.push_str(&format!(".text-secondary {{ color: {}; }}\n", colors::TEXT_SECONDARY));

        /* Botón de reproducción de episodio */
        css.push_str(&format!(
            ".episode-play-btn {{ background-color: {}; color: {}; border: none; border-radius: {}; font-weight: 700; min-width: 72px; min-height: 30px; }}\n",
            colors::ACCENT, colors::BTN_PRIMARY_TEXT, radius::SMALL,
        ));
        css.push_str(&format!(
            ".episode-play-btn:hover {{ background-color: {}; }}\n",
            colors::ACCENT_HOVER,
        ));

        /* Tarjeta de episodio en rejilla */
        css.push_str(&format!(
            ".episode-card {{ background-color: {}; border: 1px solid {}; border-radius: {}; transition: all {}; }}\n",
            colors::CARD, colors::BORDER, radius::MEDIUM, transition::FAST,
        ));
        css.push_str(&format!(
            ".episode-card:hover {{ background-color: {}; border-color: {}; transform: scale(1.02); }}\n",
            colors::SURFACE, colors::BORDER_HOVER,
        ));
        css.push_str(&format!(
            ".episode-card:selected {{ background-color: {}; border-color: {}; }}\n",
            colors::SURFACE, colors::ACCENT,
        ));

        /* Portada de episodio */
        css.push_str(&format!(
            ".episode-poster-img {{ border-radius: {}; }}\n",
            radius::SMALL,
        ));

        /* Sobrelay de estado visto en episodio */
        css.push_str(".watched-overlay { background-color: rgba(0, 0, 0, 0.5); border-radius: 6px; }\n");

        /* Estrellas de calificación */
        css.push_str(&format!(".star {{ color: {}; }}\n", colors::TEXT_MUTED));
        css.push_str(&format!(".star.filled {{ color: {}; }}\n", colors::ACCENT));

        /* Banner hero de detalles */
        css.push_str(&format!(".banner {{ border-radius: {}; }}\n", radius::LARGE));
        css.push_str(&format!(
            ".banner-title {{ font-size: 2.5em; font-weight: 800; color: {}; }}\n",
            colors::TEXT_PRIMARY,
        ));
        css.push_str(&format!(
            ".banner-subtitle {{ font-size: 1.1em; color: {}; }}\n",
            colors::TEXT_SECONDARY,
        ));

        /* Fila de acciones hero */
        css.push_str(".hero-actions { margin-top: 14px; }\n");
        css.push_str(&format!(
            ".hero-action-btn {{ min-height: 40px; padding: 0 16px; font-weight: 700; border-radius: {}; }}\n",
            radius::SMALL,
        ));
        css.push_str(&format!(
            ".hero-action-btn.primary {{ background-color: {}; color: {}; border: none; }}\n",
            colors::BTN_PRIMARY_BG, colors::BTN_PRIMARY_TEXT,
        ));
        css.push_str(&format!(
            ".hero-action-btn.primary:hover {{ background-color: {}; }}\n",
            colors::ACCENT_HOVER,
        ));
        css.push_str(&format!(
            ".hero-action-btn.secondary {{ background-color: {}; color: {}; border: 1px solid {}; }}\n",
            colors::SURFACE, colors::TEXT_SECONDARY, colors::BORDER,
        ));
        css.push_str(&format!(
            ".hero-action-btn.secondary:hover {{ border-color: {}; background-color: {}; }}\n",
            colors::BORDER_HOVER, colors::CARD,
        ));

        /* Dropdown de temporada */
        css.push_str(".season-dropdown { min-width: 180px; }\n");

        /* Esqueleto de carga (skeleton) */
        css.push_str(&format!(
            ".skeleton {{ background-color: {}; border-radius: {}; animation: pulse 1.5s infinite ease-in-out; }}\n",
            colors::SKELETON_BG, radius::MEDIUM,
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
            colors::TEXT_SECONDARY,
        ));

        /* Botón plano (flat) */
        css.push_str(&format!(
            ".flat-button {{ background-color: transparent; border: none; color: {}; font-size: 0.9em; padding: 6px 12px; }}\n",
            colors::BTN_SECONDARY_TEXT,
        ));
        css.push_str(&format!(
            ".flat-button:hover {{ color: {}; background-color: {}; border-radius: {}; }}\n",
            colors::TEXT_PRIMARY, colors::SURFACE, radius::SMALL,
        ));

        /* Rejilla de episodios */
        css.push_str(".episode-grid { margin-top: 8px; }\n");
        css.push_str(".season-grid-section { margin-top: 28px; }\n");
        css.push_str(&format!(
            ".season-grid-title {{ font-weight: 700; font-size: 1.2em; color: {}; margin-bottom: 12px; margin-left: 6px; }}\n",
            colors::TEXT_PRIMARY,
        ));

        provider.load_from_data(&css);

        if let Some(display) = gtk4::gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    });
}
