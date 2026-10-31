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
        let css = format!(
            "window {{ background-color: {}; color: {}; font-family: 'Inter', 'Roboto', 'Helvetica', 'Arial', sans-serif; }}\n\
             box {{ background-color: transparent; }}\n\
             label {{ color: {}; }}\n\
             label.dim-label {{ color: {}; }}\n\
             entry {{ background-color: {}; color: {}; border: 1px solid {}; border-radius: {}; padding: 8px 12px; }}\n\
             entry:focus {{ border-color: {}; }}\n\
             button {{ background-color: {}; color: {}; border: 1px solid {}; border-radius: {}; padding: 8px 16px; font-weight: 600; min-height: 36px; transition: all {}; }}\n\
             button:hover {{ background-color: {}; border-color: {}; }}\n\
             button:active {{ background-color: {}; border-color: {}; color: {}; }}\n\
             button.flat {{ background-color: transparent; border: none; color: {}; padding: 6px 12px; }}\n\
             button.flat:hover {{ background-color: {}; color: {}; border-radius: {}; }}\n\
             button.flat:active {{ color: {}; }}\n\
             button.primary {{ background-color: {}; color: {}; border: none; }}\n\
             button.primary:hover {{ background-color: {}; }}\n\
             button.primary:active {{ background-color: {}; }}\n\
             button.danger {{ background-color: #dc2626; color: {}; border: none; }}\n\
             button.danger:hover {{ background-color: #b91c1c; }}\n\
             scrolledwindow {{ background-color: transparent; }}\n\
             viewport {{ background-color: transparent; }}\n\
             flowbox {{ background-color: transparent; }}\n\
             overlay {{ background-color: transparent; }}\n\
             stack {{ background-color: transparent; }}\n\
             \n/* Tarjeta de medio en galería */\n\
             .media-card {{ background-color: {}; border: 1px solid {}; border-radius: {}; transition: all {}; }}\n\
             .media-card:hover {{ background-color: {}; border-color: {}; transform: scale(1.03); }}\n\
             \n/* Póster de medio */\n\
             .media-poster {{ border-radius: {}; }}\n\
             \n/* Badge de tipo (Anime / Serie / Película) */\n\
             .type-badge {{ background-color: {}; color: {}; border-radius: {}; font-size: 0.7rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.5px; padding: 2px 8px; }}\n\
             \n/* Título de sección */\n\
             .section-title {{ color: {}; font-weight: 700; font-size: 1.25rem; }}\n\
             \n/* Texto sutil */\n\
             .text-muted {{ color: {}; }}\n\
             .text-secondary {{ color: {}; }}\n\
             \n/* Botón de reproducción de episodio */\n\
             .episode-play-btn {{ background-color: {}; color: {}; border: none; border-radius: {}; font-weight: 700; min-width: 72px; min-height: 30px; }}\n\
             .episode-play-btn:hover {{ background-color: {}; }}\n\
             \n/* Tarjeta de episodio en rejilla */\n\
             .episode-card {{ background-color: {}; border: 1px solid {}; border-radius: {}; transition: all {}; }}\n\
             .episode-card:hover {{ background-color: {}; border-color: {}; transform: scale(1.02); }}\n\
             \n/* Portada de episodio */\n\
             .episode-poster-img {{ border-radius: {}; }}\n\
             \n/* Sobrelay de estado visto en episodio */\n\
             .watched-overlay {{ background-color: rgba(0, 0, 0, 0.5); border-radius: {}; }}\n\
             \n/* Estrellas de calificación */\n\
             .star {{ color: {}; }}\n\
             .star.filled {{ color: {}; }}\n\
             \n/* Badge de continuación */\n\
             .continue-badge {{ background-color: {}; color: {}; border-radius: {}; font-size: 0.7rem; font-weight: 700; padding: 2px 8px; }}\n",
            colors::BG, colors::TEXT_PRIMARY,
            colors::TEXT_PRIMARY,
            colors::TEXT_MUTED,
            colors::CARD, colors::TEXT_PRIMARY, colors::BORDER, radius::MEDIUM,
            colors::ACCENT,
            colors::CARD, colors::TEXT_PRIMARY, colors::BORDER, radius::MEDIUM, transition::FAST,
            colors::SURFACE, colors::BORDER_HOVER,
            colors::ACCENT_ACTIVE, colors::ACCENT, colors::BTN_PRIMARY_TEXT,
            colors::TEXT_SECONDARY,
            colors::SURFACE, colors::TEXT_PRIMARY, radius::SMALL,
            colors::ACCENT,
            colors::BTN_PRIMARY_BG, colors::BTN_PRIMARY_TEXT,
            colors::ACCENT_HOVER,
            colors::ACCENT_ACTIVE,
            colors::TEXT_PRIMARY,
            colors::CARD, colors::BORDER, radius::LARGE, transition::NORMAL,
            colors::SURFACE, colors::BORDER_HOVER,
            radius::MEDIUM,
            colors::ACCENT, colors::BTN_PRIMARY_TEXT, radius::SMALL,
            colors::TEXT_PRIMARY,
            colors::TEXT_MUTED,
            colors::TEXT_SECONDARY,
            colors::ACCENT, colors::BTN_PRIMARY_TEXT, radius::SMALL,
            colors::ACCENT_HOVER,
            colors::CARD, colors::BORDER, radius::MEDIUM, transition::FAST,
            colors::SURFACE, colors::BORDER_HOVER,
            colors::ACCENT,
            radius::SMALL,
            radius::SMALL,
            colors::TEXT_MUTED,
            colors::ACCENT, colors::BTN_PRIMARY_TEXT, radius::SMALL,
        );
        let css = css + "\
             .top-bar {{ background-color: #141414; border-bottom: 1px solid #333333; padding: 10px 0; }}\n";
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
