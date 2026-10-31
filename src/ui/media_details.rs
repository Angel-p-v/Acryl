//! Vista de detalles de un medio al estilo Crunchyroll.
//!
//! Distribución:
//! - Banner hero a todo el ancho (520px) con título y tipo/año.
//! - Fila de acciones en línea: Volver, Favorito, Visto, Calificación,
//!   Pistas, Continuar/Reproducir.
//! - Desplegable de temporada (solo series).
//! - Sinopsis.
//! - Rejilla de episodios con portadas.
//!
//! Todas las interacciones funcionan: hover, active, focus, teclado.

use crate::core::poster_service::cached_path;
use crate::db::media_repo::Media;
use crate::db::{MediaRepository, close_db, open_db};
use crate::ui::poster::{Banner, poster_picture};
use gtk4::gdk_pixbuf::Pixbuf as GtkPixbuf;
use gtk4::prelude::*;
use gtk4::{Box, Button, DropDown, FlowBox, GestureClick, Label, Orientation};
use std::cell::RefCell;
use std::rc::Rc;

/// Tamaño de la portada de un episodio en la rejilla (relación 16:9).
const EPISODE_POSTER_WIDTH: i32 = 180;
const EPISODE_POSTER_HEIGHT: i32 = 101;

/// Número máximo de estrellas para la calificación.
const MAX_STARS: i32 = 5;

/// Destino del botón «Volver».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackTarget {
    Parent(i64),
    List,
}

pub fn back_target(parent_id: Option<i64>) -> BackTarget {
    match parent_id.filter(|id| *id > 0) {
        Some(id) => BackTarget::Parent(id),
        None => BackTarget::List,
    }
}

/// Componente de vista de detalles.
pub struct MediaDetails {
    box_: Box,
    db_path: String,
    media_id: i64,
    parent_id: Option<i64>,
    banner: Banner,
    series_poster_url: Option<String>,
    main_button_file_path: Rc<RefCell<Option<String>>>,
    main_button_label: String,
    synopsis_label: Label,
    favorite_button: Button,
    watched_button: Button,
    rating_button: Button,
    tracks_button: Button,
    play_button: Button,
    back_button: Button,
    season_dropdown: DropDown,
    season_dropdown_model: gtk4::StringList,
    content_container: Box,
    on_back: Option<Rc<dyn Fn()>>,
    on_favorite: Option<Rc<dyn Fn()>>,
    on_watched: Option<Rc<dyn Fn()>>,
    on_rate: Option<Rc<dyn Fn(i32)>>,
    on_play_episode: Option<Rc<dyn Fn(String)>>,
    on_edit_tracks: Option<Rc<dyn Fn(String)>>,
    on_change_season: Option<Rc<dyn Fn(i64)>>,
}

impl MediaDetails {
    pub fn new(db_path: &str, media_id: i64) -> Self {
        let box_ = Box::new(Orientation::Vertical, 0);
        box_.set_margin_top(0);
        box_.set_margin_bottom(0);
        box_.set_margin_start(0);
        box_.set_margin_end(0);
        box_.set_valign(gtk4::Align::Start);
        box_.set_hexpand(true);

        let banner = Banner::new();

        let synopsis_label = Label::new(None);
        synopsis_label.set_halign(gtk4::Align::Start);
        synopsis_label.set_valign(gtk4::Align::Start);
        synopsis_label.set_xalign(0.0);
        synopsis_label.set_hexpand(true);
        synopsis_label.set_wrap(true);
        synopsis_label.add_css_class("synopsis-text");

        // Banner hero a todo el ancho.
        let details_header = Box::new(Orientation::Vertical, 0);
        details_header.add_css_class("details-header");
        details_header.set_hexpand(true);
        details_header.set_valign(gtk4::Align::Start);
        details_header.append(banner.widget());

        // Fila de acciones en línea.
        let buttons_box = Box::new(Orientation::Horizontal, 10);
        buttons_box.set_halign(gtk4::Align::Start);
        buttons_box.set_valign(gtk4::Align::Center);
        buttons_box.set_margin_top(14);
        buttons_box.set_margin_start(16);
        buttons_box.add_css_class("hero-actions");

        let back_button = Button::with_label("← Volver");
        back_button.add_css_class("flat-button");
        buttons_box.append(&back_button);

        let favorite_button = Button::with_label("★ Favorito");
        favorite_button.add_css_class("flat-button");
        buttons_box.append(&favorite_button);

        let watched_button = Button::with_label("✓ Visto");
        watched_button.add_css_class("flat-button");
        buttons_box.append(&watched_button);

        let rating_button = Button::with_label("☆☆☆☆☆ Calificar");
        rating_button.add_css_class("flat-button");
        buttons_box.append(&rating_button);

        let tracks_button = Button::with_label("🎵 Pistas");
        tracks_button.add_css_class("flat-button");
        tracks_button.set_tooltip_text(Some("Elegir pista de audio y subtítulos"));
        buttons_box.append(&tracks_button);

        let play_button = Button::with_label("▶ Reproducir");
        play_button.add_css_class("hero-action-btn");
        play_button.add_css_class("primary");
        buttons_box.append(&play_button);

        // Desplegable de temporada.
        let season_dropdown_model = gtk4::StringList::new(&[]);
        let season_dropdown = gtk4::DropDown::builder()
            .model(&season_dropdown_model)
            .build();
        season_dropdown.add_css_class("season-dropdown");
        season_dropdown.set_hexpand(false);
        season_dropdown.set_valign(gtk4::Align::Center);
        season_dropdown.set_margin_top(14);
        season_dropdown.set_margin_start(16);
        season_dropdown.set_visible(false);

        // Contenedor de contenido dinámico.
        let content_container = Box::new(Orientation::Vertical, 0);
        content_container.set_hexpand(true);
        content_container.set_valign(gtk4::Align::Start);

        box_.append(&details_header);
        box_.append(&synopsis_label);
        box_.append(&buttons_box);
        box_.append(&season_dropdown);
        box_.append(&content_container);

        let main_button_file_path = Rc::new(RefCell::new(None));

        Self {
            box_,
            db_path: db_path.to_string(),
            media_id,
            parent_id: None,
            banner,
            series_poster_url: None,
            main_button_file_path,
            main_button_label: "▶ Reproducir".to_string(),
            synopsis_label,
            favorite_button,
            watched_button,
            rating_button,
            tracks_button,
            play_button,
            back_button,
            season_dropdown,
            season_dropdown_model,
            content_container,
            on_back: None,
            on_favorite: None,
            on_watched: None,
            on_rate: None,
            on_play_episode: None,
            on_edit_tracks: None,
            on_change_season: None,
        }
    }

    pub fn set_db_path(&mut self, db_path: &str) {
        self.db_path = db_path.to_string();
    }

    pub fn load_media(&mut self, media_id: i64) {
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

        if let Some(media) = media {
            self.set_media(&media);
        }
    }

    pub fn set_media(&mut self, media: &Media) {
        self.media_id = media.id;
        self.parent_id = media.parent_id;

        self.back_button
            .set_label(match back_target(media.parent_id) {
                BackTarget::Parent(_) if media.r#type == "anime" => "← Volver al anime",
                BackTarget::Parent(_) => "← Volver a la serie",
                BackTarget::List => "← Volver a la lista",
            });

        let type_label = match media.r#type.as_str() {
            "series" => "Serie",
            "anime" => "Anime",
            "movie" => "Película",
            other => other,
        };
        let info_label = match media.year {
            Some(year) => format!("{} · {}", type_label, year),
            None => type_label.to_string(),
        };
        self.synopsis_label
            .set_label(&media.synopsis.clone().unwrap_or_default());
        self.banner.set_info(&media.title, &info_label);
        self.banner
            .set_images(media.banner_path.as_deref(), media.poster_path.as_deref());

        if media.parent_id.is_none() {
            self.series_poster_url = media.poster_path.clone();
        } else if let Some(parent_id) = media.parent_id {
            self.series_poster_url = if let Ok(mut conn) = open_db(&self.db_path) {
                let mut repo = MediaRepository::new(&mut conn);
                repo.get_poster_by_id(parent_id).ok().flatten()
            } else {
                None
            };
        }

        self.update_rating_button(media.rating);

        let (label, _file_path) = if media.parent_id.is_none() {
            if let Ok(mut conn) = open_db(&self.db_path) {
                let mut repo = MediaRepository::new(&mut conn);
                if let Ok(Some((_id, path, ep_label))) = repo.get_next_episode_to_watch(media.id) {
                    *self.main_button_file_path.borrow_mut() = Some(path.clone());
                    (format!("▶ Continuar {}", ep_label), Some(path))
                } else if let Ok(Some(path)) = repo.first_file_path_for_media(media.id) {
                    *self.main_button_file_path.borrow_mut() = Some(path.clone());
                    ("▶ Reproducir".to_string(), Some(path))
                } else {
                    *self.main_button_file_path.borrow_mut() = None;
                    ("▶ Reproducir".to_string(), None)
                }
            } else {
                *self.main_button_file_path.borrow_mut() = None;
                ("▶ Reproducir".to_string(), None)
            }
        } else if let Ok(mut conn) = open_db(&self.db_path) {
            let mut repo = MediaRepository::new(&mut conn);
            if let Ok(Some(path)) = repo.first_file_path_for_media(media.id) {
                *self.main_button_file_path.borrow_mut() = Some(path.clone());
                ("▶ Reproducir".to_string(), Some(path))
            } else {
                *self.main_button_file_path.borrow_mut() = None;
                ("▶ Reproducir".to_string(), None)
            }
        } else {
            *self.main_button_file_path.borrow_mut() = None;
            ("▶ Reproducir".to_string(), None)
        };

        self.main_button_label = label;
        self.play_button.set_label(&self.main_button_label);

        let fav_label = if media.is_favorite {
            "★ Favorito (activo)"
        } else {
            "★ Favorito"
        };
        self.favorite_button.set_label(fav_label);

        let wat_label = if media.is_watched {
            "✓ Visto (activo)"
        } else {
            "✓ Visto"
        };
        self.watched_button.set_label(wat_label);

        while let Some(child) = self.content_container.first_child() {
            self.content_container.remove(&child);
        }

        self.season_dropdown.set_visible(false);

        if media.parent_id.is_none() && media.r#type == "series" {
            self.load_seasons_and_episodes(media);
        } else if media.parent_id.is_some() {
            self.load_episodes(media.id);
        } else {
            let no_content_label = Label::new(Some("Este medio no tiene episodios disponibles."));
            no_content_label.set_halign(gtk4::Align::Start);
            no_content_label.set_valign(gtk4::Align::Start);
            no_content_label.set_margin_top(24);
            no_content_label.add_css_class("synopsis-text");
            self.content_container.append(&no_content_label);
        }
    }

    fn update_rating_button(&self, rating: i32) {
        let stars: String = (0..MAX_STARS)
            .map(|i| if i < rating { "★" } else { "☆" })
            .collect();
        self.rating_button
            .set_label(&format!("{} Calificar", stars));
    }

    /// Crea una tarjeta de episodio al estilo Crunchyroll: portada arriba,
    /// número y título debajo, botón de reproducción destacado.
    fn episode_card(
        &self,
        episode_label: &str,
        episode_number: &str,
        file_path: String,
        poster_url: Option<&str>,
    ) -> Box {
        let card = Box::new(Orientation::Vertical, 6);
        card.add_css_class("episode-card");
        card.set_hexpand(false);
        card.set_vexpand(false);
        card.set_size_request(EPISODE_POSTER_WIDTH + 20, EPISODE_POSTER_HEIGHT + 90);

        let effective_poster_url = poster_url.or(self.series_poster_url.as_deref());

        let poster_box = Box::new(Orientation::Vertical, 0);
        poster_box.set_size_request(EPISODE_POSTER_WIDTH, EPISODE_POSTER_HEIGHT);
        poster_box.set_halign(gtk4::Align::Center);
        poster_box.set_valign(gtk4::Align::Start);

        if let Some(pixbuf) = effective_poster_url.and_then(cached_path).and_then(|path| {
            GtkPixbuf::from_file_at_scale(&path, EPISODE_POSTER_WIDTH, EPISODE_POSTER_HEIGHT, true)
                .ok()
        }) {
            let picture = poster_picture(&pixbuf, EPISODE_POSTER_WIDTH, EPISODE_POSTER_HEIGHT);
            picture.add_css_class("episode-poster");
            poster_box.append(&picture);
        } else {
            let placeholder = placeholder_widget(EPISODE_POSTER_WIDTH, EPISODE_POSTER_HEIGHT);
            placeholder.add_css_class("episode-poster");
            poster_box.append(&placeholder);
        }

        card.append(&poster_box);

        let num_label = Label::new(Some(episode_number));
        num_label.set_halign(gtk4::Align::Center);
        num_label.set_valign(gtk4::Align::Start);
        num_label.set_justify(gtk4::Justification::Center);
        num_label.add_css_class("episode-number");
        card.append(&num_label);

        let title_label = Label::new(Some(episode_label));
        title_label.set_halign(gtk4::Align::Center);
        title_label.set_valign(gtk4::Align::Start);
        title_label.set_justify(gtk4::Justification::Center);
        title_label.set_wrap(true);
        title_label.set_max_width_chars(20);
        title_label.set_lines(2);
        title_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        title_label.add_css_class("episode-title");
        card.append(&title_label);

        let play_btn = Button::with_label("▶");
        play_btn.add_css_class("episode-play-btn");
        play_btn.set_halign(gtk4::Align::Center);
        play_btn.set_tooltip_text(Some(&format!("Reproducir {}", episode_label)));
        let file_path_clone = file_path.clone();
        let on_play_episode_btn = self.on_play_episode.clone();
        play_btn.connect_clicked(move |_| {
            if let Some(cb) = &on_play_episode_btn {
                cb(file_path_clone.clone());
            }
        });
        card.append(&play_btn);

        let click = GestureClick::new();
        let file_path_click = file_path.clone();
        let on_play_episode_click = self.on_play_episode.clone();
        click.connect_pressed(move |_, _, _, _| {
            if let Some(cb) = &on_play_episode_click {
                cb(file_path_click.clone());
            }
        });
        card.add_controller(click);

        card
    }

    /// Crea un rejilla (FlowBox) de tarjetas de episodio.
    fn episode_grid(&self, cards: Vec<Box>) -> FlowBox {
        let grid = FlowBox::new();
        grid.add_css_class("episode-grid");
        grid.set_hexpand(true);
        grid.set_valign(gtk4::Align::Start);
        grid.set_max_children_per_line(6);
        grid.set_min_children_per_line(1);
        grid.set_row_spacing(14);
        grid.set_column_spacing(14);
        grid.set_homogeneous(true);
        grid.set_selection_mode(gtk4::SelectionMode::None);
        grid.set_margin_top(6);
        grid.set_margin_bottom(6);
        grid.set_margin_start(6);
        grid.set_margin_end(6);

        for card in cards {
            grid.append(&card);
        }

        grid
    }

    /// Carga y muestra las temporadas y episodios directos de una serie.
    fn load_seasons_and_episodes(&mut self, _media: &Media) {
        let (seasons, direct_files) = {
            let mut conn = match open_db(&self.db_path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error abriendo BD: {}", e);
                    return;
                }
            };
            let mut repo = MediaRepository::new(&mut conn);
            let seasons = repo.list_children(self.media_id).unwrap_or_default();
            let direct_files = repo.list_files_by_media(self.media_id).unwrap_or_default();
            (seasons, direct_files)
        };

        if !seasons.is_empty() {
            self.season_dropdown.set_visible(true);
            let season_names: Vec<&str> = seasons.iter().map(|s| s.title.as_str()).collect();
            self.season_dropdown_model = gtk4::StringList::new(&season_names);
            self.season_dropdown
                .set_model(Some(&self.season_dropdown_model));
            self.season_dropdown.set_selected(0);

            let season_dropdown = self.season_dropdown.clone();
            let on_change_season = self.on_change_season.clone();
            let seasons_clone = seasons.clone();
            season_dropdown.connect_selected_notify(move |dd| {
                let idx = dd.selected() as usize;
                if let (Some(cb), Some(season)) = (&on_change_season, seasons_clone.get(idx)) {
                    cb(season.id);
                }
            });
        }

        if seasons.is_empty() && direct_files.is_empty() {
            let no_content_label = Label::new(Some("Sin temporadas ni episodios disponibles"));
            no_content_label.set_halign(gtk4::Align::Start);
            no_content_label.set_valign(gtk4::Align::Start);
            no_content_label.set_margin_top(24);
            no_content_label.add_css_class("synopsis-text");
            self.content_container.append(&no_content_label);
            return;
        }

        if !seasons.is_empty() {
            for season in seasons.iter() {
                let season_section = Box::new(Orientation::Vertical, 0);
                season_section.add_css_class("season-grid-section");
                season_section.set_hexpand(true);
                season_section.set_valign(gtk4::Align::Start);

                let season_title = Label::new(Some(&season.title));
                season_title.set_halign(gtk4::Align::Start);
                season_title.set_hexpand(true);
                season_title.set_xalign(0.0);
                season_title.add_css_class("season-grid-title");
                season_section.append(&season_title);

                let season_files = {
                    let mut conn = match open_db(&self.db_path) {
                        Ok(c) => c,
                        Err(e) => {
                            eprintln!("Error abriendo BD: {}", e);
                            continue;
                        }
                    };
                    let mut repo = MediaRepository::new(&mut conn);
                    repo.list_files_by_media(season.id).unwrap_or_default()
                };

                let season_poster_url = season.poster_path.as_deref();

                let mut cards = Vec::new();
                for f in season_files.iter() {
                    let episode_label = match (f.season, f.episode) {
                        (Some(s), Some(e)) => {
                            format!("S{}E{} - {}", s, e, f.title.clone().unwrap_or_default())
                        }
                        (None, Some(e)) => {
                            format!("Episodio {} - {}", e, f.title.clone().unwrap_or_default())
                        }
                        (Some(s), None) => {
                            format!("Temporada {} - {}", s, f.title.clone().unwrap_or_default())
                        }
                        (None, None) => f.title.clone().unwrap_or_default(),
                    };
                    let episode_number = match (f.season, f.episode) {
                        (Some(s), Some(e)) => format!("S{:02}E{:02}", s, e),
                        (None, Some(e)) => format!("E{:02}", e),
                        (Some(s), None) => format!("T{:02}", s),
                        (None, None) => String::new(),
                    };

                    let card = self.episode_card(
                        &episode_label,
                        &episode_number,
                        f.file_path.clone(),
                        season_poster_url,
                    );
                    cards.push(card);
                }

                let grid = self.episode_grid(cards);
                season_section.append(&grid);

                self.content_container.append(&season_section);
            }
        }

        if !direct_files.is_empty() {
            let episodes_title = Label::new(Some("Episodios"));
            episodes_title.set_halign(gtk4::Align::Start);
            episodes_title.set_hexpand(true);
            episodes_title.set_xalign(0.0);
            episodes_title.add_css_class("season-grid-title");
            self.content_container.append(&episodes_title);

            let mut cards = Vec::new();
            for f in direct_files.iter() {
                let episode_label = match (f.season, f.episode) {
                    (Some(s), Some(e)) => {
                        format!("S{}E{} - {}", s, e, f.title.clone().unwrap_or_default())
                    }
                    (None, Some(e)) => {
                        format!("Episodio {} - {}", e, f.title.clone().unwrap_or_default())
                    }
                    (Some(s), None) => {
                        format!("Temporada {} - {}", s, f.title.clone().unwrap_or_default())
                    }
                    (None, None) => f.title.clone().unwrap_or_default(),
                };
                let episode_number = match (f.season, f.episode) {
                    (Some(s), Some(e)) => format!("S{:02}E{:02}", s, e),
                    (None, Some(e)) => format!("E{:02}", e),
                    (Some(s), None) => format!("T{:02}", s),
                    (None, None) => String::new(),
                };

                let card =
                    self.episode_card(&episode_label, &episode_number, f.file_path.clone(), None);
                cards.push(card);
            }

            let grid = self.episode_grid(cards);
            self.content_container.append(&grid);
        }
    }

    /// Carga y muestra los episodios (archivos) de una temporada en rejilla.
    fn load_episodes(&self, media_id: i64) {
        let files = {
            let mut conn = match open_db(&self.db_path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error abriendo BD: {}", e);
                    return;
                }
            };
            let mut repo = MediaRepository::new(&mut conn);
            repo.list_files_by_media(media_id).unwrap_or_default()
        };

        if files.is_empty() {
            let no_files_label = Label::new(Some("Sin archivos disponibles"));
            no_files_label.set_halign(gtk4::Align::Start);
            no_files_label.set_valign(gtk4::Align::Start);
            no_files_label.set_margin_top(24);
            no_files_label.add_css_class("synopsis-text");
            self.content_container.append(&no_files_label);
            return;
        }

        let mut cards = Vec::new();
        for f in files.iter() {
            let episode_label = match (f.season, f.episode) {
                (Some(s), Some(e)) => {
                    format!("S{}E{} - {}", s, e, f.title.clone().unwrap_or_default())
                }
                (None, Some(e)) => {
                    format!("Episodio {} - {}", e, f.title.clone().unwrap_or_default())
                }
                (Some(s), None) => {
                    format!("Temporada {} - {}", s, f.title.clone().unwrap_or_default())
                }
                (None, None) => f.title.clone().unwrap_or_default(),
            };
            let episode_number = match (f.season, f.episode) {
                (Some(s), Some(e)) => format!("S{:02}E{:02}", s, e),
                (None, Some(e)) => format!("E{:02}", e),
                (Some(s), None) => format!("T{:02}", s),
                (None, None) => String::new(),
            };

            let card =
                self.episode_card(&episode_label, &episode_number, f.file_path.clone(), None);
            cards.push(card);
        }

        let grid = self.episode_grid(cards);
        self.content_container.append(&grid);
    }

    pub fn media_id(&self) -> i64 {
        self.media_id
    }

    pub fn parent_id(&self) -> Option<i64> {
        self.parent_id
    }

    pub fn widget(&self) -> &Box {
        &self.box_
    }

    pub fn on_back<F>(&mut self, f: F)
    where
        F: Fn() + 'static,
    {
        let cb = Rc::new(f);
        let cb_clone = cb.clone();
        self.back_button.connect_clicked(move |_| {
            cb_clone();
        });
        self.on_back = Some(cb);
    }

    pub fn on_favorite<F>(&mut self, f: F)
    where
        F: Fn() + 'static,
    {
        let cb = Rc::new(f);
        let cb_clone = cb.clone();
        self.favorite_button.connect_clicked(move |_| {
            cb_clone();
        });
        self.on_favorite = Some(cb);
    }

    pub fn on_watched<F>(&mut self, f: F)
    where
        F: Fn() + 'static,
    {
        let cb = Rc::new(f);
        let cb_clone = cb.clone();
        self.watched_button.connect_clicked(move |_| {
            cb_clone();
        });
        self.on_watched = Some(cb);
    }

    pub fn on_rate<F>(&mut self, f: F)
    where
        F: Fn(i32) + 'static,
    {
        let cb = Rc::new(f);
        let cb_clone = cb.clone();
        let rating_button_clone = self.rating_button.clone();
        self.rating_button.connect_clicked(move |_| {
            cb_clone(Self::stars_from_label(
                &rating_button_clone.label().unwrap_or_default(),
            ));
        });
        self.on_rate = Some(cb);
    }

    pub fn on_play_episode<F>(&mut self, f: F)
    where
        F: Fn(String) + 'static,
    {
        let cb = Rc::new(f);
        let cb_clone = cb.clone();
        let file_path_rc = self.main_button_file_path.clone();
        self.play_button.connect_clicked(move |_| {
            if let Some(path) = file_path_rc.borrow().as_ref() {
                cb_clone(path.clone());
            }
        });
        self.on_play_episode = Some(cb);
    }

    pub fn on_edit_tracks<F>(&mut self, f: F)
    where
        F: Fn(String) + 'static,
    {
        let cb = Rc::new(f);
        let cb_clone = cb.clone();
        let file_path_rc = self.main_button_file_path.clone();
        self.tracks_button.connect_clicked(move |_| {
            if let Some(path) = file_path_rc.borrow().as_ref() {
                cb_clone(path.clone());
            }
        });
        self.on_edit_tracks = Some(cb);
    }

    pub fn on_change_season<F>(&mut self, f: F)
    where
        F: Fn(i64) + 'static,
    {
        self.on_change_season = Some(Rc::new(f));
    }

    fn stars_from_label(label: &str) -> i32 {
        label.chars().take_while(|c| *c == '★').count() as i32
    }
}

/// Crea el marcador visual cuando no hay imagen disponible.
fn placeholder_widget(width: i32, height: i32) -> Box {
    let placeholder = Box::new(Orientation::Vertical, 0);
    placeholder.set_size_request(width, height);
    placeholder.set_halign(gtk4::Align::Center);
    placeholder.set_valign(gtk4::Align::Center);
    placeholder.add_css_class("poster-placeholder");

    let icon = gtk4::Image::from_icon_name("video-x-generic-symbolic");
    icon.set_pixel_size(32);
    icon.set_halign(gtk4::Align::Center);
    icon.set_valign(gtk4::Align::Center);
    placeholder.append(&icon);

    placeholder
}
