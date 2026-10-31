//! Componentes de imagen: pósters y banner hero de la vista de detalles.
//!
//! El banner sigue el diseño de Crunchyroll:
//! - Ancho completo, altura fija (450px).
//! - Degradado oscuro en la parte inferior para que el texto sea legible.
//! - Título y subtítulo superpuestos a la imagen.
//! - Si no hay banner, se dibuja un degradado con los colores dominantes del póster.

use crate::core::poster_service::cached_path;
use crate::ui::theme;
use gtk4::gdk_pixbuf::Pixbuf as GtkPixbuf;
use gtk4::prelude::*;
use gtk4::{Box, DrawingArea, Image, Label, Orientation, Overflow, Overlay, Picture};
use std::cell::RefCell;
use std::f64::consts::{FRAC_PI_2, PI, TAU};
use std::rc::Rc;
use std::sync::Once;

/// Altura del banner hero de la vista de detalles (Crunchyroll usa banners altos).
const BANNER_HEIGHT: i32 = 520;

/// Tamaño máximo con el que se carga la imagen del banner.
const BANNER_IMAGE_MAX_WIDTH: i32 = 1920;
const BANNER_IMAGE_MAX_HEIGHT: i32 = 1080;

/// Radio de las esquinas del banner.
const BANNER_RADIUS: f64 = 0.0; // Crunchyroll usa esquinas cuadradas en el banner hero

/// Resolución para muestrar la paleta del póster.
const PALETTE_SAMPLE_SIZE: i32 = 24;

static INSTALL_POSTER_CSS: Once = Once::new();

/// Instala los estilos de los pósters y el banner hero.
pub fn install_poster_css() {
    INSTALL_POSTER_CSS.call_once(|| {
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
             .flat-button:hover {{ color: {}; background-color: {}; border-radius: {}; }}\n\
             .details-header {{ background-color: {}; border-radius: {}; padding: 24px; margin-bottom: 24px; }}\n\
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
             .episode-play-btn:hover {{ background-color: {}; }}\n\
             .season-grid-section {{ margin-top: 28px; }}\n\
             .season-grid-title {{ font-weight: 700; font-size: 1.2em; color: {}; margin-bottom: 12px; margin-left: 6px; }}\n\
             .hero-action-btn {{ min-height: 40px; padding: 0 16px; font-weight: 700; border-radius: {}; }}\n\
             .hero-action-btn.primary {{ background-color: {}; color: {}; border: none; }}\n\
             .hero-action-btn.primary:hover {{ background-color: {}; }}\n\
             .hero-action-btn.secondary {{ background-color: {}; color: {}; border: 1px solid {}; }}\n\
             .hero-action-btn.secondary:hover {{ border-color: {}; background-color: {}; }}\n\
             .season-dropdown {{ min-width: 180px; }}\n\
             .rating-stars {{ color: {}; }}\n\
             .rating-stars .filled {{ color: {}; }}\n",
            theme::colors::CARD, theme::colors::BORDER, theme::radius::LARGE, theme::transition::NORMAL,
            theme::colors::SURFACE, theme::colors::BORDER_HOVER,
            theme::colors::SURFACE, theme::colors::ACCENT,
            theme::colors::CARD, theme::radius::MEDIUM,
            theme::colors::TEXT_PRIMARY,
            theme::colors::TEXT_SECONDARY,
            theme::radius::LARGE,
            theme::colors::TEXT_PRIMARY,
            theme::colors::TEXT_SECONDARY,
            theme::colors::CARD, theme::colors::BORDER, theme::radius::MEDIUM, theme::transition::FAST,
            theme::colors::SURFACE, theme::colors::BORDER_HOVER,
            theme::colors::SURFACE, theme::colors::ACCENT,
            theme::colors::TEXT_PRIMARY,
            theme::colors::ACCENT, theme::colors::BTN_PRIMARY_TEXT, theme::radius::SMALL,
            theme::colors::ACCENT_HOVER,
            theme::colors::TEXT_SECONDARY,
            theme::colors::TEXT_PRIMARY, theme::colors::SURFACE, theme::radius::SMALL,
            theme::colors::SURFACE, theme::radius::LARGE,
            theme::colors::TEXT_SECONDARY,
            theme::colors::ACCENT,
            theme::colors::CARD, theme::colors::BORDER, theme::radius::MEDIUM, theme::transition::FAST,
            theme::colors::SURFACE, theme::colors::BORDER_HOVER,
            theme::colors::SURFACE, theme::colors::ACCENT,
            theme::radius::SMALL,
            theme::colors::TEXT_PRIMARY,
            theme::colors::TEXT_SECONDARY,
            theme::colors::ACCENT, theme::colors::BTN_PRIMARY_TEXT, theme::radius::SMALL,
            theme::colors::ACCENT_HOVER,
            theme::colors::TEXT_PRIMARY,
            theme::radius::SMALL,
            theme::colors::ACCENT, theme::colors::BTN_PRIMARY_TEXT,
            theme::colors::ACCENT_HOVER,
            theme::colors::SURFACE, theme::colors::TEXT_PRIMARY, theme::colors::BORDER,
            theme::colors::BORDER_HOVER, theme::colors::CARD,
            theme::colors::TEXT_MUTED,
            theme::colors::ACCENT,
        );
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
/// oscuro en la parte inferior. Si no hay banner, dibuja un degradado a partir
/// de los colores del póster. El título y el subtítulo se superponen.
///
/// Debajo del banner se puede solapar el póster vertical (Crunchyroll lo hace
/// así en las páginas de serie/anime).
pub struct Banner {
    overlay: Overlay,
    picture: Picture,
    drawing: DrawingArea,
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
        overlay.set_vexpand(false);
        overlay.set_valign(gtk4::Align::Start);
        overlay.set_size_request(-1, BANNER_HEIGHT);
        overlay.set_overflow(Overflow::Hidden);
        overlay.add_css_class("banner");

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
            overlay,
            picture,
            drawing,
            palette,
            title_label,
            subtitle_label,
        }
    }

    pub fn widget(&self) -> &Overlay {
        &self.overlay
    }

    pub fn set_info(&mut self, title: &str, subtitle: &str) {
        self.title_label
            .set_markup(&format!("<b>{}</b>", escape_markup(title)));
        self.subtitle_label.set_label(subtitle);
        self.subtitle_label.set_visible(!subtitle.is_empty());
    }

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
        start: mix(base, Rgb::new(1.0, 1.0, 1.0), 0.15),
        end: mix(base, Rgb::new(0.0, 0.0, 0.0), 0.78),
    }
}

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
