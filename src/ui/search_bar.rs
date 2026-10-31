//! Barra de búsqueda de medios.

use crate::db::media_repo::{MediaRepository, SearchMediaOptions};
use crate::db::{close_db, open_db};
use gtk4::prelude::*;
use gtk4::{Box, Entry, Label, Orientation};
use std::cell::RefCell;
use std::rc::Rc;

type MediaSelectCallback = std::boxed::Box<dyn Fn(i64)>;

pub struct SearchBar {
    box_: Box,
    entry: Entry,
    results_label: Label,
    db_path: String,
    on_media_select: Rc<RefCell<Option<MediaSelectCallback>>>,
}

impl SearchBar {
    pub fn new(db_path: &str) -> Self {
        let box_ = Box::new(Orientation::Vertical, 5);
        box_.set_margin_top(10);
        box_.set_margin_bottom(10);
        box_.set_margin_start(10);
        box_.set_margin_end(10);

        let entry = Entry::new();
        entry.set_placeholder_text(Some("Buscar por título..."));
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
            on_media_select: Rc::new(RefCell::new(None)),
        }
    }

    pub fn widget(&self) -> &Box {
        &self.box_
    }

    /// Limpia el texto de búsqueda y el etiquetado de resultados.
    pub fn clear_query(&self) {
        self.entry.set_text("");
        self.results_label.set_label("");
    }

    pub fn on_select<F>(&mut self, f: F)
    where
        F: Fn(i64) + 'static,
    {
        *self.on_media_select.borrow_mut() = Some(std::boxed::Box::new(f));
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

            // Búsqueda en hilo principal: SQLite local es rápido, no bloquea perceptiblemente.
            let mut conn = match open_db(&db_path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error abriendo BD para búsqueda: {}", e);
                    return;
                }
            };
            let mut repo = MediaRepository::new(&mut conn);
            let results = repo
                .search(
                    &query,
                    SearchMediaOptions {
                        r#type: None,
                        limit: Some(20),
                    },
                )
                .unwrap_or_else(|e| {
                    eprintln!("Error en búsqueda: {}", e);
                    Vec::new()
                });
            close_db(conn);

            let count = results.len();
            results_label.set_label(&format!("{} resultados", count));

            for m in results.iter() {
                println!("Resultado: {} (id={})", m.title, m.id);
            }

            if let Some(cb) = &*on_select.borrow()
                && let Some(first) = results.first()
            {
                cb(first.id);
            }
        });
    }
}
