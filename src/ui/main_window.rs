//! Ventana principal de Acryl con GTK4 / Libadwaita.

use crate::db::media_repo::ListMediaOptions;
use crate::db::{MediaRepository, close_db, open_db};
use crate::ui::media_details::MediaDetails;
use crate::ui::media_list::MediaList;
use crate::ui::search_bar::SearchBar;
use crate::ui::settings_window::SettingsView;
use gtk4::prelude::*;
use gtk4::{Box, Button, Orientation, Stack};
use libadwaita::ApplicationWindow as AdwApplicationWindow;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;

pub struct MainWindow {
    window: AdwApplicationWindow,
    stack: Stack,
    search_bar: SearchBar,
    settings_button: Button,
    media_list: MediaList,
    media_details: Rc<RefCell<MediaDetails>>,
    settings_view: SettingsView,
    db_path: String,
}

type MainWindowRef = Rc<RefCell<MainWindow>>;

impl MainWindow {
    pub fn new(app: &gtk4::Application, config: crate::Config) -> MainWindowRef {
        let db_path = config.inner().db_path.clone();
        let config_path = config.path().to_path_buf();

        let window = AdwApplicationWindow::builder()
            .application(app)
            .title("Acryl")
            .default_width(1000)
            .default_height(700)
            .build();

        let content = Box::new(Orientation::Vertical, 0);
        content.set_vexpand(true);
        window.set_content(Some(&content));

        // Barra superior: búsqueda + botón de ajustes
        let top_bar = Box::new(Orientation::Horizontal, 10);
        top_bar.set_hexpand(true);
        top_bar.set_halign(gtk4::Align::Fill);

        let search_bar = SearchBar::new(&db_path);
        top_bar.append(search_bar.widget());

        let settings_button = Button::with_label("⚙ Ajustes");
        settings_button.set_halign(gtk4::Align::End);
        settings_button.set_hexpand(false);
        top_bar.append(&settings_button);

        content.append(&top_bar);

        let stack = Stack::new();
        stack.set_hexpand(true);
        stack.set_vexpand(true);
        content.append(&stack);

        let media_list = MediaList::new();
        stack.add_titled(media_list.widget(), Some("list"), "Lista");

        let media_details = Rc::new(RefCell::new(MediaDetails::new(&db_path, 0)));
        media_details.borrow_mut().set_db_path(&db_path);
        stack.add_titled(media_details.borrow().widget(), Some("details"), "Detalles");

        let settings_view = SettingsView::new(config.inner(), &db_path);
        stack.add_titled(settings_view.widget(), Some("settings"), "Ajustes");

        stack.set_visible_child_name("list");

        let config_rc = Rc::new(RefCell::new(config.inner().clone()));

        let this = Rc::new(RefCell::new(Self {
            window,
            stack,
            search_bar,
            settings_button,
            media_list,
            media_details,
            settings_view,
            db_path: db_path.clone(),
        }));

        this.borrow().window.show();

        // Cargar medios iniciales (SQLite local, rápido).
        Self::load_media(&this, &db_path);

        // Callback: al seleccionar un medio de la lista, mostrar detalles.
        let this_clone = this.clone();
        this.borrow_mut().media_list.on_select(move |media_id| {
            let w = this_clone.borrow();
            w.show_details(media_id);
        });

        // Callback: al buscar y seleccionar un resultado, mostrar detalles.
        let this_clone = this.clone();
        this.borrow_mut().search_bar.on_select(move |media_id| {
            let w = this_clone.borrow();
            w.show_details(media_id);
        });

        // Callback: botón de Ajustes -> navega a la vista de ajustes dentro del Stack.
        let this_clone = this.clone();
        this.borrow_mut().settings_button.connect_clicked(move |_| {
            let w = this_clone.borrow();
            w.stack.set_visible_child_name("settings");
        });

        // Callback: Guardar ajustes -> guarda en disco, limpia medios huérfanos y vuelve a la lista.
        let config_rc_save = config_rc.clone();
        let config_path_save = config_path.clone();
        let db_path_save = db_path.clone();
        this.borrow_mut().settings_view.on_save(move |new_config| {
            let folders = new_config.media_folders.clone();
            let config_path_clone = config_path_save.clone();
            let _ = crate::config::Config {
                path: config_path_clone,
                inner: new_config.clone(),
            }
            .save();
            *config_rc_save.borrow_mut() = new_config;

            // Eliminar medios de carpetas que ya no están en el config
            let _ = crate::core::library::cleanup_media_not_in_folders(&db_path_save, &folders);
        });

        // Conectar botón Volver de ajustes -> vuelve a la lista.
        let this_clone = this.clone();
        this.borrow_mut()
            .settings_view
            .cancel_button()
            .connect_clicked(move |_| {
                let w = this_clone.borrow();
                w.stack.set_visible_child_name("list");
            });

        // Conectar botón Guardar de ajustes -> guarda, vuelve a la lista y recarga medios.
        let this_clone = this.clone();
        let db_path_for_reload = db_path.clone();
        this.borrow_mut()
            .settings_view
            .save_button()
            .connect_clicked(move |_| {
                let w = this_clone.borrow();
                w.stack.set_visible_child_name("list");
                drop(w);
                // Recargar medios después de guardar ajustes (pueden haber cambiado las carpetas).
                Self::load_media(&this_clone, &db_path_for_reload);
            });

        // Callbacks de la vista de detalles.
        let this_clone = this.clone();
        this.borrow_mut()
            .media_details
            .borrow_mut()
            .on_back(move || {
                let w = this_clone.borrow();
                w.show_list();
            });

        let this_clone = this.clone();
        this.borrow_mut()
            .media_details
            .borrow_mut()
            .on_favorite(move || {
                let w = this_clone.borrow();
                w.toggle_favorite();
            });

        let this_clone = this.clone();
        this.borrow_mut()
            .media_details
            .borrow_mut()
            .on_watched(move || {
                let w = this_clone.borrow();
                w.toggle_watched();
            });

        let this_clone = this.clone();
        this.borrow_mut()
            .media_details
            .borrow_mut()
            .on_play(move || {
                let w = this_clone.borrow();
                w.play_file();
            });

        // Callback: al hacer clic en "Abrir" de una temporada, navegar a los detalles de esa temporada.
        let this_clone = this.clone();
        this.borrow_mut()
            .media_details
            .borrow_mut()
            .on_open_season(move |season_id| {
                let w = this_clone.borrow();
                w.show_details(season_id);
            });

        this
    }

    fn load_media(this: &MainWindowRef, db_path: &str) {
        let mut conn = match open_db(db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error abriendo BD: {}", e);
                return;
            }
        };
        let mut repo = MediaRepository::new(&mut conn);
        let media = repo
            .list(ListMediaOptions {
                r#type: None,
                limit: Some(500),
                offset: Some(0),
                order_by: None,
            })
            .unwrap_or_default();
        close_db(conn);

        println!("Medios cargados desde la BD: {}", media.len());

        let mut w = this.borrow_mut();
        w.media_list.set_media(media);
    }

    fn show_details(&self, media_id: i64) {
        let mut conn = match open_db(&self.db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error abriendo BD: {}", e);
                return;
            }
        };
        let mut repo = MediaRepository::new(&mut conn);
        let media = repo.find_by_id(media_id).unwrap_or(None);
        close_db(conn);

        if let Some(media) = media {
            self.media_details.borrow_mut().set_media(&media);
            self.stack.set_visible_child_name("details");
        }
    }

    fn show_list(&self) {
        self.stack.set_visible_child_name("list");
        self.media_list.refresh();
    }

    fn toggle_favorite(&self) {
        let media_id = self.media_details.borrow().media_id();
        let mut conn = match open_db(&self.db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error abriendo BD: {}", e);
                return;
            }
        };
        let mut repo = MediaRepository::new(&mut conn);
        let _ = repo.toggle_favorite(media_id);
        if let Some(m) = repo.find_by_id(media_id).unwrap_or(None) {
            close_db(conn);
            self.media_details.borrow_mut().set_media(&m);
        } else {
            close_db(conn);
        }
    }

    fn toggle_watched(&self) {
        let media_id = self.media_details.borrow().media_id();
        let mut conn = match open_db(&self.db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error abriendo BD: {}", e);
                return;
            }
        };
        let mut repo = MediaRepository::new(&mut conn);
        let _ = repo.toggle_watched(media_id);
        if let Some(m) = repo.find_by_id(media_id).unwrap_or(None) {
            close_db(conn);
            self.media_details.borrow_mut().set_media(&m);
        } else {
            close_db(conn);
        }
    }

    fn play_file(&self) {
        let media_id = self.media_details.borrow().media_id();
        let mut conn = match open_db(&self.db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error abriendo BD: {}", e);
                return;
            }
        };
        let mut repo = MediaRepository::new(&mut conn);
        let files = repo.list_files_by_media(media_id).unwrap_or_default();
        let _ = repo.record_playback(media_id, 0.0, 0.0);
        close_db(conn);

        if let Some(f) = files.first() {
            // Lanzar mpv como proceso externo (no bloquea la UI de GTK4).
            let _ = Command::new("mpv").arg(&f.file_path).spawn();
        } else {
            eprintln!("No hay archivos para el medio {}", media_id);
        }
    }

    pub fn widget(&self) -> &AdwApplicationWindow {
        &self.window
    }
}
