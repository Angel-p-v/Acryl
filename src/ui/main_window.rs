//! Ventana principal de Acryl con GTK4 / Libadwaita.

use crate::db::{open_db, close_db, MediaRepository};
use crate::ui::search_bar::SearchBar;
use crate::ui::media_list::MediaList;
use crate::core::library::LibraryService;
use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Box, Orientation};
use libadwaita::prelude::*;
use libadwaita::{ApplicationWindow as AdwApplicationWindow, HeaderBar};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;

pub struct MainWindow {
    window: AdwApplicationWindow,
    content: Box,
    media_list: Arc<Mutex<MediaList>>,
    db_path: String,
}

impl MainWindow {
    pub fn new(app: &gtk4::Application, config: crate::Config) -> Self {
        let window = AdwApplicationWindow::builder()
            .application(app)
            .title("Acryl")
            .default_width(1000)
            .default_height(700)
            .build();

        let header = HeaderBar::new();
        window.set_titlebar(Some(&header));

        let content = Box::new(gtk4::Orientation::Vertical, 0);
        window.set_content(Some(&content));

        let search_bar = SearchBar::new(&config.inner().db_path);
        content.append(&search_bar.widget());

        let media_list = MediaList::new();
        content.append(&media_list.widget());

        Self::load_media(&media_list, &config.inner().db_path);

        let media_list_clone = media_list.clone();
        let db_path = config.inner().db_path.clone();
        media_list.on_select(move |media_id| {
            Self::show_details(&media_list_clone, &db_path, media_id);
        });

        window.show();

        MainWindow {
            window,
            content,
            media_list: Arc::new(Mutex::new(media_list)),
            db_path: config.inner().db_path.clone(),
        }
    }

    fn load_media(media_list: &Arc<Mutex<MediaList>>, db_path: &str) {
        let media_list_clone = media_list.clone();
        let db_path_clone = db_path.to_string();

        std::thread::spawn(move || {
            let mut conn = match crate::db::open_db(&db_path_clone) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error abriendo BD: {}", e);
                    return;
                }
            };
            let mut repo = MediaRepository::new(&mut conn);
            let media = repo.list(crate::db::media_repo::ListMediaOptions {
                r#type: None,
                limit: Some(50),
                offset: Some(0),
                order_by: None,
            }).unwrap_or_default();
            close_db(conn);

            media_list_clone.lock().unwrap().set_media(media);
        });
    }

    fn show_details(media_list: &Arc<Mutex<MediaList>>, db_path: &str, media_id: i64) {
        let media_list_clone = media_list.clone();
        let db_path_clone = db_path.to_string();

        std::thread::spawn(move || {
            let mut conn = match crate::db::open_db(&db_path_clone) {
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
                println!("Detalles de: {} ({})", m.title, m.folder_path);
            }
        });

        glib::idle_add_once(move || {
            media_list.lock().unwrap().refresh();
        });
    }

    pub fn widget(&self) -> &AdwApplicationWindow {
        &self.window
    }
}
