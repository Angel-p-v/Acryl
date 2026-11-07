//! Tests de integración de la vista de Ajustes.
//!
//! Se ejecutan en un binario de test propio para poder inicializar GTK4 en el
//! hilo del test sin interferir con los tests E2E de `e2e_app`. Requieren un
//! display (X11/Wayland); si no hay ninguno disponible, el test se omite.
//! En CI se ejecutan bajo `xvfb-run`.

use acryl::config::{AppConfig, Config, DefaultTracks};
use acryl::core::language::language_key;
use acryl::db::connection::open_db;
use acryl::ui::load_translations;
use acryl::ui::settings_window::SettingsView;
use gtk4::prelude::*;
use rusqlite::params;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use tempfile::TempDir;

/// Inicializa GTK4 en el hilo actual y comprueba que hay display disponible.
fn gtk_available() -> bool {
    if std::panic::catch_unwind(gtk4::init).is_err() {
        return false;
    }
    gtk4::gdk::Display::default().is_some()
}

/// Crea una `Config` con rutas temporales.
fn temp_config() -> (Config, TempDir, TempDir) {
    let config_dir = TempDir::new().expect("crear directorio de config temporal");
    let data_dir = TempDir::new().expect("crear directorio de datos temporal");

    let config_path = config_dir.path().join("config.yaml");
    let db_path = data_dir.path().join("acryl.db");

    let inner = AppConfig {
        db_path: db_path.to_string_lossy().to_string(),
        media_folders: Vec::new(),
        language: "es".to_string(),
        default_player: "mpv".to_string(),
        default_tracks: DefaultTracks::default(),
        theme: "dark".to_string(),
        playback_completion_threshold_seconds: 180,
        auto_play_next_episode: true,
    };

    let config = Config {
        path: config_path,
        inner,
    };
    config.save().expect("guardar config temporal");

    (config, config_dir, data_dir)
}

/// Crea una `SettingsView` sobre una ventana padre temporal.
fn crear_settings_view(config: &Config) -> (gtk4::Window, SettingsView) {
    let parent = gtk4::Window::new();
    let translations = load_translations(&config.inner.language).expect("cargar traducciones");
    let view = SettingsView::new(
        &config.inner,
        &config.inner.db_path,
        config.path(),
        &parent,
        translations,
    );
    (parent, view)
}

fn abrir_bd(config: &Config) -> rusqlite::Connection {
    open_db(&config.inner.db_path).expect("abrir BD de test")
}

/// Inserta un medio con una pista de audio y otra de subtítulos en la BD y
/// devuelve sus claves estables.
fn insertar_pistas_prueba(conn: &mut rusqlite::Connection) -> (String, String) {
    conn.execute(
        "INSERT INTO media (type, title, sort_title, year, folder_path) \
         VALUES ('movie', 'Matrix', 'matrix', 1999, '/media/matrix')",
        [],
    )
    .expect("insertar medio de prueba");
    let media_id = conn.last_insert_rowid();

    for (stream_type, index, language, codec, title) in [
        ("audio", 1, "jpn", "aac", "Japanese"),
        ("subtitle", 2, "spa", "ass", "Spanish"),
    ] {
        conn.execute(
            "INSERT INTO media_streams \
             (media_id, file_path, stream_type, stream_index, language, codec, title) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                media_id,
                "/media/matrix/matrix.mkv",
                stream_type,
                index,
                language,
                codec,
                title
            ],
        )
        .expect("insertar pista de prueba");
    }

    (language_key(Some("ja")), language_key(Some("es")))
}

/// Pulsa Guardar y devuelve la configuración recibida por `on_save`.
fn guardar_ajustes(view: &SettingsView, guardado: &RefCell<Option<AppConfig>>) -> AppConfig {
    view.save_button().emit_clicked();
    guardado
        .borrow()
        .clone()
        .expect("el callback on_save debe ejecutarse al guardar")
}

fn escenario_carpetas_configuradas() {
    let (mut config, _config_dir, _data_dir) = temp_config();
    config.inner.media_folders = vec!["/media/peliculas".to_string(), "/media/series".to_string()];
    config.save().expect("guardar config con carpetas");

    let (parent, view) = crear_settings_view(&config);
    assert_eq!(
        view.folder_count_for_test(),
        2,
        "debe mostrarse una fila por cada carpeta configurada"
    );

    drop(view);
    drop(parent);
}

fn escenario_pistas_predeterminadas_al_guardar() {
    let (mut config, _config_dir, _data_dir) = temp_config();
    let (audio_key, subtitle_key) = {
        let mut conn = abrir_bd(&config);
        insertar_pistas_prueba(&mut conn)
    };

    config.inner.default_tracks.audio = Some(audio_key.clone());
    config.inner.default_tracks.subtitle = Some(subtitle_key.clone());
    config.save().expect("guardar config con pistas");

    let (parent, mut view) = crear_settings_view(&config);

    let guardado = Rc::new(RefCell::new(None));
    let guardado_c = guardado.clone();
    view.on_save(move |cfg| {
        *guardado_c.borrow_mut() = Some(cfg);
    });

    let cfg = guardar_ajustes(&view, &guardado);
    assert_eq!(cfg.db_path, config.inner.db_path);
    assert_eq!(
        cfg.default_tracks.audio.as_deref(),
        Some(audio_key.as_str()),
        "el audio predeterminado debe conservarse al guardar"
    );
    assert_eq!(
        cfg.default_tracks.subtitle.as_deref(),
        Some(subtitle_key.as_str()),
        "los subtítulos predeterminados deben conservarse al guardar"
    );

    drop(view);
    drop(parent);
}

fn escenario_claves_antiguas_se_migran_al_idioma() {
    let (mut config, _config_dir, _data_dir) = temp_config();
    {
        let mut conn = abrir_bd(&config);
        insertar_pistas_prueba(&mut conn);
    }

    // Claves guardadas por versiones anteriores: pista concreta, no idioma.
    config.inner.default_tracks.audio = Some("embedded:audio:jpn:aac:japanese".to_string());
    config.inner.default_tracks.subtitle = Some("embedded:subtitle:spa:ass:spanish".to_string());
    config.save().expect("guardar config con claves antiguas");

    let (parent, mut view) = crear_settings_view(&config);
    let guardado = Rc::new(RefCell::new(None));
    let guardado_c = guardado.clone();
    view.on_save(move |cfg| {
        *guardado_c.borrow_mut() = Some(cfg);
    });

    let cfg = guardar_ajustes(&view, &guardado);
    assert_eq!(cfg.default_tracks.audio.as_deref(), Some("language:ja"));
    assert_eq!(cfg.default_tracks.subtitle.as_deref(), Some("language:es"));

    drop(view);
    drop(parent);
}

fn escenario_seleccion_despues_de_registrar_callback() {
    let (config, _config_dir, _data_dir) = temp_config();
    let (audio_key, subtitle_key) = {
        let mut conn = abrir_bd(&config);
        insertar_pistas_prueba(&mut conn)
    };

    let (parent, mut view) = crear_settings_view(&config);

    let guardado = Rc::new(RefCell::new(None));
    let guardado_c = guardado.clone();
    view.on_save(move |cfg| {
        *guardado_c.borrow_mut() = Some(cfg);
    });

    // El usuario cambia la selección después de que MainWindow haya
    // registrado el callback de guardado.
    view.select_default_track_for_test(true, Some(audio_key.as_str()));
    view.select_default_track_for_test(false, Some(subtitle_key.as_str()));

    let cfg = guardar_ajustes(&view, &guardado);
    assert_eq!(
        cfg.default_tracks.audio.as_deref(),
        Some(audio_key.as_str()),
        "debe guardarse la pista de audio vigente al pulsar Guardar"
    );
    assert_eq!(
        cfg.default_tracks.subtitle.as_deref(),
        Some(subtitle_key.as_str()),
        "debe guardarse la pista de subtítulos vigente al pulsar Guardar"
    );

    // Volver a «Ninguno» también debe persistirse.
    view.select_default_track_for_test(false, None);
    let cfg = guardar_ajustes(&view, &guardado);
    assert_eq!(
        cfg.default_tracks.subtitle, None,
        "«Ninguno» debe guardarse como ausencia de pista predeterminada"
    );

    drop(view);
    drop(parent);
}

fn escenario_boton_acerca_de() {
    let (config, _config_dir, _data_dir) = temp_config();
    let (parent, mut view) = crear_settings_view(&config);

    let pulsaciones = Rc::new(Cell::new(0usize));
    let pulsaciones_c = pulsaciones.clone();
    view.on_about(move || {
        pulsaciones_c.set(pulsaciones_c.get() + 1);
    });

    view.about_button().emit_clicked();
    assert_eq!(
        pulsaciones.get(),
        1,
        "el botón «Acerca de» debe invocar su callback una vez"
    );

    drop(view);
    drop(parent);
}

fn escenario_dialogo_acerca_de() {
    let translations = load_translations("es").expect("cargar traducciones");
    let dialog = acryl::ui::about::build_about_dialog(&translations);

    assert_eq!(dialog.application_name(), "Acryl");
    assert_eq!(dialog.version(), env!("CARGO_PKG_VERSION"));
    assert_eq!(dialog.developer_name(), "Angel-p-v");
    assert_eq!(dialog.license_type(), gtk4::License::MitX11);
    assert_eq!(
        dialog.comments(),
        translations.borrow().t("about.description"),
        "la descripción debe usar el idioma activo"
    );
    assert!(
        dialog.website().contains("github.com"),
        "el diálogo debe enlazar al repositorio"
    );
    assert_eq!(
        dialog.developers().first().map(|name| name.as_str()),
        Some("Angel-p-v"),
        "el autor debe aparecer en los créditos"
    );
    assert!(
        dialog.copyright().contains("Angel-p-v"),
        "debe mostrarse el aviso de copyright"
    );
    assert!(
        dialog.support_url().contains("/discussions"),
        "debe ofrecerse un enlace de soporte"
    );
    assert!(
        dialog.debug_info().contains("GTK")
            && dialog.debug_info().contains("Libadwaita")
            && dialog.debug_info().contains(env!("CARGO_PKG_VERSION")),
        "la información de depuración debe incluir las versiones"
    );

    drop(dialog);
}

#[test]
fn ajustes_integracion() {
    if !gtk_available() {
        eprintln!("TEST OMITIDO: no hay display disponible para GTK4.");
        return;
    }

    escenario_carpetas_configuradas();
    escenario_pistas_predeterminadas_al_guardar();
    escenario_claves_antiguas_se_migran_al_idioma();
    escenario_seleccion_despues_de_registrar_callback();
    escenario_boton_acerca_de();
    escenario_dialogo_acerca_de();
}
