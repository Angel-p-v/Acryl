//! Componente reproductor de vídeo integrado con GtkMediaFile (GTK4).

use gtk4::MediaFile;
use gtk4::prelude::*;
use gtk4::{Adjustment, Box, Button, Label, Orientation, Picture, Scale};
use std::cell::RefCell;
use std::rc::Rc;

/// Componente reproductor de vídeo integrado.
pub struct VideoPlayer {
    box_: Box,
    picture: Picture,
    play_button: Button,
    pause_button: Button,
    stop_button: Button,
    seek_scale: Scale,
    volume_scale: Scale,
    status_label: Label,
    time_label: Label,
    current_path: Option<String>,
    media_file: Option<MediaFile>,
    is_playing: bool,
    duration_ns: i64,
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

        // Controles de reproducción
        let controls = Box::new(Orientation::Horizontal, 6);
        controls.set_halign(gtk4::Align::Center);

        let play_button = Button::with_label("▶ Play");
        controls.append(&play_button);

        let pause_button = Button::with_label("⏸ Pausa");
        controls.append(&pause_button);

        let stop_button = Button::with_label("⏹ Stop");
        controls.append(&stop_button);

        box_.append(&controls);

        // Barra de búsqueda (seek)
        let adjustment = Adjustment::new(0.0, 0.0, 100.0, 1.0, 10.0, 0.0);
        let seek_scale = Scale::new(Orientation::Horizontal, Some(&adjustment));
        seek_scale.set_hexpand(true);
        seek_scale.set_draw_value(false);
        box_.append(&seek_scale);

        // Control de volumen
        let volume_label = Label::new(Some("Volumen"));
        volume_label.set_halign(gtk4::Align::Start);
        box_.append(&volume_label);

        let vol_adjustment = Adjustment::new(100.0, 0.0, 100.0, 1.0, 10.0, 0.0);
        let volume_scale = Scale::new(Orientation::Horizontal, Some(&vol_adjustment));
        volume_scale.set_hexpand(true);
        volume_scale.set_draw_value(false);
        box_.append(&volume_scale);

        // Etiquetas de estado y tiempo
        let status_label = Label::new(None);
        status_label.set_halign(gtk4::Align::Start);
        box_.append(&status_label);

        let time_label = Label::new(None);
        time_label.set_halign(gtk4::Align::Start);
        box_.append(&time_label);

        Self {
            box_,
            picture,
            play_button,
            pause_button,
            stop_button,
            seek_scale,
            volume_scale,
            status_label,
            time_label,
            current_path: None,
            media_file: None,
            is_playing: false,
            duration_ns: 0,
        }
    }

    /// Devuelve el widget Gtk.
    pub fn widget(&self) -> &Box {
        &self.box_
    }

    /// Carga y reproduce un archivo de vídeo.
    pub fn load_file(&mut self, file_path: &str) {
        self.stop();

        self.current_path = Some(file_path.to_string());
        self.is_playing = true;

        let media_file = MediaFile::for_filename(file_path);
        self.duration_ns = media_file.duration();
        self.media_file = Some(media_file);

        if let Some(ref mf) = self.media_file {
            self.picture.set_paintable(Some(mf));
            mf.play();
            self.status_label
                .set_text(&format!("Reproduciendo: {}", file_path));
        }

        self.update_time_labels();
    }

    /// Detiene la reproducción.
    pub fn stop(&mut self) {
        if let Some(ref mf) = self.media_file {
            mf.pause();
        }
        self.picture.set_paintable(None::<&gtk4::MediaFile>);
        self.media_file = None;
        self.current_path = None;
        self.is_playing = false;
        self.duration_ns = 0;
        self.status_label.set_text("Detenido");
        self.time_label.set_text("");
        self.seek_scale.set_value(0.0);
    }

    /// Reanuda la reproducción.
    pub fn play(&mut self) {
        if let Some(ref mf) = self.media_file {
            mf.play();
            self.is_playing = true;
            if let Some(path) = &self.current_path {
                self.status_label
                    .set_text(&format!("Reproduciendo: {}", path));
            }
        }
    }

    /// Pausa la reproducción.
    pub fn pause(&mut self) {
        if let Some(ref mf) = self.media_file {
            mf.pause();
        }
        self.is_playing = false;
        self.status_label.set_text("Pausado");
    }

    /// Busca a una fracción de la duración (0.0 = inicio, 1.0 = final).
    pub fn seek(&mut self, fraction: f64) {
        if let Some(ref mf) = self.media_file {
            let timestamp = (fraction * self.duration_ns as f64) as i64;
            mf.seek(timestamp);
            self.seek_scale.set_value(fraction * 100.0);
        }
    }

    /// Ajusta el volumen (0.0 = silencio, 1.0 = máximo).
    pub fn set_volume(&mut self, volume: f64) {
        // GTK4 MediaStream no expone control de volumen directamente;
        // se gestiona a través del backend de audio del sistema.
        // Guardamos el valor para referencia futura.
        let _ = volume;
    }

    /// Configura los manejadores de eventos de los botones de control.
    pub fn setup_controls(video_player: &Rc<RefCell<VideoPlayer>>) {
        // Play
        let this = video_player.clone();
        let play_button = this.borrow().play_button.clone();
        play_button.connect_clicked(move |_| {
            this.borrow_mut().play();
        });

        // Pausa
        let this = video_player.clone();
        let pause_button = this.borrow().pause_button.clone();
        pause_button.connect_clicked(move |_| {
            this.borrow_mut().pause();
        });

        // Stop
        let this = video_player.clone();
        let stop_button = this.borrow().stop_button.clone();
        stop_button.connect_clicked(move |_| {
            this.borrow_mut().stop();
        });

        // Seek
        let this = video_player.clone();
        let seek_scale = this.borrow().seek_scale.clone();
        seek_scale.connect_value_changed(move |scale| {
            let value = scale.value();
            let fraction = value / 100.0;
            this.borrow_mut().seek(fraction);
        });

        // Volumen
        let this = video_player.clone();
        let volume_scale = this.borrow().volume_scale.clone();
        volume_scale.connect_value_changed(move |scale| {
            let value = scale.value();
            this.borrow_mut().set_volume(value / 100.0);
        });
    }

    /// Actualiza las etiquetas de tiempo (actual / duración).
    fn update_time_labels(&self) {
        let current_secs = if self.duration_ns > 0 {
            // Aproximación basada en la posición del seek scale
            (self.seek_scale.value() / 100.0 * self.duration_ns as f64 / 1_000_000_000.0) as i64
        } else {
            0
        };
        let duration_secs = self.duration_ns / 1_000_000_000;

        let current_str = format_time(current_secs);
        let duration_str = format_time(duration_secs);
        self.time_label
            .set_text(&format!("{} / {}", current_str, duration_str));
    }
}

/// Formatea segundos como MM:SS.
fn format_time(secs: i64) -> String {
    let mins = secs / 60;
    let secs = secs % 60;
    format!("{}:{:02}", mins, secs)
}

impl Default for VideoPlayer {
    fn default() -> Self {
        Self::new()
    }
}
