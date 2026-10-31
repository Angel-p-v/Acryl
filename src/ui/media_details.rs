//! Vista de detalles de un medio (con temporadas y episodios para series).
//!
//! Diseño inspirado en Crunchyroll: banner ancho arriba, información y
//! sinopsis debajo (sin póster redundante en series), y episodios organizados
//! en una rejilla con portadas al estilo Crunchyroll.

use crate::core::poster_service::cached_path;
use crate::db::media_repo::Media;
use crate::db::{MediaRepository, close_db, open_db};
use crate::ui::poster::{Banner, poster_picture};
use gtk4::gdk_pixbuf::Pixbuf as GtkPixbuf;
use gtk4::prelude::*;
use gtk4::{Box, Button, FlowBox, Label, Orientation};
use std::rc::Rc;

/// Tamaño del póster en la cabecera de la vista de detalles (solo películas).
const DETAIL_POSTER_WIDTH: i32 = 220;
const DETAIL_POSTER_HEIGHT: i32 = 330;

/// Tamaño de la portada de un episodio en la rejilla (relación 16:9).
const EPISODE_POSTER_WIDTH: i32 = 160;
const EPISODE_POSTER_HEIGHT: i32 = 90;

/// Destino del botón «Volver» desde la vista de detalles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackTarget {
    /// Una temporada vuelve a la vista de su serie o anime.
    Parent(i64),
    /// Una serie, película o anime vuelve a la lista principal.
    List,
}

/// Calcula el destino de «Volver»: de una temporada se vuelve a su medio
/// padre; del resto de medios, a la lista principal. Los identificadores no
/// positivos se consideran inválidos y vuelven a la lista.
pub fn back_target(parent_id: Option<i64>) -> BackTarget {
    match parent_id.filter(|parent_id| *parent_id > 0) {
        Some(parent_id) => BackTarget::Parent(parent_id),
        None => BackTarget::List,
    }
}

/// Componente de vista de detalles de un medio.
pub struct MediaDetails {
    box_: Box,
    db_path: String,
    media_id: i64,
    parent_id: Option<i64>,
    banner: Banner,
    /// Póster grande a la izquierda (solo para películas, no para series).
    poster_box: Box,
    /// URL del póster de la serie padre (fallback para portadas de episodios).
    series_poster_url: Option<String>,
    synopsis_label: Label,
    favorite_button: Button,
    watched_button: Button,
    play_button: Button,
    tracks_button: Button,
    back_button: Button,
    content_container: Box,
    on_back: Option<Rc<dyn Fn()>>,
    on_favorite: Option<Rc<dyn Fn()>>,
    on_watched: Option<Rc<dyn Fn()>>,
    on_play: Option<Rc<dyn Fn()>>,
    on_play_episode: Option<Rc<dyn Fn(String)>>,
    on_edit_tracks: Option<Rc<dyn Fn(String)>>,
    on_edit_all_tracks: Option<Rc<dyn Fn()>>,
    on_open_season: Option<Rc<dyn Fn(i64)>>,
}

impl MediaDetails {
    /// Crea una nueva vista de detalles.
    pub fn new(db_path: &str, media_id: i64) -> Self {
        let box_ = Box::new(Orientation::Vertical, 0);
        box_.set_margin_top(0);
        box_.set_margin_bottom(0);
        box_.set_margin_start(0);
        box_.set_margin_end(0);
        box_.set_valign(gtk4::Align::Start);
        box_.set_hexpand(true);

        let banner = Banner::new();

        // Póster grande a la izquierda (solo se muestra para películas;
        // en series se omite para dejar solo el banner).
        let poster_box = Box::new(Orientation::Vertical, 0);
        poster_box.set_valign(gtk4::Align::Start);
        poster_box.set_halign(gtk4::Align::Start);
        poster_box.set_visible(false);

        // Información y sinopsis debajo del banner.
        let info_box = Box::new(Orientation::Vertical, 12);
        info_box.set_valign(gtk4::Align::Start);
        info_box.set_hexpand(true);

        let synopsis_label = Label::new(None);
        synopsis_label.set_halign(gtk4::Align::Start);
        synopsis_label.set_valign(gtk4::Align::Start);
        synopsis_label.set_xalign(0.0);
        synopsis_label.set_hexpand(true);
        synopsis_label.set_wrap(true);
        synopsis_label.add_css_class("synopsis-text");

        // Cabecera: banner arriba, información y sinopsis debajo.
        // En series no se muestra póster redundante; solo el banner ocupa todo el ancho.
        let header_box = Box::new(Orientation::Horizontal, 0);
        header_box.set_hexpand(true);
        header_box.set_valign(gtk4::Align::Start);
        header_box.append(&info_box);

        let details_header = Box::new(Orientation::Vertical, 0);
        details_header.add_css_class("details-header");
        details_header.set_hexpand(true);
        details_header.set_valign(gtk4::Align::Start);
        details_header.append(banner.widget());
        details_header.append(&header_box);

        let favorite_button = Button::with_label("★ Favorito");
        favorite_button.add_css_class("flat-button");
        favorite_button.set_halign(gtk4::Align::Start);

        let watched_button = Button::with_label("✓ Visto");
        watched_button.add_css_class("flat-button");
        watched_button.set_halign(gtk4::Align::Start);

        let play_button = Button::with_label("▶ Reproducir");
        play_button.add_css_class("play-button");
        play_button.set_halign(gtk4::Align::Start);

        // Botón universal: elegir pistas de audio y subtítulos antes de reproducir.
        let tracks_button = Button::with_label("🎵 Pistas");
        tracks_button.add_css_class("flat-button");
        tracks_button.set_halign(gtk4::Align::Start);
        tracks_button.set_tooltip_text(Some(
            "Elegir pista de audio y subtítulos antes de reproducir",
        ));

        let back_button = Button::with_label("← Volver");
        back_button.add_css_class("flat-button");
        back_button.set_halign(gtk4::Align::Start);

        // Contenedor de contenido dinámico (temporadas o episodios en rejilla)
        let content_container = Box::new(Orientation::Vertical, 0);
        content_container.set_hexpand(true);
        content_container.set_valign(gtk4::Align::Start);

        box_.append(&back_button);
        box_.append(&details_header);
        box_.append(&content_container);
        box_.append(&favorite_button);
        box_.append(&watched_button);
        box_.append(&tracks_button);
        box_.append(&play_button);

        Self {
            box_,
            db_path: db_path.to_string(),
            media_id,
            parent_id: None,
            banner,
            poster_box,
            series_poster_url: None,
            synopsis_label,
            favorite_button,
            watched_button,
            play_button,
            tracks_button,
            back_button,
            content_container,
            on_back: None,
            on_favorite: None,
            on_watched: None,
            on_play: None,
            on_play_episode: None,
            on_edit_tracks: None,
            on_edit_all_tracks: None,
            on_open_season: None,
        }
    }

    /// Establece la ruta de la base de datos.
    pub fn set_db_path(&mut self, db_path: &str) {
        self.db_path = db_path.to_string();
    }

    /// Carga los datos de un medio desde la BD y actualiza la vista.
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

    /// Carga los datos de un medio en la vista.
    /// Si el medio es una serie padre (sin parent_id), muestra sus temporadas
    /// y episodios en rejilla, sin póster redundante (solo el banner).
    /// Si es una temporada (con parent_id), muestra sus episodios en rejilla.
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

        // En series no se muestra el póster redundante: solo el banner.
        // En películas sí se muestra el póster a la izquierda.
        self.poster_box.set_visible(false);
        if media.parent_id.is_some() || media.r#type != "series" {
            // Películas o temporadas: mostrar póster si existe en caché.
            self.update_poster(media.poster_path.as_deref());
        }

        // Guardar el póster de la serie padre como fallback para portadas de episodios.
        // Si es una serie padre, usamos su propio póster. Si es una temporada,
        // consultamos el póster de la serie padre.
        if media.parent_id.is_none() {
            self.series_poster_url = media.poster_path.clone();
        } else {
            // Temporada: obtener el póster de la serie padre desde la BD.
            self.series_poster_url = if let Ok(mut conn) = open_db(&self.db_path) {
                let mut repo = MediaRepository::new(&mut conn);
                repo.get_poster_by_id(media.parent_id.unwrap())
                    .ok()
                    .flatten()
            } else {
                None
            };
        }

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

        // Limpiar contenido previo
        while let Some(child) = self.content_container.first_child() {
            self.content_container.remove(&child);
        }

        // Si es una serie padre (no tiene parent_id), mostrar temporadas y/o episodios directos
        if media.parent_id.is_none() {
            self.load_seasons_and_episodes();
        } else {
            // Es una temporada, mostrar archivos directamente en rejilla
            self.load_episodes(media.id);
        }
    }

    /// Muestra el póster del medio en la cabecera solo si ya está en caché.
    /// Si falta, no se usa un placeholder.
    fn update_poster(&self, poster_url: Option<&str>) {
        while let Some(child) = self.poster_box.first_child() {
            self.poster_box.remove(&child);
        }
        self.poster_box.set_visible(false);

        let Some(pixbuf) = poster_url.and_then(cached_path).and_then(|path| {
            GtkPixbuf::from_file_at_scale(&path, DETAIL_POSTER_WIDTH, DETAIL_POSTER_HEIGHT, true)
                .ok()
        }) else {
            return;
        };

        let picture = poster_picture(&pixbuf, DETAIL_POSTER_WIDTH, DETAIL_POSTER_HEIGHT);
        self.poster_box.append(&picture);
        self.poster_box.set_visible(true);
    }

    /// Crea una tarjeta de episodio al estilo Crunchyroll: portada arriba,
    /// número y título debajo, botón de reproducir destacado.
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
        card.set_size_request(EPISODE_POSTER_WIDTH + 20, EPISODE_POSTER_HEIGHT + 80);

        // Portada del episodio (usar póster de temporada si existe,
        // si no usar el póster de la serie padre, y si no un marcador).
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

        // Número de episodio (ej. S01E01).
        let num_label = Label::new(Some(episode_number));
        num_label.set_halign(gtk4::Align::Center);
        num_label.set_valign(gtk4::Align::Start);
        num_label.set_justify(gtk4::Justification::Center);
        num_label.add_css_class("episode-number");
        card.append(&num_label);

        // Título del episodio.
        let title_label = Label::new(Some(episode_label));
        title_label.set_halign(gtk4::Align::Center);
        title_label.set_valign(gtk4::Align::Start);
        title_label.set_justify(gtk4::Justification::Center);
        title_label.set_wrap(true);
        title_label.set_max_width_chars(18);
        title_label.set_lines(2);
        title_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        title_label.add_css_class("episode-title");
        card.append(&title_label);

        // Botón de reproducir destacado al pie de la tarjeta.
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

        // El resto de la tarjeta es clickeable para reproducir.
        let click = gtk4::GestureClick::new();
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
        grid.set_row_spacing(12);
        grid.set_column_spacing(12);
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
    /// Si no hay temporadas ni archivos directos, muestra mensaje vacío.
    fn load_seasons_and_episodes(&self) {
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

        // Si no hay temporadas ni archivos directos, mostrar mensaje vacío
        if seasons.is_empty() && direct_files.is_empty() {
            let no_content_label = Label::new(Some("Sin temporadas ni episodios disponibles"));
            no_content_label.set_halign(gtk4::Align::Start);
            no_content_label.set_valign(gtk4::Align::Start);
            no_content_label.set_margin_top(24);
            no_content_label.add_css_class("synopsis-text");
            self.content_container.append(&no_content_label);
            return;
        }

        // Mostrar temporadas si las hay (como secciones con rejilla de episodios)
        if !seasons.is_empty() {
            for season in seasons.iter() {
                // Sección de temporada
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

                // Obtener archivos de esta temporada (abrir BD de nuevo)
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

                // Póster de temporada para las portadas de episodio
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

        // Mostrar episodios directos de la serie (videos sin temporada)
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

    /// Devuelve el ID del medio cargado.
    pub fn media_id(&self) -> i64 {
        self.media_id
    }

    /// Devuelve el medio padre cuando se está viendo una temporada.
    pub fn parent_id(&self) -> Option<i64> {
        self.parent_id
    }

    /// Devuelve el widget Gtk.
    pub fn widget(&self) -> &Box {
        &self.box_
    }

    /// Establece el callback de volver atrás.
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

    /// Establece el callback de favorito.
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

    /// Establece el callback de visto.
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

    /// Establece el callback de reproducir.
    pub fn on_play<F>(&mut self, f: F)
    where
        F: Fn() + 'static,
    {
        let cb = Rc::new(f);
        let cb_clone = cb.clone();
        self.play_button.connect_clicked(move |_| {
            cb_clone();
        });
        self.on_play = Some(cb);
    }

    /// Establece el callback para abrir una temporada (navegar a sus detalles).
    pub fn on_open_season<F>(&mut self, f: F)
    where
        F: Fn(i64) + 'static,
    {
        self.on_open_season = Some(Rc::new(f));
    }

    /// Establece el callback de reproducir un episodio (recibe la ruta del archivo).
    pub fn on_play_episode<F>(&mut self, f: F)
    where
        F: Fn(String) + 'static,
    {
        self.on_play_episode = Some(Rc::new(f));
    }

    /// Establece el callback para editar las pistas de audio y subtítulos de
    /// un archivo (recibe la ruta del archivo).
    pub fn on_edit_tracks<F>(&mut self, f: F)
    where
        F: Fn(String) + 'static,
    {
        self.on_edit_tracks = Some(Rc::new(f));
    }

    /// Establece el callback para editar las pistas de audio y subtítulos de
    /// todos los episodios del medio actual (sin argumentos).
    pub fn on_edit_all_tracks<F>(&mut self, f: F)
    where
        F: Fn() + 'static,
    {
        let cb = Rc::new(f);
        let cb_clone = cb.clone();
        self.tracks_button.connect_clicked(move |_| {
            cb_clone();
        });
        self.on_edit_all_tracks = Some(cb);
    }
}

/// Crea el marcador visual que se muestra cuando no hay imagen disponible.
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
