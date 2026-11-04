//! Vista de ajustes integrada en el Stack de la aplicación principal.

use crate::config::AppConfig;
use crate::core::library::{IndexResult, LibraryService};
use crate::db::media_repo::MediaRepository;
use crate::db::{close_db, open_db};
use crate::ui::i18n::TranslationsRef;
use gtk4::prelude::*;
use gtk4::{
    Box, Button, DropDown, Entry, GestureClick, Label, ListBox, ListBoxRow, Orientation, Popover,
    ScrolledWindow, Spinner, gio, glib,
};
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{self, TryRecvError};
use std::time::Duration;

/// Normaliza una ruta de carpeta: la convierte en absoluta, resuelve enlaces
/// simbólicos y elimina el separador final. Devuelve `None` si no es un
/// directorio accesible.
fn normalize_folder_path(path: &str) -> Option<String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return None;
    }

    let path = PathBuf::from(trimmed);
    let absolute = if path.is_absolute() {
        path
    } else {
        std::env::current_dir().ok()?.join(path)
    };

    let canonical = std::fs::canonicalize(absolute).ok()?;
    if !canonical.is_dir() {
        return None;
    }

    let mut normalized = canonical.to_string_lossy().to_string();
    if normalized.len() > 1 {
        normalized = normalized.trim_end_matches('/').to_string();
    }
    Some(normalized)
}

/// Resumen de una operación de indexado en segundo plano.
struct IndexSummary {
    total_folders: usize,
    indexed_folders: usize,
    media_added: i64,
    files_added: i64,
    errors: Vec<String>,
}

/// Callback invocado cuando cambia la configuración de carpetas.
type ConfigCallback = Rc<dyn Fn(AppConfig)>;

/// Callback invocado cuando la biblioteca indexada cambia.
type LibraryCallback = Rc<dyn Fn()>;

/// Gestión de carpetas compartida por los controles de la vista de ajustes.
///
/// Se clona para pasarla a los callbacks de GTK; todos sus campos son clones
/// baratos de widgets o `Rc`.
#[derive(Clone)]
struct FolderManager {
    config: Rc<RefCell<AppConfig>>,
    config_path: PathBuf,
    db_path: String,
    window: gtk4::Window,
    folders_list: Box,
    indexing: Rc<Cell<bool>>,
    add_button: Button,
    browse_button: Button,
    reindex_button: Button,
    index_spinner: Spinner,
    index_status: Label,
    on_config_changed: Rc<RefCell<Option<ConfigCallback>>>,
    on_library_changed: Rc<RefCell<Option<LibraryCallback>>>,
    translations: TranslationsRef,
}

impl FolderManager {
    /// Traduce una clave de i18n al idioma actual.
    fn t(&self, key: &str) -> String {
        self.translations.borrow().t(key)
    }

    fn append_folder_row(&self, folder: &str) {
        let row = Box::new(Orientation::Horizontal, 5);
        row.set_hexpand(true);

        let label = Label::new(Some(folder));
        label.set_hexpand(true);
        label.set_halign(gtk4::Align::Start);
        row.append(&label);

        let remove_button = Button::with_label(&self.t("dialog.remove_folder.delete"));
        row.append(&remove_button);

        let manager = self.clone();
        let folder = folder.to_string();
        let row_clone = row.clone();
        remove_button.connect_clicked(move |_| {
            manager.remove_folder(&folder, &row_clone);
        });

        self.folders_list.append(&row);
    }

    /// Actualiza las etiquetas de los botones «Eliminar» de las carpetas ya
    /// mostradas, para que el cambio de idioma se aplique sin reindexar.
    fn refresh_row_labels(&self) {
        let remove_label = self.t("dialog.remove_folder.delete");
        let mut child = self.folders_list.first_child();
        while let Some(row) = child {
            if let Some(widget) = row.last_child()
                && let Ok(button) = widget.downcast::<Button>()
            {
                button.set_label(&remove_label);
            }
            child = row.next_sibling();
        }
    }

    /// Añade una carpeta a la configuración, la guarda y lanza el indexado.
    /// Devuelve `true` si la carpeta se añadió correctamente.
    fn add_folder(&self, input: &str) -> bool {
        if self.indexing.get() {
            self.set_status(&self.t("status.wait_indexing"));
            return false;
        }

        let Some(path) = normalize_folder_path(input) else {
            self.show_error(
                &self.t("error.invalid_folder.title"),
                &self.t("error.invalid_folder.detail"),
            );
            self.set_status(&self.t("error.invalid_folder.status"));
            return false;
        };

        let path_buf = PathBuf::from(&path);
        {
            let mut config = self.config.borrow_mut();
            if config
                .media_folders
                .iter()
                .any(|folder| Path::new(folder) == path_buf)
            {
                drop(config);
                self.set_status(&self.t("status.folder_exists"));
                return false;
            }
            config.media_folders.push(path.clone());
        }

        if let Err(error) = self.persist_config() {
            self.config
                .borrow_mut()
                .media_folders
                .retain(|folder| folder != &path);
            self.show_error(&self.t("error.save_config.title"), &error.to_string());
            return false;
        }

        self.append_folder_row(&path);
        self.notify_config_changed();
        self.start_indexing(vec![path], &self.t("status.indexing_folder"));
        true
    }

    /// Pide confirmación y elimina la carpeta de la configuración y de la BD.
    fn remove_folder(&self, folder: &str, row: &Box) {
        if self.indexing.get() {
            self.set_status(&self.t("status.wait_remove"));
            return;
        }

        let title_template = self.t("dialog.remove_folder.title_format");
        let title = title_template.replace("{}", folder);
        let cancel_label = self.t("dialog.remove_folder.cancel");
        let delete_label = self.t("dialog.remove_folder.delete");
        let dialog = gtk4::AlertDialog::builder()
            .modal(true)
            .message(title)
            .detail(self.t("dialog.remove_folder.detail"))
            .buttons([cancel_label.as_str(), delete_label.as_str()])
            .cancel_button(0)
            .default_button(0)
            .build();

        let manager = self.clone();
        let folder = folder.to_string();
        let row = row.clone();
        dialog.choose(
            Some(&self.window),
            None::<&gio::Cancellable>,
            move |response| {
                if matches!(response, Ok(1)) {
                    manager.apply_remove_folder(&folder, &row);
                }
            },
        );
    }

    fn apply_remove_folder(&self, folder: &str, row: &Box) {
        let deleted = match crate::core::library::remove_folder(&self.db_path, folder) {
            Ok(deleted) => deleted,
            Err(error) => {
                let title = self.t("error.remove_folder.title");
                let detail_template = self.t("error.remove_folder.detail_format");
                let detail =
                    detail_template
                        .replacen("{}", folder, 1)
                        .replacen("{}", &error.to_string(), 1);
                self.show_error(&title, &detail);
                return;
            }
        };

        {
            let mut config = self.config.borrow_mut();
            config.media_folders.retain(|item| item != folder);
        }

        if let Err(error) = self.persist_config() {
            self.config
                .borrow_mut()
                .media_folders
                .push(folder.to_string());
            self.show_error(&self.t("error.save_config.title"), &error.to_string());
            return;
        }

        self.folders_list.remove(row);
        let removed_template = self.t("status.folder_removed_format");
        let text = removed_template.replacen("{}", &deleted.to_string(), 1);
        self.set_status(&text);
        self.notify_config_changed();
        self.notify_library_changed();
    }

    fn reindex_all(&self) {
        if self.indexing.get() {
            self.set_status(&self.t("status.indexing_in_progress"));
            return;
        }

        let folders = self.config.borrow().media_folders.clone();
        if folders.is_empty() {
            self.set_status(&self.t("status.no_folders_to_reindex"));
            return;
        }

        self.start_indexing(folders, &self.t("status.reindexing"));
    }

    fn start_indexing(&self, folders: Vec<String>, message: &str) {
        if self.indexing.get() {
            self.set_status(&self.t("status.indexing_in_progress"));
            return;
        }

        self.indexing.set(true);
        self.set_buttons_sensitive(false);
        self.index_spinner.set_spinning(true);
        self.index_spinner.set_visible(true);
        self.set_status(message);

        let total_folders = folders.len();
        let db_path = self.db_path.clone();
        let (sender, receiver) = mpsc::channel();

        std::thread::spawn(move || {
            let mut summary = IndexSummary {
                total_folders,
                indexed_folders: 0,
                media_added: 0,
                files_added: 0,
                errors: Vec::new(),
            };

            for folder in &folders {
                match LibraryService::index_folder(&db_path, folder) {
                    Ok(IndexResult {
                        media_added,
                        files_added,
                        ..
                    }) => {
                        summary.indexed_folders += 1;
                        summary.media_added += media_added;
                        summary.files_added += files_added;

                        // Extrae las pistas de audio/subtítulos de los archivos
                        // nuevos sin bloquear la UI. Un fallo aquí no invalida
                        // la indexación: la reproducción seguirá funcionando con
                        // las pistas predeterminadas de mpv.
                        if let Err(error) =
                            crate::core::library::index_missing_streams(&db_path, Some(folder))
                        {
                            eprintln!(
                                "No se pudieron analizar las pistas de '{}': {}",
                                folder, error
                            );
                        }
                    }
                    Err(error) => summary.errors.push(format!("{}: {}", folder, error)),
                }
            }

            let _ = sender.send(summary);
        });

        let manager = self.clone();
        glib::timeout_add_local(Duration::from_millis(200), move || {
            match receiver.try_recv() {
                Ok(summary) => {
                    manager.finish_indexing(summary);
                    glib::ControlFlow::Break
                }
                Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(TryRecvError::Disconnected) => {
                    manager.finish_indexing_error();
                    glib::ControlFlow::Break
                }
            }
        });
    }

    fn finish_indexing(&self, summary: IndexSummary) {
        self.indexing.set(false);
        self.index_spinner.set_spinning(false);
        self.index_spinner.set_visible(false);
        self.set_buttons_sensitive(true);

        let completed_template = self.t("status.indexing_completed_format");
        let mut message = completed_template
            .replacen("{}", &summary.media_added.to_string(), 1)
            .replacen("{}", &summary.files_added.to_string(), 1)
            .replacen("{}", &summary.indexed_folders.to_string(), 1)
            .replacen("{}", &summary.total_folders.to_string(), 1);
        if !summary.errors.is_empty() {
            let failed = self.t("status.folders_failed");
            message.push_str(&failed.replacen("{}", &summary.errors.len().to_string(), 1));
        }
        self.set_status(&message);

        if !summary.errors.is_empty() {
            self.show_error(
                &self.t("error.some_folders_failed.title"),
                &summary.errors.join("\n"),
            );
        }

        self.notify_library_changed();
    }

    fn finish_indexing_error(&self) {
        self.indexing.set(false);
        self.index_spinner.set_spinning(false);
        self.index_spinner.set_visible(false);
        self.set_buttons_sensitive(true);
        self.set_status(&self.t("status.indexing_interrupted"));
    }

    fn set_buttons_sensitive(&self, sensitive: bool) {
        self.add_button.set_sensitive(sensitive);
        self.browse_button.set_sensitive(sensitive);
        self.reindex_button.set_sensitive(sensitive);
    }

    fn set_status(&self, message: &str) {
        self.index_status.set_label(message);
        self.index_status.set_visible(!message.is_empty());
    }

    fn persist_config(&self) -> Result<(), std::boxed::Box<dyn std::error::Error>> {
        crate::config::Config {
            path: self.config_path.clone(),
            inner: self.config.borrow().clone(),
        }
        .save()
    }

    fn notify_config_changed(&self) {
        let config = self.config.borrow().clone();
        if let Some(callback) = self.on_config_changed.borrow().as_ref() {
            callback(config);
        }
    }

    fn notify_library_changed(&self) {
        if let Some(callback) = self.on_library_changed.borrow().as_ref() {
            callback();
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
}

/// Vista de ajustes integrada (se muestra dentro del Stack de MainWindow).
///
/// El contenido desplegable (secciones de carpetas, idioma, metadatos y
/// pistas predeterminadas) vive dentro de un `ScrolledWindow` para que, cuando
/// la ventana no tenga suficiente altura, el usuario pueda desplazarse y llegar
/// hasta los botones de guardar y volver sin que quede nada fuera de área
/// visible. Los botones de acción se mantienen fijos al final de la página.
pub struct SettingsView {
    /// Página completa: área scrollable con las secciones de configuración y
    /// los botones de acción fijados al final.
    page: Box,
    title: Label,
    folders_title: Label,
    folders_label: Label,
    folder_entry: Entry,
    browse_button: Button,
    add_button: Button,
    reindex_button: Button,
    language_title: Label,
    language_combo: DropDown,
    metadata_title: Label,
    metadata_label: Label,
    save_button: Button,
    cancel_button: Button,
    metadata_button: Button,
    metadata_spinner: Spinner,
    metadata_status: Label,
    config: Rc<RefCell<AppConfig>>,
    folder_manager: FolderManager,
    on_save: Option<Rc<dyn Fn(AppConfig)>>,
    on_refresh_metadata: Option<Rc<dyn Fn()>>,
    translations: TranslationsRef,
    /// Opciones de pista de la BD (sin "Ninguno"), para reconstruir los
    /// desplegables al cambiar de idioma.
    audio_options: Vec<(String, String)>,
    subtitle_options: Vec<(String, String)>,
    /// Selectores de pista predeterminada con búsqueda y filtrado real.
    /// Se usan Entry + Popover + ListBox para que escribir en el buscador
    /// oculte las opciones que no coincidan (filtrado visual).
    default_audio_selector: TrackSelector,
    default_subtitle_selector: TrackSelector,
}

/// Selector de pista con búsqueda y filtrado visual.
///
/// Combina un Entry (para escribir/filtrar), un Popover y un ListBox
/// dentro de un ScrolledWindow. Escribir en el Entry filtra las opciones
/// del ListBox (oculta las que no coinciden con el texto escrito).
#[allow(dead_code)]
struct TrackSelector {
    entry_widget: Box,
    entry: Entry,
    popover: Popover,
    list_box: ListBox,
    /// Todas las opciones disponibles: (clave, etiqueta mostrada).
    options: Rc<RefCell<Vec<(String, String)>>>,
    /// Clave actualmente seleccionada (None = "Ninguno").
    selected_key: Rc<RefCell<Option<String>>>,
}

impl TrackSelector {
    /// Crea un nuevo selector con su etiqueta, opciones y valor actual.
    ///
    /// Devuelve el contenedor Entry que se puede añadir a un layout, junto
    /// con el selector para poder consultar la selección.
    fn new(
        title: &str,
        options: &[(String, String)],
        current: Option<&str>,
        translations: &TranslationsRef,
    ) -> (Box, Self) {
        let container = Box::new(Orientation::Vertical, 6);
        container.set_hexpand(true);
        container.set_valign(gtk4::Align::Start);

        let label = Label::new(Some(title));
        label.set_halign(gtk4::Align::Start);
        container.append(&label);

        let entry = Entry::new();
        entry.set_hexpand(true);
        entry.set_placeholder_text(Some(
            &translations
                .borrow()
                .t("settings.default_tracks.search_placeholder"),
        ));
        container.append(&entry);

        // Popover con ListBox scrollable
        let popover = Popover::new();
        popover.set_parent(&entry);

        let scrolled = ScrolledWindow::new();
        scrolled.set_hexpand(true);
        scrolled.set_vexpand(true);
        scrolled.set_hscrollbar_policy(gtk4::PolicyType::Never);
        scrolled.set_vscrollbar_policy(gtk4::PolicyType::Automatic);

        let list_box = ListBox::new();
        list_box.set_selection_mode(gtk4::SelectionMode::None);
        list_box.set_activate_on_single_click(true);

        scrolled.set_child(Some(&list_box));
        popover.set_child(Some(&scrolled));

        let options_rc = Rc::new(RefCell::new(options.to_vec()));
        let selected_key = Rc::new(RefCell::new(current.map(String::from)));

        // Mostrar el valor actual en el entry
        if let Some(current_key) = current
            && let Some((_, label)) = options.iter().find(|(k, _)| k == current_key)
        {
            entry.set_text(label);
        }

        // Poblar el listbox según el filtro del entry
        let populate = {
            let entry_c = entry.clone();
            let list_box_c = list_box.clone();
            let options_c = options_rc.clone();
            move || {
                let text = entry_c.text().to_string();
                let filter_lower = text.to_lowercase();
                let opts = options_c.borrow();

                while let Some(child) = list_box_c.first_child() {
                    list_box_c.remove(&child);
                }

                for (_key, label) in opts.iter().filter(|(_, label)| {
                    text.is_empty() || label.to_lowercase().starts_with(&filter_lower)
                }) {
                    let row = ListBoxRow::new();
                    let lbl = Label::new(Some(label));
                    lbl.set_halign(gtk4::Align::Start);
                    lbl.set_xalign(0.0);
                    row.set_child(Some(&lbl));
                    list_box_c.append(&row);
                }
            }
        };

        // Al activar una fila, seleccionar y cerrar el popover
        let list_box_for_activate = list_box.clone();
        let popover_for_activate = popover.clone();
        let entry_for_activate = entry.clone();
        let selected_key_for_activate = selected_key.clone();
        let options_for_activate = options_rc.clone();
        list_box.connect_row_activated(move |_list_box, row| {
            let index = {
                let mut idx = 0usize;
                let mut found = None;
                let mut child = list_box_for_activate.first_child();
                while let Some(row_widget) = child {
                    if let Some(r) = row_widget.downcast_ref::<ListBoxRow>() {
                        if r == row {
                            found = Some(idx);
                            break;
                        }
                        idx += 1;
                    }
                    child = row_widget.next_sibling();
                }
                found
            };
            if let Some(index) = index {
                let opts = options_for_activate.borrow();
                if let Some((key, label)) = opts.get(index) {
                    *selected_key_for_activate.borrow_mut() = Some(key.clone());
                    entry_for_activate.set_text(label);
                }
            }
            popover_for_activate.popdown();
        });

        // Al escribir, filtrar y mostrar el popover
        let entry_for_filter = entry.clone();
        let popover_for_filter = popover.clone();
        let populate_for_filter = populate.clone();
        entry.connect_changed(move |_| {
            populate_for_filter();
            if !entry_for_filter.text().is_empty() {
                popover_for_filter.popup();
            }
        });

        // Al hacer clic en el entry, (re)poblar y mostrar el popover.
        // Usamos PropagationPhase::Capture para que el clic se dispare antes
        // de que el Entry lo consuma internamente.
        let popover_for_click = popover.clone();
        let populate_for_click = populate.clone();
        let click = GestureClick::new();
        click.set_propagation_phase(gtk4::PropagationPhase::Capture);
        click.connect_pressed(move |_, _, _, _| {
            populate_for_click();
            popover_for_click.popup();
        });
        entry.add_controller(click);

        (
            container.clone(),
            Self {
                entry_widget: container,
                entry,
                popover,
                list_box,
                options: options_rc,
                selected_key,
            },
        )
    }

    /// Devuelve la clave actualmente seleccionada (None si "Ninguno").
    fn selected_key(&self) -> Option<String> {
        self.selected_key.borrow().clone()
    }

    /// Actualiza las opciones y el valor actual (usado al cambiar de idioma).
    fn set_options(&self, options: &[(String, String)], current: Option<&str>) {
        *self.options.borrow_mut() = options.to_vec();
        *self.selected_key.borrow_mut() = current.map(String::from);
        self.entry.set_text("");
        if let Some(current_key) = current
            && let Some((_, label)) = options.iter().find(|(k, _)| k == current_key)
        {
            self.entry.set_text(label);
        }
    }
}

impl SettingsView {
    /// Crea una nueva vista de ajustes.
    pub fn new<P: IsA<gtk4::Window>>(
        config: &AppConfig,
        db_path: &str,
        config_path: &Path,
        parent: &P,
        translations: TranslationsRef,
    ) -> Self {
        let t = |key: &str| -> String { translations.borrow().t(key) };

        let content = Box::new(Orientation::Vertical, 10);
        content.set_margin_top(20);
        content.set_margin_bottom(20);
        content.set_margin_start(20);
        content.set_margin_end(20);
        content.set_hexpand(true);
        content.set_vexpand(false);

        let scrolled = ScrolledWindow::new();
        scrolled.set_hexpand(true);
        scrolled.set_vexpand(true);
        scrolled.set_hscrollbar_policy(gtk4::PolicyType::Never);
        scrolled.set_vscrollbar_policy(gtk4::PolicyType::Automatic);
        scrolled.set_child(Some(&content));

        // Botones de acción, fijados al final de la página (fuera del área
        // scrollable) para que siempre sean accesibles.
        let button_box = Box::new(Orientation::Horizontal, 10);
        button_box.set_halign(gtk4::Align::End);
        button_box.set_hexpand(true);
        button_box.set_margin_top(12);
        button_box.set_margin_end(0);

        let cancel_button = Button::with_label(&t("button.cancel"));
        button_box.append(&cancel_button);

        let save_button = Button::with_label(&t("button.save"));
        save_button.add_css_class("suggested-action");
        button_box.append(&save_button);

        // Página completa: área scrollable + botones fijos al final.
        let page = Box::new(Orientation::Vertical, 0);
        page.set_hexpand(true);
        page.set_vexpand(true);
        page.append(&scrolled);
        page.append(&button_box);

        let title = Label::new(Some(&t("settings.title")));
        title.set_halign(gtk4::Align::Start);
        title.add_css_class("heading");
        content.append(&title);

        // --- Sección: Rutas de medios ---
        let folders_section = Box::new(Orientation::Vertical, 10);
        folders_section.set_hexpand(true);
        folders_section.set_margin_top(10);
        folders_section.set_margin_bottom(20);

        let folders_title = Label::new(Some(&t("settings.folders.title")));
        folders_title.set_halign(gtk4::Align::Start);
        folders_title.add_css_class("subheading");
        folders_section.append(&folders_title);

        let folders_label = Label::new(Some(&t("settings.folders.description")));
        folders_label.set_halign(gtk4::Align::Start);
        folders_label.set_wrap(true);
        folders_section.append(&folders_label);

        let scrolled = ScrolledWindow::new();
        scrolled.set_hexpand(true);
        scrolled.set_vexpand(true);
        scrolled.set_hscrollbar_policy(gtk4::PolicyType::Never);
        scrolled.set_vscrollbar_policy(gtk4::PolicyType::Automatic);

        let folders_list = Box::new(Orientation::Vertical, 5);
        folders_list.set_hexpand(true);
        scrolled.set_child(Some(&folders_list));
        folders_section.append(&scrolled);

        let add_box = Box::new(Orientation::Horizontal, 5);
        add_box.set_hexpand(true);

        let folder_entry = Entry::new();
        folder_entry.set_placeholder_text(Some(&t("settings.folders.entry_placeholder")));
        folder_entry.set_hexpand(true);
        add_box.append(&folder_entry);

        let browse_button = Button::with_label(&t("button.browse"));
        browse_button.set_hexpand(false);
        add_box.append(&browse_button);

        let add_button = Button::with_label(&t("button.add_folder"));
        add_box.append(&add_button);
        folders_section.append(&add_box);

        let reindex_box = Box::new(Orientation::Horizontal, 10);
        reindex_box.set_hexpand(true);

        let reindex_button = Button::with_label(&t("button.reindex"));
        reindex_button.set_halign(gtk4::Align::Start);
        reindex_box.append(&reindex_button);

        let index_spinner = Spinner::new();
        index_spinner.set_visible(false);
        index_spinner.set_valign(gtk4::Align::Center);
        reindex_box.append(&index_spinner);
        folders_section.append(&reindex_box);

        let index_status = Label::new(None);
        index_status.set_halign(gtk4::Align::Start);
        index_status.set_wrap(true);
        index_status.set_visible(false);
        index_status.add_css_class("dim-label");
        folders_section.append(&index_status);

        // --- Sección: Idioma ---
        let language_section = Box::new(Orientation::Vertical, 10);
        language_section.set_hexpand(true);
        language_section.set_margin_top(20);
        language_section.set_margin_bottom(20);

        let language_title = Label::new(Some(&t("settings.language.title")));
        language_title.set_halign(gtk4::Align::Start);
        language_title.add_css_class("subheading");
        language_section.append(&language_title);

        let language_options: Vec<String> =
            vec![t("settings.language.es"), t("settings.language.en")];
        let language_combo = DropDown::from_strings(
            &language_options
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        );
        language_combo.set_selected(if config.language == "es" { 0 } else { 1 });
        language_section.append(&language_combo);

        // --- Sección: Metadatos ---
        let metadata_section = Box::new(Orientation::Vertical, 10);
        metadata_section.set_hexpand(true);
        metadata_section.set_margin_top(20);
        metadata_section.set_margin_bottom(20);

        let metadata_title = Label::new(Some(&t("settings.metadata.title")));
        metadata_title.set_halign(gtk4::Align::Start);
        metadata_title.add_css_class("subheading");
        metadata_section.append(&metadata_title);

        let metadata_label = Label::new(Some(&t("settings.metadata.description")));
        metadata_label.set_halign(gtk4::Align::Start);
        metadata_label.set_wrap(true);
        metadata_section.append(&metadata_label);

        let metadata_row = Box::new(Orientation::Horizontal, 10);
        metadata_row.set_hexpand(true);

        let metadata_button = Button::with_label(&t("button.refresh_metadata"));
        metadata_button.set_halign(gtk4::Align::Start);
        metadata_row.append(&metadata_button);

        let metadata_spinner = Spinner::new();
        metadata_spinner.set_visible(false);
        metadata_spinner.set_valign(gtk4::Align::Center);
        metadata_row.append(&metadata_spinner);

        metadata_section.append(&metadata_row);

        let metadata_status = Label::new(None);
        metadata_status.set_halign(gtk4::Align::Start);
        metadata_status.set_wrap(true);
        metadata_status.set_visible(false);
        metadata_status.add_css_class("dim-label");
        metadata_section.append(&metadata_status);

        content.append(&folders_section);
        content.append(&language_section);
        content.append(&metadata_section);

        // --- Sección: Pistas predeterminadas ---
        let default_tracks_section = Box::new(Orientation::Vertical, 10);
        default_tracks_section.set_hexpand(true);
        default_tracks_section.set_margin_top(20);
        default_tracks_section.set_margin_bottom(20);

        let default_tracks_title = Label::new(Some(&t("settings.default_tracks.title")));
        default_tracks_title.set_halign(gtk4::Align::Start);
        default_tracks_title.add_css_class("subheading");
        default_tracks_section.append(&default_tracks_title);

        // Audio y subtítulos predeterminados en la misma línea.
        let default_tracks_row = Box::new(Orientation::Horizontal, 12);
        default_tracks_row.set_hexpand(true);
        default_tracks_row.set_valign(gtk4::Align::Start);

        // Leer opciones de pista de la biblioteca.
        let (audio_options, subtitle_options) = match open_db(db_path) {
            Ok(mut conn) => {
                let mut repo = MediaRepository::new(&mut conn);
                let audio = repo.list_available_tracks("audio").unwrap_or_default();
                let subtitle = repo.list_available_tracks("subtitle").unwrap_or_default();
                close_db(conn);
                (audio, subtitle)
            }
            Err(_) => (Vec::new(), Vec::new()),
        };

        // Opciones completas: "Ninguno" + pistas de la biblioteca.
        let none_label = t("settings.default_tracks.none");
        let mut all_audio: Vec<(String, String)> = vec![("none".to_string(), none_label.clone())];
        all_audio.extend(audio_options.iter().cloned());
        let mut all_subtitle: Vec<(String, String)> = vec![("none".to_string(), none_label)];
        all_subtitle.extend(subtitle_options.iter().cloned());

        // Audio predeterminado: selector con búsqueda y filtrado visual.
        // Escribir en el Entry filtra las opciones del ListBox dentro del
        // Popover (oculta las que no coinciden con el texto escrito).
        let (audio_entry_widget, default_audio_selector) = TrackSelector::new(
            &t("settings.default_tracks.audio"),
            &all_audio,
            config.default_tracks.audio.as_deref(),
            &translations,
        );
        default_tracks_row.append(&audio_entry_widget);

        // Subtítulos predeterminados: selector con búsqueda y filtrado visual.
        let (subtitle_entry_widget, default_subtitle_selector) = TrackSelector::new(
            &t("settings.default_tracks.subtitle"),
            &all_subtitle,
            config.default_tracks.subtitle.as_deref(),
            &translations,
        );
        default_tracks_row.append(&subtitle_entry_widget);

        default_tracks_section.append(&default_tracks_row);

        content.append(&default_tracks_section);

        let config_rc = Rc::new(RefCell::new(config.clone()));
        let window = parent.upcast_ref::<gtk4::Window>().clone();

        let folder_manager = FolderManager {
            config: config_rc.clone(),
            config_path: config_path.to_path_buf(),
            db_path: db_path.to_string(),
            window,
            folders_list,
            indexing: Rc::new(Cell::new(false)),
            add_button: add_button.clone(),
            browse_button: browse_button.clone(),
            reindex_button: reindex_button.clone(),
            index_spinner: index_spinner.clone(),
            index_status: index_status.clone(),
            on_config_changed: Rc::new(RefCell::new(None)),
            on_library_changed: Rc::new(RefCell::new(None)),
            translations: translations.clone(),
        };

        // Cargar carpetas existentes.
        for folder in &config.media_folders {
            folder_manager.append_folder_row(folder);
        }

        // Agregar carpeta al hacer clic en el botón.
        let manager_click = folder_manager.clone();
        let entry_click = folder_entry.clone();
        add_button.connect_clicked(move |_| {
            if manager_click.add_folder(&entry_click.text()) {
                entry_click.set_text("");
            }
        });

        // Agregar carpeta al pulsar Enter.
        let manager_enter = folder_manager.clone();
        let entry_enter = folder_entry.clone();
        folder_entry.connect_activate(move |_| {
            if manager_enter.add_folder(&entry_enter.text()) {
                entry_enter.set_text("");
            }
        });

        // Botón Examinar: selector nativo de carpetas de GTK4.
        let entry_browse = folder_entry.clone();
        let window_browse = parent.upcast_ref::<gtk4::Window>().clone();
        let translations_browse = translations.clone();
        browse_button.connect_clicked(move |_| {
            let dialog = gtk4::FileDialog::new();
            dialog.set_title(
                &translations_browse
                    .borrow()
                    .t("settings.folders.dialog_title"),
            );
            let entry = entry_browse.clone();
            dialog.select_folder(
                Some(&window_browse),
                None::<&gio::Cancellable>,
                move |result| match result {
                    Ok(file) => {
                        if let Some(path) = file.path() {
                            entry.set_text(&path.to_string_lossy());
                        }
                    }
                    Err(error) => {
                        if !error.matches(gio::IOErrorEnum::Cancelled) {
                            eprintln!("No se pudo seleccionar la carpeta: {}", error);
                        }
                    }
                },
            );
        });

        // Reindexar todas las carpetas configuradas.
        let manager_reindex = folder_manager.clone();
        reindex_button.connect_clicked(move |_| {
            manager_reindex.reindex_all();
        });

        Self {
            page,
            title,
            folders_title,
            folders_label,
            folder_entry,
            browse_button,
            add_button,
            reindex_button,
            language_title,
            language_combo,
            metadata_title,
            metadata_label,
            save_button,
            cancel_button,
            metadata_button,
            metadata_spinner,
            metadata_status,
            config: config_rc,
            folder_manager,
            on_save: None,
            on_refresh_metadata: None,
            translations,
            audio_options,
            subtitle_options,
            default_audio_selector,
            default_subtitle_selector,
        }
    }

    /// Devuelve el widget raíz de la vista de ajustes: un contenedor vertical
    /// con el área scrollable de secciones y los botones de acción fijos al
    /// final.
    pub fn widget(&self) -> &Box {
        &self.page
    }

    /// Reescribe todas las etiquetas de la vista con el idioma actual, de modo
    /// que el cambio de idioma se vea al instante sin reiniciar la aplicación.
    pub fn refresh_texts(&self) {
        let t = |key: &str| -> String { self.translations.borrow().t(key) };

        self.title.set_label(&t("settings.title"));
        self.folders_title.set_label(&t("settings.folders.title"));
        self.folders_label
            .set_label(&t("settings.folders.description"));
        self.folder_entry
            .set_placeholder_text(Some(&t("settings.folders.entry_placeholder")));
        self.browse_button.set_label(&t("button.browse"));
        self.add_button.set_label(&t("button.add_folder"));
        self.reindex_button.set_label(&t("button.reindex"));
        self.language_title.set_label(&t("settings.language.title"));

        // Actualizar las opciones del desplegable de idioma conservando la
        // selección actual del usuario.
        let selected = self.language_combo.selected();
        let options = [t("settings.language.es"), t("settings.language.en")];
        let option_refs: Vec<&str> = options.iter().map(String::as_str).collect();
        let model = gtk4::StringList::new(&option_refs);
        self.language_combo.set_model(Some(&model));
        self.language_combo.set_selected(selected.min(1));

        self.metadata_title.set_label(&t("settings.metadata.title"));
        self.metadata_label
            .set_label(&t("settings.metadata.description"));
        self.metadata_button
            .set_label(&t("button.refresh_metadata"));
        self.cancel_button.set_label(&t("button.cancel"));
        self.save_button.set_label(&t("button.save"));

        // Actualizar los selectores de pista con la nueva etiqueta de "Ninguno",
        // conservando la selección actual.
        let none_label = t("settings.default_tracks.none");
        let mut all_audio: Vec<(String, String)> = vec![("none".to_string(), none_label.clone())];
        all_audio.extend(self.audio_options.iter().cloned());
        let mut all_subtitle: Vec<(String, String)> = vec![("none".to_string(), none_label)];
        all_subtitle.extend(self.subtitle_options.iter().cloned());
        self.default_audio_selector.set_options(
            &all_audio,
            self.default_audio_selector.selected_key().as_deref(),
        );
        self.default_subtitle_selector.set_options(
            &all_subtitle,
            self.default_subtitle_selector.selected_key().as_deref(),
        );

        self.folder_manager.refresh_row_labels();
    }

    /// Establece el callback de guardar (se ejecuta al hacer clic en Guardar).
    pub fn on_save<F>(&mut self, f: F)
    where
        F: Fn(AppConfig) + 'static,
    {
        self.on_save = Some(Rc::new(f));

        let config = self.config.clone();
        let language_combo = self.language_combo.clone();
        let on_save = self.on_save.clone();
        let default_audio_key = self.default_audio_selector.selected_key();
        let default_subtitle_key = self.default_subtitle_selector.selected_key();

        self.save_button.connect_clicked(move |_| {
            {
                let mut config = config.borrow_mut();
                config.language = if language_combo.selected() == 0 {
                    "es".to_string()
                } else {
                    "en".to_string()
                };

                // Guardar pistas predeterminadas seleccionadas en los selectores.
                config.default_tracks.audio = default_audio_key.clone();
                config.default_tracks.subtitle = default_subtitle_key.clone();
            }

            if let Some(callback) = &on_save {
                callback(config.borrow().clone());
            }
        });
    }

    /// Notifica cambios inmediatos en las carpetas (ya persistidos).
    pub fn on_config_changed<F>(&mut self, f: F)
    where
        F: Fn(AppConfig) + 'static,
    {
        *self.folder_manager.on_config_changed.borrow_mut() = Some(Rc::new(f));
    }

    /// Notifica que la biblioteca ha cambiado y la lista debe recargarse.
    pub fn on_library_changed<F>(&mut self, f: F)
    where
        F: Fn() + 'static,
    {
        *self.folder_manager.on_library_changed.borrow_mut() = Some(Rc::new(f));
    }

    /// Establece el callback del botón de actualización de metadatos.
    pub fn on_refresh_metadata<F>(&mut self, f: F)
    where
        F: Fn() + 'static,
    {
        let cb = Rc::new(f);
        self.on_refresh_metadata = Some(cb.clone());
        self.metadata_button.connect_clicked(move |_| {
            cb();
        });
    }

    /// Muestra u oculta el indicador de progreso y bloquea el botón mientras
    /// la actualización está en curso.
    pub fn set_metadata_refreshing(&self, refreshing: bool) {
        self.metadata_spinner.set_spinning(refreshing);
        self.metadata_spinner.set_visible(refreshing);
        self.metadata_button.set_sensitive(!refreshing);
    }

    /// Actualiza el mensaje de estado de la sección de metadatos.
    pub fn set_metadata_status(&self, text: &str) {
        self.metadata_status.set_label(text);
        self.metadata_status.set_visible(!text.is_empty());
    }

    /// Devuelve el botón Guardar para conectarlo desde MainWindow.
    pub fn save_button(&self) -> &Button {
        &self.save_button
    }

    /// Devuelve el botón Volver para conectarlo desde MainWindow.
    pub fn cancel_button(&self) -> &Button {
        &self.cancel_button
    }
}

impl Default for SettingsView {
    fn default() -> Self {
        unimplemented!("SettingsView requires config, db_path, config_path and parent")
    }
}
