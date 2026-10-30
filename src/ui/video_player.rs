//! Componente reproductor de vídeo con libmpv.

use crate::core::player::PlayerService;
use gtk4::prelude::*;
use gtk4::{Box, Button, Entry, Label, Orientation};

/// Componente reproductor de vídeo.
pub struct VideoPlayer {
    box_: Box,
    path_entry: Entry,
    play_button: Button,
    #[allow(dead_code)]
    stop_button: Button,
    status_label: Label,
    db_path: String,
}

impl VideoPlayer {
    /// Crea un nuevo reproductor.
    pub fn new(db_path: &str) -> Self {
        let box_ = Box::new(Orientation::Vertical, 10);
        box_.set_margin_top(10);
        box_.set_margin_bottom(10);
        box_.set_margin_start(10);
        box_.set_margin_end(10);

        let path_entry = Entry::new();
        path_entry.set_placeholder_text(Some("Ruta del archivo de vídeo..."));
        path_entry.set_hexpand(true);
        box_.append(&path_entry);

        let play_button = Button::with_label("Abrir");
        box_.append(&play_button);

        let stop_button = Button::with_label("Detener");
        box_.append(&stop_button);

        let status_label = Label::new(None);
        status_label.set_halign(gtk4::Align::Start);
        box_.append(&status_label);

        Self {
            box_,
            path_entry,
            play_button,
            stop_button,
            status_label,
            db_path: db_path.to_string(),
        }
    }

    /// Devuelve el widget Gtk.
    pub fn widget(&self) -> &Box {
        &self.box_
    }

    /// Configura los manejadores de eventos.
    pub fn setup(&mut self, media_id: i64) {
        let db_path = self.db_path.clone();
        let play_button = self.play_button.clone();
        let path_entry = self.path_entry.clone();
        let status_label = self.status_label.clone();

        play_button.connect_clicked(move |_| {
            let path = path_entry.text();
            if path.is_empty() {
                status_label.set_text("Ingrese una ruta de archivo");
                return;
            }

            let db_path = db_path.clone();
            let status_label = status_label.clone();
            let media_id = media_id;
            let path = path.to_string();

            let result = PlayerService::open(&db_path, media_id, &path);
            match result {
                Ok(_) => {
                    status_label.set_text(&format!("Reproduciendo: {}", path));
                }
                Err(e) => {
                    status_label.set_text(&format!("Error: {}", e));
                }
            }
        });
    }
}
