//! Vista de ajustes integrada en el Stack de la aplicación principal.

use crate::config::AppConfig;
use crate::core::language;
use crate::core::library::{IndexResult, LibraryService};
use crate::db::media_repo::{AvailableTrack, MediaRepository};
use crate::db::{close_db, open_db};
use crate::ui::i18n::{TranslationsRef, language_label};
use gtk4::prelude::*;
use gtk4::{
    Box, Button, DropDown, Entry, GestureClick, Label, ListBox, ListBoxRow, Orientation, Popover,
    ScrolledWindow, Spinner, Switch, gio, glib,
};
use libadwaita::ButtonContent;
use std::cell::{Cell, RefCell};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{self, TryRecvError};
use std::time::Duration;

/// Normaliza una ruta de carpeta: la convierte en absoluta, resuelve enlaces
/// simbólicos y elimina el separador final. Devuelve `None` si el directorio
/// no existe, no es accesible o no se puede leer (falta de permisos).
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

    // Comprobar explícitamente que se puede leer el contenido del directorio
    // (permisos de lectura). canonicalize + is_dir solo verifican existencia y
    // tipo; read_dir verifica que el usuario tenga permiso de lectura.
    if fs::read_dir(&canonical).is_err() {
        return None;
    }

    let mut normalized = canonical.to_string_lossy().to_string();
    if normalized.len() > 1 {
        normalized = normalized.trim_end_matches('/').to_string();
    }
    Some(normalized)
}

/// Muestra un diálogo de error modal sobre `parent`.
fn show_alert(parent: &gtk4::Window, title: &str, detail: &str) {
    gtk4::AlertDialog::builder()
        .modal(true)
        .message(title)
        .detail(detail)
        .build()
        .show(Some(parent));
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
    about_button: Button,
    about_title: Label,
    about_label: Label,
    about_button_content: ButtonContent,
    metadata_button: Button,
    metadata_spinner: Spinner,
    metadata_status: Label,
    data_title: Label,
    data_label: Label,
    backup_button: Button,
    restore_button: Button,
    #[allow(dead_code)]
    data_status: Label,
    theme_title: Label,
    theme_combo: DropDown,
    auto_play_next_switch: Switch,
    config: Rc<RefCell<AppConfig>>,
    folder_manager: FolderManager,
    on_save: Option<Rc<dyn Fn(AppConfig)>>,
    on_refresh_metadata: Option<Rc<dyn Fn()>>,
    translations: TranslationsRef,
    /// Idiomas disponibles en la BD (sin "Ninguno"), para reconstruir los
    /// selectores con las etiquetas del idioma activo.
    audio_tracks: Vec<AvailableTrack>,
    subtitle_tracks: Vec<AvailableTrack>,
    /// Selectores de pista predeterminada con búsqueda y filtrado real.
    /// Se usan Entry + Popover + ListBox para que escribir en el buscador
    /// oculte las opciones que no coincidan (filtrado visual).
    default_audio_selector: TrackSelector,
    default_subtitle_selector: TrackSelector,
}

/// Filtra las opciones de pista según el texto escrito en el buscador.
///
/// Si el texto está vacío devuelve todas las opciones. En caso contrario
/// devuelve solo aquellas cuya etiqueta empieza por el texto indicado,
/// sin distinguir mayúsculas de minúsculas.
pub fn filtrar_opciones(texto: &str, opciones: &[(String, String)]) -> Vec<(String, String)> {
    let texto_lower = texto.to_lowercase();
    opciones
        .iter()
        .filter(|(_, label)| texto.is_empty() || label.to_lowercase().starts_with(&texto_lower))
        .cloned()
        .collect()
}

/// Devuelve la etiqueta asociada a una clave dentro de la lista de opciones.
///
/// Devuelve `None` si la clave no existe.
pub fn obtener_label_para_clave<'a>(
    opciones: &'a [(String, String)],
    clave: &str,
) -> Option<&'a str> {
    opciones
        .iter()
        .find(|(k, _)| k == clave)
        .map(|(_, label)| label.as_str())
}

/// Ejecuta `apply` con una copia de la opción `index` sin mantener préstamos
/// sobre `displayed`.
///
/// El callback puede volver a prestar el `RefCell` (por ejemplo, el handler
/// `changed` del Entry al actualizar su texto). Mantener el préstamo de
/// lectura durante esa reentrada provoca un panic por `RefCell already
/// borrowed`.
fn with_selected_option<F>(displayed: &RefCell<Vec<(String, String)>>, index: usize, apply: F)
where
    F: FnOnce(String, String),
{
    let selection = displayed.borrow().get(index).cloned();
    if let Some((key, label)) = selection {
        apply(key, label);
    }
}

/// Construye las opciones del selector de pistas a partir de los idiomas
/// disponibles, traduciendo sus nombres al idioma activo de la interfaz.
fn build_track_options(
    tracks: &[AvailableTrack],
    translations: &TranslationsRef,
    none_label: &str,
) -> Vec<(String, String)> {
    let mut options = vec![("none".to_string(), none_label.to_string())];
    options.extend(tracks.iter().map(|track| {
        (
            track.key.clone(),
            language_label(translations, track.language.as_deref()),
        )
    }));
    options
}

/// Normaliza la clave guardada en la configuración a la forma canónica por
/// idioma. Las claves antiguas (`embedded:audio:jpn:...`) se migran al
/// cargar la vista; las claves que no representan un idioma se conservan.
fn normalize_default_track_key(key: Option<&str>) -> Option<String> {
    key.map(language::normalize_track_key)
}

/// Selector de pista con búsqueda y filtrado visual.
///
/// Combina un Entry (para escribir/filtrar), un Popover y un ListBox
/// dentro de un ScrolledWindow. Escribir en el Entry filtra las opciones
/// del ListBox (oculta las que no coinciden con el texto escrito).
struct TrackSelector {
    entry: Entry,
    /// Todas las opciones disponibles: (clave, etiqueta mostrada).
    options: Rc<RefCell<Vec<(String, String)>>>,
    /// Clave actualmente seleccionada (None = "Ninguno").
    selected_key: Rc<RefCell<Option<String>>>,
    /// Evita filtrar y abrir el popover cuando el texto del entry se actualiza
    /// por código (cambio de idioma o selección programática). Sin esto, el
    /// `changed` de `set_text` puede reentrar mientras otra parte del selector
    /// mantiene prestado su estado compartido.
    suppress_changed: Rc<Cell<bool>>,
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
        popover.set_autohide(true);

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
        let suppress_changed = Rc::new(Cell::new(false));
        // Opciones visibles en el ListBox en este momento (el listado completo
        // o un subconjunto filtrado). Permite mapear la fila activada a su
        // clave real cuando el filtro está activo.
        let displayed = Rc::new(RefCell::new(options.to_vec()));

        // Mostrar el valor actual en el entry
        if let Some(current_key) = current
            && let Some(label) = obtener_label_para_clave(options, current_key)
        {
            entry.set_text(label);
        }

        // Constantes para el alto dinámico del popover según cantidad de filas
        const ALTO_FILA: i32 = 34;
        const ALTO_MIN: i32 = 100;
        const ALTO_MAX: i32 = 400;

        fn ajustar_alto_scrolled(scrolled: &ScrolledWindow, num_filas: usize) {
            let alto = (num_filas as i32 * ALTO_FILA).clamp(ALTO_MIN, ALTO_MAX);
            scrolled.set_size_request(250, alto);
        }

        // Poblar el listbox con TODAS las opciones (al abrir el popover)
        let populate_all = {
            let list_box_c = list_box.clone();
            let options_c = options_rc.clone();
            let displayed_c = displayed.clone();
            let scrolled_c = scrolled.clone();
            move || {
                let opts = options_c.borrow().clone();
                *displayed_c.borrow_mut() = opts.clone();

                while let Some(child) = list_box_c.first_child() {
                    list_box_c.remove(&child);
                }

                for (_key, label) in &opts {
                    let row = ListBoxRow::new();
                    let lbl = Label::new(Some(label));
                    lbl.set_halign(gtk4::Align::Start);
                    lbl.set_xalign(0.0);
                    row.set_child(Some(&lbl));
                    list_box_c.append(&row);
                }
                ajustar_alto_scrolled(&scrolled_c, opts.len());
            }
        };

        // Poblar el listbox filtrado según el texto del entry (al escribir)
        let populate_filtered = {
            let entry_c = entry.clone();
            let list_box_c = list_box.clone();
            let options_c = options_rc.clone();
            let displayed_c = displayed.clone();
            let scrolled_c = scrolled.clone();
            move || {
                let text = entry_c.text().to_string();
                let filtradas = filtrar_opciones(&text, &options_c.borrow());
                *displayed_c.borrow_mut() = filtradas.clone();

                while let Some(child) = list_box_c.first_child() {
                    list_box_c.remove(&child);
                }

                for (_key, label) in &filtradas {
                    let row = ListBoxRow::new();
                    let lbl = Label::new(Some(label));
                    lbl.set_halign(gtk4::Align::Start);
                    lbl.set_xalign(0.0);
                    row.set_child(Some(&lbl));
                    list_box_c.append(&row);
                }
                ajustar_alto_scrolled(&scrolled_c, filtradas.len());
            }
        };

        // Al activar una fila, seleccionar y cerrar el popover
        let list_box_for_activate = list_box.clone();
        let popover_for_activate = popover.clone();
        let entry_for_activate = entry.clone();
        let selected_key_for_activate = selected_key.clone();
        let displayed_for_activate = displayed.clone();
        let suppress_changed_for_activate = suppress_changed.clone();
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
                // `with_selected_option` suelta el préstamo de `displayed`
                // antes de actualizar el Entry. El `changed` de `set_text`
                // reentra en el selector y necesita prestar ese mismo estado;
                // mantenerlo prestado provocaba un panic por
                // `RefCell already borrowed`.
                with_selected_option(&displayed_for_activate, index, |key, label| {
                    *selected_key_for_activate.borrow_mut() = Some(key);
                    suppress_changed_for_activate.set(true);
                    entry_for_activate.set_text(&label);
                    suppress_changed_for_activate.set(false);
                });
            }
            popover_for_activate.popdown();
        });

        // Al escribir, filtrar y mostrar el popover
        let entry_for_filter = entry.clone();
        let popover_for_filter = popover.clone();
        let populate_filtered_clone = populate_filtered.clone();
        let suppress_changed_for_filter = suppress_changed.clone();
        entry.connect_changed(move |_| {
            if suppress_changed_for_filter.get() {
                return;
            }
            populate_filtered_clone();
            if !entry_for_filter.text().is_empty() {
                popover_for_filter.popup();
            }
        });

        // Al hacer clic en el entry, (re)poblar con TODAS las opciones y mostrar el popover.
        // Usamos PropagationPhase::Capture para que el clic se dispare antes
        // de que el Entry lo consuma internamente.
        let popover_for_click = popover.clone();
        let populate_all_clone = populate_all.clone();
        let click = GestureClick::new();
        click.set_propagation_phase(gtk4::PropagationPhase::Capture);
        click.connect_pressed(move |_, _, _, _| {
            populate_all_clone();
            popover_for_click.popup();
        });
        entry.add_controller(click);

        (
            container.clone(),
            Self {
                entry,
                options: options_rc,
                selected_key,
                suppress_changed,
            },
        )
    }

    /// Devuelve la clave actualmente seleccionada (None si "Ninguno").
    fn selected_key(&self) -> Option<String> {
        self.selected_key.borrow().clone()
    }

    /// Actualiza la clave seleccionada y el texto mostrado en el entry.
    fn select_key(&self, key: Option<&str>) {
        *self.selected_key.borrow_mut() = key.map(String::from);
        let label = key.and_then(|clave| {
            obtener_label_para_clave(&self.options.borrow(), clave).map(String::from)
        });
        self.suppress_changed.set(true);
        self.entry.set_text(label.as_deref().unwrap_or_default());
        self.suppress_changed.set(false);
    }

    /// Actualiza las opciones y el valor actual (usado al cambiar de idioma).
    fn set_options(&self, options: &[(String, String)], current: Option<&str>) {
        *self.options.borrow_mut() = options.to_vec();
        *self.selected_key.borrow_mut() = current.map(String::from);
        self.suppress_changed.set(true);
        self.entry.set_text("");
        if let Some(current_key) = current
            && let Some(label) = obtener_label_para_clave(options, current_key)
        {
            self.entry.set_text(label);
        }
        self.suppress_changed.set(false);
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
        let (audio_tracks, subtitle_tracks) = match open_db(db_path) {
            Ok(mut conn) => {
                let mut repo = MediaRepository::new(&mut conn);
                let audio = repo.list_available_tracks("audio").unwrap_or_default();
                let subtitle = repo.list_available_tracks("subtitle").unwrap_or_default();
                close_db(conn);
                (audio, subtitle)
            }
            Err(_) => (Vec::new(), Vec::new()),
        };

        // Opciones completas: "Ninguno" + idiomas de la biblioteca. Cada
        // idioma aparece una sola vez, ya normalizado, y su etiqueta usa el
        // idioma activo de la interfaz.
        let none_label = t("settings.default_tracks.none");
        let all_audio = build_track_options(&audio_tracks, &translations, &none_label);
        let all_subtitle = build_track_options(&subtitle_tracks, &translations, &none_label);

        // Las claves antiguas por pista se migran a la clave canónica por
        // idioma para que la selección siga siendo visible y reutilizable.
        let current_audio_key = normalize_default_track_key(config.default_tracks.audio.as_deref());
        let current_subtitle_key =
            normalize_default_track_key(config.default_tracks.subtitle.as_deref());

        // Audio predeterminado: selector con búsqueda y filtrado visual.
        // Escribir en el Entry filtra las opciones del ListBox dentro del
        // Popover (oculta las que no coinciden con el texto escrito).
        let (audio_entry_widget, default_audio_selector) = TrackSelector::new(
            &t("settings.default_tracks.audio"),
            &all_audio,
            current_audio_key.as_deref(),
            &translations,
        );
        default_tracks_row.append(&audio_entry_widget);

        // Subtítulos predeterminados: selector con búsqueda y filtrado visual.
        let (subtitle_entry_widget, default_subtitle_selector) = TrackSelector::new(
            &t("settings.default_tracks.subtitle"),
            &all_subtitle,
            current_subtitle_key.as_deref(),
            &translations,
        );
        default_tracks_row.append(&subtitle_entry_widget);

        default_tracks_section.append(&default_tracks_row);

        content.append(&default_tracks_section);

        // --- Sección: Datos ---
        let data_section = Box::new(Orientation::Vertical, 10);
        data_section.set_hexpand(true);
        data_section.set_margin_top(20);
        data_section.set_margin_bottom(20);

        let data_title = Label::new(Some(&t("settings.data.title")));
        data_title.set_halign(gtk4::Align::Start);
        data_title.add_css_class("subheading");
        data_section.append(&data_title);

        let data_label = Label::new(Some(&t("settings.data.description")));
        data_label.set_halign(gtk4::Align::Start);
        data_label.set_wrap(true);
        data_section.append(&data_label);

        let data_row = Box::new(Orientation::Horizontal, 10);
        data_row.set_hexpand(true);

        let backup_button = Button::with_label(&t("button.backup_db"));
        backup_button.set_halign(gtk4::Align::Start);
        data_row.append(&backup_button);

        let restore_button = Button::with_label(&t("button.restore_db"));
        restore_button.set_halign(gtk4::Align::Start);
        data_row.append(&restore_button);

        data_section.append(&data_row);

        let data_status = Label::new(None);
        data_status.set_halign(gtk4::Align::Start);
        data_status.set_wrap(true);
        data_status.set_visible(false);
        data_status.add_css_class("dim-label");
        data_section.append(&data_status);

        let theme_row = Box::new(Orientation::Horizontal, 10);
        theme_row.set_hexpand(true);

        let theme_title = Label::new(Some(&t("settings.theme.title")));
        theme_title.set_halign(gtk4::Align::Start);
        theme_title.set_valign(gtk4::Align::Center);
        theme_row.append(&theme_title);

        let theme_combo = DropDown::from_strings(&[&t("theme.dark"), &t("theme.light")]);
        theme_combo.set_selected(if config.theme == "light" { 1 } else { 0 });
        theme_row.append(&theme_combo);

        data_section.append(&theme_row);

        // Toggle: siguiente episodio automático.
        let auto_play_next_row = Box::new(Orientation::Horizontal, 10);
        auto_play_next_row.set_hexpand(true);

        let auto_play_next_title = Label::new(Some(&t("settings.auto_play_next.title")));
        auto_play_next_title.set_halign(gtk4::Align::Start);
        auto_play_next_title.set_valign(gtk4::Align::Center);
        auto_play_next_row.append(&auto_play_next_title);

        let auto_play_next_switch = Switch::new();
        auto_play_next_switch.set_valign(gtk4::Align::Center);
        auto_play_next_switch.set_active(config.auto_play_next_episode);
        auto_play_next_row.append(&auto_play_next_switch);

        data_section.append(&auto_play_next_row);

        content.append(&data_section);

        // --- Sección: Acerca de ---
        let about_section = Box::new(Orientation::Vertical, 10);
        about_section.set_hexpand(true);
        about_section.set_margin_top(20);
        about_section.set_margin_bottom(20);

        let about_title = Label::new(Some(&t("settings.about.title")));
        about_title.set_halign(gtk4::Align::Start);
        about_title.add_css_class("subheading");
        about_section.append(&about_title);

        let about_label = Label::new(Some(&t("settings.about.description")));
        about_label.set_halign(gtk4::Align::Start);
        about_label.set_wrap(true);
        about_section.append(&about_label);

        let about_button_content = ButtonContent::builder()
            .icon_name("help-about-symbolic")
            .label(t("button.about"))
            .build();
        let about_button = Button::new();
        about_button.set_child(Some(&about_button_content));
        about_button.set_halign(gtk4::Align::Start);
        about_button.add_css_class("flat");
        about_section.append(&about_button);

        content.append(&about_section);

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

        // Copia de seguridad de la base de datos.
        let config_backup = config_rc.clone();
        let config_path_backup = config_path.to_path_buf();
        let window_backup = parent.upcast_ref::<gtk4::Window>().clone();
        let translations_backup = translations.clone();
        let data_status_backup = data_status.clone();
        backup_button.connect_clicked(move |_| {
            let dialog = gtk4::FileDialog::new();
            dialog.set_title(&translations_backup.borrow().t("dialog.backup_db.title"));
            dialog.set_initial_name(Some("acryl-backup.db"));
            let config = config_backup.clone();
            let config_path = config_path_backup.clone();
            let window = window_backup.clone();
            let window_for_dialog = window.clone();
            let translations = translations_backup.clone();
            let status = data_status_backup.clone();
            dialog.save(
                Some(&window_for_dialog),
                None::<&gio::Cancellable>,
                move |result| {
                    // Clonar valores para poder usarlos en el manejo de pánico
                    // fuera del catch_unwind (que es FnOnce y permite mover).
                    let window_for_panic = window.clone();
                    let translations_for_panic = translations.clone();
                    let catch_result = std::panic::catch_unwind(
                        std::panic::AssertUnwindSafe(move || match result {
                            Ok(file) => {
                                let Some(path) = file.path() else {
                                    return;
                                };
                                let config = crate::config::Config {
                                    path: config_path.clone(),
                                    inner: config.borrow().clone(),
                                };
                                match config.backup_db_to(&path) {
                                    Ok(()) => {
                                        let message = translations
                                            .borrow()
                                            .t("status.backup_done_format")
                                            .replacen("{}", &path.display().to_string(), 1);
                                        status.set_label(&message);
                                        status.set_visible(true);
                                    }
                                    Err(error) => {
                                        eprintln!(
                                            "No se pudo guardar la copia de seguridad: {}",
                                            error
                                        );
                                        let title =
                                            translations.borrow().t("error.backup.title");
                                        show_alert(&window, &title, &error.to_string());
                                    }
                                }
                            }
                            Err(error) => {
                                if !error.matches(gio::IOErrorEnum::Cancelled) {
                                    eprintln!(
                                        "No se pudo elegir el destino del backup: {}",
                                        error
                                    );
                                }
                            }
                        }),
                    );
                    if let Err(_panic) = catch_result {
                        eprintln!("Pánico inesperado en backup.");
                        let title =
                            translations_for_panic.borrow().t("error.backup.title");
                        show_alert(
                            &window_for_panic,
                            &title,
                            "Error interno inesperado al guardar la copia de seguridad.",
                        );
                    }
                },
            );
        });

        // Restauración de una copia de seguridad de la base de datos.
        let config_restore = config_rc.clone();
        let config_path_restore = config_path.to_path_buf();
        let window_restore = parent.upcast_ref::<gtk4::Window>().clone();
        let translations_restore = translations.clone();
        let data_status_restore = data_status.clone();
        let folder_manager_restore = folder_manager.clone();
        let metadata_spinner_restore = metadata_spinner.clone();
        restore_button.connect_clicked(move |_| {
            if folder_manager_restore.indexing.get() || metadata_spinner_restore.is_visible() {
                let title = translations_restore.borrow().t("error.restore_busy.title");
                let detail = translations_restore.borrow().t("error.restore_busy.detail");
                show_alert(&window_restore, &title, &detail);
                return;
            }

            let dialog = gtk4::FileDialog::new();
            dialog.set_title(&translations_restore.borrow().t("dialog.restore_db.title"));
            let config = config_restore.clone();
            let config_path = config_path_restore.clone();
            let window = window_restore.clone();
            let window_for_dialog = window.clone();
            let translations = translations_restore.clone();
            let status = data_status_restore.clone();
            let folder_manager = folder_manager_restore.clone();
            // Clonar valores necesarios para el manejo de pánico fuera del
            // catch_unwind.
            let translations_for_panic = translations.clone();
            let window_for_panic = window_for_dialog.clone();
            dialog.open(
                Some(&window_for_dialog),
                None::<&gio::Cancellable>,
                move |result| {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                        match result {
                            Ok(file) => {
                                let Some(path) = file.path() else {
                                    return;
                                };
                                let message =
                                    translations.borrow().t("dialog.restore_db.confirm.title");
                                let detail =
                                    translations.borrow().t("dialog.restore_db.confirm.detail");
                                let cancel = translations.borrow().t("button.cancel");
                                let restore =
                                    translations.borrow().t("dialog.restore_db.confirm.restore");
                                let confirm = gtk4::AlertDialog::builder()
                                    .modal(true)
                                    .message(&message)
                                    .detail(&detail)
                                    .buttons([cancel.as_str(), restore.as_str()])
                                    .cancel_button(0)
                                    .default_button(0)
                                    .build();

                                let config = config.clone();
                                let config_path = config_path.clone();
                                let translations = translations.clone();
                                let status = status.clone();
                                let folder_manager = folder_manager.clone();
                                let window_for_choose = window.clone();
                                let window_for_alert = window.clone();
                                confirm.choose(
                                    Some(&window_for_choose),
                                    None::<&gio::Cancellable>,
                                    move |response| {
                                        if !matches!(response, Ok(1)) {
                                            return;
                                        }
                                        let config = crate::config::Config {
                                            path: config_path.clone(),
                                            inner: config.borrow().clone(),
                                        };
                                        match config.restore_db_from(&path) {
                                            Ok(()) => {
                                                status.set_label(
                                                    &translations
                                                        .borrow()
                                                        .t("status.restore_done"),
                                                );
                                                status.set_visible(true);
                                                // La base de datos cambió:
                                                // recargar la biblioteca desde
                                                // el disco.
                                                let callback =
                                                    folder_manager
                                                        .on_library_changed
                                                        .borrow()
                                                        .clone();
                                                if let Some(callback) = callback {
                                                    callback();
                                                }
                                            }
                                            Err(error) => {
                                                eprintln!(
                                                    "No se pudo restaurar la copia de seguridad: {}",
                                                    error
                                                );
                                                let title =
                                                    translations
                                                        .borrow()
                                                        .t("error.restore.title");
                                                show_alert(
                                                    &window_for_alert,
                                                    &title,
                                                    &error.to_string(),
                                                );
                                            }
                                        }
                                    },
                                );
                            }
                            Err(error) => {
                                if !error.matches(gio::IOErrorEnum::Cancelled) {
                                    eprintln!(
                                        "No se pudo elegir el backup a restaurar: {}",
                                        error
                                    );
                                }
                            }
                        }
                    }));
                    if let Err(panic) = result {
                        eprintln!(
                            "Pánico inesperado en el callback de restaurar backup: {:?}",
                            panic
                        );
                        let title =
                            translations_for_panic.borrow().t("error.restore.title");
                        show_alert(
                            &window_for_panic,
                            &title,
                            "Error interno inesperado al restaurar la copia de seguridad.",
                        );
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
            about_button,
            about_title,
            about_label,
            about_button_content,
            metadata_button,
            metadata_spinner,
            metadata_status,
            data_title,
            data_label,
            backup_button,
            restore_button,
            data_status,
            theme_title,
            theme_combo,
            auto_play_next_switch,
            config: config_rc,
            folder_manager,
            on_save: None,
            on_refresh_metadata: None,
            translations,
            audio_tracks,
            subtitle_tracks,
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
        self.data_title.set_label(&t("settings.data.title"));
        self.data_label.set_label(&t("settings.data.description"));
        self.backup_button.set_label(&t("button.backup_db"));
        self.restore_button.set_label(&t("button.restore_db"));
        self.theme_title.set_label(&t("settings.theme.title"));

        // Actualizar las opciones del desplegable de tema conservando la
        // selección actual del usuario al cambiar de idioma.
        let theme_selected = self.theme_combo.selected();
        let theme_options = [t("theme.dark"), t("theme.light")];
        let theme_option_refs: Vec<&str> = theme_options.iter().map(String::as_str).collect();
        let theme_model = gtk4::StringList::new(&theme_option_refs);
        self.theme_combo.set_model(Some(&theme_model));
        self.theme_combo.set_selected(theme_selected.min(1));
        self.auto_play_next_switch
            .set_active(self.config.borrow().auto_play_next_episode);
        // El label del switch se actualiza con el texto del título.
        // GTK4 Switch no tiene label propio; usamos el Label separado.

        self.cancel_button.set_label(&t("button.cancel"));
        self.save_button.set_label(&t("button.save"));
        self.about_title.set_label(&t("settings.about.title"));
        self.about_label.set_label(&t("settings.about.description"));
        self.about_button_content.set_label(&t("button.about"));

        // Actualizar los selectores de pista con las etiquetas del nuevo
        // idioma, conservando la selección actual.
        let none_label = t("settings.default_tracks.none");
        let all_audio = build_track_options(&self.audio_tracks, &self.translations, &none_label);
        let all_subtitle =
            build_track_options(&self.subtitle_tracks, &self.translations, &none_label);
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
        // Se capturan las referencias compartidas, no los valores, para que el
        // guardado use la selección vigente en el momento del clic y no la que
        // había al registrar el callback.
        let default_audio_key = self.default_audio_selector.selected_key.clone();
        let default_subtitle_key = self.default_subtitle_selector.selected_key.clone();
        let theme_combo = self.theme_combo.clone();
        let auto_play_next_switch = self.auto_play_next_switch.clone();

        self.save_button.connect_clicked(move |_| {
            let new_theme = if theme_combo.selected() == 0 {
                "dark".to_string()
            } else {
                "light".to_string()
            };
            {
                let mut config = config.borrow_mut();
                config.language = if language_combo.selected() == 0 {
                    "es".to_string()
                } else {
                    "en".to_string()
                };

                // Guardar pistas predeterminadas seleccionadas en los selectores.
                config.default_tracks.audio = default_audio_key.borrow().clone();
                config.default_tracks.subtitle = default_subtitle_key.borrow().clone();

                // Persistir el modo de tema elegido.
                config.theme = new_theme.clone();

                // Persistir el siguiente episodio automático.
                config.auto_play_next_episode = auto_play_next_switch.is_active();
            }

            // Aplicar el tema inmediatamente (CSS y AdwStyleManager).
            let mode = if new_theme == "light" {
                crate::ui::theme::ThemeMode::Light
            } else {
                crate::ui::theme::ThemeMode::Dark
            };
            crate::ui::theme::set_theme_mode(mode);

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

    /// Establece el callback del botón «Acerca de».
    pub fn on_about<F>(&mut self, f: F)
    where
        F: Fn() + 'static,
    {
        let cb = Rc::new(f);
        self.about_button.connect_clicked(move |_| {
            cb();
        });
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

    /// Método público para tests E2E: número de carpetas mostradas.
    pub fn folder_count_for_test(&self) -> usize {
        let mut count = 0;
        let mut child = self.folder_manager.folders_list.first_child();
        while let Some(row) = child {
            count += 1;
            child = row.next_sibling();
        }
        count
    }

    /// Método público para tests E2E: selecciona una pista predeterminada.
    ///
    /// `audio` indica si se modifica el selector de audio (si es `false`, el
    /// de subtítulos). `None` representa la opción «Ninguno».
    pub fn select_default_track_for_test(&mut self, audio: bool, key: Option<&str>) {
        if audio {
            self.default_audio_selector.select_key(key);
        } else {
            self.default_subtitle_selector.select_key(key);
        }
    }

    /// Devuelve el botón Guardar para conectarlo desde MainWindow.
    pub fn save_button(&self) -> &Button {
        &self.save_button
    }

    /// Devuelve el botón «Acerca de» para conectarlo desde MainWindow.
    pub fn about_button(&self) -> &Button {
        &self.about_button
    }

    /// Devuelve el botón Volver para conectarlo desde MainWindow.
    pub fn cancel_button(&self) -> &Button {
        &self.cancel_button
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opciones() -> Vec<(String, String)> {
        vec![
            ("none".to_string(), "Ninguno".to_string()),
            ("spa".to_string(), "Español".to_string()),
            ("spa-5.1".to_string(), "Español 5.1".to_string()),
            ("eng".to_string(), "Inglés".to_string()),
        ]
    }

    fn claves(opciones: &[(String, String)]) -> Vec<String> {
        opciones.iter().map(|(k, _)| k.clone()).collect()
    }

    #[test]
    fn filtrar_sin_texto_devuelve_todas_las_opciones() {
        let original = opciones();
        let filtradas = filtrar_opciones("", &original);
        assert_eq!(filtradas, original);
    }

    #[test]
    fn filtrar_por_prefijo_ignora_mayusculas() {
        let filtradas = filtrar_opciones("ESP", &opciones());
        assert_eq!(claves(&filtradas), vec!["spa", "spa-5.1"]);
    }

    #[test]
    fn filtrar_sin_coincidencias_devuelve_lista_vacia() {
        let filtradas = filtrar_opciones("zzz", &opciones());
        assert!(filtradas.is_empty());
    }

    #[test]
    fn filtrar_no_modifica_las_opciones_originales() {
        let original = opciones();
        let _ = filtrar_opciones("spa", &original);
        assert_eq!(original, opciones());
    }

    #[test]
    fn filtrar_sobre_lista_vacia_devuelve_vacio() {
        assert!(filtrar_opciones("", &[]).is_empty());
        assert!(filtrar_opciones("spa", &[]).is_empty());
    }

    #[test]
    fn obtener_label_devuelve_la_etiqueta_de_una_clave_existente() {
        assert_eq!(obtener_label_para_clave(&opciones(), "eng"), Some("Inglés"));
    }

    #[test]
    fn obtener_label_devuelve_none_si_la_clave_no_existe() {
        assert_eq!(obtener_label_para_clave(&opciones(), "jpn"), None);
        assert_eq!(obtener_label_para_clave(&opciones(), ""), None);
    }

    #[test]
    fn normalizar_ruta_vacia_o_en_blanco_devuelve_none() {
        assert_eq!(normalize_folder_path(""), None);
        assert_eq!(normalize_folder_path("   "), None);
    }

    #[test]
    fn normalizar_ruta_inexistente_devuelve_none() {
        assert_eq!(normalize_folder_path("/ruta/que/no/existe/acryl"), None);
    }

    #[test]
    fn normalizar_ruta_de_archivo_devuelve_none() {
        let dir = tempfile::tempdir().expect("crear directorio temporal");
        let file = dir.path().join("archivo.txt");
        std::fs::write(&file, b"contenido").expect("crear archivo temporal");
        assert_eq!(
            normalize_folder_path(file.to_str().expect("ruta UTF-8")),
            None
        );
    }

    #[test]
    fn normalizar_ruta_de_directorio_elimina_separador_final() {
        let dir = tempfile::tempdir().expect("crear directorio temporal");
        let canonical = std::fs::canonicalize(dir.path()).expect("canonicalizar");
        let expected = canonical.to_string_lossy().to_string();

        let con_separador = format!("{}/", expected);
        assert_eq!(
            normalize_folder_path(&con_separador),
            Some(expected.clone())
        );
        assert_eq!(normalize_folder_path(&expected), Some(expected));
    }

    #[test]
    fn seleccionar_opcion_no_mantiene_el_prestamo_durante_el_callback() {
        let displayed = RefCell::new(opciones());
        let mut seleccionada = None;

        with_selected_option(&displayed, 2, |key, label| {
            // Si el préstamo de lectura siguiera activo, este `borrow_mut`
            // entraría en panic, igual que ocurría al seleccionar una fila
            // del selector de pistas en Ajustes.
            *displayed.borrow_mut() = Vec::new();
            seleccionada = Some((key, label));
        });

        assert_eq!(
            seleccionada,
            Some(("spa-5.1".to_string(), "Español 5.1".to_string()))
        );
    }

    #[test]
    fn normalizar_ruta_recorta_espacios_exteriores() {
        let dir = tempfile::tempdir().expect("crear directorio temporal");
        let esperado = std::fs::canonicalize(dir.path())
            .expect("canonicalizar")
            .to_string_lossy()
            .to_string();
        let con_espacios = format!("  {}  ", esperado);
        assert_eq!(normalize_folder_path(&con_espacios), Some(esperado));
    }
}
