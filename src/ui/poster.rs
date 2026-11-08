//! Componentes de imagen: pósters y banner hero de la vista de detalles.
//!
//! El banner sigue el diseño de Crunchyroll:
//! - Ancho completo y altura proporcional a la imagen, con un máximo para que
//!   el hero no ocupe toda la ventana; si la proporción no coincide, la imagen
//!   se recorta para llenar el ancho.
//! - Degradado oscuro en la parte inferior para que el texto sea legible.
//! - Título y subtítulo superpuestos a la imagen.
//! - Si no hay banner, se muestra un color de fondo del tema con una altura
//!   mínima para que la cabecera no colapse.

use crate::core::poster_service::cached_path;
use crate::ui::theme;
use gtk4::gdk_pixbuf::Pixbuf as GtkPixbuf;
use gtk4::prelude::*;
use gtk4::{
    Box, CssProvider, DrawingArea, Image, Label, Orientation, Overflow, Overlay, Picture,
    gdk::Display,
};
use std::cell::RefCell;
use std::f64::consts::{FRAC_PI_2, PI, TAU};
use std::rc::Rc;

/// Altura máxima del banner: por encima de este valor la imagen se recorta
/// para llenar el ancho, en lugar de ocupar toda la ventana.
const BANNER_MAX_HEIGHT: i32 = 620;

/// Altura de reserva del banner cuando el medio no tiene imagen.
const BANNER_FALLBACK_HEIGHT: i32 = BANNER_MAX_HEIGHT;

/// Tamaño máximo con el que se carga la imagen del banner.
const BANNER_IMAGE_MAX_WIDTH: i32 = 1920;
const BANNER_IMAGE_MAX_HEIGHT: i32 = 1080;

/// Radio de las esquinas del banner.
const BANNER_RADIUS: f64 = 0.0; // Crunchyroll usa esquinas cuadradas en el banner hero

thread_local! {
    /// Proveedor CSS de los pósters instalado actualmente (para reemplazarlo
    /// al cambiar de tema).
    static POSTER_PROVIDER: RefCell<Option<CssProvider>> = const { RefCell::new(None) };
}

mod banner_layout {
    use super::{BANNER_FALLBACK_HEIGHT, banner_height_for_width};
    use gtk4::glib;
    use gtk4::prelude::*;
    use gtk4::subclass::prelude::*;
    use std::cell::Cell;

    glib::wrapper! {
        pub struct BannerLayout(ObjectSubclass<imp::BannerLayout>)
            @extends gtk4::LayoutManager;
    }

    impl BannerLayout {
        pub fn new() -> Self {
            glib::Object::builder().build()
        }

        /// Actualiza la proporción del banner.
        pub fn set_aspect(&self, aspect: Option<f64>) {
            self.imp().aspect.set(aspect);
        }
    }

    impl Default for BannerLayout {
        fn default() -> Self {
            Self::new()
        }
    }

    mod imp {
        use super::*;

        #[derive(Default)]
        pub struct BannerLayout {
            pub aspect: Cell<Option<f64>>,
        }

        #[glib::object_subclass]
        impl ObjectSubclass for BannerLayout {
            const NAME: &'static str = "AcrylBannerLayout";
            type Type = super::BannerLayout;
            type ParentType = gtk4::LayoutManager;
        }

        impl ObjectImpl for BannerLayout {}

        impl LayoutManagerImpl for BannerLayout {
            fn request_mode(&self, _widget: &gtk4::Widget) -> gtk4::SizeRequestMode {
                // La altura depende del ancho disponible.
                gtk4::SizeRequestMode::HeightForWidth
            }

            fn measure(
                &self,
                widget: &gtk4::Widget,
                orientation: gtk4::Orientation,
                for_size: i32,
            ) -> (i32, i32, i32, i32) {
                if orientation == gtk4::Orientation::Vertical {
                    // La altura sigue al ancho hasta el máximo: el banner
                    // llena el ancho disponible y nunca colapsa.
                    let height = match self.aspect.get() {
                        Some(aspect) => banner_height_for_width(for_size, aspect),
                        None => BANNER_FALLBACK_HEIGHT,
                    };
                    return (height, height, -1, -1);
                }
                widget
                    .first_child()
                    .map_or((0, 0, -1, -1), |child| child.measure(orientation, for_size))
            }

            fn allocate(&self, widget: &gtk4::Widget, width: i32, height: i32, baseline: i32) {
                let mut child = widget.first_child();
                while let Some(current) = child {
                    current.allocate(width, height, baseline, None);
                    child = current.next_sibling();
                }
            }
        }
    }
}

/// Contenedor del banner que fija su altura según la proporción de la imagen.
///
/// Se usa un `LayoutManager` propio porque un `GtkBox` normal mide la altura
/// natural del hijo, pero su mínimo es 0: en la vista de detalles, donde el
/// contenido supera la altura de la ventana, el banner acababa colapsando.
struct BannerFrame {
    container: gtk4::Box,
    layout: banner_layout::BannerLayout,
}

impl BannerFrame {
    fn new() -> Self {
        let container = gtk4::Box::new(Orientation::Vertical, 0);
        let layout = banner_layout::BannerLayout::new();
        container.set_layout_manager(Some(layout.clone()));
        Self { container, layout }
    }

    fn append(&self, child: &impl IsA<gtk4::Widget>) {
        self.container.append(child);
    }

    fn set_aspect(&self, aspect: Option<f64>) {
        self.layout.set_aspect(aspect);
        self.container.queue_resize();
    }

    fn queue_resize(&self) {
        self.container.queue_resize();
    }

    fn widget(&self) -> &gtk4::Box {
        &self.container
    }
}

/// Instala los estilos de los pósters y el banner hero.
pub fn install_poster_css() {
    let c = theme::ThemeColors::for_mode(theme::current_mode());
    let provider = gtk4::CssProvider::new();
    let css = format!(
        ".poster-card {{ background-color: {}; border: 1px solid {}; border-radius: {}; padding: 8px; transition: all {}; }}\n\
             .poster-card:hover {{ background-color: {}; border-color: {}; transform: scale(1.02); }}\n\
             .poster-card:selected {{ background-color: {}; border-color: {}; }}\n\
             .poster-placeholder {{ background-color: {}; border-radius: {}; }}\n\
             .poster-title {{ font-weight: 600; font-size: 0.95em; color: {}; }}\n\
             .poster-year {{ font-size: 0.85em; color: {}; }}\n\
             .banner {{ border-radius: {}; }}\n\
             .banner-title {{ font-size: 2.5em; font-weight: 800; color: {}; }}\n\
             .banner-subtitle {{ font-size: 1.1em; color: {}; }}\n\
             .episode-row {{ background-color: {}; border: 1px solid {}; border-radius: {}; padding: 8px 12px; margin: 4px 0; transition: all {}; }}\n\
             .episode-row:hover {{ background-color: {}; border-color: {}; }}\n\
             .episode-row:selected {{ background-color: {}; border-color: {}; }}\n\
             .season-section {{ margin-top: 24px; }}\n\
             .season-title {{ font-weight: 700; font-size: 1.2em; color: {}; margin-bottom: 12px; }}\n\
             .play-button {{ background-color: {}; color: {}; border-radius: {}; font-weight: 700; min-width: 120px; min-height: 44px; }}\n\
             .play-button:hover {{ background-color: {}; }}\n\
             .flat-button {{ background-color: transparent; border: none; color: {}; font-size: 0.9em; padding: 6px 12px; }}\n\
             .flat-button:hover {{ color: {}; background-color: {}; border-radius: {}; box-shadow: inset 0 0 0 1px {}; }}\n\
             .details-header {{ background-color: {}; border-radius: {}; padding: 0; margin-bottom: 24px; }}\n\
             .synopsis-text {{ color: {}; line-height: 1.6; }}\n\
             .media-type {{ font-size: 0.75rem; text-transform: uppercase; letter-spacing: 1px; color: {}; font-weight: 700; }}\n\
             .episode-grid {{ margin-top: 8px; }}\n\
             .episode-card {{ background-color: {}; border: 1px solid {}; border-radius: {}; padding: 8px; margin: 6px; min-width: 170px; transition: all {}; }}\n\
             .episode-card:hover {{ background-color: {}; border-color: {}; transform: scale(1.02); }}\n\
             .episode-card:selected {{ background-color: {}; border-color: {}; }}\n\
             .episode-poster {{ border-radius: {}; margin-bottom: 8px; }}\n\
             .episode-title {{ font-weight: 600; font-size: 0.82em; color: {}; }}\n\
             .episode-number {{ font-size: 0.75em; color: {}; margin-bottom: 4px; }}\n\
             .episode-play-btn {{ background-color: {}; color: {}; border: none; border-radius: {}; font-weight: 700; min-width: 72px; min-height: 30px; }}\n\
             .episode-play-btn:hover {{ background-color: {}; box-shadow: inset 0 0 0 2px rgba(255, 255, 255, 0.55); }}\n\
             .season-grid-section {{ margin-top: 28px; }}\n\
             .season-grid-title {{ font-weight: 700; font-size: 1.2em; color: {}; margin-bottom: 12px; margin-left: 6px; }}\n\
             .hero-action-btn {{ min-height: 40px; padding: 0 16px; font-weight: 700; border-radius: {}; }}\n\
             .hero-action-btn.primary {{ background-color: {}; color: {}; border: none; }}\n\
             .hero-action-btn.primary:hover {{ background-color: {}; box-shadow: inset 0 0 0 2px rgba(255, 255, 255, 0.55); }}\n\
             .hero-action-btn.secondary {{ background-color: {}; color: {}; border: 1px solid {}; }}\n\
             .hero-action-btn.secondary:hover {{ border-color: {}; background-color: {}; }}\n\
             .season-dropdown {{ min-width: 180px; }}\n\
             .rating-stars {{ color: {}; }}\n\
             .rating-stars .filled {{ color: {}; }}\n\
             .thumbnail-placeholder {{ background-color: {}; border: 1px dashed {}; border-radius: {}; }}\n",
        c.card,
        c.border,
        theme::radius::LARGE,
        theme::transition::NORMAL,
        c.card,
        c.border_hover,
        c.surface,
        c.accent,
        c.card,
        theme::radius::MEDIUM,
        c.text_primary,
        c.text_secondary,
        theme::radius::LARGE,
        c.text_primary,
        c.text_secondary,
        c.card,
        c.border,
        theme::radius::MEDIUM,
        theme::transition::FAST,
        c.card,
        c.border_hover,
        c.surface,
        c.accent,
        c.text_primary,
        c.accent,
        c.btn_primary_text,
        theme::radius::SMALL,
        c.accent,
        c.text_secondary,
        c.text_primary,
        "transparent",
        theme::radius::SMALL,
        c.border_hover,
        c.surface,
        theme::radius::LARGE,
        c.text_secondary,
        c.accent,
        c.card,
        c.border,
        theme::radius::MEDIUM,
        theme::transition::FAST,
        c.card,
        c.border_hover,
        c.surface,
        c.accent,
        theme::radius::SMALL,
        c.text_primary,
        c.text_secondary,
        c.accent,
        c.btn_primary_text,
        theme::radius::SMALL,
        c.accent,
        c.text_primary,
        theme::radius::SMALL,
        c.accent,
        c.btn_primary_text,
        c.accent,
        c.surface,
        c.text_primary,
        c.border,
        c.border_hover,
        c.surface,
        c.text_muted,
        c.accent,
        c.surface,
        c.border,
        theme::radius::SMALL,
    );
    provider.load_from_data(&css);

    if let Some(display) = Display::default() {
        POSTER_PROVIDER.with(|p| {
            if let Some(prev) = p.borrow_mut().take() {
                gtk4::style_context_remove_provider_for_display(&display, &prev);
            }
        });
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        POSTER_PROVIDER.with(|p| *p.borrow_mut() = Some(provider));
    }
}

/// Crea un widget de póster que escala la imagen al espacio disponible.
pub fn poster_picture(pixbuf: &GtkPixbuf, width: i32, height: i32) -> Picture {
    let picture = Picture::new();
    picture.set_size_request(width, height);
    picture.set_hexpand(false);
    picture.set_vexpand(false);
    picture.set_halign(gtk4::Align::Center);
    picture.set_valign(gtk4::Align::Center);
    picture.set_alternative_text(Some("Póster"));

    let texture = gtk4::gdk::Texture::for_pixbuf(pixbuf);
    picture.set_paintable(Some(&texture));
    picture
}

/// Crea el marcador visual cuando un medio no tiene póster.
pub fn placeholder_widget(width: i32, height: i32) -> Box {
    let placeholder = Box::new(Orientation::Vertical, 0);
    placeholder.set_size_request(width, height);
    placeholder.set_halign(gtk4::Align::Center);
    placeholder.set_valign(gtk4::Align::Center);
    placeholder.add_css_class("poster-placeholder");

    let icon = Image::from_icon_name("video-x-generic-symbolic");
    icon.set_pixel_size(72);
    icon.set_halign(gtk4::Align::Center);
    icon.set_valign(gtk4::Align::Center);
    icon.set_vexpand(true);
    placeholder.append(&icon);

    placeholder
}

/// Color RGB normalizado (0.0–1.0).
#[derive(Debug, Clone, Copy)]
struct Rgb {
    r: f64,
    g: f64,
    b: f64,
}

impl Rgb {
    const fn new(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b }
    }
}

/// Paleta de dos tonos para el fondo del banner.
#[derive(Debug, Clone, Copy)]
struct BannerPalette {
    start: Rgb,
    end: Rgb,
}

impl Default for BannerPalette {
    fn default() -> Self {
        Self {
            start: Rgb::new(0.20, 0.20, 0.25),
            end: Rgb::new(0.08, 0.08, 0.12),
        }
    }
}

/// Banner hero al estilo Crunchyroll.
///
/// Muestra la imagen ancha (`banner_path`) a todo el ancho con un degradado
/// oscuro en la parte inferior. Si no hay banner, muestra un color de fondo
/// del tema. El título y el subtítulo se superponen.
///
/// En las vistas de detalle se muestra exclusivamente el banner como imagen
/// principal; el póster vertical ya no se duplica junto al banner.
pub struct Banner {
    frame: BannerFrame,
    overlay: Overlay,
    picture: Picture,
    drawing: DrawingArea,
    text_box: Box,
    palette: Rc<RefCell<BannerPalette>>,
    title_label: Label,
    subtitle_label: Label,
}

impl Banner {
    pub fn new() -> Self {
        install_poster_css();

        let palette = Rc::new(RefCell::new(BannerPalette::default()));

        let picture = Picture::new();
        picture.set_hexpand(true);
        picture.set_vexpand(true);
        picture.set_can_shrink(true);
        // `Cover` llena todo el ancho disponible; la altura se limita con
        // `BANNER_MAX_HEIGHT` para que el hero no ocupe toda la ventana.
        picture.set_content_fit(gtk4::ContentFit::Cover);
        picture.set_alternative_text(Some("Banner del medio"));
        picture.set_visible(false);

        let drawing = DrawingArea::new();
        drawing.set_hexpand(true);
        drawing.set_vexpand(true);

        let palette_for_draw = palette.clone();
        drawing.set_draw_func(move |_area, cr, width, height| {
            draw_banner(cr, width, height, *palette_for_draw.borrow());
        });

        let overlay = Overlay::new();
        overlay.set_child(Some(&drawing));
        overlay.set_hexpand(true);
        overlay.set_vexpand(true);
        overlay.set_valign(gtk4::Align::Fill);
        overlay.set_overflow(Overflow::Hidden);
        overlay.add_css_class("banner");

        // El contenedor calcula la altura mínima a partir del ancho y de la
        // proporción de la imagen. Sin ese mínimo la cabecera colapsa cuando
        // el resto de la vista de detalles necesita más altura que la ventana.
        let frame = BannerFrame::new();
        frame.widget().set_hexpand(true);
        frame.widget().set_vexpand(false);
        frame.append(&overlay);

        overlay.add_overlay(&picture);
        overlay.set_measure_overlay(&picture, true);

        let title_label = Label::new(None);
        title_label.set_halign(gtk4::Align::Start);
        title_label.set_xalign(0.0);
        title_label.set_wrap(true);
        title_label.set_max_width_chars(60);
        title_label.add_css_class("banner-title");

        let subtitle_label = Label::new(None);
        subtitle_label.set_halign(gtk4::Align::Start);
        subtitle_label.set_xalign(0.0);
        subtitle_label.add_css_class("banner-subtitle");

        let text_box = Box::new(Orientation::Vertical, 6);
        text_box.set_halign(gtk4::Align::Fill);
        text_box.set_valign(gtk4::Align::End);
        text_box.set_margin_top(24);
        text_box.set_margin_bottom(30);
        text_box.set_margin_start(28);
        text_box.set_margin_end(28);
        text_box.set_can_target(false);
        text_box.append(&title_label);
        text_box.append(&subtitle_label);
        overlay.add_overlay(&text_box);

        Self {
            frame,
            overlay,
            picture,
            drawing,
            text_box,
            palette,
            title_label,
            subtitle_label,
        }
    }

    pub fn widget(&self) -> &gtk4::Box {
        self.frame.widget()
    }

    pub fn set_info(&mut self, title: &str, subtitle: &str) {
        self.title_label
            .set_markup(&format!("<b>{}</b>", escape_markup(title)));
        self.subtitle_label.set_label(subtitle);
        self.subtitle_label.set_visible(!subtitle.is_empty());
    }

    pub fn set_images(&mut self, banner_url: Option<&str>) {
        let banner = banner_url
            .filter(|url| !url.is_empty())
            .and_then(cached_path)
            .and_then(|cache_path| {
                GtkPixbuf::from_file_at_scale(
                    &cache_path,
                    BANNER_IMAGE_MAX_WIDTH,
                    BANNER_IMAGE_MAX_HEIGHT,
                    true,
                )
                .ok()
            });

        match banner {
            Some(pixbuf) => {
                let aspect = f64::from(pixbuf.width()) / f64::from(pixbuf.height());
                let texture = gtk4::gdk::Texture::for_pixbuf(&pixbuf);
                self.picture.set_paintable(Some(&texture));
                self.picture.set_visible(true);
                // El contenedor ajusta la altura (con un máximo) a la
                // proporción de la imagen.
                self.frame.set_aspect(Some(aspect));
                self.frame.queue_resize();
                // Forzar redibujado inmediato para que el nuevo banner se vea
                // sin necesidad de reiniciar la aplicación.
                self.picture.queue_resize();
                self.picture.queue_draw();
                self.drawing.queue_draw();
                self.overlay.queue_resize();
                self.overlay.queue_draw();
            }
            None => {
                // No hay banner propio: mostrar color de fondo del tema en lugar
                // de un degradado vacío. El póster y el banner son formatos
                // diferentes (vertical vs horizontal) y nunca se usan como
                // sustituto del otro.
                self.frame.set_aspect(None);
                self.picture.set_paintable(gtk4::gdk::Texture::NONE);
                self.picture.set_visible(false);
                self.frame.queue_resize();
                *self.palette.borrow_mut() = BannerPalette::default();
                self.drawing.queue_draw();
                self.overlay.queue_resize();
                self.overlay.queue_draw();
            }
        }

        // El póster vertical ya no se superpone al banner en las vistas de
        // detalle (solo el banner es la imagen principal). El margen del texto
        // se mantiene siempre igual.
        self.text_box.set_margin_start(28);
    }
}

impl Default for Banner {
    fn default() -> Self {
        Self::new()
    }
}

/// Altura que debe reservar el banner para mostrar la imagen entera.
///
/// Sin un ancho válido (aún no asignado) o con una proporción inválida se usa
/// la altura de reserva, de modo que la cabecera nunca colapse. La altura se
/// limita a `BANNER_MAX_HEIGHT` para que el hero no ocupe toda la ventana.
fn banner_height_for_width(width: i32, aspect: f64) -> i32 {
    if width <= 0 || !aspect.is_finite() || aspect <= 0.0 {
        return BANNER_FALLBACK_HEIGHT;
    }
    ((f64::from(width) / aspect).round() as i32).clamp(1, BANNER_MAX_HEIGHT)
}

/// Dibuja el fondo del banner: degradado oscuro y círculos translúcidos.
fn draw_banner(cr: &gtk4::cairo::Context, width: i32, height: i32, palette: BannerPalette) {
    let width = f64::from(width);
    let height = f64::from(height);
    if width <= 0.0 || height <= 0.0 {
        return;
    }

    rounded_rectangle(cr, 0.0, 0.0, width, height, BANNER_RADIUS);
    cr.clip();

    let gradient = gtk4::cairo::LinearGradient::new(0.0, 0.0, width, height);
    gradient.add_color_stop_rgb(0.0, palette.start.r, palette.start.g, palette.start.b);
    gradient.add_color_stop_rgb(1.0, palette.end.r, palette.end.g, palette.end.b);
    let _ = cr.set_source(&gradient);
    let _ = cr.paint();

    cr.set_source_rgba(1.0, 1.0, 1.0, 0.05);
    cr.arc(width * 0.85, height * 0.15, height * 0.9, 0.0, TAU);
    let _ = cr.fill();

    let shade = gtk4::cairo::LinearGradient::new(0.0, height, 0.0, 0.0);
    shade.add_color_stop_rgba(0.0, 0.0, 0.0, 0.0, 0.80);
    shade.add_color_stop_rgba(0.50, 0.0, 0.0, 0.0, 0.15);
    shade.add_color_stop_rgba(1.0, 0.0, 0.0, 0.0, 0.0);
    let _ = cr.set_source(&shade);
    let _ = cr.paint();
}

fn rounded_rectangle(
    cr: &gtk4::cairo::Context,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    radius: f64,
) {
    let radius = radius.min(width / 2.0).min(height / 2.0);
    cr.new_sub_path();
    cr.arc(x + width - radius, y + radius, radius, -FRAC_PI_2, 0.0);
    cr.arc(
        x + width - radius,
        y + height - radius,
        radius,
        0.0,
        FRAC_PI_2,
    );
    cr.arc(x + radius, y + height - radius, radius, FRAC_PI_2, PI);
    cr.arc(x + radius, y + radius, radius, PI, 3.0 * FRAC_PI_2);
    cr.close_path();
}

fn escape_markup(text: &str) -> String {
    gtk4::glib::markup_escape_text(text).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_height_respeta_la_proporcion_y_limita_la_altura() {
        // A 873 px de ancho, 16:9 son 491 px y caben por debajo del máximo.
        assert_eq!(banner_height_for_width(873, 16.0 / 9.0), 491);
        // En ventanas anchas la altura se limita para no ocupar toda la vista.
        assert_eq!(banner_height_for_width(1200, 16.0 / 9.0), BANNER_MAX_HEIGHT);
        assert_eq!(banner_height_for_width(1920, 16.0 / 9.0), BANNER_MAX_HEIGHT);
        // Las imágenes más panorámicas conservan su altura proporcional.
        assert_eq!(banner_height_for_width(1200, 3.0), 400);
    }

    #[test]
    fn banner_height_usa_la_reserva_sin_ancho_o_proporcion_validos() {
        assert_eq!(
            banner_height_for_width(0, 16.0 / 9.0),
            BANNER_FALLBACK_HEIGHT
        );
        assert_eq!(
            banner_height_for_width(-1, 16.0 / 9.0),
            BANNER_FALLBACK_HEIGHT
        );
        assert_eq!(banner_height_for_width(1200, 0.0), BANNER_FALLBACK_HEIGHT);
        assert_eq!(
            banner_height_for_width(1200, f64::NAN),
            BANNER_FALLBACK_HEIGHT
        );
        assert_eq!(
            banner_height_for_width(1200, f64::INFINITY),
            BANNER_FALLBACK_HEIGHT
        );
    }
}
