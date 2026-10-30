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
    db_path: String,
    current_path: Option<String>,
}

impl VideoPlayer {
    /// Crea un nuevo reproductor.
    pub fn new(db_path: &str) -> Self {
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
            db_path: db_path.to_string(),
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
        // MediaFile implementa gdk::Paintable, se puede mostrar en un Picture
        self.picture.set_paintable(Some(&media_file));

        // GTK comienza a cargar y reproducir automáticamente al asignar el archivo
        self.status_label
            .set_text(&format!("Cargando: {}", file_path));

        // Registrar reproducción en la BD
        let db_path = self.db_path.clone();
        let path = file_path.to_string();
        // Nota: PlayerService::open requiere media_id; aquí se deja para integración posterior
        let _ = db_path;
        let _ = path;
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

        play_button.connect_clicked({
            let status_label = self.status_label.clone();
            move |_| {
                status_label.set_text("Play solicitado (controles por implementar)");
            }
        });

        pause_button.connect_clicked({
            let status_label = self.status_label.clone();
            move |_| {
                status_label.set_text("Pausa solicitada (controles por implementar)");
            }
        });

        stop_button.connect_clicked({
            let status_label = self.status_label.clone();
            move |_| {
                status_label.set_text("Stop solicitado (controles por implementar)");
            }
        });
    }
}
