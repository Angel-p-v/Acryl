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
use crate::ui::i18n::TranslationsRef;
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
    translations: TranslationsRef,
    on_saved: Rc<dyn Fn()>,
) {
    let window = Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(translations.borrow().t("track.title"))
        .default_width(460)
        .build();

    let content = Box::new(Orientation::Vertical, 12);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(18);
    content.set_margin_end(18);

    let heading = Label::new(None);
    heading.set_markup(&format!(
        "<b>{}</b>",
        translations.borrow().t("track.heading")
    ));
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

    let audio_label = Label::new(None);
    audio_label.set_halign(gtk4::Align::Start);
    audio_label.set_markup(&format!(
        "<b>{}</b>",
        translations.borrow().t("track.audio")
    ));
    content.append(&audio_label);

    let audio_options = [translations.borrow().t("track.auto")];
    let audio_option_refs: Vec<&str> = audio_options.iter().map(String::as_str).collect();
    let audio_dropdown = DropDown::from_strings(&audio_option_refs);
    audio_dropdown.set_hexpand(true);
    content.append(&audio_dropdown);

    let subtitle_label = Label::new(None);
    subtitle_label.set_halign(gtk4::Align::Start);
    subtitle_label.set_markup(&format!(
        "<b>{}</b>",
        translations.borrow().t("track.subtitles")
    ));
    content.append(&subtitle_label);

    let subtitle_options = [
        translations.borrow().t("track.auto"),
        translations.borrow().t("track.none"),
    ];
    let subtitle_option_refs: Vec<&str> = subtitle_options.iter().map(String::as_str).collect();
    let subtitle_dropdown = DropDown::from_strings(&subtitle_option_refs);
    subtitle_dropdown.set_hexpand(true);
    content.append(&subtitle_dropdown);

    // Botón para forzar la re-detección con ffprobe (útil cuando la BD tiene
    // pistas cacheadas de una indexación previa incompleta).
    let redetect_button = Button::with_label(&translations.borrow().t("button.redetect"));
    redetect_button.set_halign(gtk4::Align::Start);
    redetect_button.set_tooltip_text(Some(&translations.borrow().t("tooltip.redetect")));
    content.append(&redetect_button);

    let buttons = Box::new(Orientation::Horizontal, 8);
    buttons.set_halign(gtk4::Align::End);
    buttons.set_margin_top(6);

    let cancel_button = Button::with_label(&translations.borrow().t("dialog.remove_folder.cancel"));
    let save_button = Button::with_label(&translations.borrow().t("button.save"));
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
        &translations,
    );

    if has_stored_streams {
        status_label.set_visible(false);
    } else {
        status_label.set_label(&translations.borrow().t("status.detecting"));
        detect_streams_in_background(
            false,
            db_path.to_string(),
            file_path.to_string(),
            status_label.clone(),
            audio_dropdown.clone(),
            subtitle_dropdown.clone(),
            audio_streams.clone(),
            subtitle_streams.clone(),
            selection.clone(),
            translations.clone(),
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
    let translations_for_save = translations.clone();
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
                status_for_save
                    .set_label(&translations_for_save.borrow().t("status.not_in_library"));
                status_for_save.set_visible(true);
            }
            Err(error) => {
                let text = translations_for_save
                    .borrow()
                    .t("status.save_error_format")
                    .replacen("{}", &error.to_string(), 1);
                status_for_save.set_label(&text);
                status_for_save.set_visible(true);
            }
        }
    });

    // Botón: volver a detectar todas las pistas con ffprobe (ignora caché de BD).
    let redetect_for_click = redetect_button.clone();
    let db_path_red = db_path.to_string();
    let file_path_red = file_path.to_string();
    let status_red = status_label.clone();
    let audio_dropdown_red = audio_dropdown.clone();
    let subtitle_dropdown_red = subtitle_dropdown.clone();
    let audio_streams_red = audio_streams.clone();
    let subtitle_streams_red = subtitle_streams.clone();
    let selection_red = selection.clone();
    let translations_red = translations.clone();
    redetect_for_click.connect_clicked(move |_| {
        status_red.set_label(&translations_red.borrow().t("status.detecting"));
        status_red.set_visible(true);
        detect_streams_in_background(
            true,
            db_path_red.to_string(),
            file_path_red.to_string(),
            status_red.clone(),
            audio_dropdown_red.clone(),
            subtitle_dropdown_red.clone(),
            audio_streams_red.clone(),
            subtitle_streams_red.clone(),
            selection_red.clone(),
            translations_red.clone(),
        );
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
/// Cuando `force` es `true`, ignora la caché de la BD y vuelve a ejecutar
/// `ffprobe` para leer todas las pistas del archivo.
#[allow(clippy::too_many_arguments)]
fn detect_streams_in_background(
    force: bool,
    db_path: String,
    file_path: String,
    status_label: Label,
    audio_dropdown: DropDown,
    subtitle_dropdown: DropDown,
    audio_streams: Rc<RefCell<Vec<MediaStream>>>,
    subtitle_streams: Rc<RefCell<Vec<MediaStream>>>,
    selection: Rc<Cell<TrackSelection>>,
    translations: TranslationsRef,
) {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result =
            ensure_media_streams(&db_path, &file_path, force).map_err(|error| error.to_string());
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
                    &translations,
                );

                if audio_streams.borrow().is_empty() && subtitle_streams.borrow().is_empty() {
                    status_label.set_label(&translations.borrow().t("status.no_streams"));
                    status_label.set_visible(true);
                } else {
                    status_label.set_visible(false);
                }
                gtk4::glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                let text = translations
                    .borrow()
                    .t("status.detect_error_format")
                    .replacen("{}", &error, 1);
                status_label.set_label(&text);
                status_label.set_visible(true);
                gtk4::glib::ControlFlow::Break
            }
            Err(TryRecvError::Empty) => gtk4::glib::ControlFlow::Continue,
            Err(TryRecvError::Disconnected) => {
                status_label.set_label(&translations.borrow().t("status.disconnected"));
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
    translations: &TranslationsRef,
) {
    let auto_label = translations.borrow().t("track.auto");
    let none_label = translations.borrow().t("track.none");

    let mut audio_options = vec![auto_label.clone()];
    audio_options.extend(
        audio_streams
            .iter()
            .map(|stream| stream_label(translations, stream)),
    );
    let audio_model =
        gtk4::StringList::new(&audio_options.iter().map(String::as_str).collect::<Vec<_>>());
    audio_dropdown.set_model(Some(&audio_model));
    audio_dropdown.set_selected(audio_position(audio_streams, selection.audio_stream_index));

    let mut subtitle_options = vec![auto_label, none_label];
    subtitle_options.extend(
        subtitle_streams
            .iter()
            .map(|stream| stream_label(translations, stream)),
    );
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
fn stream_label(translations: &TranslationsRef, stream: &MediaStream) -> String {
    let track_word = translations.borrow().t("stream.track_word");
    let mut parts = vec![format!("{} {}", track_word, stream.stream_index + 1)];
    if let Some(language) = stream.language.as_deref() {
        parts.push(language_name(translations, language));
    }
    if let Some(title) = stream.title.as_deref().filter(|title| !title.is_empty()) {
        parts.push(title.to_string());
    }
    if let Some(codec) = stream.codec.as_deref() {
        parts.push(codec.to_string());
    }
    parts.join(" · ")
}

/// Nombre traducido de los códigos de idioma más habituales.
fn language_name(translations: &TranslationsRef, code: &str) -> String {
    let key = match code.to_ascii_lowercase().as_str() {
        "spa" | "es" | "es-es" | "es-mx" => "language.es",
        "eng" | "en" | "en-us" | "en-gb" => "language.en",
        "jpn" | "ja" => "language.ja",
        "fra" | "fre" | "fr" => "language.fr",
        "deu" | "ger" | "de" => "language.de",
        "ita" | "it" => "language.it",
        "por" | "pt" | "pt-br" => "language.pt",
        "rus" | "ru" => "language.ru",
        "zho" | "chi" | "zh" => "language.zh",
        "kor" | "ko" => "language.ko",
        "ara" | "ar" => "language.ar",
        "hin" | "hi" => "language.hi",
        "und" => "language.unknown",
        _ => return code.to_uppercase(),
    };
    translations.borrow().t(key)
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
        let translations = crate::ui::i18n::load_translations("es").expect("cargar español");
        let label = stream_label(&translations, &stream("audio", 1, "eng"));
        assert_eq!(label, "Pista 2 · Inglés · aac");
    }

    #[test]
    fn etiqueta_de_pista_se_traduce_al_ingles() {
        let translations = crate::ui::i18n::load_translations("en").expect("cargar inglés");
        let label = stream_label(&translations, &stream("audio", 1, "eng"));
        assert_eq!(label, "Track 2 · English · aac");
    }
}
