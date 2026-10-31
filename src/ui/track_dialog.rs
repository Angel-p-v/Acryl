//! Selector de pistas de audio y subtítulos de un archivo.
//!
//! La ventana permite elegir la pista de audio, una pista de subtítulos
//! concreta o desactivarlos. La elección se guarda por archivo en
//! `media_files`, de modo que cada episodio mantiene su propia preferencia.
//! Si el archivo aún no tiene pistas analizadas, se ejecuta `ffprobe` en
//! segundo plano para no bloquear la interfaz.

use crate::core::library::ensure_media_streams;
use crate::db::media_repo::MediaStream;
use crate::db::{FileRepository, TrackSelection, close_db, open_db};
use gtk4::prelude::*;
use gtk4::{Box, Button, DropDown, Label, Orientation, Window};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc::{self, TryRecvError};
use std::time::Duration;

/// Abre el selector de pistas para `file_path`.
///
/// `on_saved` se invoca solo cuando la preferencia se ha guardado
/// correctamente. La ventana se cierra al guardar o al cancelar.
pub fn show_track_dialog(
    parent: &impl IsA<gtk4::Window>,
    db_path: &str,
    file_path: &str,
    on_saved: Rc<dyn Fn()>,
) {
    let window = Window::builder()
        .transient_for(parent)
        .modal(true)
        .title("Pistas de audio y subtítulos")
        .default_width(460)
        .build();

    let content = Box::new(Orientation::Vertical, 12);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(18);
    content.set_margin_end(18);

    let heading = Label::new(None);
    heading.set_markup("<b>Pistas de audio y subtítulos</b>");
    heading.set_halign(gtk4::Align::Start);
    content.append(&heading);

    let file_label = Label::new(Some(file_path));
    file_label.add_css_class("dim-label");
    file_label.set_halign(gtk4::Align::Start);
    file_label.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
    file_label.set_tooltip_text(Some(file_path));
    content.append(&file_label);

    let status_label = Label::new(None);
    status_label.set_halign(gtk4::Align::Start);
    status_label.set_wrap(true);
    status_label.add_css_class("dim-label");
    content.append(&status_label);

    let audio_label = Label::new(Some("Audio"));
    audio_label.set_halign(gtk4::Align::Start);
    audio_label.set_markup("<b>Audio</b>");
    content.append(&audio_label);

    let audio_dropdown = DropDown::from_strings(&["Automático (predeterminado)"]);
    audio_dropdown.set_hexpand(true);
    content.append(&audio_dropdown);

    let subtitle_label = Label::new(Some("Subtítulos"));
    subtitle_label.set_halign(gtk4::Align::Start);
    subtitle_label.set_markup("<b>Subtítulos</b>");
    content.append(&subtitle_label);

    let subtitle_dropdown =
        DropDown::from_strings(&["Automático (predeterminado)", "Ninguno (desactivados)"]);
    subtitle_dropdown.set_hexpand(true);
    content.append(&subtitle_dropdown);

    let buttons = Box::new(Orientation::Horizontal, 8);
    buttons.set_halign(gtk4::Align::End);
    buttons.set_margin_top(6);

    let cancel_button = Button::with_label("Cancelar");
    let save_button = Button::with_label("Guardar");
    save_button.add_css_class("suggested-action");
    buttons.append(&cancel_button);
    buttons.append(&save_button);
    content.append(&buttons);

    window.set_child(Some(&content));

    // Estado compartido entre la carga inicial, la detección en segundo plano
    // y el guardado.
    let audio_streams: Rc<RefCell<Vec<MediaStream>>> = Rc::new(RefCell::new(Vec::new()));
    let subtitle_streams: Rc<RefCell<Vec<MediaStream>>> = Rc::new(RefCell::new(Vec::new()));
    let selection = Rc::new(Cell::new(load_selection(db_path, file_path)));

    // Carga rápida desde la base de datos: si ya hay pistas analizadas, la
    // ventana aparece completa al instante.
    let (stored_audio, stored_subtitle) = load_streams(db_path, file_path);
    let has_stored_streams = !stored_audio.is_empty() || !stored_subtitle.is_empty();
    *audio_streams.borrow_mut() = stored_audio;
    *subtitle_streams.borrow_mut() = stored_subtitle;
    refresh_dropdowns(
        &audio_dropdown,
        &subtitle_dropdown,
        &audio_streams.borrow(),
        &subtitle_streams.borrow(),
        selection.get(),
    );

    if has_stored_streams {
        status_label.set_visible(false);
    } else {
        status_label.set_label("Detectando pistas con ffprobe…");
        detect_streams_in_background(
            db_path.to_string(),
            file_path.to_string(),
            status_label.clone(),
            audio_dropdown.clone(),
            subtitle_dropdown.clone(),
            audio_streams.clone(),
            subtitle_streams.clone(),
            selection.clone(),
        );
    }

    let window_for_cancel = window.clone();
    cancel_button.connect_clicked(move |_| {
        window_for_cancel.close();
    });

    let window_for_save = window.clone();
    let status_for_save = status_label.clone();
    let audio_for_save = audio_dropdown.clone();
    let subtitle_for_save = subtitle_dropdown.clone();
    let audio_streams_for_save = audio_streams.clone();
    let subtitle_streams_for_save = subtitle_streams.clone();
    let db_path_for_save = db_path.to_string();
    let file_path_for_save = file_path.to_string();
    save_button.connect_clicked(move |_| {
        let selection = current_selection(
            &audio_for_save,
            &subtitle_for_save,
            &audio_streams_for_save.borrow(),
            &subtitle_streams_for_save.borrow(),
        );

        match save_selection(&db_path_for_save, &file_path_for_save, selection) {
            Ok(true) => {
                on_saved();
                window_for_save.close();
            }
            Ok(false) => {
                status_for_save.set_label(
                    "No se encontró el archivo en la biblioteca; vuelve a indexar la carpeta.",
                );
                status_for_save.set_visible(true);
            }
            Err(error) => {
                status_for_save.set_label(&format!("No se pudo guardar la selección: {}", error));
                status_for_save.set_visible(true);
            }
        }
    });

    window.present();
}

/// Carga las pistas guardadas, separadas por tipo.
fn load_streams(db_path: &str, file_path: &str) -> (Vec<MediaStream>, Vec<MediaStream>) {
    let Ok(mut conn) = open_db(db_path) else {
        return (Vec::new(), Vec::new());
    };
    let streams = {
        let mut repo = crate::db::MediaRepository::new(&mut conn);
        repo.list_streams_by_file(file_path).unwrap_or_default()
    };
    close_db(conn);

    let mut audio = Vec::new();
    let mut subtitle = Vec::new();
    for stream in streams {
        match stream.stream_type.as_str() {
            "audio" => audio.push(stream),
            "subtitle" => subtitle.push(stream),
            _ => {}
        }
    }
    (audio, subtitle)
}

/// Carga la preferencia guardada; si falla, usa la selección automática.
fn load_selection(db_path: &str, file_path: &str) -> TrackSelection {
    let Ok(mut conn) = open_db(db_path) else {
        return TrackSelection::default();
    };
    let selection = {
        let mut repo = FileRepository::new(&mut conn);
        repo.track_selection(file_path).unwrap_or_default()
    };
    close_db(conn);
    selection
}

/// Guarda la preferencia. Devuelve `Ok(false)` si el archivo no existe en la
/// biblioteca, para poder avisar al usuario sin tratarlo como error interno.
fn save_selection(
    db_path: &str,
    file_path: &str,
    selection: TrackSelection,
) -> Result<bool, std::boxed::Box<dyn std::error::Error>> {
    let mut conn = open_db(db_path)?;
    let updated = {
        let mut repo = FileRepository::new(&mut conn);
        repo.set_track_selection(file_path, selection)?
    };
    close_db(conn);
    Ok(updated > 0)
}

/// Lanza `ffprobe` en un hilo y actualiza la ventana en el hilo de GTK.
#[allow(clippy::too_many_arguments)]
fn detect_streams_in_background(
    db_path: String,
    file_path: String,
    status_label: Label,
    audio_dropdown: DropDown,
    subtitle_dropdown: DropDown,
    audio_streams: Rc<RefCell<Vec<MediaStream>>>,
    subtitle_streams: Rc<RefCell<Vec<MediaStream>>>,
    selection: Rc<Cell<TrackSelection>>,
) {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = ensure_media_streams(&db_path, &file_path).map_err(|error| error.to_string());
        let _ = sender.send(result);
    });

    gtk4::glib::timeout_add_local(Duration::from_millis(100), move || {
        match receiver.try_recv() {
            Ok(Ok(streams)) => {
                let mut audio = Vec::new();
                let mut subtitle = Vec::new();
                for stream in streams {
                    match stream.stream_type.as_str() {
                        "audio" => audio.push(stream),
                        "subtitle" => subtitle.push(stream),
                        _ => {}
                    }
                }
                *audio_streams.borrow_mut() = audio;
                *subtitle_streams.borrow_mut() = subtitle;
                refresh_dropdowns(
                    &audio_dropdown,
                    &subtitle_dropdown,
                    &audio_streams.borrow(),
                    &subtitle_streams.borrow(),
                    selection.get(),
                );

                if audio_streams.borrow().is_empty() && subtitle_streams.borrow().is_empty() {
                    status_label.set_label(
                        "No se han detectado pistas de audio ni subtítulos en este archivo.",
                    );
                    status_label.set_visible(true);
                } else {
                    status_label.set_visible(false);
                }
                gtk4::glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                status_label.set_label(&format!("No se pudieron detectar las pistas: {}", error));
                status_label.set_visible(true);
                gtk4::glib::ControlFlow::Break
            }
            Err(TryRecvError::Empty) => gtk4::glib::ControlFlow::Continue,
            Err(TryRecvError::Disconnected) => {
                status_label.set_label("No se pudieron detectar las pistas del archivo.");
                status_label.set_visible(true);
                gtk4::glib::ControlFlow::Break
            }
        }
    });
}

/// Reconstruye los desplegables conservando la selección guardada.
fn refresh_dropdowns(
    audio_dropdown: &DropDown,
    subtitle_dropdown: &DropDown,
    audio_streams: &[MediaStream],
    subtitle_streams: &[MediaStream],
    selection: TrackSelection,
) {
    let mut audio_options = vec!["Automático (predeterminado)".to_string()];
    audio_options.extend(audio_streams.iter().map(stream_label));
    let audio_model =
        gtk4::StringList::new(&audio_options.iter().map(String::as_str).collect::<Vec<_>>());
    audio_dropdown.set_model(Some(&audio_model));
    audio_dropdown.set_selected(audio_position(audio_streams, selection.audio_stream_index));

    let mut subtitle_options = vec![
        "Automático (predeterminado)".to_string(),
        "Ninguno (desactivados)".to_string(),
    ];
    subtitle_options.extend(subtitle_streams.iter().map(stream_label));
    let subtitle_model = gtk4::StringList::new(
        &subtitle_options
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    );
    subtitle_dropdown.set_model(Some(&subtitle_model));
    subtitle_dropdown.set_selected(subtitle_position(subtitle_streams, selection));
}

/// Posición de la pista de audio en el desplegable (0 = automático).
fn audio_position(streams: &[MediaStream], selected: Option<i32>) -> u32 {
    selected
        .and_then(|index| {
            streams
                .iter()
                .position(|stream| stream.stream_index == index)
        })
        .map(|position| position as u32 + 1)
        .unwrap_or(0)
}

/// Posición de la pista de subtítulos (0 = automático, 1 = ninguno).
fn subtitle_position(streams: &[MediaStream], selection: TrackSelection) -> u32 {
    if selection.subtitles_disabled {
        return 1;
    }
    selection
        .subtitle_stream_index
        .and_then(|index| {
            streams
                .iter()
                .position(|stream| stream.stream_index == index)
        })
        .map(|position| position as u32 + 2)
        .unwrap_or(0)
}

/// Traduce la posición de los desplegables a índices relativos al tipo.
fn current_selection(
    audio_dropdown: &DropDown,
    subtitle_dropdown: &DropDown,
    audio_streams: &[MediaStream],
    subtitle_streams: &[MediaStream],
) -> TrackSelection {
    let audio_position = audio_dropdown.selected() as usize;
    let audio_stream_index = if audio_position == 0 {
        None
    } else {
        audio_streams
            .get(audio_position - 1)
            .map(|stream| stream.stream_index)
    };

    let subtitle_position = subtitle_dropdown.selected() as usize;
    let (subtitle_stream_index, subtitles_disabled) = match subtitle_position {
        0 => (None, false),
        1 => (None, true),
        position => (
            subtitle_streams
                .get(position - 2)
                .map(|stream| stream.stream_index),
            false,
        ),
    };

    TrackSelection {
        audio_stream_index,
        subtitle_stream_index,
        subtitles_disabled,
    }
}

/// Texto legible de una pista: número, idioma, título y códec.
fn stream_label(stream: &MediaStream) -> String {
    let mut parts = vec![format!("Pista {}", stream.stream_index + 1)];
    if let Some(language) = stream.language.as_deref() {
        parts.push(language_name(language));
    }
    if let Some(title) = stream.title.as_deref().filter(|title| !title.is_empty()) {
        parts.push(title.to_string());
    }
    if let Some(codec) = stream.codec.as_deref() {
        parts.push(codec.to_string());
    }
    parts.join(" · ")
}

/// Nombre en español de los códigos de idioma más habituales.
fn language_name(code: &str) -> String {
    match code.to_ascii_lowercase().as_str() {
        "spa" | "es" | "es-es" | "es-mx" => "Español".to_string(),
        "eng" | "en" | "en-us" | "en-gb" => "Inglés".to_string(),
        "jpn" | "ja" => "Japonés".to_string(),
        "fra" | "fre" | "fr" => "Francés".to_string(),
        "deu" | "ger" | "de" => "Alemán".to_string(),
        "ita" | "it" => "Italiano".to_string(),
        "por" | "pt" | "pt-br" => "Portugués".to_string(),
        "rus" | "ru" => "Ruso".to_string(),
        "zho" | "chi" | "zh" => "Chino".to_string(),
        "kor" | "ko" => "Coreano".to_string(),
        "ara" | "ar" => "Árabe".to_string(),
        "hin" | "hi" => "Hindi".to_string(),
        "und" => "Desconocido".to_string(),
        other => other.to_uppercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stream(stream_type: &str, index: i32, language: &str) -> MediaStream {
        MediaStream {
            id: 0,
            media_id: 1,
            file_path: "/tmp/video.mkv".to_string(),
            stream_type: stream_type.to_string(),
            stream_index: index,
            language: Some(language.to_string()),
            codec: Some("aac".to_string()),
            title: None,
        }
    }

    #[test]
    fn posicion_de_audio_traduce_indice_relativo_a_desplegable() {
        let audio = vec![stream("audio", 0, "spa"), stream("audio", 1, "eng")];
        assert_eq!(audio_position(&audio, None), 0);
        assert_eq!(audio_position(&audio, Some(0)), 1);
        assert_eq!(audio_position(&audio, Some(1)), 2);
        assert_eq!(audio_position(&audio, Some(9)), 0);
    }

    #[test]
    fn posicion_de_subtitulos_distingue_automatico_y_ninguno() {
        let subtitles = vec![stream("subtitle", 0, "spa"), stream("subtitle", 1, "eng")];
        assert_eq!(subtitle_position(&subtitles, TrackSelection::default()), 0);
        assert_eq!(
            subtitle_position(
                &subtitles,
                TrackSelection {
                    subtitle_stream_index: None,
                    subtitles_disabled: true,
                    ..Default::default()
                }
            ),
            1
        );
        assert_eq!(
            subtitle_position(
                &subtitles,
                TrackSelection {
                    subtitle_stream_index: Some(1),
                    ..Default::default()
                }
            ),
            3
        );
    }

    #[test]
    fn etiqueta_de_pista_incluye_idioma_y_codec() {
        let label = stream_label(&stream("audio", 1, "eng"));
        assert_eq!(label, "Pista 2 · Inglés · aac");
    }
}
