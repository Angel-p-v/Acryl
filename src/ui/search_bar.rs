//! Barra de búsqueda de medios.

use crate::db::{open_db, close_db, MediaRepository};
use crate::core::library::LibraryService;
use gtk4::prelude::*;
use gtk4::{Box, Entry, Label, Orientation};
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::Arc;
use std::sync::Mutex;

pub struct SearchBar {
    box_: Box,
    entry: Entry,
    results_label: Label,
    db_path: String,
    on_media_select: Arc<Mutex<Option<Rc<dyn Fn(i64)>>>>,
}

impl SearchBar {
    pub fn new(db_path: &str) -> Self {
        let box_ = Box::new(Orientation::Vertical, 5);
        box_.set_margin_top(10);
        box_.set_margin_bottom(10);
        box_.set_margin_start(10);
        box_.set_margin_end(10);

        let entry = Entry::new();
        entry.set_placeholder_text("Buscar por título...");
        entry.set_hexpand(true);
        box_.append(&entry);

        let results_label = Label::new(None);
        results_label.set_halign(gtk4::Align::Start);
        results_label.set_hexpand(true);
        box_.append(&results_label);

        Self {
            box_,
            entry,
            results_label,
            db_path: db_path.to_string(),
            on_media_select: Arc::new(Mutex::new(None)),
        }
    }

    pub fn widget(&self) -> &Box {
        &self.box_
    }

    pub fn on_select<F>(&mut self, f: F)
    where
        F: Fn(i64) + 'static,
    {
        self.on_media_select = Arc::new(Mutex::new(Some(Rc::new(f))));
        self.setup_search_handler();
    }

    fn setup_search_handler(&self) {
        let entry = self.entry.clone();
        let results_label = self.results_label.clone();
        let db_path = self.db_path.clone();
        let on_select = self.on_media_select.clone();

        entry.connect_activate(move |e| {
            let query = e.text();
            if query.is_empty() {
                results_label.set_label("");
                return;
            }

            let db_path = db_path.clone();
            let on_select = on_select.clone();

            std::thread::spawn(move || {
                let results = crate::core::library::LibraryService::search_media(
                    &db_path,
                    &query,
                    None,
                    Some(20),
                )
                .unwrap_or_default();

                let count = results.len();
                let results_clone = results.clone();
                let on_select_inner = on_select.clone();

                glib::idle_add_once(move || {
                    results_label.set_label(&format!("{} resultados", count));

                    for m in results_clone.iter() {
                        let m_id = m.id;
                        let title = m.title.clone();
                        let on_select = on_select.clone();
                        glib::idle_add_once(move || {
                            on_select(m_id);
                            println!("Seleccionado: {} (id={})", title, m_id);
                        });
                    }
                });
            });
        });
    }
}
