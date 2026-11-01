//! Ventana principal de Acryl con GTK4 / Libadwaita.

use crate::core::metadata::RefreshMetadataResult;
use crate::db::media_repo::ListMediaOptions;
use crate::db::{MediaRepository, close_db, open_db};
use crate::ui::i18n::{Translations, TranslationsRef, load_translations};
use crate::ui::media_details::{MediaDetails, SeasonSelection};
use crate::ui::media_list::MediaList;
use crate::ui::search_bar::SearchBar;
use crate::ui::settings_window::SettingsView;
use gtk4::prelude::*;
use gtk4::{Box, Button, CenterBox, FlowBox, Label, Orientation, ScrolledWindow, Stack, gio, glib};
use libadwaita::prelude::*;
use libadwaita::{ApplicationWindow as AdwApplicationWindow, StatusPage, Toast, ToastOverlay};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc::{self, TryRecvError};
use std::time::Duration;

/// Vista recordada en la pila de navegación. Permite que «Volver» y la salida
/// de Ajustes restauren exactamente la pantalla anterior, incluida la
/// temporada seleccionada de una serie.
#[derive(Debug, Clone, PartialEq)]
enum View {
    Page(String),
    Details {
        media_id: i64,
        season: Option<SeasonSelection>,
    },
}

pub struct MainWindow {
    window: AdwApplicationWindow,
    stack: Stack,
    search_bar: SearchBar,
    settings_button: Button,
    categories_button: Button,
    media_list: MediaList,
    media_details: Rc<RefCell<MediaDetails>>,
    settings_view: SettingsView,
    library_status: Label,
    empty_page: StatusPage,
    config: Rc<RefCell<crate::config::AppConfig>>,
    media_count: usize,
    db_path: String,
    metadata_sync_running: Rc<Cell<bool>>,
    toast_overlay: ToastOverlay,
    translations: TranslationsRef,
    /// Chips de categorías de la página de filtro.
    categories_flow: FlowBox,
    /// Mensaje mostrado cuando todavía no hay categorías extraídas.
    categories_empty: Label,
    /// Título y subtítulo de la página de categorías.
    categories_title: Label,
    categories_subtitle: Label,
    /// Botón "Ver todo" para limpiar el filtro de categoría.
    clear_category_button: Button,
    /// Barra superior de la biblioteca que indica el filtro activo.
    filter_bar: Box,
    filter_label: Label,
    filter_clear_button: Button,
    /// Pila de vistas visitadas. El último elemento es la pantalla a la que
    /// debe volver el botón «Volver» o la salida de Ajustes.
    history: RefCell<Vec<View>>,
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
        // GtkCenterBox reparte las tres zonas sin márgenes manuales por
        // elemento: el centro queda centrado respecto a la ventana.
        let top_bar = CenterBox::new();
        top_bar.set_hexpand(true);
        top_bar.set_vexpand(false);
        top_bar.add_css_class("top-bar");

        // Sección izquierda: logo / nombre de la app (navega al inicio al
        // hacer clic).
        let left_section = Box::new(Orientation::Horizontal, 0);
        left_section.set_halign(gtk4::Align::Start);
        left_section.set_valign(gtk4::Align::Center);

        let logo_label = Label::new(None);
        logo_label.set_halign(gtk4::Align::Start);
        logo_label.set_valign(gtk4::Align::Center);
        logo_label.set_markup("<b><span size='x-large' foreground='#f75c03'>A</span>cryl</b>");

        let logo_button = Button::new();
        logo_button.add_css_class("logo-button");
        logo_button.set_valign(gtk4::Align::Center);
        logo_button.set_tooltip_text(Some(&t("tooltip.home")));
        logo_button.set_child(Some(&logo_label));
        left_section.append(&logo_button);
        top_bar.set_start_widget(Some(&left_section));

        // Sección centro: navegación principal. Solo Categorías; Novedades,
        // Popular y Simulcasts se retiraron al no tener datos reales.
        let nav_box = Box::new(Orientation::Horizontal, 0);
        nav_box.set_halign(gtk4::Align::Center);
        nav_box.set_valign(gtk4::Align::Center);

        top_bar.set_center_widget(Some(&nav_box));

        // Sección derecha: búsqueda y acciones.
        let right_section = Box::new(Orientation::Horizontal, 12);
        right_section.set_halign(gtk4::Align::End);
        right_section.set_valign(gtk4::Align::Center);

        // Barra de búsqueda.
        let search_bar = SearchBar::new(&db_path, translations.clone());
        search_bar.widget().set_hexpand(false);
        search_bar.widget().set_width_request(280);
        search_bar.widget().set_halign(gtk4::Align::Center);
        search_bar.widget().set_valign(gtk4::Align::Center);
        right_section.append(search_bar.widget());

        // Indicador de biblioteca.
        let library_status = Label::new(None);
        library_status.add_css_class("dim-label");
        library_status.set_valign(gtk4::Align::Center);
        right_section.append(&library_status);

        // Botón de ajustes.
        let settings_button = Button::with_label(&t("button.settings"));
        settings_button.set_halign(gtk4::Align::End);
        settings_button.set_hexpand(false);
        settings_button.set_valign(gtk4::Align::Center);
        settings_button.add_css_class("flat-button");
        right_section.append(&settings_button);

        top_bar.set_end_widget(Some(&right_section));

        content.append(&top_bar);

        let stack = Stack::new();
        stack.set_hexpand(true);
        stack.set_vexpand(true);
        content.append(&stack);

        let media_list = MediaList::new(translations.clone());

        // Página principal: barra de filtro activo + galería responsive.
        let list_page = Box::new(Orientation::Vertical, 0);
        list_page.set_hexpand(true);
        list_page.set_vexpand(true);

        let filter_bar = Box::new(Orientation::Horizontal, 8);
        filter_bar.add_css_class("filter-bar");
        filter_bar.set_halign(gtk4::Align::Fill);
        filter_bar.set_valign(gtk4::Align::Center);
        filter_bar.set_visible(false);

        let filter_label = Label::new(None);
        filter_label.set_halign(gtk4::Align::Start);
        filter_label.set_valign(gtk4::Align::Center);
        filter_label.set_hexpand(true);
        filter_label.set_xalign(0.0);
        filter_label.add_css_class("filter-label");
        filter_bar.append(&filter_label);

        let filter_clear_button = Button::with_label(&t("button.clear_category"));
        filter_clear_button.add_css_class("flat-button");
        filter_clear_button.set_valign(gtk4::Align::Center);
        filter_bar.append(&filter_clear_button);

        list_page.append(&filter_bar);
        list_page.append(media_list.widget());
        stack.add_titled(&list_page, Some("list"), &t("stack.list"));

        // Página vacía: AdwStatusPage cuando no hay medios indexados.
        let empty_page = StatusPage::new();
        empty_page.set_title(&t("empty.title"));
        empty_page.set_description(Some(&t("empty.description")));
        empty_page.set_icon_name(Some("media-library-symbolic"));
        empty_page.set_vexpand(true);
        empty_page.set_hexpand(true);
        stack.add_titled(&empty_page, Some("empty"), &t("stack.empty"));
        // --- Página de categorías: selección y filtrado de la biblioteca ---
        let categories_page = Box::new(Orientation::Vertical, 0);
        categories_page.set_hexpand(true);
        categories_page.set_vexpand(true);

        let categories_header = Box::new(Orientation::Horizontal, 12);
        categories_header.set_halign(gtk4::Align::Fill);
        categories_header.set_valign(gtk4::Align::Center);
        categories_header.set_margin_top(24);
        categories_header.set_margin_start(24);
        categories_header.set_margin_end(24);

        let categories_title = Label::new(Some(&t("stack.categories")));
        categories_title.set_halign(gtk4::Align::Start);
        categories_title.set_hexpand(true);
        categories_title.set_xalign(0.0);
        categories_title.add_css_class("page-title");
        categories_header.append(&categories_title);

        // Botón para limpiar el filtro y mostrar todos los medios.
        let clear_category_button = Button::with_label(&t("button.clear_category"));
        clear_category_button.add_css_class("flat-button");
        clear_category_button.set_halign(gtk4::Align::End);
        clear_category_button.set_valign(gtk4::Align::Center);
        clear_category_button.set_visible(false);
        categories_header.append(&clear_category_button);
        categories_page.append(&categories_header);

        let categories_subtitle = Label::new(Some(&t("categories.subtitle")));
        categories_subtitle.set_halign(gtk4::Align::Start);
        categories_subtitle.set_xalign(0.0);
        categories_subtitle.set_wrap(true);
        categories_subtitle.set_margin_start(24);
        categories_subtitle.set_margin_end(24);
        categories_subtitle.set_margin_bottom(18);
        categories_subtitle.add_css_class("dim-label");
        categories_page.append(&categories_subtitle);

        let categories_empty = Label::new(Some(&t("categories.empty")));
        categories_empty.set_halign(gtk4::Align::Start);
        categories_empty.set_xalign(0.0);
        categories_empty.set_wrap(true);
        categories_empty.set_margin_start(24);
        categories_empty.set_margin_end(24);
        categories_empty.set_visible(false);
        categories_empty.add_css_class("synopsis-text");
        categories_page.append(&categories_empty);

        let categories_scrolled = ScrolledWindow::new();
        categories_scrolled.set_hexpand(true);
        categories_scrolled.set_vexpand(true);
        categories_scrolled.set_hscrollbar_policy(gtk4::PolicyType::Never);
        categories_scrolled.set_vscrollbar_policy(gtk4::PolicyType::Automatic);

        let categories_flow = FlowBox::new();
        categories_flow.set_hexpand(true);
        categories_flow.set_vexpand(false);
        categories_flow.set_halign(gtk4::Align::Fill);
        categories_flow.set_valign(gtk4::Align::Start);
        categories_flow.set_homogeneous(false);
        categories_flow.set_selection_mode(gtk4::SelectionMode::None);
        categories_flow.set_max_children_per_line(10);
        categories_flow.set_min_children_per_line(1);
        categories_flow.set_row_spacing(10);
        categories_flow.set_column_spacing(10);
        categories_flow.set_margin_start(24);
        categories_flow.set_margin_end(24);
        categories_flow.set_margin_bottom(24);
        categories_scrolled.set_child(Some(&categories_flow));
        categories_page.append(&categories_scrolled);

        stack.add_titled(&categories_page, Some("categories"), &t("stack.categories"));

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
        stack.add_titled(&details_scrolled, Some("details"), &t("stack.details"));

        let config_rc = Rc::new(RefCell::new(config.inner().clone()));

        let settings_view = SettingsView::new(
            config.inner(),
            &db_path,
            config_path.as_path(),
            &window,
            translations.clone(),
        );
        stack.add_titled(
            settings_view.widget(),
            Some("settings"),
            &t("stack.settings"),
        );

        stack.set_visible_child_name("list");

        let this = Rc::new(RefCell::new(Self {
            window,
            stack,
            search_bar,
            settings_button,
            categories_button: Button::new(),
            media_list,
            media_details,
            settings_view,
            library_status,
            empty_page,
            config: config_rc.clone(),
            media_count: 0,
            db_path: db_path.clone(),
            metadata_sync_running: Rc::new(Cell::new(false)),
            toast_overlay,
            translations,
            categories_flow,
            categories_empty,
            categories_title,
            categories_subtitle,
            clear_category_button,
            filter_bar,
            filter_label,
            filter_clear_button,
            history: RefCell::new(Vec::new()),
            current_category: Rc::new(RefCell::new(None)),
            current_category_name: Rc::new(RefCell::new(None)),
        }));

        this.borrow().window.present();

        // Cargar medios iniciales (SQLite local, rápido).
        Self::load_media(&this);

        // Logo: acceso directo al inicio desde cualquier vista. Limpia la
        // búsqueda y el filtro, sin reiniciar la aplicación ni recargar la
        // biblioteca si no hay filtro que retirar.
        let this_logo = this.clone();
        logo_button.connect_clicked(move |_| {
            Self::go_home(&this_logo);
        });

        // Botón "Categorías": abre el selector de categorías para filtrar.
        let categories_button = Button::with_label(&t("button.categories"));
        categories_button.add_css_class("nav-button");
        categories_button.set_valign(gtk4::Align::Center);
        let this_cat = this.clone();
        categories_button.connect_clicked(move |_| {
            {
                let w = this_cat.borrow();
                w.stack.set_visible_child_name("categories");
            }
            Self::populate_categories_page(&this_cat);
        });
        nav_box.append(&categories_button);
        {
            let mut w = this.borrow_mut();
            w.categories_button = categories_button;
        }

        // Callbacks: ambos botones "Ver todo" retiran el filtro activo.
        let this_clear = this.clone();
        this.borrow_mut()
            .clear_category_button
            .connect_clicked(move |_| {
                Self::clear_category_filter(&this_clear);
            });
        let this_filter_clear = this.clone();
        this.borrow_mut()
            .filter_clear_button
            .connect_clicked(move |_| {
                Self::clear_category_filter(&this_filter_clear);
            });

        // Callback: al seleccionar un medio de la lista, mostrar detalles.
        let this_clone = this.clone();
        this.borrow_mut().media_list.on_select(move |media_id| {
            let w = this_clone.borrow();
            w.navigate_to_details(media_id);
        });

        // Callback: al buscar y seleccionar un resultado, mostrar detalles.
        let this_clone = this.clone();
        this.borrow_mut().search_bar.on_select(move |media_id| {
            let w = this_clone.borrow();
            w.navigate_to_details(media_id);
        });

        // Callback: botón de Ajustes -> navega a la vista de ajustes dentro del
        // Stack, guardando la vista exacta de origen para volver a ella.
        let this_clone = this.clone();
        this.borrow_mut().settings_button.connect_clicked(move |_| {
            let w = this_clone.borrow();
            w.push_current_view();
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
            {
                let mut w = this_save.borrow_mut();
                let _ = w.set_language(&language);
            }
            Self::refresh_views(&this_save);
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
            Self::load_media(&this_clone);
            let this = this_clone.clone();
            glib::timeout_add_local(Duration::from_millis(500), move || {
                let w = this.borrow();
                w.show_toast(&w.t("toast.indexing_completed"));
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
                w.return_from_settings();
            });

        // Conectar botón Guardar de ajustes -> guarda, recarga medios y vuelve
        // a la vista exacta de origen (no siempre al inicio).
        let this_clone = this.clone();
        let db_path_for_reload = db_path.clone();
        this.borrow_mut()
            .settings_view
            .save_button()
            .connect_clicked(move |_| {
                // Recargar medios después de guardar ajustes (pueden haber
                // cambiado las carpetas).
                Self::load_media(&this_clone);
                Self::start_metadata_sync(&this_clone, db_path_for_reload.clone());
                // Volver a la vista de origen después de guardar.
                let w = this_clone.borrow();
                w.return_from_settings();
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
                // Temporada/episodio -> serie. Película/serie -> vista previa.
                if let Some(parent_id) = parent_id
                    && w.show_details(parent_id, None)
                {
                    return;
                }
                w.go_back();
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
                    w.show_error(&w.t("error.play_episode"), &error.to_string());
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
                    w.translations.clone(),
                    Rc::new({
                        let this_for_toast = this.clone();
                        move || {
                            let w2 = this_for_toast.borrow();
                            w2.show_toast(&w2.t("toast.tracks_saved"));
                        }
                    }),
                );
            });

        // Completar metadatos y pósters que falten sin bloquear la UI.
        Self::start_metadata_sync(&this, db_path.clone());

        this
    }

    /// Carga en la galería los medios visibles según el filtro de categoría
    /// activo. Solo cambia de página cuando la vista actual es la biblioteca;
    /// así una sincronización de metadatos no interrumpe Detalles o Ajustes.
    fn load_media(this: &MainWindowRef) {
        let (db_path, category_id) = {
            let w = this.borrow();
            (w.db_path.clone(), *w.current_category.borrow())
        };

        let mut conn = match open_db(&db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error abriendo BD: {}", e);
                return;
            }
        };
        let media = {
            let mut repo = MediaRepository::new(&mut conn);
            let options = ListMediaOptions {
                r#type: None,
                limit: Some(500),
                offset: Some(0),
                order_by: None,
            };
            match category_id {
                Some(id) => repo.list_by_category(id, options),
                None => repo.list(options),
            }
            .unwrap_or_default()
        };
        close_db(conn);

        let media_count = media.len();
        let mut w = this.borrow_mut();
        w.media_count = media_count;
        w.media_list.set_media(media);

        let visible = w.stack.visible_child_name().map(|name| name.to_string());
        if matches!(visible.as_deref(), Some("list") | Some("empty")) {
            let filtered = w.current_category.borrow().is_some();
            if media_count == 0 && !filtered {
                w.stack.set_visible_child_name("empty");
            } else {
                w.stack.set_visible_child_name("list");
            }
        }

        w.update_filter_bar();
        Self::update_library_status(&w);
    }

    /// Refresca la barra que indica la categoría activa y el botón «Ver todo».
    fn update_filter_bar(&self) {
        let name = self.current_category_name.borrow().clone();
        match name {
            Some(name) => {
                let text = self.t("categories.filter_format").replacen("{}", &name, 1);
                self.filter_label.set_label(&text);
                self.filter_bar.set_visible(true);
            }
            None => self.filter_bar.set_visible(false),
        }

        self.clear_category_button
            .set_visible(self.current_category.borrow().is_some());
    }

    /// Muestra una notificación temporal (AdwToast) en la parte inferior de la
    /// ventana. Se usa para confirmar acciones al usuario sin bloquear la UI.
    fn show_toast(&self, message: &str) {
        let toast = Toast::new(message);
        // `AdwToast::set_timeout` se expresa en segundos (no en milisegundos).
        toast.set_timeout(3);
        self.toast_overlay.add_toast(toast);
    }

    /// Traduce una clave de i18n al idioma actual.
    fn t(&self, key: &str) -> String {
        self.translations.borrow().t(key)
    }

    /// Recarga el diccionario de traducciones para `language` y refresca las
    /// vistas visibles para que el cambio de idioma se aplique sin reiniciar.
    fn set_language(
        &mut self,
        language: &str,
    ) -> Result<(), std::boxed::Box<dyn std::error::Error>> {
        self.translations.borrow_mut().set_language(language)?;
        self.show_toast(&self.t("status.language_applied"));
        self.refresh_ui_texts();
        Ok(())
    }

    /// Reescribe el texto de todos los widgets ya creados con el idioma actual.
    /// Se llama después de cambiar el idioma para que los cambios se vean
    /// sin reiniciar la aplicación.
    fn refresh_ui_texts(&mut self) {
        // Barra superior
        self.settings_button.set_label(&self.t("button.settings"));
        self.categories_button
            .set_label(&self.t("button.categories"));
        self.clear_category_button
            .set_label(&self.t("button.clear_category"));

        // Página vacía
        self.empty_page.set_title(&self.t("empty.title"));
        self.empty_page
            .set_description(Some(&self.t("empty.description")));

        // Indicador de biblioteca
        Self::update_library_status(self);

        // Placeholder del buscador
        self.search_bar.set_placeholder(&self.translations);

        // Página de categorías: textos fijos y chip "Todas las categorías"
        // (los nombres de categoría provienen de la BD y no se traducen).
        self.categories_title.set_label(&self.t("stack.categories"));
        self.categories_subtitle
            .set_label(&self.t("categories.subtitle"));
        self.categories_empty.set_label(&self.t("categories.empty"));
        self.filter_clear_button
            .set_label(&self.t("button.clear_category"));
        if let Some(first) = self.categories_flow.first_child()
            && let Ok(button) = first.downcast::<Button>()
        {
            button.set_label(&self.t("button.all_categories"));
        }
        self.update_filter_bar();

        // Vista de ajustes (título, secciones, botones, filas de carpetas)
        self.settings_view.refresh_texts();
    }

    /// Actualiza el indicador de carpetas y medios de la barra superior.
    fn update_library_status(w: &MainWindow) {
        let config = w.config.borrow();
        let folder_count = config.media_folders.len();

        let text = match folder_count {
            0 => w.t("status.no_folders"),
            1 => {
                let one = w.t("status.one_folder");
                one.replacen("{}", &w.media_count.to_string(), 1)
            }
            total => {
                let many = w.t("status.many_folders");
                many.replacen("{}", &w.media_count.to_string(), 1).replacen(
                    "{}",
                    &total.to_string(),
                    1,
                )
            }
        };
        w.library_status.set_label(&text);

        let tooltip = if config.media_folders.is_empty() {
            None
        } else {
            Some(config.media_folders.join("\n"))
        };
        w.library_status.set_tooltip_text(tooltip.as_deref());
    }

    /// Muestra los detalles de un medio, conservando la temporada solicitada
    /// si sigue existiendo. Devuelve `false` si el medio ya no existe.
    fn show_details(&self, media_id: i64, season: Option<SeasonSelection>) -> bool {
        let mut conn = match open_db(&self.db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error abriendo BD: {}", e);
                return false;
            }
        };
        let media = {
            let mut repo = MediaRepository::new(&mut conn);
            repo.find_by_id(media_id).unwrap_or(None)
        };
        close_db(conn);

        if let Some(media) = media {
            self.media_details
                .borrow_mut()
                .set_media_with_season(&media, season);
            self.stack.set_visible_child_name("details");
            true
        } else {
            false
        }
    }

    /// Describe la vista actual para poder restaurarla desde el historial.
    fn current_view(&self) -> View {
        let page = self
            .stack
            .visible_child_name()
            .map(|name| name.to_string())
            .unwrap_or_else(|| "list".to_string());
        if page == "details" {
            let details = self.media_details.borrow();
            View::Details {
                media_id: details.media_id(),
                season: details.season_selection(),
            }
        } else {
            View::Page(page)
        }
    }

    /// Guarda la vista actual en el historial, evitando duplicados seguidos.
    fn push_current_view(&self) {
        let view = self.current_view();
        let mut history = self.history.borrow_mut();
        if history.last() != Some(&view) {
            history.push(view);
        }
        // Límite defensivo: la navegación no necesita un historial infinito.
        if history.len() > 32 {
            history.remove(0);
        }
    }

    /// Abre los detalles recordando la pantalla desde la que se navega.
    fn navigate_to_details(&self, media_id: i64) {
        self.push_current_view();
        if self.show_details(media_id, None) {
            return;
        }
        let previous = self.history.borrow_mut().pop();
        match previous {
            Some(view) => self.restore_view(&view),
            None => self.show_list(),
        }
    }

    /// Restaura una vista concreta sin volver a añadirla al historial.
    fn restore_view(&self, view: &View) {
        match view {
            View::Page(page) if page == "details" => self.show_list(),
            View::Page(page) => {
                self.stack.set_visible_child_name(page);
            }
            View::Details { media_id, season } => {
                if !self.show_details(*media_id, *season) {
                    self.show_list();
                }
            }
        }
    }

    /// Vuelve a la vista anterior de la pila. Si no existe, vuelve al inicio.
    fn go_back(&self) {
        while let Some(view) = self.history.borrow_mut().pop() {
            if let View::Page(page) = &view
                && page == "details"
            {
                continue;
            }
            self.restore_view(&view);
            return;
        }
        self.show_list();
    }

    /// Salida de Ajustes: restaura exactamente la pantalla de origen.
    fn return_from_settings(&self) {
        self.go_back();
    }

    /// Muestra la biblioteca (o la página vacía si no hay medios ni filtro).
    fn show_list(&self) {
        let filtered = self.current_category.borrow().is_some();
        if self.media_count == 0 && !filtered {
            self.stack.set_visible_child_name("empty");
        } else {
            self.stack.set_visible_child_name("list");
        }
    }

    /// Navega al inicio limpiando búsqueda, filtro e historial. No recarga la
    /// biblioteca si no había un filtro que retirar.
    fn go_home(this: &MainWindowRef) {
        let had_filter = {
            let w = this.borrow();
            w.search_bar.clear_query();
            let had_filter = w.current_category.borrow().is_some();
            w.current_category.replace(None);
            w.current_category_name.replace(None);
            w.history.borrow_mut().clear();
            w.update_filter_bar();
            had_filter
        };

        if had_filter {
            Self::load_media(this);
        }
        this.borrow().show_list();
    }

    /// Aplica o retira el filtro de categoría y vuelve a la biblioteca.
    fn set_category_filter(this: &MainWindowRef, category: Option<(i64, String)>) {
        {
            let w = this.borrow();
            match category {
                Some((id, name)) => {
                    w.current_category.replace(Some(id));
                    w.current_category_name.replace(Some(name));
                }
                None => {
                    w.current_category.replace(None);
                    w.current_category_name.replace(None);
                }
            }
            w.update_filter_bar();
        }

        Self::load_media(this);
        this.borrow().show_list();
    }

    /// Retira el filtro de categoría activo.
    fn clear_category_filter(this: &MainWindowRef) {
        Self::set_category_filter(this, None);
    }

    /// Rellena la página de categorías con las categorías reales de la BD,
    /// incluyendo el número de medios de cada una.
    fn populate_categories_page(this: &MainWindowRef) {
        let (db_path, active_id) = {
            let w = this.borrow();
            (w.db_path.clone(), *w.current_category.borrow())
        };

        let mut conn = match open_db(&db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error abriendo BD para categorías: {}", e);
                return;
            }
        };
        let categories = {
            let mut repo = MediaRepository::new(&mut conn);
            repo.list_categories().unwrap_or_default()
        };
        close_db(conn);

        // Las categorías sin medios no aportan nada al filtro.
        let categories: Vec<_> = categories
            .into_iter()
            .filter(|category| category.media_count > 0)
            .collect();

        let w = this.borrow();
        while let Some(child) = w.categories_flow.first_child() {
            w.categories_flow.remove(&child);
        }
        w.categories_empty.set_visible(categories.is_empty());

        // Chip "Todas" para retirar cualquier filtro.
        let all_button = Button::with_label(&w.t("button.all_categories"));
        all_button.add_css_class("category-chip");
        all_button.set_valign(gtk4::Align::Center);
        if active_id.is_none() {
            all_button.add_css_class("active");
        }
        let this_all = this.clone();
        all_button.connect_clicked(move |_| {
            Self::clear_category_filter(&this_all);
        });
        w.categories_flow.append(&all_button);

        for category in categories {
            let label = w
                .t("categories.chip_format")
                .replacen("{}", &category.name, 1)
                .replacen("{}", &category.media_count.to_string(), 1);
            let btn = Button::with_label(&label);
            btn.add_css_class("category-chip");
            btn.set_valign(gtk4::Align::Center);
            if active_id == Some(category.id) {
                btn.add_css_class("active");
            }

            let this_cat = this.clone();
            let cat_id = category.id;
            let cat_name = category.name.clone();
            btn.connect_clicked(move |_| {
                // Volver a pulsar la categoría activa la desactiva.
                let is_active = {
                    let w = this_cat.borrow();
                    *w.current_category.borrow() == Some(cat_id)
                };
                let selection = if is_active {
                    None
                } else {
                    Some((cat_id, cat_name.clone()))
                };
                Self::set_category_filter(&this_cat, selection);
            });
            w.categories_flow.append(&btn);
        }

        w.update_filter_bar();
        drop(w);
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
            let w = this.borrow();
            let text = w.t("status.metadata.already_running");
            w.settings_view.set_metadata_status(&text);
            return;
        }

        let (window, message, detail, cancel, update) = {
            let w = this.borrow();
            (
                w.window.clone(),
                w.t("metadata.refresh.title"),
                w.t("metadata.refresh.detail"),
                w.t("metadata.refresh.cancel"),
                w.t("metadata.refresh.update"),
            )
        };

        let dialog = gtk4::AlertDialog::builder()
            .modal(true)
            .message(&message)
            .detail(&detail)
            .buttons([cancel.as_str(), update.as_str()])
            .cancel_button(0)
            .default_button(0)
            .build();

        let this = this.clone();
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
            let w = this.borrow();
            let text = w.t("status.metadata.already_running");
            w.settings_view.set_metadata_status(&text);
            return;
        }

        let db_path = this.borrow().db_path.clone();
        this.borrow().metadata_sync_running.set(true);
        {
            let w = this.borrow();
            w.settings_view.set_metadata_refreshing(true);
            let text = w.t("status.metadata.refreshing");
            w.settings_view.set_metadata_status(&text);
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
                        let summary = metadata_refresh_summary(&result, &w.translations);
                        w.settings_view.set_metadata_status(&summary);
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
                        let status = w.t("status.metadata.error_detail");
                        w.settings_view.set_metadata_status(&status);
                        let title = w.t("status.metadata.error_title");
                        w.show_error(&title, &error.to_string());
                    }
                    glib::ControlFlow::Break
                }
                Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(TryRecvError::Disconnected) => {
                    let w = this.borrow();
                    w.metadata_sync_running.set(false);
                    w.settings_view.set_metadata_refreshing(false);
                    let status = w.t("status.metadata.interrupted");
                    w.settings_view.set_metadata_status(&status);
                    glib::ControlFlow::Break
                }
            }
        });
    }

    fn refresh_views(this: &MainWindowRef) {
        let (visible, media_id, season) = {
            let w = this.borrow();
            let details = w.media_details.borrow();
            (
                w.stack.visible_child_name().map(|name| name.to_string()),
                details.media_id(),
                details.season_selection(),
            )
        };

        Self::load_media(this);

        if visible.as_deref() == Some("details") && media_id > 0 {
            this.borrow().show_details(media_id, season);
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
fn metadata_refresh_summary(
    result: &RefreshMetadataResult,
    translations: &TranslationsRef,
) -> String {
    let t = |key: &str| translations.borrow().t(key);

    if result.total == 0 {
        return t("metadata.sync.no_media");
    }

    let mut summary = t("metadata.sync.success")
        .replacen("{}", &result.updated.to_string(), 1)
        .replacen("{}", &result.total.to_string(), 1);
    if result.failed > 0 {
        summary.push_str(&t("metadata.sync.failed").replacen("{}", &result.failed.to_string(), 1));
    }
    if result.poster_failed > 0 {
        summary.push_str(&t("metadata.sync.poster_failed").replacen(
            "{}",
            &result.poster_failed.to_string(),
            1,
        ));
    }
    if result.banner_failed > 0 {
        summary.push_str(&t("metadata.sync.banner_failed").replacen(
            "{}",
            &result.banner_failed.to_string(),
            1,
        ));
    }
    summary
}
