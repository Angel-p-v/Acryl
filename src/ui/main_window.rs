//! Ventana principal de Acryl con GTK4 / Libadwaita.

use crate::db::media_repo::ListMediaOptions;
use crate::db::{MediaRepository, close_db, open_db};
use crate::ui::media_list::MediaList;
use crate::ui::search_bar::SearchBar;
use gtk4::Box;
use gtk4::prelude::*;
use libadwaita::prelude::*;
use libadwaita::{ApplicationWindow as AdwApplicationWindow, HeaderBar};

#[allow(dead_code)]
pub struct MainWindow {
    window: AdwApplicationWindow,
    content: Box,
    media_list: MediaList,
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

        let mut search_bar = SearchBar::new(&config.inner().db_path);
        content.append(search_bar.widget());

        let mut media_list = MediaList::new();
        content.append(media_list.widget());

        Self::load_media(&mut media_list, &config.inner().db_path);

        let db_path = config.inner().db_path.clone();
        search_bar.on_select(move |media_id| {
            Self::show_details(&db_path, media_id);
        });

        window.show();

        MainWindow {
            window,
            content,
            media_list,
            db_path: config.inner().db_path.clone(),
        }
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

    fn show_details(db_path: &str, media_id: i64) {
        let mut conn = match open_db(db_path) {
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
    }

    pub fn widget(&self) -> &AdwApplicationWindow {
        &self.window
    }
}
