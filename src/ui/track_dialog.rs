//! Selector de pistas de audio y subtítulos de un archivo.
//!
//! La ventana permite elegir la pista de audio, una pista de subtítulos
//! concreta o desactivarlos. La elección se guarda por archivo en
//! `media_files`, de modo que cada episodio mantiene su propia preferencia.
//! Si el archivo aún no tiene pistas analizadas, se ejecuta `ffprobe` en
//! segundo plano para no bloquear la interfaz.

use crate::config::DefaultTracks;
use crate::core::library::{discover_external_subtitles, ensure_media_streams};
use crate::db::media_repo::MediaStream;
use crate::db::{FileRepository, TrackPreference, TrackSelection, close_db, open_db};
use crate::ui::i18n::TranslationsRef;
use gtk4::prelude::*;
use gtk4::{Box, Button, DropDown, Label, Orientation, ScrolledWindow, Window};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{self, TryRecvError};
use std::time::Duration;

/// Abre el selector de pistas para `file_path`.
///
/// `on_saved` se invoca solo cuando la preferencia se ha guardado
/// correctamente. La ventana se cierra al guardar o al cancelar.
///
/// `default_tracks` son las pistas predeterminadas configuradas en Ajustes.
/// Si se proporcionan, se añade una opción "Usar predeterminadas de Ajustes"
/// a cada desplegable para que el usuario pueda elegir usarlas para este
/// archivo concreto.
pub fn show_track_dialog(
    parent: &impl IsA<gtk4::Window>,
    db_path: &str,
    file_path: &str,
    translations: TranslationsRef,
    on_saved: Rc<dyn Fn()>,
    default_tracks: Option<DefaultTracks>,
) {
    let window = Window::builder()
        .transient_for(parent)
        .modal(true)
        .resizable(true)
        .title(translations.borrow().t("track.title"))
        .default_width(520)
        .default_height(640)
        .build();

    let content = Box::new(Orientation::Vertical, 12);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(18);
    content.set_margin_end(18);
    content.set_hexpand(true);
    // El contenido no se expande verticalmente: su altura natural es la suma
    // de los controles y el ScrolledWindow desplaza cuando supera la altura
    // disponible, evitando que quede fuera de área en pantallas pequeñas.
    content.set_vexpand(false);

    let heading = Label::new(None);
    heading.set_markup(&format!(
        "<b>{}</b>",
        translations.borrow().t("track.heading")
    ));
    heading.set_halign(gtk4::Align::Start);
    content.append(&heading);

    let scrolled = ScrolledWindow::new();
    scrolled.set_hexpand(true);
    scrolled.set_vexpand(true);
    scrolled.set_hscrollbar_policy(gtk4::PolicyType::Never);
    scrolled.set_vscrollbar_policy(gtk4::PolicyType::Automatic);
    scrolled.set_child(Some(&content));

    window.set_child(Some(&scrolled));

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
    let preference = Rc::new(RefCell::new(load_preference(db_path, file_path)));

    // Carga rápida desde la base de datos: si ya hay pistas analizadas, la
    // ventana aparece completa al instante.
    let (stored_audio, stored_subtitle, has_stored_streams) = load_streams(db_path, file_path);
    *audio_streams.borrow_mut() = stored_audio;
    *subtitle_streams.borrow_mut() = stored_subtitle;
    refresh_dropdowns(
        &audio_dropdown,
        &subtitle_dropdown,
        &audio_streams.borrow(),
        &subtitle_streams.borrow(),
        &preference.borrow(),
        &translations,
        default_tracks.as_ref(),
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
            preference.clone(),
            translations.clone(),
            default_tracks.clone(),
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
    let default_tracks_for_save = default_tracks.clone();
    save_button.connect_clicked(move |_| {
        let preference = current_preference(
            &audio_for_save,
            &subtitle_for_save,
            &audio_streams_for_save.borrow(),
            &subtitle_streams_for_save.borrow(),
            default_tracks_for_save.as_ref(),
        );

        match save_preference(&db_path_for_save, &file_path_for_save, &preference) {
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
    let preference_red = preference.clone();
    let translations_red = translations.clone();
    let default_tracks_for_redetect = default_tracks.clone();
    redetect_for_click.connect_clicked(move |_| {
        let default_tracks_for_redetect = default_tracks_for_redetect.clone();
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
            preference_red.clone(),
            translations_red.clone(),
            default_tracks_for_redetect,
        );
    });

    window.present();
}

/// Carga las pistas guardadas, separadas por tipo.
///
/// El tercer valor indica si la base de datos ya tenía pistas analizadas; si
/// no, la ventana lanzará `ffprobe` en segundo plano aunque haya sidecars.
fn load_streams(db_path: &str, file_path: &str) -> (Vec<MediaStream>, Vec<MediaStream>, bool) {
    let Ok(mut conn) = open_db(db_path) else {
        return (Vec::new(), Vec::new(), false);
    };
    let mut streams = {
        let mut repo = crate::db::MediaRepository::new(&mut conn);
        repo.list_streams_by_file(file_path).unwrap_or_default()
    };
    close_db(conn);
    let has_stored_streams = !streams.is_empty();

    // Los sidecars pueden no estar en una BD indexada antes de que existiera
    // su soporte. Se muestran igualmente para poder elegirlos y guardarlos.
    let next_index = streams
        .iter()
        .filter(|stream| stream.stream_type == "subtitle")
        .map(|stream| stream.stream_index + 1)
        .max()
        .unwrap_or(0);
    for subtitle in discover_external_subtitles(file_path, next_index) {
        if !streams
            .iter()
            .any(|stream| stream.external_path == subtitle.external_path)
        {
            streams.push(subtitle.into_media_stream());
        }
    }

    let mut audio = Vec::new();
    let mut subtitle = Vec::new();
    for stream in streams {
        match stream.stream_type.as_str() {
            "audio" => audio.push(stream),
            "subtitle" => subtitle.push(stream),
            _ => {}
        }
    }
    (audio, subtitle, has_stored_streams)
}

/// Carga la preferencia guardada; si falla, usa la selección automática.
fn load_preference(db_path: &str, file_path: &str) -> TrackPreference {
    let Ok(mut conn) = open_db(db_path) else {
        return TrackPreference::default();
    };
    let preference = {
        let mut repo = FileRepository::new(&mut conn);
        repo.track_preference(file_path).unwrap_or_default()
    };
    close_db(conn);
    preference
}

/// Guarda la preferencia, incluidas las claves estables de cada pista.
/// Devuelve `Ok(false)` si el archivo no existe en la biblioteca, para poder
/// avisar al usuario sin tratarlo como error interno.
fn save_preference(
    db_path: &str,
    file_path: &str,
    preference: &TrackPreference,
) -> Result<bool, std::boxed::Box<dyn std::error::Error>> {
    let mut conn = open_db(db_path)?;
    let updated = {
        let mut repo = FileRepository::new(&mut conn);
        repo.set_track_preference(file_path, preference)?
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
    preference: Rc<RefCell<TrackPreference>>,
    translations: TranslationsRef,
    default_tracks: Option<DefaultTracks>,
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
                    &preference.borrow(),
                    &translations,
                    default_tracks.as_ref(),
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
///
/// Si `default_tracks` tiene alguna predeterminada configurada, se añade una
/// opción "Usar predeterminadas de Ajustes" en cada desplegable correspondiente.
fn refresh_dropdowns(
    audio_dropdown: &DropDown,
    subtitle_dropdown: &DropDown,
    audio_streams: &[MediaStream],
    subtitle_streams: &[MediaStream],
    preference: &TrackPreference,
    translations: &TranslationsRef,
    default_tracks: Option<&DefaultTracks>,
) {
    let auto_label = translations.borrow().t("track.auto");
    let none_label = translations.borrow().t("track.none");
    let default_label = translations.borrow().t("track.use_defaults");

    // --- Audio ---
    let mut audio_options = vec![auto_label.clone()];
    // Opción "Usar predeterminadas de Ajustes" si hay predeterminada de audio.
    if default_tracks.map(|d| d.audio.is_some()).unwrap_or(false) {
        audio_options.push(default_label.clone());
    }
    audio_options.extend(
        audio_streams
            .iter()
            .map(|stream| stream_label(translations, stream)),
    );
    let audio_model =
        gtk4::StringList::new(&audio_options.iter().map(String::as_str).collect::<Vec<_>>());
    audio_dropdown.set_model(Some(&audio_model));
    audio_dropdown.set_selected(audio_position(audio_streams, preference, default_tracks));

    // --- Subtítulos ---
    let mut subtitle_options = vec![auto_label, none_label];
    // Opción "Usar predeterminadas de Ajustes" si hay predeterminada de subtítulos.
    if default_tracks
        .map(|d| d.subtitle.is_some())
        .unwrap_or(false)
    {
        subtitle_options.push(default_label.clone());
    }
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
    subtitle_dropdown.set_selected(subtitle_position(
        subtitle_streams,
        preference,
        default_tracks,
    ));
}

/// Posición de la pista de audio en el desplegable (0 = automático, 1 = predeterminadas).
///
/// La clave estable tiene prioridad. El índice guardado solo se usa como
/// respaldo para bases de datos antiguas que no tienen clave.
fn audio_position(
    streams: &[MediaStream],
    preference: &TrackPreference,
    default_tracks: Option<&DefaultTracks>,
) -> u32 {
    // Si el usuario eligió "Usar predeterminadas de Ajustes", devolver directamente
    // la posición 1 (el desplegable tiene: 0=Automático, 1=Predeterminadas, 2+=pistas).
    if preference.audio_track_key.as_deref() == Some("default:audio")
        && default_tracks.map(|d| d.audio.is_some()).unwrap_or(false)
    {
        return 1;
    }

    let position = match preference.audio_track_key.as_deref() {
        Some(key) => streams.iter().position(|stream| stream.track_key() == key),
        None => preference.selection.audio_stream_index.and_then(|index| {
            streams
                .iter()
                .position(|stream| stream.stream_index == index)
        }),
    };
    // 0 = Automático, 2+ = pistas del vec (índice del stream + 2)
    position.map(|position| position as u32 + 2).unwrap_or(0)
}

/// Posición de la pista de subtítulos (0 = automático, 1 = ninguno, 2 = predeterminadas).
fn subtitle_position(
    streams: &[MediaStream],
    preference: &TrackPreference,
    default_tracks: Option<&DefaultTracks>,
) -> u32 {
    if preference.selection.subtitles_disabled {
        return 1;
    }
    // Si el usuario eligió "Usar predeterminadas de Ajustes", devolver directamente
    // la posición 2 (el desplegable tiene: 0=Automático, 1=Ninguno, 2=Predeterminadas, 3+=pistas).
    if preference.subtitle_track_key.as_deref() == Some("default:subtitle")
        && default_tracks
            .map(|d| d.subtitle.is_some())
            .unwrap_or(false)
    {
        return 2;
    }

    let position = match preference.subtitle_track_key.as_deref() {
        Some(key) => streams.iter().position(|stream| stream.track_key() == key),
        None => preference
            .selection
            .subtitle_stream_index
            .and_then(|index| {
                streams
                    .iter()
                    .position(|stream| stream.stream_index == index)
            }),
    };
    // 0 = Automático, 1 = Ninguno, 3+ = pistas del vec (índice del stream + 3)
    position.map(|position| position as u32 + 3).unwrap_or(0)
}

/// Traduce la posición de los desplegables a una preferencia con identidad
/// estable. Guardar las claves permite recuperar la misma pista aunque cambie
/// el orden de los streams o se trate de un subtítulo externo.
///
/// Si se selecciona "Usar predeterminadas de Ajustes", se guarda la clave
/// especial `default:audio` o `default:subtitle` para que el reproductor sepa
/// que debe aplicar las pistas predeterminadas de Ajustes para este archivo.
fn current_preference(
    audio_dropdown: &DropDown,
    subtitle_dropdown: &DropDown,
    audio_streams: &[MediaStream],
    subtitle_streams: &[MediaStream],
    default_tracks: Option<&DefaultTracks>,
) -> TrackPreference {
    let audio_position = audio_dropdown.selected() as usize;
    let (audio_stream, audio_use_defaults) = match audio_position {
        0 => (None, false),
        1 if default_tracks.map(|d| d.audio.is_some()).unwrap_or(false) => (None, true),
        position => (audio_streams.get(position.saturating_sub(1)), false),
    };

    let subtitle_position = subtitle_dropdown.selected() as usize;
    let (subtitle_stream, subtitle_use_defaults, subtitles_disabled) = match subtitle_position {
        0 => (None, false, false),
        1 => (None, false, true),
        2 if default_tracks
            .map(|d| d.subtitle.is_some())
            .unwrap_or(false) =>
        {
            (None, true, false)
        }
        position => (
            subtitle_streams.get(position.saturating_sub(2)),
            false,
            false,
        ),
    };

    TrackPreference {
        selection: TrackSelection {
            audio_stream_index: audio_stream.map(|stream| stream.stream_index),
            subtitle_stream_index: subtitle_stream.map(|stream| stream.stream_index),
            subtitles_disabled,
        },
        audio_track_key: if audio_use_defaults {
            Some("default:audio".to_string())
        } else {
            audio_stream.map(MediaStream::track_key)
        },
        subtitle_track_key: if subtitle_use_defaults {
            Some("default:subtitle".to_string())
        } else {
            subtitle_stream.map(MediaStream::track_key)
        },
        subtitle_external_path: subtitle_stream.and_then(|stream| stream.external_path.clone()),
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
            external_path: None,
        }
    }

    fn preference(index: Option<i32>) -> TrackPreference {
        TrackPreference {
            selection: TrackSelection {
                audio_stream_index: index,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn posicion_de_audio_traduce_indice_relativo_a_desplegable() {
        let audio = vec![stream("audio", 0, "spa"), stream("audio", 1, "eng")];
        assert_eq!(audio_position(&audio, &TrackPreference::default(), None), 0);
        // Posición 1 = "Usar predeterminadas de Ajustes", las pistas empiezan en 2
        assert_eq!(audio_position(&audio, &preference(Some(0)), None), 2);
        assert_eq!(audio_position(&audio, &preference(Some(1)), None), 3);
        assert_eq!(audio_position(&audio, &preference(Some(9)), None), 0);
    }

    #[test]
    fn posicion_de_audio_prioriza_la_clave_estable() {
        let audio = vec![stream("audio", 0, "spa"), stream("audio", 1, "eng")];
        let mut saved = preference(Some(0));
        saved.audio_track_key = Some(audio[1].track_key());

        assert_eq!(audio_position(&audio, &saved, None), 3);
    }

    #[test]
    fn posicion_de_subtitulos_distingue_automatico_y_ninguno() {
        let subtitles = vec![stream("subtitle", 0, "spa"), stream("subtitle", 1, "eng")];
        assert_eq!(
            subtitle_position(&subtitles, &TrackPreference::default(), None),
            0
        );
        let mut disabled = TrackPreference::default();
        disabled.selection.subtitles_disabled = true;
        assert_eq!(subtitle_position(&subtitles, &disabled, None), 1);

        let mut saved = TrackPreference::default();
        saved.selection.subtitle_stream_index = Some(1);
        assert_eq!(subtitle_position(&subtitles, &saved, None), 4);
    }

    #[test]
    fn posicion_de_subtitulos_encuentra_sidecar_por_clave() {
        let mut external = stream("subtitle", 2, "spa");
        external.external_path = Some("/tmp/video.es.srt".to_string());
        let subtitles = vec![stream("subtitle", 0, "spa"), external.clone()];
        let saved = TrackPreference {
            subtitle_track_key: Some(external.track_key()),
            ..Default::default()
        };

        assert_eq!(subtitle_position(&subtitles, &saved, None), 4);
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
