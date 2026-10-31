//! Ventana principal de Acryl con GTK4 / Libadwaita.

use crate::core::metadata::RefreshMetadataResult;
use crate::db::media_repo::ListMediaOptions;
use crate::db::{MediaRepository, close_db, open_db};
use crate::ui::media_details::MediaDetails;
use crate::ui::media_list::MediaList;
use crate::ui::search_bar::SearchBar;
use crate::ui::settings_window::SettingsView;
use gtk4::prelude::*;
use gtk4::{Box, Button, Label, Orientation, ScrolledWindow, Stack, gio, glib};
use libadwaita::prelude::*;
use libadwaita::{
    ApplicationWindow as AdwApplicationWindow, Clamp, StatusPage, Toast, ToastOverlay,
};
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
    library_status: Label,
    config: Rc<RefCell<crate::config::AppConfig>>,
    media_count: usize,
    db_path: String,
    metadata_sync_running: Rc<Cell<bool>>,
    toast_overlay: ToastOverlay,
}

type MainWindowRef = Rc<RefCell<MainWindow>>;

impl MainWindow {
    pub fn new(app: &gtk4::Application, config: crate::Config) -> MainWindowRef {
        crate::ui::theme::install_theme_css();

        let db_path = config.inner().db_path.clone();
        let config_path = config.path().to_path_buf();

        let window = AdwApplicationWindow::builder()
            .application(app)
            .title("Acryl")
            .default_width(1000)
            .default_height(700)
            .build();

        // AdwToastOverlay permite mostrar notificaciones temporales (AdwToast)
        // en la parte inferior de la ventana sin bloquear la UI.
        let toast_overlay = ToastOverlay::new();

        let content = Box::new(Orientation::Vertical, 0);
        content.set_vexpand(true);
        toast_overlay.set_child(Some(&content));
        window.set_content(Some(&toast_overlay));

        // Barra superior al estilo Crunchyroll: logo a la izquierda,
        // búsqueda en el centro, ajustes a la derecha.
        let top_bar = Box::new(Orientation::Horizontal, 0);
        top_bar.set_hexpand(true);
        top_bar.set_halign(gtk4::Align::Fill);
        top_bar.add_css_class("top-bar");

        // Logo / nombre de la app a la izquierda.
        let logo_label = Label::new(Some("Acryl"));
        logo_label.set_halign(gtk4::Align::Start);
        logo_label.set_valign(gtk4::Align::Center);
        logo_label.set_margin_start(16);
        logo_label.set_margin_end(24);
        logo_label.set_xalign(0.0);
        logo_label.set_hexpand(false);
        logo_label.set_markup("<b><span size='large' foreground='#f75c03'>A</span>cryl</b>");
        top_bar.append(&logo_label);

        // Barra de búsqueda central.
        let search_bar = SearchBar::new(&db_path);
        search_bar.widget().set_hexpand(true);
        search_bar.widget().set_margin_start(0);
        search_bar.widget().set_margin_end(0);
        top_bar.append(search_bar.widget());

        // Indicador de biblioteca y botón de ajustes a la derecha.
        let right_box = Box::new(Orientation::Horizontal, 0);
        right_box.set_halign(gtk4::Align::End);
        right_box.set_valign(gtk4::Align::Center);
        right_box.set_margin_end(16);

        let library_status = Label::new(None);
        library_status.add_css_class("dim-label");
        library_status.set_valign(gtk4::Align::Center);
        library_status.set_margin_end(16);
        right_box.append(&library_status);

        let settings_button = Button::with_label("⚙ Ajustes");
        settings_button.set_halign(gtk4::Align::End);
        settings_button.set_hexpand(false);
        settings_button.set_valign(gtk4::Align::Center);
        settings_button.add_css_class("flat-button");
        right_box.append(&settings_button);

        top_bar.append(&right_box);

        content.append(&top_bar);

        let stack = Stack::new();
        stack.set_hexpand(true);
        stack.set_vexpand(true);
        content.append(&stack);

        let media_list = MediaList::new();

        // AdwClamp limita el ancho del contenido en pantallas anchas para que
        // las listas y pósters no se estiren desproporcionadamente.
        let list_clamp = Clamp::new();
        list_clamp.set_maximum_size(900);
        list_clamp.set_tightening_threshold(700);
        list_clamp.set_child(Some(media_list.widget()));

        // Página principal: lista de medios (envuelta en Clamp).
        stack.add_titled(&list_clamp, Some("list"), "Lista");

        // Página vacía: AdwStatusPage cuando no hay medios indexados.
        let empty_page = StatusPage::new();
        empty_page.set_title("Sin medios indexados");
        empty_page.set_description(Some(
            "Agrega una carpeta de videos en Ajustes para comenzar a indexar tu biblioteca.",
        ));
        empty_page.set_icon_name(Some("media-library-symbolic"));
        empty_page.set_vexpand(true);
        empty_page.set_hexpand(true);
        stack.add_titled(&empty_page, Some("empty"), "Vacía");

        let media_details = Rc::new(RefCell::new(MediaDetails::new(&db_path, 0)));
        media_details.borrow_mut().set_db_path(&db_path);

        // La vista de detalles puede crecer con la sinopsis y los episodios:
        // se envuelve en un ScrolledWindow para que el póster grande no recorte
        // el resto del contenido.
        let details_scrolled = ScrolledWindow::new();
        details_scrolled.set_hexpand(true);
        details_scrolled.set_vexpand(true);
        details_scrolled.set_hscrollbar_policy(gtk4::PolicyType::Never);
        details_scrolled.set_child(Some(media_details.borrow().widget()));
        stack.add_titled(&details_scrolled, Some("details"), "Detalles");

        let config_rc = Rc::new(RefCell::new(config.inner().clone()));

        let settings_view =
            SettingsView::new(config.inner(), &db_path, config_path.as_path(), &window);
        stack.add_titled(settings_view.widget(), Some("settings"), "Ajustes");

        stack.set_visible_child_name("list");

        let this = Rc::new(RefCell::new(Self {
            window,
            stack,
            search_bar,
            settings_button,
            media_list,
            media_details,
            settings_view,
            library_status,
            config: config_rc.clone(),
            media_count: 0,
            db_path: db_path.clone(),
            metadata_sync_running: Rc::new(Cell::new(false)),
            toast_overlay,
        }));

        this.borrow().window.present();

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

        // Callback: cambios inmediatos de carpetas (ya persistidos por la vista).
        let this_clone = this.clone();
        this.borrow_mut()
            .settings_view
            .on_config_changed(move |new_config| {
                let w = this_clone.borrow();
                *w.config.borrow_mut() = new_config;
                Self::update_library_status(&w);
            });

        // Callback: la indexación terminó -> recargar la galería.
        let this_clone = this.clone();
        this.borrow_mut().settings_view.on_library_changed(move || {
            let db_path = this_clone.borrow().db_path.clone();
            Self::load_media(&this_clone, &db_path);
            let this = this_clone.clone();
            glib::timeout_add_local(Duration::from_millis(500), move || {
                let w = this.borrow();
                w.show_toast("Indexación completada");
                glib::ControlFlow::Break
            });
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

        // Conectar botón de metadatos -> confirma y lanza la actualización forzada.
        let this_clone = this.clone();
        this.borrow_mut()
            .settings_view
            .on_refresh_metadata(move || {
                Self::confirm_metadata_refresh(&this_clone);
            });

        // Callbacks de la vista de detalles.
        let this_clone = this.clone();
        this.borrow_mut()
            .media_details
            .borrow_mut()
            .on_back(move || {
                let parent_id = this_clone.borrow().media_details.borrow().parent_id();
                let w = this_clone.borrow();
                if let Some(parent_id) = parent_id
                    && w.show_details(parent_id)
                {
                    return;
                }
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
            .on_rate(move |rating| {
                let this = this_clone.clone();
                let w = this.borrow();
                let media_id = w.media_details.borrow().media_id();
                let db_path = w.db_path.clone();
                let mut conn = match open_db(&db_path) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Error abriendo BD: {}", e);
                        return;
                    }
                };
                let mut repo = MediaRepository::new(&mut conn);
                if let Err(e) = repo.set_rating(media_id, rating) {
                    eprintln!("Error calificando medio: {}", e);
                }
                close_db(conn);
                // Refrescar para mostrar la nueva calificación.
                w.media_details.borrow_mut().load_media(media_id);
                w.show_toast(&format!("Calificación: {}★", rating));
            });

        // Callback: reproducir el archivo principal (episodio por ver o primero).
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

        // Callback: abrir el selector de audio y subtítulos del archivo principal.
        let this_clone = this.clone();
        this.borrow_mut()
            .media_details
            .borrow_mut()
            .on_edit_tracks(move |file_path| {
                let this = this_clone.clone();
                let w = this.borrow();
                crate::ui::track_dialog::show_track_dialog(
                    &w.window,
                    &w.db_path,
                    &file_path,
                    Rc::new({
                        let this_for_toast = this.clone();
                        move || {
                            let w2 = this_for_toast.borrow();
                            w2.show_toast("Pistas guardadas");
                        }
                    }),
                );
            });

        // Callback: cambiar de temporada (navegar a los detalles de esa temporada).
        // Se difiere con timeout_add_local para evitar reentrancy de RefCell
        // (el notify del dropdown se dispara dentro de set_media, que ya tiene
        // borrow_mut de media_details).
        let this_clone = this.clone();
        this.borrow_mut()
            .media_details
            .borrow_mut()
            .on_change_season(move |season_id| {
                let this = this_clone.clone();
                glib::timeout_add_local(std::time::Duration::from_millis(0), move || {
                    let w = this.borrow();
                    w.show_details(season_id);
                    glib::ControlFlow::Break
                });
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

        let media_count = media.len();
        let mut w = this.borrow_mut();
        w.media_count = media_count;
        w.media_list.set_media(media);

        // Alternar entre la lista de medios y la página vacía (AdwStatusPage)
        // según si hay medios indexados.
        if media_count > 0 {
            w.stack.set_visible_child_name("list");
        } else {
            w.stack.set_visible_child_name("empty");
        }

        Self::update_library_status(&w);
    }

    /// Muestra una notificación temporal (AdwToast) en la parte inferior de la
    /// ventana. Se usa para confirmar acciones al usuario sin bloquear la UI.
    fn show_toast(&self, message: &str) {
        let toast = Toast::new(message);
        toast.set_timeout(3000);
        self.toast_overlay.add_toast(toast);
    }

    /// Actualiza el indicador de carpetas y medios de la barra superior.
    fn update_library_status(w: &MainWindow) {
        let config = w.config.borrow();
        let folder_count = config.media_folders.len();

        let text = match folder_count {
            0 => "Sin carpetas indexadas".to_string(),
            1 => format!("{} medios · 1 carpeta", w.media_count),
            total => format!("{} medios · {} carpetas", w.media_count, total),
        };
        w.library_status.set_label(&text);

        let tooltip = if config.media_folders.is_empty() {
            None
        } else {
            Some(config.media_folders.join("\n"))
        };
        w.library_status.set_tooltip_text(tooltip.as_deref());
    }

    /// Muestra los detalles de un medio. Devuelve `false` si el medio no
    /// existe, para que la navegación pueda volver a la lista.
    fn show_details(&self, media_id: i64) -> bool {
        let mut conn = match open_db(&self.db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error abriendo BD: {}", e);
                return false;
            }
        };
        let mut repo = MediaRepository::new(&mut conn);
        let media = repo.find_by_id(media_id).unwrap_or(None);
        close_db(conn);

        if let Some(media) = media {
            self.media_details.borrow_mut().set_media(&media);
            self.stack.set_visible_child_name("details");
            true
        } else {
            false
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

    /// Pide confirmación antes de sobrescribir todos los metadatos indexados.
    fn confirm_metadata_refresh(this: &MainWindowRef) {
        if this.borrow().metadata_sync_running.get() {
            this.borrow()
                .settings_view
                .set_metadata_status("Ya hay una sincronización de metadatos en curso.");
            return;
        }

        let dialog = gtk4::AlertDialog::builder()
            .modal(true)
            .message("¿Actualizar todos los metadatos?")
            .detail(
                "Se consultará TVmaze (y AniList para anime) y se sobrescribirán el título, el año, la sinopsis, el póster y el banner de todos los medios indexados. Esta acción no se puede deshacer.",
            )
            .buttons(["Cancelar", "Actualizar"])
            .cancel_button(0)
            .default_button(0)
            .build();

        let this = this.clone();
        let window = this.borrow().window.clone();
        dialog.choose(Some(&window), None::<&gio::Cancellable>, move |response| {
            if matches!(response, Ok(1)) {
                Self::start_metadata_refresh(&this);
            }
        });
    }

    /// Fuerza la actualización de metadatos en segundo plano y muestra el
    /// progreso en Ajustes sin bloquear la UI.
    fn start_metadata_refresh(this: &MainWindowRef) {
        if this.borrow().metadata_sync_running.get() {
            this.borrow()
                .settings_view
                .set_metadata_status("Ya hay una sincronización de metadatos en curso.");
            return;
        }

        let db_path = this.borrow().db_path.clone();
        this.borrow().metadata_sync_running.set(true);
        {
            let w = this.borrow();
            w.settings_view.set_metadata_refreshing(true);
            w.settings_view
                .set_metadata_status("Consultando TVmaze y actualizando metadatos...");
        }

        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let result = crate::core::metadata::MetadataService::refresh_all_metadata(&db_path);
            let _ = sender.send(result);
        });

        let this = this.clone();
        glib::timeout_add_local(Duration::from_millis(200), move || {
            match receiver.try_recv() {
                Ok(Ok(result)) => {
                    {
                        let w = this.borrow();
                        w.metadata_sync_running.set(false);
                        w.settings_view.set_metadata_refreshing(false);
                        w.settings_view
                            .set_metadata_status(&metadata_refresh_summary(&result));
                    }
                    if result.updated > 0 {
                        Self::refresh_views(&this);
                    }
                    glib::ControlFlow::Break
                }
                Ok(Err(error)) => {
                    {
                        let w = this.borrow();
                        w.metadata_sync_running.set(false);
                        w.settings_view.set_metadata_refreshing(false);
                        w.settings_view
                            .set_metadata_status("No se pudieron actualizar los metadatos.");
                        w.show_error(
                            "No se pudieron actualizar los metadatos",
                            &error.to_string(),
                        );
                    }
                    glib::ControlFlow::Break
                }
                Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(TryRecvError::Disconnected) => {
                    let w = this.borrow();
                    w.metadata_sync_running.set(false);
                    w.settings_view.set_metadata_refreshing(false);
                    w.settings_view
                        .set_metadata_status("La actualización se interrumpió inesperadamente.");
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
        gtk4::AlertDialog::builder()
            .modal(true)
            .message(message)
            .detail(detail)
            .build()
            .show(Some(&self.window));
    }

    pub fn widget(&self) -> &AdwApplicationWindow {
        &self.window
    }
}

/// Resumen legible del resultado de una actualización forzada de metadatos.
fn metadata_refresh_summary(result: &RefreshMetadataResult) -> String {
    if result.total == 0 {
        return "No hay medios indexados que actualizar.".to_string();
    }

    let mut summary = format!(
        "Metadatos actualizados en {} de {} medios.",
        result.updated, result.total
    );
    if result.failed > 0 {
        summary.push_str(&format!(
            " {} no se pudieron actualizar (sin resultados en TVmaze o error de red).",
            result.failed
        ));
    }
    if result.poster_failed > 0 {
        summary.push_str(&format!(
            " {} pósters no se pudieron descargar; se conservó el anterior.",
            result.poster_failed
        ));
    }
    if result.banner_failed > 0 {
        summary.push_str(&format!(
            " {} banners no se pudieron descargar; se conservó el anterior.",
            result.banner_failed
        ));
    }
    summary
}
