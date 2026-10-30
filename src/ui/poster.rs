//! Componente Poster: renderiza pósters con gdk-pixbuf.
//! Carga desde caché local si está disponible, de lo contrario muestra placeholder.

use crate::core::poster_service::cached_path;
use gtk4::gdk_pixbuf::Pixbuf as GtkPixbuf;
use gtk4::prelude::*;
use gtk4::{Box, Image, Label, Orientation, Picture, Stack};
use std::sync::Once;

/// Tamaño del póster en la vista de detalles (proporción 2:3).
const DETAIL_POSTER_WIDTH: i32 = 240;
const DETAIL_POSTER_HEIGHT: i32 = 360;

/// Estilos compartidos por las tarjetas y los pósters.
static INSTALL_POSTER_CSS: Once = Once::new();

/// Instala una sola vez los estilos de las tarjetas y los pósters.
pub fn install_poster_css() {
    INSTALL_POSTER_CSS.call_once(|| {
        let provider = gtk4::CssProvider::new();
        provider.load_from_data(
            ".poster-card { \
                 background-color: rgba(128, 128, 128, 0.10); \
                 border-radius: 14px; \
                 padding: 8px; \
             } \
             .poster-card:hover { \
                 background-color: rgba(128, 128, 128, 0.18); \
             } \
             .poster-placeholder { \
                 background-color: rgba(128, 128, 128, 0.12); \
                 border-radius: 12px; \
             } \
             .poster-title { font-weight: 600; } \
             .poster-year { font-size: 0.9em; opacity: 0.7; }",
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
/// `GtkPicture` (con ajuste `contain` por defecto) para mostrar el póster completo.
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

pub struct Poster {
    box_: Box,
    stack: Stack,
    picture: Picture,
    label: Label,
    title: String,
}

impl Poster {
    /// Crea un nuevo póster con título y URL opcional.
    /// Si la URL está en caché, se muestra inmediatamente.
    pub fn new(title: &str, poster_url: Option<&str>) -> Self {
        install_poster_css();

        let box_ = Box::new(Orientation::Vertical, 5);
        box_.set_margin_top(5);
        box_.set_margin_bottom(5);
        box_.set_margin_start(5);
        box_.set_margin_end(5);

        let picture = Picture::new();
        picture.set_hexpand(false);
        picture.set_vexpand(false);
        picture.set_halign(gtk4::Align::Center);
        picture.set_valign(gtk4::Align::Center);
        picture.set_alternative_text(Some("Póster"));

        let placeholder = placeholder_widget(DETAIL_POSTER_WIDTH, DETAIL_POSTER_HEIGHT);

        let stack = Stack::new();
        stack.set_size_request(DETAIL_POSTER_WIDTH, DETAIL_POSTER_HEIGHT);
        stack.set_halign(gtk4::Align::Center);
        stack.set_valign(gtk4::Align::Center);
        stack.add_named(&picture, Some("picture"));
        stack.add_named(&placeholder, Some("placeholder"));
        stack.set_visible_child_name("placeholder");

        let label = Label::new(Some(title));
        label.set_wrap(true);
        label.set_max_width_chars(24);
        label.set_halign(gtk4::Align::Center);
        label.add_css_class("poster-title");

        box_.append(&stack);
        box_.append(&label);

        let mut poster = Self {
            box_,
            stack,
            picture,
            label,
            title: title.to_string(),
        };

        if let Some(url) = poster_url {
            poster.set_poster_url(url);
        }

        poster
    }

    /// Devuelve el widget Gtk.
    pub fn widget(&self) -> &Box {
        &self.box_
    }

    /// Establece el título del póster.
    pub fn set_title(&mut self, title: &str) {
        self.label.set_label(title);
        self.title = title.to_string();
    }

    /// Establece la URL del póster y lo carga desde caché si está disponible.
    pub fn set_poster_url(&mut self, poster_url: &str) {
        let pixbuf = cached_path(poster_url).and_then(|cache_path| {
            GtkPixbuf::from_file_at_scale(
                &cache_path,
                DETAIL_POSTER_WIDTH,
                DETAIL_POSTER_HEIGHT,
                true,
            )
            .ok()
        });

        match pixbuf {
            Some(pixbuf) => {
                let texture = gtk4::gdk::Texture::for_pixbuf(&pixbuf);
                self.picture.set_paintable(Some(&texture));
                self.stack.set_visible_child_name("picture");
            }
            None => {
                self.stack.set_visible_child_name("placeholder");
            }
        }
    }
}

impl Default for Poster {
    fn default() -> Self {
        Self::new("", None)
    }
}
