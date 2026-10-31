//! Componentes de imagen: pósters de la galería y banner de la vista de detalles.
//! Los pósters y banners se cargan desde la caché local; si el medio no tiene
//! banner, se dibuja un degradado con los colores dominantes del póster.

use crate::core::poster_service::cached_path;
use gtk4::gdk_pixbuf::Pixbuf as GtkPixbuf;
use gtk4::prelude::*;
use gtk4::{Box, DrawingArea, Image, Label, Orientation, Overflow, Overlay, Picture};
use std::cell::RefCell;
use std::f64::consts::{FRAC_PI_2, PI, TAU};
use std::rc::Rc;
use std::sync::Once;

/// Altura del banner de la vista de detalles.
const BANNER_HEIGHT: i32 = 400;

/// Tamaño máximo con el que se carga la imagen del banner (se recorta con
/// `ContentFit::Cover` al espacio disponible).
const BANNER_IMAGE_MAX_WIDTH: i32 = 1920;
const BANNER_IMAGE_MAX_HEIGHT: i32 = 1080;

/// Radio de las esquinas del banner, en píxeles.
const BANNER_RADIUS: f64 = 16.0;

/// Resolución a la que se muestrea el póster para extraer su paleta.
const PALETTE_SAMPLE_SIZE: i32 = 24;

/// Estilos compartidos por las tarjetas, los pósters y el banner.
static INSTALL_POSTER_CSS: Once = Once::new();

/// Instala una sola vez los estilos de las tarjetas, los pósters y el banner.
pub fn install_poster_css() {
    INSTALL_POSTER_CSS.call_once(|| {
        let provider = gtk4::CssProvider::new();
        provider.load_from_data(
            ".poster-card { \
                 background-color: #2a2a2a; \
                 border: 1px solid #3a3a3a; \
                 border-radius: 10px; \
                 padding: 8px; \
                 transition: all 200ms ease; \
             } \
             .poster-card:hover { \
                 background-color: #333333; \
                 border-color: #ff6b35; \
                 transform: scale(1.02); \
             } \
             .poster-card:selected { \
                 background-color: #3a3a3a; \
                 border-color: #ff6b35; \
             } \
             .poster-placeholder { \
                 background-color: #333333; \
                 border-radius: 8px; \
             } \
             .poster-title { \
                 font-weight: 600; \
                 font-size: 0.95em; \
                 color: #ffffff; \
             } \
             .poster-year { \
                 font-size: 0.85em; \
                 color: #a0a0a0; \
             } \
             .banner { border-radius: 16px; } \
             .banner-title { \
                 font-size: 2.2em; \
                 font-weight: 800; \
                 color: #ffffff; \
             } \
             .banner-subtitle { \
                 font-size: 1.1em; \
                 color: #cccccc; \
             } \
             .episode-row { \
                 background-color: #2a2a2a; \
                 border: 1px solid #3a3a3a; \
                 border-radius: 8px; \
                 padding: 8px 12px; \
                 margin: 4px 0; \
                 transition: all 150ms ease; \
             } \
             .episode-row:hover { \
                 background-color: #333333; \
                 border-color: #ff6b35; \
             } \
             .episode-row:selected { \
                 background-color: #3a3a3a; \
                 border-color: #ff6b35; \
             } \
             .season-section { \
                 margin-top: 24px; \
             } \
             .season-title { \
                 font-weight: 700; \
                 font-size: 1.1em; \
                 color: #ffffff; \
                 margin-bottom: 8px; \
             } \
             .play-button { \
                 background-color: #ff6b35; \
                 color: #ffffff; \
                 border-radius: 4px; \
                 font-weight: 600; \
                 min-width: 100px; \
                 min-height: 36px; \
             } \
             .play-button:hover { \
                 background-color: #e85a2c; \
             } \
             .flat-button { \
                 background-color: transparent; \
                 border: none; \
                 color: #cccccc; \
                 font-size: 0.9em; \
             } \
             .flat-button:hover { \
                 color: #ffffff; \
                 background-color: #3a3a3a; \
                 border-radius: 4px; \
             } \
             .details-header { \
                 background-color: #1a1a1a; \
                 border-radius: 16px; \
                 padding: 20px; \
                 margin-bottom: 16px; \
             } \
             .synopsis-text { \
                 color: #e0e0e0; \
                 line-height: 1.5; \
             } \
             .media-type { \
                 font-size: 0.8em; \
                 text-transform: uppercase; \
                 letter-spacing: 1px; \
                 color: #ff6b35; \
                 font-weight: 700; \
             } \
             /* Grid de episodios al estilo Crunchyroll */ \
             .episode-grid { \
                 margin-top: 8px; \
             } \
             .episode-card { \
                 background-color: #2a2a2a; \
                 border: 1px solid #3a3a3a; \
                 border-radius: 8px; \
                 padding: 8px; \
                 margin: 6px; \
                 min-width: 170px; \
                 transition: all 150ms ease; \
             } \
             .episode-card:hover { \
                 background-color: #333333; \
                 border-color: #ff6b35; \
                 transform: scale(1.03); \
             } \
             .episode-card:selected { \
                 background-color: #3a3a3a; \
                 border-color: #ff6b35; \
             } \
             .episode-poster { \
                 border-radius: 6px; \
                 margin-bottom: 8px; \
             } \
             .episode-title { \
                 font-weight: 600; \
                 font-size: 0.82em; \
                 color: #e0e0e0; \
             } \
             .episode-number { \
                 font-size: 0.75em; \
                 color: #a0a0a0; \
                 margin-bottom: 4px; \
             } \
             .episode-play-btn { \
                 background-color: #ff6b35; \
                 color: #ffffff; \
                 border-radius: 4px; \
                 font-weight: 600; \
                 font-size: 0.8em; \
                 min-width: 80px; \
                 min-height: 30px; \
                 margin-top: 6px; \
             } \
             .episode-play-btn:hover { \
                 background-color: #e85a2c; \
             } \
             .season-grid-section { \
                 margin-top: 28px; \
             } \
             .season-grid-title { \
                 font-weight: 700; \
                 font-size: 1.1em; \
                 color: #ffffff; \
                 margin-bottom: 12px; \
                 margin-left: 6px; \
             }",
        );

        if let Some(display) = gtk4::gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    });
}

/// Crea un widget de póster que escala la imagen al espacio disponible.
///
/// `GtkImage` dibuja los pixbufs al tamaño de icono del tema, por lo que se usa
/// `GtkPicture` para mostrar el póster completo a su tamaño real.
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

/// Crea el marcador visual que se muestra cuando un medio no tiene póster.
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

/// Paleta de dos tonos usada para dibujar el fondo del banner.
#[derive(Debug, Clone, Copy)]
struct BannerPalette {
    start: Rgb,
    end: Rgb,
}

impl Default for BannerPalette {
    fn default() -> Self {
        Self {
            start: Rgb::new(0.27, 0.33, 0.52),
            end: Rgb::new(0.07, 0.08, 0.14),
        }
    }
}

/// Banner ancho de la vista de detalles.
///
/// Muestra la imagen ancha del medio (`banner_path`) recortada a lo ancho. Si
/// el medio no tiene banner, dibuja un degradado con los colores dominantes del
/// póster. En ambos casos superpone el título y el subtítulo.
pub struct Banner {
    overlay: Overlay,
    picture: Picture,
    drawing: DrawingArea,
    palette: Rc<RefCell<BannerPalette>>,
    title_label: Label,
    subtitle_label: Label,
}

impl Banner {
    /// Crea un banner vacío. El título y las imágenes se asignan después.
    pub fn new() -> Self {
        install_poster_css();

        let palette = Rc::new(RefCell::new(BannerPalette::default()));

        let picture = Picture::new();
        picture.set_hexpand(true);
        picture.set_vexpand(true);
        picture.set_can_shrink(true);
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
        overlay.set_valign(gtk4::Align::Start);
        overlay.set_size_request(-1, BANNER_HEIGHT);
        overlay.set_overflow(Overflow::Hidden);
        overlay.add_css_class("banner");

        // La imagen se añade como capa sin influir en el tamaño del banner:
        // así la altura queda fijada en BANNER_HEIGHT y se recorta a lo ancho.
        overlay.add_overlay(&picture);
        overlay.set_measure_overlay(&picture, false);

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

        let text_box = Box::new(Orientation::Vertical, 4);
        text_box.set_halign(gtk4::Align::Fill);
        text_box.set_valign(gtk4::Align::End);
        text_box.set_margin_top(16);
        text_box.set_margin_bottom(18);
        text_box.set_margin_start(20);
        text_box.set_margin_end(20);
        text_box.set_can_target(false);
        text_box.append(&title_label);
        text_box.append(&subtitle_label);
        overlay.add_overlay(&text_box);

        Self {
            overlay,
            picture,
            drawing,
            palette,
            title_label,
            subtitle_label,
        }
    }

    /// Devuelve el widget Gtk.
    pub fn widget(&self) -> &Overlay {
        &self.overlay
    }

    /// Establece el título y el subtítulo (tipo y año) del banner.
    pub fn set_info(&mut self, title: &str, subtitle: &str) {
        self.title_label
            .set_markup(&format!("<b>{}</b>", escape_markup(title)));
        self.subtitle_label.set_label(subtitle);
        self.subtitle_label.set_visible(!subtitle.is_empty());
    }

    /// Establece las imágenes del banner.
    ///
    /// Si `banner_url` está en caché se muestra recortada a lo ancho. Si no, se
    /// dibuja un degradado a partir de los colores dominantes del póster (sin
    /// mostrar el póster). Si tampoco hay póster, se usa la paleta por defecto.
    pub fn set_images(&mut self, banner_url: Option<&str>, poster_url: Option<&str>) {
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
                let texture = gtk4::gdk::Texture::for_pixbuf(&pixbuf);
                self.picture.set_paintable(Some(&texture));
                self.picture.set_visible(true);
            }
            None => {
                self.picture.set_paintable(gtk4::gdk::Texture::NONE);
                self.picture.set_visible(false);
                *self.palette.borrow_mut() = poster_palette(poster_url);
                self.drawing.queue_draw();
            }
        }
    }
}

impl Default for Banner {
    fn default() -> Self {
        Self::new()
    }
}

/// Dibuja el fondo del banner: degradado, formas suaves y sombra inferior.
fn draw_banner(cr: &gtk4::cairo::Context, width: i32, height: i32, palette: BannerPalette) {
    let width = f64::from(width);
    let height = f64::from(height);
    if width <= 0.0 || height <= 0.0 {
        return;
    }

    // Recorta el dibujo a las esquinas redondeadas del banner.
    rounded_rectangle(cr, 0.0, 0.0, width, height, BANNER_RADIUS);
    cr.clip();

    let gradient = gtk4::cairo::LinearGradient::new(0.0, 0.0, width, height);
    gradient.add_color_stop_rgb(0.0, palette.start.r, palette.start.g, palette.start.b);
    gradient.add_color_stop_rgb(1.0, palette.end.r, palette.end.g, palette.end.b);
    let _ = cr.set_source(&gradient);
    let _ = cr.paint();

    // Círculos translúcidos que aportan profundidad al degradado.
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.07);
    cr.arc(width * 0.82, height * 0.16, height * 0.8, 0.0, TAU);
    let _ = cr.fill();

    cr.set_source_rgba(0.0, 0.0, 0.0, 0.12);
    cr.arc(width * 0.08, height * 1.05, height * 0.95, 0.0, TAU);
    let _ = cr.fill();

    // Sombra inferior para que el título y el subtítulo sean legibles.
    let shade = gtk4::cairo::LinearGradient::new(0.0, height, 0.0, 0.0);
    shade.add_color_stop_rgba(0.0, 0.0, 0.0, 0.0, 0.72);
    shade.add_color_stop_rgba(0.55, 0.0, 0.0, 0.0, 0.10);
    shade.add_color_stop_rgba(1.0, 0.0, 0.0, 0.0, 0.0);
    let _ = cr.set_source(&shade);
    let _ = cr.paint();
}

/// Traza un rectángulo con esquinas redondeadas.
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

/// Calcula una paleta a partir del color dominante del póster en caché.
fn poster_palette(poster_url: Option<&str>) -> BannerPalette {
    let Some(url) = poster_url.filter(|url| !url.is_empty()) else {
        return BannerPalette::default();
    };
    let Some(cache_path) = cached_path(url) else {
        return BannerPalette::default();
    };
    let Ok(pixbuf) =
        GtkPixbuf::from_file_at_scale(&cache_path, PALETTE_SAMPLE_SIZE, PALETTE_SAMPLE_SIZE, false)
    else {
        return BannerPalette::default();
    };
    let Some(base) = dominant_color(&pixbuf) else {
        return BannerPalette::default();
    };

    BannerPalette {
        start: mix(base, Rgb::new(1.0, 1.0, 1.0), 0.20),
        end: mix(base, Rgb::new(0.0, 0.0, 0.0), 0.72),
    }
}

/// Extrae el color dominante del póster priorizando la saturación y el brillo.
fn dominant_color(pixbuf: &GtkPixbuf) -> Option<Rgb> {
    let width = pixbuf.width() as usize;
    let height = pixbuf.height() as usize;
    let channels = pixbuf.n_channels() as usize;
    let rowstride = pixbuf.rowstride() as usize;
    let pixels = pixbuf.pixel_bytes()?;

    let mut samples: Vec<(f64, Rgb)> = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            let offset = y * rowstride + x * channels;
            let Some(pixel) = pixels.get(offset..offset + 3) else {
                continue;
            };
            let color = Rgb::new(
                f64::from(pixel[0]) / 255.0,
                f64::from(pixel[1]) / 255.0,
                f64::from(pixel[2]) / 255.0,
            );
            let max = color.r.max(color.g).max(color.b);
            let min = color.r.min(color.g).min(color.b);
            if max < 0.08 {
                continue;
            }
            let saturation = (max - min) / max;
            let score = saturation * 0.75 + max * 0.25;
            samples.push((score, color));
        }
    }

    if samples.is_empty() {
        return None;
    }

    samples.sort_by(|a, b| b.0.total_cmp(&a.0));
    let top = &samples[..samples.len().min(16)];
    let count = top.len() as f64;

    Some(Rgb::new(
        top.iter().map(|(_, color)| color.r).sum::<f64>() / count,
        top.iter().map(|(_, color)| color.g).sum::<f64>() / count,
        top.iter().map(|(_, color)| color.b).sum::<f64>() / count,
    ))
}

/// Mezcla dos colores: `t = 0.0` devuelve `a` y `t = 1.0` devuelve `b`.
fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    Rgb::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
    )
}

fn escape_markup(text: &str) -> String {
    gtk4::glib::markup_escape_text(text).to_string()
}
