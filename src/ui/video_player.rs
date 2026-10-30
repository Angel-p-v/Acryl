//! Componente reproductor de vídeo integrado con GtkMediaFile (GTK4).

use gtk4::prelude::*;
use gtk4::{Box, Button, Label, Orientation, Picture};

/// Componente reproductor de vídeo integrado.
pub struct VideoPlayer {
    box_: Box,
    picture: Picture,
    play_button: Button,
    pause_button: Button,
    stop_button: Button,
    status_label: Label,
    current_path: Option<String>,
}

impl VideoPlayer {
    /// Crea un nuevo reproductor.
    pub fn new() -> Self {
        let box_ = Box::new(Orientation::Vertical, 10);
        box_.set_margin_top(10);
        box_.set_margin_bottom(10);
        box_.set_margin_start(10);
        box_.set_margin_end(10);

        let picture = Picture::new();
        picture.set_hexpand(true);
        picture.set_vexpand(true);
        picture.set_valign(gtk4::Align::Center);
        box_.append(&picture);

        let controls = Box::new(Orientation::Horizontal, 6);
        controls.set_halign(gtk4::Align::Center);

        let play_button = Button::with_label("▶ Play");
        controls.append(&play_button);

        let pause_button = Button::with_label("⏸ Pausa");
        controls.append(&pause_button);

        let stop_button = Button::with_label("⏹ Stop");
        controls.append(&stop_button);

        box_.append(&controls);

        let status_label = Label::new(None);
        status_label.set_halign(gtk4::Align::Start);
        box_.append(&status_label);

        Self {
            box_,
            picture,
            play_button,
            pause_button,
            stop_button,
            status_label,
            current_path: None,
        }
    }

    /// Devuelve el widget Gtk.
    pub fn widget(&self) -> &Box {
        &self.box_
    }

    /// Carga y reproduce un archivo de vídeo.
    pub fn load_file(&mut self, file_path: &str) {
        self.current_path = Some(file_path.to_string());

        let media_file = gtk4::MediaFile::for_filename(file_path);
        self.picture.set_paintable(Some(&media_file));

        self.status_label
            .set_text(&format!("Reproduciendo: {}", file_path));
    }

    /// Detiene la reproducción.
    pub fn stop(&mut self) {
        self.picture.set_paintable(None::<&gtk4::MediaFile>);
        self.current_path = None;
        self.status_label.set_text("Detenido");
    }

    /// Configura los manejadores de eventos de los botones de control.
    pub fn setup_controls(&mut self) {
        let play_button = self.play_button.clone();
        let pause_button = self.pause_button.clone();
        let stop_button = self.stop_button.clone();
        let picture = self.picture.clone();

        let status_label_play = self.status_label.clone();
        play_button.connect_clicked(move |_| {
            status_label_play.set_text("Play");
        });

        let status_label_pause = self.status_label.clone();
        pause_button.connect_clicked(move |_| {
            status_label_pause.set_text("Pausa");
        });

        let status_label_stop = self.status_label.clone();
        stop_button.connect_clicked(move |_| {
            picture.set_paintable(None::<&gtk4::MediaFile>);
            status_label_stop.set_text("Detenido");
        });
    }
}

impl Default for VideoPlayer {
    fn default() -> Self {
        Self::new()
    }
}
