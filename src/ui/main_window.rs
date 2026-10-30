//! Ventana principal de Acryl con GTK4 / Libadwaita.

use crate::db::media_repo::ListMediaOptions;
use crate::db::{MediaRepository, close_db, open_db};
use crate::ui::media_details::MediaDetails;
use crate::ui::media_list::MediaList;
use crate::ui::search_bar::SearchBar;
use crate::ui::settings_window::SettingsView;
use gtk4::prelude::*;
use gtk4::{Box, Button, Orientation, Stack, glib};
use libadwaita::ApplicationWindow as AdwApplicationWindow;
use libadwaita::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc::{self, TryRecvError};
use std::time::Duration;

pub struct MainWindow {
    window: AdwApplicationWindow,
    stack: Stack,
    search_bar: SearchBar,
    settings_button: Button,
    media_list: MediaList,
    media_details: Rc<RefCell<MediaDetails>>,
    settings_view: SettingsView,
    db_path: String,
    metadata_sync_running: Rc<Cell<bool>>,
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
            metadata_sync_running: Rc::new(Cell::new(false)),
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
                Self::start_metadata_sync(&this_clone, db_path_for_reload.clone());
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

        // Callback: reproducir un episodio concreto con mpv (proceso externo).
        let this_clone = this.clone();
        this.borrow_mut()
            .media_details
            .borrow_mut()
            .on_play_episode(move |file_path| {
                let w = this_clone.borrow();
                if let Err(error) = w.play_episode(&file_path) {
                    eprintln!("No se pudo reproducir '{}': {}", file_path, error);
                    w.show_error("No se pudo reproducir el episodio", &error.to_string());
                }
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

        // Completar metadatos y pósters que falten sin bloquear la UI.
        Self::start_metadata_sync(&this, db_path.clone());

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
        close_db(conn);

        if let Some(f) = files.first() {
            if let Err(error) =
                crate::core::player::play_external(&self.db_path, Some(media_id), &f.file_path)
            {
                eprintln!("No se pudo reproducir '{}': {}", f.file_path, error);
                self.show_error("No se pudo reproducir", &error.to_string());
            }
        } else {
            eprintln!("No hay archivos para el medio {}", media_id);
            self.show_error(
                "No hay archivos para reproducir",
                "El medio seleccionado no tiene ningún archivo indexado.",
            );
        }
    }

    fn play_episode(&self, file_path: &str) -> Result<(), std::boxed::Box<dyn std::error::Error>> {
        let media_id = {
            let mut conn = open_db(&self.db_path)?;
            let mut repo = MediaRepository::new(&mut conn);
            let media_id = repo.find_media_id_by_file(file_path)?;
            close_db(conn);
            media_id
        };

        crate::core::player::play_external(&self.db_path, media_id, file_path)
    }

    /// Sincroniza metadatos y pósters en segundo plano y refresca la UI al terminar.
    fn start_metadata_sync(this: &MainWindowRef, db_path: String) {
        if this.borrow().metadata_sync_running.get() {
            return;
        }
        this.borrow().metadata_sync_running.set(true);

        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let result = crate::core::metadata::MetadataService::sync_missing_metadata(&db_path);
            let _ = sender.send(result);
        });

        let this = this.clone();
        glib::timeout_add_local(Duration::from_millis(200), move || {
            match receiver.try_recv() {
                Ok(Ok(changed)) => {
                    this.borrow().metadata_sync_running.set(false);
                    if changed > 0 {
                        Self::refresh_views(&this);
                    }
                    glib::ControlFlow::Break
                }
                Ok(Err(error)) => {
                    eprintln!("Error sincronizando metadatos: {}", error);
                    this.borrow().metadata_sync_running.set(false);
                    glib::ControlFlow::Break
                }
                Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(TryRecvError::Disconnected) => {
                    this.borrow().metadata_sync_running.set(false);
                    glib::ControlFlow::Break
                }
            }
        });
    }

    fn refresh_views(this: &MainWindowRef) {
        let (db_path, visible, media_id) = {
            let w = this.borrow();
            (
                w.db_path.clone(),
                w.stack.visible_child_name().map(|name| name.to_string()),
                w.media_details.borrow().media_id(),
            )
        };

        Self::load_media(this, &db_path);

        if visible.as_deref() == Some("details") && media_id > 0 {
            this.borrow().show_details(media_id);
        }
    }

    fn show_error(&self, message: &str, detail: &str) {
        let dialog = gtk4::MessageDialog::builder()
            .transient_for(&self.window)
            .modal(true)
            .message_type(gtk4::MessageType::Error)
            .text(message)
            .secondary_text(detail)
            .build();
        dialog.present();
    }

    pub fn widget(&self) -> &AdwApplicationWindow {
        &self.window
    }
}
