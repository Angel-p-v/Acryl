//! Ventana principal de Acryl con GTK4 / Libadwaita.

#![allow(dead_code)]

use crate::db::media_repo::ListMediaOptions;
use crate::db::{MediaRepository, close_db, open_db};
use crate::ui::media_details::MediaDetails;
use crate::ui::media_list::MediaList;
use crate::ui::search_bar::SearchBar;
use gtk4::prelude::*;
use gtk4::{Box, Orientation, Stack};
use libadwaita::prelude::*;
use libadwaita::{ApplicationWindow as AdwApplicationWindow, HeaderBar};
use std::cell::RefCell;
use std::rc::Rc;

pub struct MainWindow {
    window: AdwApplicationWindow,
    content: Box,
    stack: Stack,
    media_list: MediaList,
    media_details: MediaDetails,
    db_path: String,
    search_bar: SearchBar,
}

type MainWindowRef = Rc<RefCell<MainWindow>>;

impl MainWindow {
    pub fn new(app: &gtk4::Application, config: crate::Config) -> MainWindowRef {
        let window = AdwApplicationWindow::builder()
            .application(app)
            .title("Acryl")
            .default_width(1000)
            .default_height(700)
            .build();

        let header = HeaderBar::new();
        window.set_titlebar(Some(&header));

        let content = Box::new(Orientation::Vertical, 0);
        window.set_content(Some(&content));

        let db_path = config.inner().db_path.clone();

        let stack = Stack::new();
        stack.set_hexpand(true);
        stack.set_vexpand(true);
        content.append(&stack);

        let search_bar = SearchBar::new(&config.inner().db_path);
        content.append(search_bar.widget());

        let mut media_list = MediaList::new();
        stack.add_titled(media_list.widget(), Some("list"), "Lista");

        let media_details = MediaDetails::new(&db_path, 0);
        stack.add_titled(media_details.widget(), Some("details"), "Detalles");

        stack.set_visible_child_name("list");

        Self::load_media(&mut media_list, &db_path);

        let this = Rc::new(RefCell::new(Self {
            window,
            content,
            stack,
            media_list,
            media_details,
            db_path: db_path.clone(),
            search_bar,
        }));

        // Conectar callbacks después de crear this (los widgets están dentro de Self)
        {
            let mut w = this.borrow_mut();
            let this_clone = this.clone();
            w.search_bar.on_select(move |media_id| {
                let mut w2 = this_clone.borrow_mut();
                w2.show_details(media_id);
            });
        }
        {
            let mut w = this.borrow_mut();
            let this_clone = this.clone();
            w.media_details.on_back(move || {
                let mut w2 = this_clone.borrow_mut();
                w2.show_list();
            });
        }
        {
            let mut w = this.borrow_mut();
            let this_clone = this.clone();
            w.media_details.on_favorite(move || {
                let mut w2 = this_clone.borrow_mut();
                w2.toggle_favorite();
            });
        }
        {
            let mut w = this.borrow_mut();
            let this_clone = this.clone();
            w.media_details.on_watched(move || {
                let mut w2 = this_clone.borrow_mut();
                w2.toggle_watched();
            });
        }
        {
            let mut w = this.borrow_mut();
            let this_clone = this.clone();
            w.media_details.on_play(move || {
                let mut w2 = this_clone.borrow_mut();
                w2.play_file();
            });
        }

        // Mostrar ventana antes de moverla al struct (ya está en Self, usar borrow)
        this.borrow().window.show();

        this
    }

    fn load_media(media_list: &mut MediaList, db_path: &str) {
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
                limit: Some(50),
                offset: Some(0),
                order_by: None,
            })
            .unwrap_or_default();
        close_db(conn);

        media_list.set_media(media);
    }

    fn show_details(&mut self, media_id: i64) {
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

        if let Some(m) = media {
            self.media_details.set_media(&m);
            self.stack.set_visible_child_name("details");
        }
    }

    fn show_list(&mut self) {
        self.stack.set_visible_child_name("list");
        self.media_list.refresh();
    }

    fn toggle_favorite(&mut self) {
        let media_id = self.media_details.media_id();
        let mut conn = match open_db(&self.db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error abriendo BD: {}", e);
                return;
            }
        };
        let mut repo = MediaRepository::new(&mut conn);
        let _ = repo.toggle_favorite(media_id);
        close_db(conn);

        self.refresh_details(media_id);
    }

    fn toggle_watched(&mut self) {
        let media_id = self.media_details.media_id();
        let mut conn = match open_db(&self.db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error abriendo BD: {}", e);
                return;
            }
        };
        let mut repo = MediaRepository::new(&mut conn);
        let _ = repo.toggle_watched(media_id);
        close_db(conn);

        self.refresh_details(media_id);
    }

    fn refresh_details(&mut self, media_id: i64) {
        let mut conn = match open_db(&self.db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error abriendo BD: {}", e);
                return;
            }
        };
        let mut repo = MediaRepository::new(&mut conn);
        if let Some(m) = repo.find_by_id(media_id).unwrap_or(None) {
            self.media_details.set_media(&m);
        }
        close_db(conn);
    }

    fn play_file(&mut self) {
        let media_id = self.media_details.media_id();
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
            println!("Reproduciendo: {}", f.file_path);
        } else {
            eprintln!("No hay archivos para el medio {}", media_id);
        }
    }

    pub fn widget(&self) -> &AdwApplicationWindow {
        &self.window
    }
}
