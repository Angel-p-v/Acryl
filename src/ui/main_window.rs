//! Ventana principal de Acryl con GTK4 / Libadwaita.

use crate::core::metadata::RefreshMetadataResult;
use crate::db::media_repo::ListMediaOptions;
use crate::db::{MediaRepository, close_db, open_db};
use crate::ui::i18n::{Translations, TranslationsRef, load_translations};
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
    translations: TranslationsRef,
    /// Lista de categorías de la página de filtro.
    categories_list: Box,
    /// Botón "Ver todo" para limpiar el filtro de categoría.
    clear_category_button: Button,
    /// Página de la que se abrió Ajustes, para volver a ella (no siempre al
    /// inicio) al pulsar Volver.
    previous_page: Rc<Cell<Option<String>>>,
    /// Categoría activa como filtro de la biblioteca (`None` = sin filtro).
    current_category: Rc<RefCell<Option<i64>>>,
    /// Nombre de la categoría activa, para mostrarlo en la interfaz.
    current_category_name: Rc<RefCell<Option<String>>>,
}

type MainWindowRef = Rc<RefCell<MainWindow>>;

impl MainWindow {
    pub fn new(app: &gtk4::Application, config: crate::Config) -> MainWindowRef {
        crate::ui::theme::install_theme_css();

        let db_path = config.inner().db_path.clone();
        let config_path = config.path().to_path_buf();

        let language = config.inner().language.clone();
        let translations = load_translations(&language)
            .unwrap_or_else(|_| Rc::new(RefCell::new(Translations::default())));
        let t = {
            let t_ref = translations.clone();
            move |key: &str| -> String { t_ref.borrow().t(key) }
        };

        let window = AdwApplicationWindow::builder()
            .application(app)
            .title("Acryl")
            .default_width(1200)
            .default_height(800)
            .build();

        // AdwToastOverlay permite mostrar notificaciones temporales (AdwToast)
        // en la parte inferior de la ventana sin bloquear la UI.
        let toast_overlay = ToastOverlay::new();

        let content = Box::new(Orientation::Vertical, 0);
        content.set_hexpand(true);
        content.set_vexpand(true);
        toast_overlay.set_child(Some(&content));
        window.set_content(Some(&toast_overlay));

        // Barra superior al estilo Crunchyroll: logo a la izquierda,
        // navegación en el centro, acciones y búsqueda a la derecha.
        let top_bar = Box::new(Orientation::Horizontal, 0);
        top_bar.set_hexpand(true);
        top_bar.set_vexpand(false);
        top_bar.set_halign(gtk4::Align::Fill);
        top_bar.set_valign(gtk4::Align::Center);
        top_bar.add_css_class("top-bar");

        // Sección izquierda: logo / nombre de la app (navega al inicio al
        // hacer clic).
        let left_section = Box::new(Orientation::Horizontal, 0);
        left_section.set_halign(gtk4::Align::Start);
        left_section.set_valign(gtk4::Align::Center);
        left_section.set_hexpand(false);

        let logo_label = Label::new(Some("Acryl"));
        logo_label.set_halign(gtk4::Align::Start);
        logo_label.set_valign(gtk4::Align::Center);
        logo_label.set_margin_start(16);
        logo_label.set_margin_end(24);
        logo_label.set_xalign(0.0);
        logo_label.set_hexpand(false);
        logo_label.set_markup("<b><span size='x-large' foreground='#f75c03'>A</span>cryl</b>");
        left_section.append(&logo_label);

        top_bar.append(&left_section);

        // Sección centro: navegación horizontal (se expande para centrar
        // los elementos sin dejar huecos entre ellos).
        let nav_box = Box::new(Orientation::Horizontal, 8);
        nav_box.set_halign(gtk4::Align::Center);
        nav_box.set_valign(gtk4::Align::Center);
        nav_box.set_hexpand(true);
        nav_box.add_css_class("nav-box");

        top_bar.append(&nav_box);

        // Sección derecha: búsqueda y acciones.
        let right_section = Box::new(Orientation::Horizontal, 0);
        right_section.set_halign(gtk4::Align::End);
        right_section.set_valign(gtk4::Align::Center);
        right_section.set_hexpand(false);
        right_section.set_margin_end(16);

        // Barra de búsqueda.
        let search_bar = SearchBar::new(&db_path);
        search_bar.widget().set_hexpand(false);
        search_bar.widget().set_width_request(300);
        search_bar.widget().set_halign(gtk4::Align::Center);
        search_bar.widget().set_valign(gtk4::Align::Center);
        right_section.append(search_bar.widget());

        // Indicador de biblioteca.
        let library_status = Label::new(None);
        library_status.add_css_class("dim-label");
        library_status.set_valign(gtk4::Align::Center);
        library_status.set_margin_start(16);
        library_status.set_margin_end(16);
        right_section.append(&library_status);

        // Botón de ajustes.
        let settings_button = Button::with_label(&t("button.settings"));
        settings_button.set_halign(gtk4::Align::End);
        settings_button.set_hexpand(false);
        settings_button.set_valign(gtk4::Align::Center);
        settings_button.add_css_class("flat-button");
        right_section.append(&settings_button);

        top_bar.append(&right_section);

        content.append(&top_bar);

        let stack = Stack::new();
        stack.set_hexpand(true);
        stack.set_vexpand(true);
        content.append(&stack);

        let media_list = MediaList::new();

        // AdwClamp limita el ancho del contenido en pantallas anchas para que
        // las listas y pósters no se estiren desproporcionadamente.
        let list_clamp = Clamp::new();
        list_clamp.set_maximum_size(1600);
        list_clamp.set_tightening_threshold(1200);
        list_clamp.set_child(Some(media_list.widget()));

        // Página principal: lista de medios (envuelta en Clamp).
        stack.add_titled(&list_clamp, Some("list"), "Lista");

        // Configurar columnas responsivas para la galería según el ancho
        // inicial de la ventana. Se calcula restando los márgenes del
        // FlowBox y del Clamp.
        media_list.set_responsive_columns(1160);

        // Página vacía: AdwStatusPage cuando no hay medios indexados.
        let empty_page = StatusPage::new();
        empty_page.set_title(&t("empty.title"));
        empty_page.set_description(Some(&t("empty.description")));
        empty_page.set_icon_name(Some("media-library-symbolic"));
        empty_page.set_vexpand(true);
        empty_page.set_hexpand(true);
        stack.add_titled(&empty_page, Some("empty"), "Vacía");

        // --- Página de categorías: selección y filtrado de la biblioteca ---
        let categories_page = Box::new(Orientation::Vertical, 0);
        categories_page.set_hexpand(true);
        categories_page.set_vexpand(true);

        let categories_title = Label::new(Some(&t("stack.categories")));
        categories_title.set_halign(gtk4::Align::Start);
        categories_title.set_margin_top(20);
        categories_title.set_margin_start(24);
        categories_title.set_margin_bottom(12);
        categories_title.add_css_class("heading");
        categories_page.append(&categories_title);

        let categories_scrolled = ScrolledWindow::new();
        categories_scrolled.set_hexpand(true);
        categories_scrolled.set_vexpand(true);
        categories_scrolled.set_hscrollbar_policy(gtk4::PolicyType::Never);
        categories_scrolled.set_vscrollbar_policy(gtk4::PolicyType::Automatic);

        let categories_list = Box::new(Orientation::Vertical, 6);
        categories_list.set_hexpand(true);
        categories_list.set_margin_start(24);
        categories_list.set_margin_end(24);
        categories_scrolled.set_child(Some(&categories_list));
        categories_page.append(&categories_scrolled);

        // Botón para limpiar el filtro y mostrar todos los medios.
        let clear_category_button = Button::with_label(&t("button.clear_category"));
        clear_category_button.add_css_class("flat-button");
        clear_category_button.set_halign(gtk4::Align::Start);
        clear_category_button.set_margin_start(24);
        clear_category_button.set_margin_bottom(16);
        clear_category_button.set_visible(false);
        categories_page.append(&clear_category_button);

        stack.add_titled(&categories_page, Some("categories"), "Categorías");

        let media_details = Rc::new(RefCell::new(MediaDetails::new(
            &db_path,
            0,
            translations.clone(),
        )));
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
            translations,
            categories_list,
            clear_category_button,
            previous_page: Rc::new(Cell::new(None)),
            current_category: Rc::new(RefCell::new(None)),
            current_category_name: Rc::new(RefCell::new(None)),
        }));

        this.borrow().window.present();

        // Cargar medios iniciales (SQLite local, rápido).
        Self::load_media(&this, &db_path);

        // Logo: acceso directo a la pantalla principal desde cualquier vista.
        let this_logo = this.clone();
        let logo_click = gtk4::GestureClick::new();
        logo_click.connect_pressed(move |_, _, _, _| {
            let w = this_logo.borrow();
            w.go_home();
        });
        logo_label.add_controller(logo_click);

        // Botón "Categorías": abre el selector de categorías para filtrar.
        let categories_button = Button::with_label(&t("button.categories"));
        categories_button.add_css_class("nav-button");
        categories_button.set_valign(gtk4::Align::Center);
        let this_cat = this.clone();
        categories_button.connect_clicked(move |_| {
            let w = this_cat.borrow();
            w.stack.set_visible_child_name("categories");
            drop(w);
            Self::populate_categories_page(&this_cat);
        });
        nav_box.append(&categories_button);

        // Callback: botón "Ver todo" -> quita el filtro de categoría.
        let this_clear = this.clone();
        this.borrow_mut()
            .clear_category_button
            .connect_clicked(move |_| {
                {
                    let w = this_clear.borrow();
                    w.current_category.replace(None);
                    w.current_category_name.replace(None);
                }
                Self::load_media_by_category(&this_clear, None);
                let w = this_clear.borrow();
                w.stack.set_visible_child_name("list");
                w.media_list.refresh();
            });

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

        // Callback: botón de Ajustes -> navega a la vista de ajustes dentro del Stack,
        // recordando la página de origen para poder volver a ella.
        let this_clone = this.clone();
        this.borrow_mut().settings_button.connect_clicked(move |_| {
            let w = this_clone.borrow();
            let current = w.stack.visible_child_name().map(|name| name.to_string());
            w.previous_page.set(current);
            w.stack.set_visible_child_name("settings");
        });

        // Callback: Guardar ajustes -> guarda en disco, limpia medios huérfanos
        // y vuelve a la lista. Aplica el idioma en caliente sin reiniciar.
        let config_rc_save = config_rc.clone();
        let config_path_save = config_path.clone();
        let db_path_save = db_path.clone();
        let this_save = this.clone();
        this.borrow_mut().settings_view.on_save(move |new_config| {
            let folders = new_config.media_folders.clone();
            let language = new_config.language.clone();
            let config_path_clone = config_path_save.clone();
            let _ = crate::config::Config {
                path: config_path_clone,
                inner: new_config.clone(),
            }
            .save();
            {
                let mut config = config_rc_save.borrow_mut();
                *config = new_config;
            }

            // Eliminar medios de carpetas que ya no están en el config
            let _ = crate::core::library::cleanup_media_not_in_folders(&db_path_save, &folders);

            // Aplicar el idioma en caliente y refrescar las vistas visibles.
            let _ = Self::set_language(&this_save, &language);
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

        // Conectar botón Volver de ajustes -> vuelve a la página de origen
        // (serie, película, temporada o lista), no siempre al inicio.
        let this_clone = this.clone();
        this.borrow_mut()
            .settings_view
            .cancel_button()
            .connect_clicked(move |_| {
                let w = this_clone.borrow();
                w.go_back_to_previous();
            });

        // Conectar botón Guardar de ajustes -> guarda, recarga medios y vuelve
        // a la página de origen (no siempre al inicio).
        let this_clone = this.clone();
        let db_path_for_reload = db_path.clone();
        this.borrow_mut()
            .settings_view
            .save_button()
            .connect_clicked(move |_| {
                let w = this_clone.borrow();
                // Recargar medios después de guardar ajustes (pueden haber
                // cambiado las carpetas).
                drop(w);
                Self::load_media(&this_clone, &db_path_for_reload);
                Self::start_metadata_sync(&this_clone, db_path_for_reload.clone());
                // Volver a la página de origen después de guardar.
                let w = this_clone.borrow();
                w.go_back_to_previous();
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

        let this_clone = this.clone();
        this.borrow_mut()
            .media_details
            .borrow_mut()
            .on_change_season(move |season_id| {
                let this = this_clone.clone();
                glib::timeout_add_local(std::time::Duration::from_millis(0), move || {
                    let w = this.borrow();
                    // Recargar únicamente los episodios de la temporada
                    // seleccionada, sin mezclar temporadas distintas.
                    w.media_details
                        .borrow_mut()
                        .load_episodes_for_season(season_id);
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

    /// Traduce una clave de i18n al idioma actual.
    fn t(&self, key: &str) -> String {
        self.translations.borrow().t(key)
    }

    /// Recarga el diccionario de traducciones para `language` y refresca las
    /// vistas visibles para que el cambio de idioma se aplique sin reiniciar.
    ///
    /// El borrow de `this` se acota y se libera antes de llamar a
    /// `refresh_views` (que necesita `borrow_mut`), evitando
    /// `RefCell already borrowed`.
    fn set_language(
        this: &MainWindowRef,
        language: &str,
    ) -> Result<(), std::boxed::Box<dyn std::error::Error>> {
        {
            let w = this.borrow();
            w.translations.borrow_mut().set_language(language)?;
            w.show_toast(&w.t("status.language_applied"));
        }
        Self::refresh_views(this);
        Ok(())
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

    /// Navega a la pantalla principal de la biblioteca, limpiando filtros o
    /// estado temporal. Se usa al pulsar el logo desde cualquier pantalla.
    fn go_home(&self) {
        // Limpiar el buscador para mostrar la biblioteca completa.
        self.search_bar.clear_query();
        self.stack.set_visible_child_name("list");
        self.media_list.refresh();
    }

    /// Vuelve a la página de origen de Ajustes (serie, película, temporada o
    /// lista). Si no hay página registrada, vuelve a la lista principal.
    fn go_back_to_previous(&self) {
        let previous = self.previous_page.take();
        match previous.as_deref() {
            Some("details") => {
                let media_id = self.media_details.borrow().media_id();
                if media_id > 0 {
                    self.show_details(media_id);
                } else {
                    self.show_list();
                }
            }
            Some(page) => {
                self.stack.set_visible_child_name(page);
            }
            None => {
                self.show_list();
            }
        }
    }

    /// Rellena la página de categorías con la lista disponible en la BD y
    /// actualiza el estado visual del filtro activo.
    fn populate_categories_page(this: &MainWindowRef) {
        let w = this.borrow();
        let db_path = w.db_path.clone();
        let active_id = *w.current_category.borrow();
        drop(w);

        let mut conn = match open_db(&db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error abriendo BD para categorías: {}", e);
                return;
            }
        };
        let mut repo = MediaRepository::new(&mut conn);
        let categories = repo.list_categories().unwrap_or_default();
        close_db(conn);

        let w = this.borrow();
        // Limpiar lista existente.
        while let Some(child) = w.categories_list.first_child() {
            w.categories_list.remove(&child);
        }

        // Botón "Todas" para quitar el filtro.
        let all_button = Button::with_label("Todas las categorías");
        all_button.add_css_class("nav-button");
        all_button.set_hexpand(false);
        all_button.set_valign(gtk4::Align::Center);
        if active_id.is_some() {
            all_button.add_css_class("active");
        }
        let this_all = this.clone();
        all_button.connect_clicked(move |_| {
            {
                let w = this_all.borrow();
                w.current_category.replace(None);
                w.current_category_name.replace(None);
            }
            Self::load_media_by_category(&this_all, None);
            let w = this_all.borrow();
            w.stack.set_visible_child_name("list");
            w.media_list.refresh();
        });
        w.categories_list.append(&all_button);

        for category in categories.iter() {
            let btn = Button::with_label(&category.name);
            btn.add_css_class("nav-button");
            btn.set_hexpand(false);
            btn.set_valign(gtk4::Align::Center);
            if active_id == Some(category.id) {
                btn.add_css_class("active");
            }
            let this_cat = this.clone();
            let cat_id = category.id;
            let cat_name = category.name.clone();
            btn.connect_clicked(move |_| {
                // Primero actualizamos el estado y soltamos el borrow antes de
                // llamar a load_media_by_category (que usa borrow_mut).
                {
                    let w = this_cat.borrow();
                    w.current_category.replace(Some(cat_id));
                    w.current_category_name.replace(Some(cat_name.clone()));
                }
                Self::load_media_by_category(&this_cat, Some(cat_id));
                let w = this_cat.borrow();
                w.stack.set_visible_child_name("list");
                w.media_list.refresh();
            });
            w.categories_list.append(&btn);
        }

        // Mostrar u ocultar el botón "Ver todo" según si hay filtro activo.
        let active = w.current_category.borrow().is_some();
        if active {
            w.clear_category_button.set_visible(true);
        } else {
            w.clear_category_button.set_visible(false);
        }
        drop(w);
    }

    /// Carga los medios filtrados por categoría (o todos si es None) en la
    /// galería y actualiza el contador.
    fn load_media_by_category(this: &MainWindowRef, category_id: Option<i64>) {
        let db_path = {
            let w = this.borrow();
            w.db_path.clone()
        };
        let mut conn = match open_db(&db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error abriendo BD: {}", e);
                return;
            }
        };
        let mut repo = MediaRepository::new(&mut conn);
        let media = match category_id {
            Some(id) => repo
                .list_by_category(
                    id,
                    crate::db::media_repo::ListMediaOptions {
                        r#type: None,
                        limit: Some(500),
                        offset: Some(0),
                        order_by: None,
                    },
                )
                .unwrap_or_default(),
            None => repo
                .list(crate::db::media_repo::ListMediaOptions {
                    r#type: None,
                    limit: Some(500),
                    offset: Some(0),
                    order_by: None,
                })
                .unwrap_or_default(),
        };
        close_db(conn);

        let media_count = media.len();
        let mut w = this.borrow_mut();
        w.media_count = media_count;
        w.media_list.set_media(media);
        Self::update_library_status(&w);
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
