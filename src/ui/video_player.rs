//! Componente reproductor de vídeo que delega en `mpv` (proceso externo).
//!
//! La reproducción real se ejecuta en `mpv` como proceso externo (véase
//! `crate::core::player::MpvPlayer`). Esta vista solo proporciona los
//! controles GTK4 y delega las acciones al proceso `mpv` a través de su
//! socket IPC.

use crate::core::player::MpvPlayer;
use gtk4::prelude::*;
use gtk4::{Adjustment, Box, Button, Label, Orientation, Scale};
use std::cell::RefCell;
use std::rc::Rc;

/// Componente reproductor de vídeo.
pub struct VideoPlayer {
    box_: Box,
    play_button: Button,
    pause_button: Button,
    stop_button: Button,
    seek_scale: Scale,
    volume_scale: Scale,
    status_label: Label,
    time_label: Label,
    player: MpvPlayer,
    current_path: Option<String>,
    is_playing: bool,
}

impl VideoPlayer {
    /// Crea un nuevo reproductor.
    pub fn new() -> Self {
        let box_ = Box::new(Orientation::Vertical, 10);
        box_.set_margin_top(10);
        box_.set_margin_bottom(10);
        box_.set_margin_start(10);
        box_.set_margin_end(10);

        // Lugar reservado para el vídeo (mpv renderiza en su propia ventana).
        let placeholder = Label::new(Some("El vídeo se reproduce en una ventana de mpv"));
        placeholder.set_hexpand(true);
        placeholder.set_vexpand(true);
        placeholder.set_valign(gtk4::Align::Center);
        box_.append(&placeholder);

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
            play_button,
            pause_button,
            stop_button,
            seek_scale,
            volume_scale,
            status_label,
            time_label,
            player: MpvPlayer::new(),
            current_path: None,
            is_playing: false,
        }
    }

    /// Devuelve el widget Gtk.
    pub fn widget(&self) -> &Box {
        &self.box_
    }

    /// Carga y reproduce un archivo de vídeo mediante `mpv`.
    pub fn load_file(&mut self, file_path: &str) {
        println!("[VIDEO] load_file llamado con: {}", file_path);
        self.stop();

        self.current_path = Some(file_path.to_string());

        if let Err(e) = self.player.load_file(file_path) {
            eprintln!("Error cargando archivo en mpv: {}", e);
            self.status_label
                .set_text(&format!("Error al reproducir: {}", e));
            self.is_playing = false;
            self.current_path = None;
            return;
        }

        self.is_playing = true;
        self.status_label
            .set_text(&format!("Reproduciendo (mpv): {}", file_path));
        println!("[VIDEO] mpv iniciado, reproduciendo: {}", file_path);
        self.update_time_labels();
    }

    /// Detiene la reproducción y cierra el proceso mpv.
    pub fn stop(&mut self) {
        if let Err(e) = self.player.stop() {
            eprintln!("Error deteniendo mpv: {}", e);
        }
        self.current_path = None;
        self.is_playing = false;
        self.status_label.set_text("Detenido");
        self.time_label.set_text("");
        self.seek_scale.set_value(0.0);
    }

    /// Reanuda la reproducción.
    pub fn play(&mut self) {
        if let Err(e) = self.player.play() {
            eprintln!("Error reanudando mpv: {}", e);
        }
        self.is_playing = true;
        if let Some(path) = &self.current_path {
            self.status_label
                .set_text(&format!("Reproduciendo (mpv): {}", path));
        }
    }

    /// Pausa la reproducción.
    pub fn pause(&mut self) {
        if let Err(e) = self.player.pause() {
            eprintln!("Error pausando mpv: {}", e);
        }
        self.is_playing = false;
        self.status_label.set_text("Pausado");
    }

    /// Busca a una fracción de la duración (0.0 = inicio, 1.0 = final).
    pub fn seek(&mut self, fraction: f64) {
        if let Err(e) = self.player.seek_fraction(fraction) {
            eprintln!("Error haciendo seek en mpv: {}", e);
            return;
        }
        self.seek_scale.set_value(fraction.clamp(0.0, 1.0) * 100.0);
    }

    /// Ajusta el volumen (0.0 = silencio, 1.0 = máximo).
    pub fn set_volume(&mut self, volume: f64) {
        if let Err(e) = self.player.set_volume(volume) {
            eprintln!("Error ajustando volumen en mpv: {}", e);
        }
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
        let duration_secs = self.player.duration_seconds().unwrap_or(0.0).max(0.0);
        let current_secs = (self.seek_scale.value() / 100.0 * duration_secs).round() as i64;
        let duration_secs_i = duration_secs as i64;

        let current_str = format_time(current_secs);
        let duration_str = format_time(duration_secs_i);
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
