//! Componente Poster: renderiza pósters con gdk-pixbuf.
//! Carga desde caché local si está disponible, de lo contrario muestra placeholder.

use crate::core::poster_service::cached_path;
use gtk4::gdk_pixbuf::Pixbuf as GtkPixbuf;
use gtk4::prelude::*;
use gtk4::{Box, Image, Label, Orientation};

pub struct Poster {
    box_: Box,
    image: Image,
    label: Label,
    title: String,
}

impl Poster {
    /// Crea un nuevo póster con título y URL opcional.
    /// Si la URL está en caché, se muestra inmediatamente.
    pub fn new(title: &str, poster_url: Option<&str>) -> Self {
        let box_ = Box::new(Orientation::Vertical, 5);
        box_.set_margin_top(5);
        box_.set_margin_bottom(5);
        box_.set_margin_start(5);
        box_.set_margin_end(5);

        let image = Image::new();
        image.set_size_request(150, 200);
        image.set_halign(gtk4::Align::Center);

        let label = Label::new(Some(title));
        label.set_wrap(true);
        label.set_max_width_chars(15);
        label.set_halign(gtk4::Align::Center);

        box_.append(&image);
        box_.append(&label);

        let mut poster = Self {
            box_,
            image,
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
        #[allow(clippy::collapsible_if)]
        if let Some(cache_path) = cached_path(poster_url) {
            if let Ok(pixbuf) = GtkPixbuf::from_file_at_scale(&cache_path, 150, 200, true) {
                self.image.set_from_pixbuf(Some(&pixbuf));
            }
        }
        // Si no está en caché, se queda vacío (placeholder). La descarga
        // asíncrona se implementará en un hito posterior.
    }
}

impl Default for Poster {
    fn default() -> Self {
        Self::new("", None)
    }
}
