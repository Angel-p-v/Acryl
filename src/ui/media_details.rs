//! Vista de detalles de un medio (con temporadas y episodios para series).

use crate::core::poster_service::cached_path;
use crate::db::media_repo::Media;
use crate::db::{MediaRepository, close_db, open_db};
use crate::ui::poster::{Banner, poster_picture};
use gtk4::gdk_pixbuf::Pixbuf as GtkPixbuf;
use gtk4::prelude::*;
use gtk4::{Box, Button, Label, Orientation};
use std::rc::Rc;

/// Tamaño del póster en la cabecera de la vista de detalles (proporción 2:3).
const DETAIL_POSTER_WIDTH: i32 = 160;
const DETAIL_POSTER_HEIGHT: i32 = 240;

/// Tamaño del póster de una temporada en la lista de temporadas.
const SEASON_POSTER_WIDTH: i32 = 56;
const SEASON_POSTER_HEIGHT: i32 = 84;

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
    poster_box: Box,
    synopsis_label: Label,
    favorite_button: Button,
    watched_button: Button,
    play_button: Button,
    back_button: Button,
    content_container: Box,
    on_back: Option<Rc<dyn Fn()>>,
    on_favorite: Option<Rc<dyn Fn()>>,
    on_watched: Option<Rc<dyn Fn()>>,
    on_play: Option<Rc<dyn Fn()>>,
    on_play_episode: Option<Rc<dyn Fn(String)>>,
    on_edit_tracks: Option<Rc<dyn Fn(String)>>,
    on_open_season: Option<Rc<dyn Fn(i64)>>,
}

impl MediaDetails {
    /// Crea una nueva vista de detalles.
    pub fn new(db_path: &str, media_id: i64) -> Self {
        let box_ = Box::new(Orientation::Vertical, 10);
        box_.set_margin_top(10);
        box_.set_margin_bottom(10);
        box_.set_margin_start(10);
        box_.set_margin_end(10);
        // La vista vive dentro de un ScrolledWindow. Sin este anclaje, el
        // GtkBox se estira a todo el alto disponible y el espacio sobrante
        // aparece como un hueco enorme entre el banner y la sinopsis.
        box_.set_valign(gtk4::Align::Start);

        let banner = Banner::new();

        let poster_box = Box::new(Orientation::Vertical, 0);
        poster_box.set_valign(gtk4::Align::Start);
        poster_box.set_halign(gtk4::Align::Start);
        poster_box.set_visible(false);

        let synopsis_label = Label::new(None);
        synopsis_label.set_halign(gtk4::Align::Start);
        synopsis_label.set_valign(gtk4::Align::Start);
        synopsis_label.set_xalign(0.0);
        synopsis_label.set_hexpand(true);
        synopsis_label.set_wrap(true);

        // Cabecera: póster a la izquierda (solo si está en caché) y sinopsis
        // a la derecha. Así el póster del medio sigue visible aunque se
        // muestren temporadas.
        let header_box = Box::new(Orientation::Horizontal, 16);
        header_box.set_hexpand(true);
        header_box.set_valign(gtk4::Align::Start);
        header_box.append(&poster_box);
        header_box.append(&synopsis_label);

        let favorite_button = Button::with_label("★ Favorito");
        favorite_button.set_halign(gtk4::Align::Start);

        let watched_button = Button::with_label("✓ Visto");
        watched_button.set_halign(gtk4::Align::Start);

        let play_button = Button::with_label("▶ Reproducir");
        play_button.set_halign(gtk4::Align::Start);
        play_button.add_css_class("suggested-action");

        let back_button = Button::with_label("← Volver");
        back_button.set_halign(gtk4::Align::Start);

        // Contenedor de contenido dinámico (temporadas o episodios)
        let content_container = Box::new(Orientation::Vertical, 5);
        content_container.set_hexpand(true);

        box_.append(&back_button);
        box_.append(banner.widget());
        box_.append(&header_box);
        box_.append(&content_container);
        box_.append(&favorite_button);
        box_.append(&watched_button);
        box_.append(&play_button);

        Self {
            box_,
            db_path: db_path.to_string(),
            media_id,
            parent_id: None,
            banner,
            poster_box,
            synopsis_label,
            favorite_button,
            watched_button,
            play_button,
            back_button,
            content_container,
            on_back: None,
            on_favorite: None,
            on_watched: None,
            on_play: None,
            on_play_episode: None,
            on_edit_tracks: None,
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
    /// Si el medio es una serie padre (sin parent_id), muestra sus temporadas.
    /// Si es una temporada (con parent_id), muestra sus episodios.
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
        self.update_poster(media.poster_path.as_deref());

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
            // Es una temporada, mostrar archivos directamente
            self.load_episodes(media.id);
        }
    }

    /// Muestra el póster del medio en la cabecera solo si ya está en caché.
    /// Si falta, no se usa un placeholder: la sinopsis ocupa su lugar.
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

    /// Carga y muestra las temporadas y episodios directos de una serie.
    /// Si no hay temporadas pero hay videos directos, muestra esos videos.
    fn load_seasons_and_episodes(&self) {
        let mut conn = match open_db(&self.db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error abriendo BD: {}", e);
                return;
            }
        };
        let mut repo = MediaRepository::new(&mut conn);

        // Obtener temporadas (hijos con parent_id = self.media_id)
        let seasons = repo.list_children(self.media_id).unwrap_or_default();

        // Obtener archivos directos de esta serie (media_id = self.media_id, season = None)
        let direct_files = repo.list_files_by_media(self.media_id).unwrap_or_default();

        close_db(conn);

        // Si no hay temporadas ni archivos directos, mostrar mensaje vacío
        if seasons.is_empty() && direct_files.is_empty() {
            let no_content_label = Label::new(Some("Sin temporadas ni episodios disponibles"));
            no_content_label.set_halign(gtk4::Align::Start);
            no_content_label.set_margin_top(10);
            self.content_container.append(&no_content_label);
            return;
        }

        // Mostrar temporadas si las hay
        if !seasons.is_empty() {
            let seasons_title = Label::new(Some("Temporadas"));
            seasons_title.set_halign(gtk4::Align::Start);
            seasons_title.set_markup("<b>Temporadas</b>");
            seasons_title.set_margin_top(10);
            self.content_container.append(&seasons_title);

            for season in seasons.iter() {
                let row = Box::new(Orientation::Horizontal, 10);
                row.set_hexpand(true);

                // Póster de temporada: solo si existe y ya está en caché.
                // Si falta, no se muestra un placeholder engañoso.
                if let Some(pixbuf) = season
                    .poster_path
                    .as_deref()
                    .and_then(cached_path)
                    .and_then(|path| {
                        GtkPixbuf::from_file_at_scale(
                            &path,
                            SEASON_POSTER_WIDTH,
                            SEASON_POSTER_HEIGHT,
                            true,
                        )
                        .ok()
                    })
                {
                    row.append(&poster_picture(
                        &pixbuf,
                        SEASON_POSTER_WIDTH,
                        SEASON_POSTER_HEIGHT,
                    ));
                }

                let season_label = Label::new(Some(&season.title));
                season_label.set_halign(gtk4::Align::Start);
                season_label.set_hexpand(true);
                season_label.set_markup(&format!("<b>{}</b>", escape_markup(&season.title)));
                row.append(&season_label);

                let open_btn = Button::with_label("Abrir");
                let season_id = season.id;
                let on_open_season = self.on_open_season.clone();
                open_btn.connect_clicked(move |_| {
                    if let Some(cb) = &on_open_season {
                        cb(season_id);
                    }
                });
                row.append(&open_btn);

                self.content_container.append(&row);
            }
        }

        // Mostrar episodios directos de la serie (videos sin temporada)
        if !direct_files.is_empty() {
            let episodes_title = Label::new(Some("Episodios"));
            episodes_title.set_halign(gtk4::Align::Start);
            episodes_title.set_markup("<b>Episodios</b>");
            episodes_title.set_margin_top(10);
            self.content_container.append(&episodes_title);

            for f in direct_files.iter() {
                let row = Box::new(Orientation::Horizontal, 10);
                row.set_hexpand(true);

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

                let ep_label = Label::new(Some(&episode_label));
                ep_label.set_halign(gtk4::Align::Start);
                ep_label.set_hexpand(true);
                row.append(&ep_label);

                let tracks_btn = Button::with_label("Pistas");
                tracks_btn.add_css_class("flat");
                tracks_btn.set_tooltip_text(Some("Elegir pista de audio y subtítulos"));
                let tracks_file_path = f.file_path.clone();
                let on_edit_tracks = self.on_edit_tracks.clone();
                tracks_btn.connect_clicked(move |_| {
                    if let Some(cb) = &on_edit_tracks {
                        cb(tracks_file_path.clone());
                    }
                });
                row.append(&tracks_btn);

                let play_btn = Button::with_label("▶");
                play_btn.add_css_class("flat");
                play_btn.set_tooltip_text(Some(&format!("Reproducir {}", episode_label)));
                let file_path = f.file_path.clone();
                let on_play_episode = self.on_play_episode.clone();
                play_btn.connect_clicked(move |_| {
                    if let Some(cb) = &on_play_episode {
                        cb(file_path.clone());
                    }
                });
                row.append(&play_btn);

                self.content_container.append(&row);
            }
        }
    }

    /// Carga y muestra los episodios (archivos) de una temporada.
    fn load_episodes(&self, media_id: i64) {
        let mut conn = match open_db(&self.db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error abriendo BD: {}", e);
                return;
            }
        };
        let mut repo = MediaRepository::new(&mut conn);
        let files = repo.list_files_by_media(media_id).unwrap_or_default();
        close_db(conn);

        if files.is_empty() {
            let no_files_label = Label::new(Some("Sin archivos disponibles"));
            no_files_label.set_halign(gtk4::Align::Start);
            no_files_label.set_margin_top(10);
            self.content_container.append(&no_files_label);
            return;
        }

        for f in files.iter() {
            let row = Box::new(Orientation::Horizontal, 10);
            row.set_hexpand(true);

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

            let ep_label = Label::new(Some(&episode_label));
            ep_label.set_halign(gtk4::Align::Start);
            ep_label.set_hexpand(true);
            row.append(&ep_label);

            let tracks_btn = Button::with_label("Pistas");
            tracks_btn.add_css_class("flat");
            tracks_btn.set_tooltip_text(Some("Elegir pista de audio y subtítulos"));
            let tracks_file_path = f.file_path.clone();
            let on_edit_tracks = self.on_edit_tracks.clone();
            tracks_btn.connect_clicked(move |_| {
                if let Some(cb) = &on_edit_tracks {
                    cb(tracks_file_path.clone());
                }
            });
            row.append(&tracks_btn);

            let play_btn = Button::with_label("▶");
            play_btn.add_css_class("flat");
            play_btn.set_tooltip_text(Some(&format!("Reproducir {}", episode_label)));
            let file_path = f.file_path.clone();
            let on_play_episode = self.on_play_episode.clone();
            play_btn.connect_clicked(move |_| {
                if let Some(cb) = &on_play_episode {
                    cb(file_path.clone());
                }
            });
            row.append(&play_btn);

            self.content_container.append(&row);
        }
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
}

fn escape_markup(text: &str) -> String {
    gtk4::glib::markup_escape_text(text).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volver_desde_una_temporada_va_al_medio_padre() {
        assert_eq!(back_target(Some(7)), BackTarget::Parent(7));
    }

    #[test]
    fn volver_desde_un_medio_raiz_va_a_la_lista() {
        assert_eq!(back_target(None), BackTarget::List);
    }

    #[test]
    fn volver_ignora_padres_invalidos() {
        assert_eq!(back_target(Some(0)), BackTarget::List);
        assert_eq!(back_target(Some(-3)), BackTarget::List);
    }
}
