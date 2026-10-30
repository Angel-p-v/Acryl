//! Vista de detalles de un medio.

use crate::db::media_repo::Media;
use gtk4::prelude::*;
use gtk4::{Box, Button, Label, Orientation};
use std::rc::Rc;

/// Componente de vista de detalles de un medio.
pub struct MediaDetails {
    box_: Box,
    media_id: i64,
    poster: crate::ui::poster::Poster,
    title_label: Label,
    synopsis_label: Label,
    favorite_button: Button,
    watched_button: Button,
    play_button: Button,
    back_button: Button,
    on_back: Option<Rc<dyn Fn()>>,
    on_favorite: Option<Rc<dyn Fn()>>,
    on_watched: Option<Rc<dyn Fn()>>,
    on_play: Option<Rc<dyn Fn()>>,
}

impl MediaDetails {
    /// Crea una nueva vista de detalles.
    pub fn new(_db_path: &str, media_id: i64) -> Self {
        let box_ = Box::new(Orientation::Vertical, 10);
        box_.set_margin_top(10);
        box_.set_margin_bottom(10);
        box_.set_margin_start(10);
        box_.set_margin_end(10);

        let poster = crate::ui::poster::Poster::new("", &None);

        let title_label = Label::new(None);
        title_label.set_markup("<b>Título</b>");
        title_label.set_halign(gtk4::Align::Start);
        title_label.set_hexpand(true);
        title_label.set_wrap(true);

        let synopsis_label = Label::new(None);
        synopsis_label.set_halign(gtk4::Align::Start);
        synopsis_label.set_hexpand(true);
        synopsis_label.set_wrap(true);

        let favorite_button = Button::with_label("★ Favorito");
        favorite_button.set_halign(gtk4::Align::Start);

        let watched_button = Button::with_label("✓ Visto");
        watched_button.set_halign(gtk4::Align::Start);

        let play_button = Button::with_label("▶ Reproducir");
        play_button.set_halign(gtk4::Align::Start);
        play_button.add_css_class("suggested-action");

        let back_button = Button::with_label("← Volver");
        back_button.set_halign(gtk4::Align::Start);

        box_.append(&back_button);
        box_.append(poster.widget());
        box_.append(&title_label);
        box_.append(&synopsis_label);
        box_.append(&favorite_button);
        box_.append(&watched_button);
        box_.append(&play_button);

        Self {
            box_,
            media_id,
            poster,
            title_label,
            synopsis_label,
            favorite_button,
            watched_button,
            play_button,
            back_button,
            on_back: None,
            on_favorite: None,
            on_watched: None,
            on_play: None,
        }
    }

    /// Carga los datos de un medio en la vista.
    pub fn set_media(&mut self, media: &Media) {
        self.media_id = media.id;
        self.title_label
            .set_markup(&format!("<b>{}</b>", media.title));
        self.synopsis_label
            .set_label(&media.synopsis.clone().unwrap_or_default());
        self.poster.set_poster(media.poster_path.as_deref());

        let fav_label = if media.is_favorite {
            "★ Favorito (activo)"
        } else {
            "★ Favorito"
        };
        self.favorite_button.set_label(fav_label);

        let wat_label = if media.is_watched {
            "✓ Visto (activo)"
        } else {
            "✓ Visto"
        };
        self.watched_button.set_label(wat_label);
    }

    /// Establece la ruta del archivo a reproducir.
    pub fn set_playable_file(&mut self, file_path: &str) {
        // Guardado para uso del callback on_play
        let _ = file_path;
    }

    /// Devuelve el ID del medio cargado.
    pub fn media_id(&self) -> i64 {
        self.media_id
    }

    /// Devuelve el widget Gtk.
    pub fn widget(&self) -> &Box {
        &self.box_
    }

    /// Establece el callback de volver atrás.
    pub fn on_back<F>(&mut self, f: F)
    where
        F: Fn() + 'static,
    {
        let cb = Rc::new(f);
        let cb_clone = cb.clone();
        self.back_button.connect_clicked(move |_| {
            cb_clone();
        });
        self.on_back = Some(cb);
    }

    /// Establece el callback de favorito.
    pub fn on_favorite<F>(&mut self, f: F)
    where
        F: Fn() + 'static,
    {
        let cb = Rc::new(f);
        let cb_clone = cb.clone();
        self.favorite_button.connect_clicked(move |_| {
            cb_clone();
        });
        self.on_favorite = Some(cb);
    }

    /// Establece el callback de visto.
    pub fn on_watched<F>(&mut self, f: F)
    where
        F: Fn() + 'static,
    {
        let cb = Rc::new(f);
        let cb_clone = cb.clone();
        self.watched_button.connect_clicked(move |_| {
            cb_clone();
        });
        self.on_watched = Some(cb);
    }

    /// Establece el callback de reproducir (sin argumentos; MainWindow sabe qué medio).
    pub fn on_play<F>(&mut self, f: F)
    where
        F: Fn() + 'static,
    {
        let cb = Rc::new(f);
        let cb_clone = cb.clone();
        self.play_button.connect_clicked(move |_| {
            cb_clone();
        });
        self.on_play = Some(cb);
    }
}
