//! Vista de ajustes integrada en el Stack de la aplicación principal.

use crate::config::AppConfig;
use crate::core::library::LibraryService;
use gtk4::prelude::*;
use gtk4::{Box, Button, ComboBoxText, Entry, Label, Orientation, ScrolledWindow};
use std::cell::RefCell;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;

/// Normaliza una ruta de carpeta asegurando que termine con "/"
fn normalize_folder_path(path: &str) -> String {
    let mut s = path.trim().to_string();
    if !s.ends_with('/') && PathBuf::from(&s).is_dir() {
        s.push('/');
    }
    s
}

/// Vista de ajustes integrada (se muestra dentro del Stack de MainWindow).
pub struct SettingsView {
    content: Box,
    folders_list: Box,
    language_combo: ComboBoxText,
    save_button: Button,
    cancel_button: Button,
    config: Rc<RefCell<AppConfig>>,
    on_save: Option<Rc<dyn Fn(AppConfig)>>,
}

impl SettingsView {
    /// Crea una nueva vista de ajustes.
    pub fn new(config: &AppConfig, db_path: &str) -> Self {
        let content = Box::new(Orientation::Vertical, 10);
        content.set_margin_top(20);
        content.set_margin_bottom(20);
        content.set_margin_start(20);
        content.set_margin_end(20);
        content.set_vexpand(true);
        content.set_hexpand(true);

        let title = Label::new(Some("Ajustes"));
        title.set_halign(gtk4::Align::Start);
        title.add_css_class("heading");
        content.append(&title);

        // --- Sección: Rutas de medios ---
        let folders_section = Box::new(Orientation::Vertical, 10);
        folders_section.set_hexpand(true);

        let folders_title = Label::new(Some("Rutas de medios"));
        folders_title.set_halign(gtk4::Align::Start);
        folders_title.add_css_class("subheading");
        folders_section.append(&folders_title);

        let folders_label = Label::new(Some(
            "Carpetas donde se indexan los archivos multimedia:\nEscribe o pega la ruta y pulsa Agregar o Enter.",
        ));
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
        folder_entry.set_placeholder_text(Some("/ruta/a/tus/medios"));
        folder_entry.set_hexpand(true);
        add_box.append(&folder_entry);

        let browse_button = Button::with_label("Examinar...");
        browse_button.set_hexpand(false);
        add_box.append(&browse_button);

        let add_button = Button::with_label("Agregar carpeta");
        add_box.append(&add_button);
        folders_section.append(&add_box);

        // --- Sección: Idioma ---
        let language_section = Box::new(Orientation::Vertical, 10);
        language_section.set_hexpand(true);

        let language_title = Label::new(Some("Idioma"));
        language_title.set_halign(gtk4::Align::Start);
        language_title.add_css_class("subheading");
        language_section.append(&language_title);

        let language_combo = ComboBoxText::new();
        language_combo.append_text("Español");
        language_combo.append_text("Inglés");
        language_combo.set_active(if config.language == "es" {
            Some(0)
        } else {
            Some(1)
        });
        language_section.append(&language_combo);

        content.append(&folders_section);
        content.append(&language_section);

        // --- Botones Guardar / Volver ---
        let button_box = Box::new(Orientation::Horizontal, 10);
        button_box.set_halign(gtk4::Align::End);
        button_box.set_hexpand(true);

        let cancel_button = Button::with_label("Volver");
        button_box.append(&cancel_button);

        let save_button = Button::with_label("Guardar");
        save_button.add_css_class("suggested-action");
        button_box.append(&save_button);

        content.append(&button_box);

        // Cargar carpetas existentes.
        let config_clone = Rc::new(RefCell::new(config.clone()));
        for folder in &config.media_folders {
            Self::add_folder_row(&folders_list, folder, &config_clone);
        }

        let db_path_owned = db_path.to_string();

        // Agregar carpeta al hacer clic en el botón.
        let config_add_click = config_clone.clone();
        let folders_list_add_click = folders_list.clone();
        let db_path_add_click = db_path_owned.clone();
        let folder_entry_click = folder_entry.clone();
        add_button.connect_clicked(move |_| {
            let path = folder_entry_click.text().to_string();
            if path.is_empty() {
                return;
            }
            let path_str = normalize_folder_path(&path);
            if !std::path::Path::new(&path_str).exists() {
                eprintln!("La carpeta no existe: {}", path_str);
                return;
            }
            let mut cfg = config_add_click.borrow_mut();
            if !cfg.media_folders.contains(&path_str) {
                cfg.media_folders.push(path_str.clone());
            }
            drop(cfg);
            Self::add_folder_row(&folders_list_add_click, &path_str, &config_add_click);
            let _ = LibraryService::index_folder(&db_path_add_click, &path_str);
            folder_entry_click.set_text("");
        });

        // Botón Examinar: abre el explorador de archivos para seleccionar carpeta.
        let folder_entry_browse = folder_entry.clone();
        browse_button.connect_clicked(move |_| {
            let folder_entry_browse_inner = folder_entry_browse.clone();

            // Intentar con zenity (GNOME)
            let output = Command::new("zenity")
                .args([
                    "--file-selection",
                    "--directory",
                    "--title=Seleccionar carpeta de medios",
                ])
                .output();

            if let Ok(output) = output
                && output.status.success()
            {
                let path_str =
                    normalize_folder_path(String::from_utf8_lossy(&output.stdout).trim());
                if !path_str.is_empty() {
                    folder_entry_browse_inner.set_text(&path_str);
                    return;
                }
            }

            // Intentar con kdialog (KDE)
            let output = Command::new("kdialog")
                .args(["--getdirectory", "/home", "Seleccionar carpeta de medios"])
                .output();

            if let Ok(output) = output
                && output.status.success()
            {
                let path_str =
                    normalize_folder_path(String::from_utf8_lossy(&output.stdout).trim());
                if !path_str.is_empty() {
                    folder_entry_browse_inner.set_text(&path_str);
                }
            }
        });

        // Agregar carpeta al pulsar Enter.
        let config_add_enter = config_clone.clone();
        let folders_list_add_enter = folders_list.clone();
        let db_path_add_enter = db_path_owned.clone();
        let folder_entry_enter_signal = folder_entry.clone();
        let folder_entry_enter_closure = folder_entry.clone();
        folder_entry_enter_signal.connect_activate(move |_| {
            let entry = folder_entry_enter_closure.clone();
            let path = entry.text().to_string();
            if path.is_empty() {
                return;
            }
            let path_str = normalize_folder_path(&path);
            if !std::path::Path::new(&path_str).exists() {
                eprintln!("La carpeta no existe: {}", path_str);
                return;
            }
            let mut cfg = config_add_enter.borrow_mut();
            if !cfg.media_folders.contains(&path_str) {
                cfg.media_folders.push(path_str.clone());
            }
            drop(cfg);
            Self::add_folder_row(&folders_list_add_enter, &path_str, &config_add_enter);
            let _ = LibraryService::index_folder(&db_path_add_enter, &path_str);
            entry.set_text("");
        });

        Self {
            content,
            folders_list,
            language_combo,
            save_button,
            cancel_button,
            config: config_clone,
            on_save: None,
        }
    }

    /// Añade una fila para una carpeta de medios en la lista.
    fn add_folder_row(folders_list: &Box, folder: &str, config: &Rc<RefCell<AppConfig>>) {
        let row = Box::new(Orientation::Horizontal, 5);
        row.set_hexpand(true);

        let label = Label::new(Some(folder));
        label.set_hexpand(true);
        row.append(&label);

        let remove_button = Button::with_label("Eliminar");
        row.append(&remove_button);

        let config_clone = config.clone();
        let folder_clone = folder.to_string();
        let row_clone = row.clone();
        let folders_list_clone = folders_list.clone();
        remove_button.connect_clicked(move |_| {
            let mut cfg = config_clone.borrow_mut();
            cfg.media_folders.retain(|f| f != &folder_clone);
            drop(cfg);
            folders_list_clone.remove(&row_clone);
        });

        folders_list.append(&row);
    }

    /// Devuelve el widget Gtk.
    pub fn widget(&self) -> &Box {
        &self.content
    }

    /// Establece el callback de guardar (se ejecuta al hacer clic en Guardar).
    pub fn on_save<F>(&mut self, f: F)
    where
        F: Fn(AppConfig) + 'static,
    {
        self.on_save = Some(Rc::new(f));

        let config = self.config.clone();
        let folders_list = self.folders_list.clone();
        let language_combo = self.language_combo.clone();
        let on_save = self.on_save.clone();

        self.save_button.connect_clicked(move |_| {
            let mut cfg = config.borrow_mut();

            // Recopilar carpetas de la lista visual.
            let mut new_folders = Vec::new();
            if let Some(child) = folders_list.first_child() {
                let mut current = child.clone();
                loop {
                    if let Some(child_widget) = current.first_child()
                        && let Ok(label) = child_widget.downcast::<Label>()
                    {
                        let text = label.text();
                        new_folders.push(text.to_string());
                    }
                    match current.next_sibling() {
                        Some(sibling) => current = sibling,
                        None => break,
                    }
                }
            }
            cfg.media_folders = new_folders;

            // Idioma.
            cfg.language = if language_combo.active() == Some(0) {
                "es".to_string()
            } else {
                "en".to_string()
            };

            drop(cfg);

            if let Some(cb) = &on_save {
                cb(config.borrow().clone());
            }
        });
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
        unimplemented!("SettingsView requires config and db_path")
    }
}
