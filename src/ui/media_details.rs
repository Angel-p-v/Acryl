//! Vista de detalles de un medio.

use crate::db::{MediaRepository, close_db, open_db};
use gtk4::prelude::*;
use gtk4::{Box, Button, Label, Orientation};
use std::rc::Rc;

use crate::ui::poster::Poster;

/// Componente de vista de detalles de un medio.
pub struct MediaDetails {
    box_: Box,
    media_id: i64,
    db_path: String,
    poster: Poster,
    title_label: Label,
    synopsis_label: Label,
    on_back: Option<Rc<dyn Fn()>>,
    on_play: Option<Rc<dyn Fn(String)>>,
}

impl MediaDetails {
    /// Crea una nueva vista de detalles.
    pub fn new(db_path: &str, media_id: i64) -> Self {
        let box_ = Box::new(Orientation::Vertical, 10);
        box_.set_margin_top(10);
        box_.set_margin_bottom(10);
        box_.set_margin_start(10);
        box_.set_margin_end(10);

        let poster = Poster::new("", &None);

        let title_label = Label::new(None);
        title_label.set_markup("<b>Título</b>");
        title_label.set_halign(gtk4::Align::Start);
        title_label.set_hexpand(true);
        title_label.set_wrap(true);

        let synopsis_label = Label::new(None);
        synopsis_label.set_halign(gtk4::Align::Start);
        synopsis_label.set_hexpand(true);
        synopsis_label.set_wrap(true);

        let back_button = Button::with_label("← Volver");
        back_button.set_halign(gtk4::Align::Start);

        box_.append(&back_button);
        box_.append(poster.widget());
        box_.append(&title_label);
        box_.append(&synopsis_label);

        Self {
            box_,
            media_id,
            db_path: db_path.to_string(),
            poster,
            title_label,
            synopsis_label,
            on_back: None,
            on_play: None,
        }
    }

    /// Devuelve el widget Gtk.
    pub fn widget(&self) -> &Box {
        &self.box_
    }

    /// Carga los detalles del medio.
    pub fn load(&mut self) {
        let mut conn = match open_db(&self.db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error abriendo BD: {}", e);
                return;
            }
        };
        let mut repo = MediaRepository::new(&mut conn);
        let details = repo.get_details(self.media_id).unwrap_or(None);
        close_db(conn);

        if let Some(details) = details {
            self.title_label
                .set_markup(&format!("<b>{}</b>", details.media.title));
            self.synopsis_label
                .set_label(&details.media.synopsis.unwrap_or_default());
            self.poster.set_poster(details.media.poster_path.as_deref());
        }
    }

    /// Establece el callback de volver atrás.
    pub fn on_back<F>(&mut self, f: F)
    where
        F: Fn() + 'static,
    {
        self.on_back = Some(Rc::new(f));
    }

    /// Establece el callback de reproducir.
    pub fn on_play<F>(&mut self, f: F)
    where
        F: Fn(String) + 'static,
    {
        self.on_play = Some(Rc::new(f));
    }
}
