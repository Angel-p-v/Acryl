//! Vista de detalles de un medio al estilo Crunchyroll.
//!
//! Distribución:
//! - Banner hero a todo el ancho como imagen principal (sin póster duplicado).
//! - Puntuación de metadatos (vote_average de TVmaze) mostrada directamente.
//! - Fila de acciones: Volver, Pistas, Continuar/Reproducir.
//! - Desplegable de temporada con una temporada seleccionada por defecto.
//! - Sinopsis.
//! - Rejilla de episodios de la temporada seleccionada, cada uno con su propia
//!   imagen individual y un fallback visual cuando no existe.
//!
//! El desplegable se conecta una sola vez. La selección programática se
//! realiza con una bandera de supresión para no acumular handlers ni disparar
//! recargas duplicadas al cambiar de medio.

use crate::core::poster_service::cached_path;
use crate::db::file_repo::MediaFile;
use crate::db::media_repo::Media;
use crate::db::{MediaRepository, close_db, open_db};
use crate::ui::i18n::TranslationsRef;
use crate::ui::poster::{Banner, placeholder_widget, poster_picture};
use gtk4::gdk_pixbuf::Pixbuf as GtkPixbuf;
use gtk4::prelude::*;
use gtk4::{Box, Button, DropDown, FlowBox, GestureClick, Label, Orientation};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Tamaño de la portada de un episodio en la rejilla (relación 16:9).
const EPISODE_POSTER_WIDTH: i32 = 240;
const EPISODE_POSTER_HEIGHT: i32 = 135;

/// Identifica la temporada seleccionada dentro de un medio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeasonSelection {
    /// Temporada hija de la serie (`media.parent_id = root_id`).
    Child(i64),
    /// Número de temporada de los archivos directos del medio raíz.
    Number(i32),
}

struct SeasonOption {
    key: SeasonSelection,
    label: String,
}

/// Callback de reproducción compartido con las tarjetas de episodio.
type PlayEpisodeCallback = Rc<dyn Fn(String)>;
type SharedPlayEpisodeCallback = Rc<RefCell<Option<PlayEpisodeCallback>>>;

/// Componente de vista de detalles.
pub struct MediaDetails {
    box_: Box,
    db_path: Rc<RefCell<String>>,
    media_id: i64,
    parent_id: Option<i64>,
    /// Medio raíz de la vista; cambia con `set_media` y lo usa el handler del
    /// desplegable para consultar los archivos correctos.
    root_media_id: Rc<Cell<i64>>,
    banner: Banner,
    main_button_file_path: Rc<RefCell<Option<String>>>,
    main_button_label: String,
    synopsis_label: Label,
    tracks_button: Button,
    play_button: Button,
    back_button: Button,
    vote_label: Label,
    season_dropdown: DropDown,
    season_dropdown_model: gtk4::StringList,
    content_container: Box,
    seasons: Rc<RefCell<Vec<SeasonOption>>>,
    selected_season: Rc<RefCell<Option<SeasonSelection>>>,
    suppress_season_change: Rc<Cell<bool>>,
    on_back: Option<Rc<dyn Fn()>>,
    on_play_episode: SharedPlayEpisodeCallback,
    on_edit_tracks: Option<Rc<dyn Fn(String)>>,
    translations: TranslationsRef,
}

impl MediaDetails {
    pub fn new(db_path: &str, media_id: i64, translations: TranslationsRef) -> Self {
        let box_ = Box::new(Orientation::Vertical, 0);
        box_.set_valign(gtk4::Align::Start);
        box_.set_hexpand(true);
        box_.set_vexpand(true);

        let banner = Banner::new();
        let t = |key: &str| -> String { translations.borrow().t(key) };

        let synopsis_label = Label::new(None);
        synopsis_label.set_halign(gtk4::Align::Start);
        synopsis_label.set_valign(gtk4::Align::Start);
        synopsis_label.set_xalign(0.0);
        synopsis_label.set_hexpand(true);
        synopsis_label.set_wrap(true);
        synopsis_label.set_max_width_chars(100);
        synopsis_label.add_css_class("synopsis-text");

        let details_header = Box::new(Orientation::Vertical, 0);
        details_header.add_css_class("details-header");
        details_header.set_hexpand(true);
        details_header.set_valign(gtk4::Align::Start);
        details_header.append(banner.widget());

        let buttons_box = Box::new(Orientation::Horizontal, 10);
        buttons_box.set_halign(gtk4::Align::Start);
        buttons_box.set_valign(gtk4::Align::Center);
        buttons_box.set_margin_top(14);
        buttons_box.set_margin_start(16);
        buttons_box.add_css_class("hero-actions");

        let back_button = Button::with_label(&t("back.previous"));
        back_button.add_css_class("flat-button");
        buttons_box.append(&back_button);

        let tracks_button = Button::with_label(&t("button.tracks"));
        tracks_button.add_css_class("flat-button");
        tracks_button.set_tooltip_text(Some(&t("tooltip.tracks")));
        buttons_box.append(&tracks_button);

        let play_button = Button::with_label(&t("button.play"));
        play_button.add_css_class("hero-action-btn");
        play_button.add_css_class("primary");
        buttons_box.append(&play_button);

        let vote_label = Label::new(None);
        vote_label.set_halign(gtk4::Align::Start);
        vote_label.set_valign(gtk4::Align::Center);
        vote_label.set_margin_start(16);
        vote_label.add_css_class("vote-average");
        buttons_box.append(&vote_label);

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

        let content_container = Box::new(Orientation::Vertical, 0);
        content_container.set_hexpand(true);
        content_container.set_vexpand(true);
        content_container.set_valign(gtk4::Align::Start);

        box_.append(&details_header);
        box_.append(&synopsis_label);
        box_.append(&buttons_box);
        box_.append(&season_dropdown);
        box_.append(&content_container);

        let main_button_file_path = Rc::new(RefCell::new(None));
        let seasons: Rc<RefCell<Vec<SeasonOption>>> = Rc::new(RefCell::new(Vec::new()));
        let selected_season = Rc::new(RefCell::new(None));
        let suppress_season_change = Rc::new(Cell::new(false));
        let on_play_episode: SharedPlayEpisodeCallback = Rc::new(RefCell::new(None));
        let db_path = Rc::new(RefCell::new(db_path.to_string()));
        let root_media_id = Rc::new(Cell::new(media_id));

        // Un único handler para el desplegable. La selección programática usa
        // `suppress_season_change` para no volver a renderizar ni acumular
        // conexiones cuando se abre otro medio.
        {
            let seasons = seasons.clone();
            let selected_season = selected_season.clone();
            let suppress = suppress_season_change.clone();
            let db_path = db_path.clone();
            let root_media_id = root_media_id.clone();
            let content_container = content_container.clone();
            let translations = translations.clone();
            let on_play_episode = on_play_episode.clone();

            season_dropdown.connect_selected_notify(move |dropdown| {
                if suppress.get() {
                    return;
                }
                let index = dropdown.selected() as usize;
                let selection = seasons.borrow().get(index).map(|option| option.key);
                let Some(selection) = selection else {
                    return;
                };

                *selected_season.borrow_mut() = Some(selection);
                let db_path = db_path.borrow().clone();
                render_selection(
                    &db_path,
                    &content_container,
                    root_media_id.get(),
                    selection,
                    &translations,
                    &on_play_episode,
                );
            });
        }

        Self {
            box_,
            db_path,
            media_id,
            parent_id: None,
            root_media_id,
            banner,
            main_button_file_path,
            main_button_label: t("button.play"),
            synopsis_label,
            tracks_button,
            play_button,
            back_button,
            vote_label,
            season_dropdown,
            season_dropdown_model,
            content_container,
            seasons,
            selected_season,
            suppress_season_change,
            on_back: None,
            on_play_episode,
            on_edit_tracks: None,
            translations,
        }
    }

    fn t(&self, key: &str) -> String {
        self.translations.borrow().t(key)
    }

    pub fn set_db_path(&mut self, db_path: &str) {
        *self.db_path.borrow_mut() = db_path.to_string();
    }

    pub fn load_media(&mut self, media_id: i64) {
        let mut conn = match open_db(&self.db_path.borrow()) {
            Ok(conn) => conn,
            Err(error) => {
                eprintln!("Error abriendo BD: {}", error);
                return;
            }
        };
        let media = {
            let mut repo = MediaRepository::new(&mut conn);
            repo.find_by_id(media_id).unwrap_or(None)
        };
        close_db(conn);

        if let Some(media) = media {
            self.set_media(&media);
        }
    }

    pub fn set_media(&mut self, media: &Media) {
        self.set_media_with_season(media, None);
    }

    /// Muestra un medio conservando la temporada indicada (si sigue existiendo).
    /// Es la entrada usada al volver desde Ajustes o al refrescar metadatos.
    pub fn set_media_with_season(&mut self, media: &Media, requested: Option<SeasonSelection>) {
        self.media_id = media.id;
        self.parent_id = media.parent_id;
        self.root_media_id.set(media.id);
        self.season_dropdown.set_visible(false);
        self.seasons.borrow_mut().clear();
        self.selected_season.replace(None);

        let back_label = match media.parent_id {
            Some(_) if media.r#type == "anime" => self.t("back.anime"),
            Some(_) => self.t("back.series"),
            None => self.t("back.previous"),
        };
        self.back_button.set_label(&back_label);

        let type_label = match media.r#type.as_str() {
            "series" => self.t("media.type.series"),
            "anime" => self.t("media.type.anime"),
            "movie" => self.t("media.type.movie"),
            other => other.to_string(),
        };
        let info_label = match media.year {
            Some(year) => format!("{} · {}", type_label, year),
            None => type_label,
        };
        self.synopsis_label
            .set_label(media.synopsis.as_deref().unwrap_or(""));
        self.banner.set_info(&media.title, &info_label);
        self.banner.set_images(media.banner_path.as_deref(), None);

        if let Some(vote) = media.vote_average {
            self.vote_label.set_label(&format!("⭐ {:.1} / 10", vote));
            self.vote_label.set_visible(true);
        } else {
            self.vote_label.set_visible(false);
        }

        // Contexto del medio: temporadas hijas, archivos directos y objetivo
        // principal de reproducción. Todo se obtiene con una sola conexión.
        let (children, direct_files, first_file, next_episode) = {
            let mut conn = match open_db(&self.db_path.borrow()) {
                Ok(conn) => conn,
                Err(error) => {
                    eprintln!("Error abriendo BD: {}", error);
                    return;
                }
            };
            let (children, direct_files, first_file, next_episode) = {
                let mut repo = MediaRepository::new(&mut conn);
                let children = repo.list_children(media.id).unwrap_or_default();
                let direct_files = repo.list_files_by_media(media.id).unwrap_or_default();
                let first_file = repo.first_file_path_for_media(media.id).unwrap_or(None);
                let next_episode = if media.parent_id.is_none() && media.r#type != "movie" {
                    repo.get_next_episode_to_watch(media.id).unwrap_or(None)
                } else {
                    None
                };
                (children, direct_files, first_file, next_episode)
            };
            close_db(conn);
            (children, direct_files, first_file, next_episode)
        };

        let (label, file_path) = if media.r#type == "movie" {
            match first_file {
                Some(path) => (self.t("button.play"), Some(path)),
                None => (self.t("button.play"), None),
            }
        } else if let Some((_id, path, episode_label)) = next_episode {
            (
                format!("{} {}", self.t("button.continue_prefix"), episode_label),
                Some(path),
            )
        } else if let Some(path) = first_file {
            (self.t("button.play"), Some(path))
        } else {
            (self.t("button.play"), None)
        };
        *self.main_button_file_path.borrow_mut() = file_path;
        self.main_button_label = label;
        self.play_button.set_label(&self.main_button_label);

        while let Some(child) = self.content_container.first_child() {
            self.content_container.remove(&child);
        }

        // Las temporadas hijas tienen prioridad. Solo si no existen se usan
        // los archivos directos del medio raíz.
        let options: Vec<SeasonOption> = if !children.is_empty() {
            children
                .iter()
                .map(|season| SeasonOption {
                    key: SeasonSelection::Child(season.id),
                    label: season.title.clone(),
                })
                .collect()
        } else {
            season_numbers(&direct_files)
                .into_iter()
                .map(|number| SeasonOption {
                    key: SeasonSelection::Number(number),
                    label: self
                        .t("season.label_format")
                        .replacen("{}", &number.to_string(), 1),
                })
                .collect()
        };

        if options.is_empty() {
            if direct_files.is_empty() {
                let text = if media.parent_id.is_some() {
                    self.t("content.no_files")
                } else {
                    self.t("content.no_episodes")
                };
                append_message(&self.content_container, &text);
            } else {
                render_files(
                    &self.content_container,
                    None,
                    &direct_files,
                    &self.translations,
                    &self.on_play_episode,
                );
            }
            return;
        }

        self.seasons
            .borrow_mut()
            .extend(options.iter().map(|option| SeasonOption {
                key: option.key,
                label: option.label.clone(),
            }));
        let labels: Vec<&str> = options.iter().map(|option| option.label.as_str()).collect();
        self.season_dropdown_model = gtk4::StringList::new(&labels);
        self.season_dropdown
            .set_model(Some(&self.season_dropdown_model));

        let index = requested
            .and_then(|requested| options.iter().position(|option| option.key == requested))
            .unwrap_or(0);
        self.suppress_season_change.set(true);
        self.season_dropdown.set_selected(index as u32);
        self.suppress_season_change.set(false);
        self.season_dropdown.set_visible(true);

        let selection = options[index].key;
        self.selected_season.replace(Some(selection));
        let db_path = self.db_path.borrow().clone();
        render_selection(
            &db_path,
            &self.content_container,
            self.media_id,
            selection,
            &self.translations,
            &self.on_play_episode,
        );
    }

    pub fn media_id(&self) -> i64 {
        self.media_id
    }

    pub fn parent_id(&self) -> Option<i64> {
        self.parent_id
    }

    pub fn season_selection(&self) -> Option<SeasonSelection> {
        *self.selected_season.borrow()
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

    pub fn on_play_episode<F>(&mut self, f: F)
    where
        F: Fn(String) + 'static,
    {
        let cb: Rc<dyn Fn(String)> = Rc::new(f);
        let cb_clone = cb.clone();
        let file_path_rc = self.main_button_file_path.clone();
        self.play_button.connect_clicked(move |_| {
            if let Some(path) = file_path_rc.borrow().as_ref() {
                cb_clone(path.clone());
            }
        });
        *self.on_play_episode.borrow_mut() = Some(cb);
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
}

/// Representa un archivo de episodio en la rejilla. Mantiene el texto y el
/// número coherentes con la temporada seleccionada.
fn render_selection(
    db_path: &str,
    container: &Box,
    root_media_id: i64,
    selection: SeasonSelection,
    translations: &TranslationsRef,
    on_play_episode: &SharedPlayEpisodeCallback,
) {
    let (title, files) = match selection {
        SeasonSelection::Child(season_id) => {
            let mut conn = match open_db(db_path) {
                Ok(conn) => conn,
                Err(error) => {
                    eprintln!("Error abriendo BD: {}", error);
                    return;
                }
            };
            let (title, files) = {
                let mut repo = MediaRepository::new(&mut conn);
                let title = repo
                    .find_by_id(season_id)
                    .unwrap_or(None)
                    .map(|season| season.title);
                let files = repo.list_files_by_media(season_id).unwrap_or_default();
                (title, files)
            };
            close_db(conn);
            (title, files)
        }
        SeasonSelection::Number(number) => {
            let files = {
                let mut conn = match open_db(db_path) {
                    Ok(conn) => conn,
                    Err(error) => {
                        eprintln!("Error abriendo BD: {}", error);
                        return;
                    }
                };
                let files = {
                    let mut repo = MediaRepository::new(&mut conn);
                    repo.list_files_by_media(root_media_id).unwrap_or_default()
                };
                close_db(conn);
                files
            };
            let files: Vec<MediaFile> = files
                .into_iter()
                .filter(|file| file.season == Some(number))
                .collect();
            let title = translations.borrow().t("season.label_format").replacen(
                "{}",
                &number.to_string(),
                1,
            );
            (Some(title), files)
        }
    };

    render_files(
        container,
        title.as_deref(),
        &files,
        translations,
        on_play_episode,
    );
}

fn render_files(
    container: &Box,
    title: Option<&str>,
    files: &[MediaFile],
    translations: &TranslationsRef,
    on_play_episode: &SharedPlayEpisodeCallback,
) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    if let Some(title) = title.filter(|title| !title.is_empty()) {
        let season_title = Label::new(Some(title));
        season_title.set_halign(gtk4::Align::Start);
        season_title.set_hexpand(true);
        season_title.set_xalign(0.0);
        season_title.set_margin_top(24);
        season_title.add_css_class("season-grid-title");
        container.append(&season_title);
    }

    if files.is_empty() {
        append_message(
            container,
            &translations.borrow().t("content.no_episodes_season"),
        );
        return;
    }

    let mut cards = Vec::with_capacity(files.len());
    for file in files {
        cards.push(build_episode_card(file, translations, on_play_episode));
    }
    container.append(&episode_grid(cards));
}

fn append_message(container: &Box, text: &str) {
    let label = Label::new(Some(text));
    label.set_halign(gtk4::Align::Start);
    label.set_valign(gtk4::Align::Start);
    label.set_margin_top(24);
    label.add_css_class("synopsis-text");
    container.append(&label);
}

fn season_numbers(files: &[MediaFile]) -> Vec<i32> {
    let mut numbers: Vec<i32> = files.iter().filter_map(|file| file.season).collect();
    numbers.sort_unstable();
    numbers.dedup();
    numbers
}

fn build_episode_card(
    file: &MediaFile,
    translations: &TranslationsRef,
    on_play_episode: &SharedPlayEpisodeCallback,
) -> Box {
    let t = |key: &str| translations.borrow().t(key);
    let (episode_label, episode_number) = episode_texts(file, translations);

    let card = Box::new(Orientation::Vertical, 6);
    card.add_css_class("episode-card");
    card.set_hexpand(false);
    card.set_vexpand(false);
    card.set_halign(gtk4::Align::Start);
    card.set_size_request(EPISODE_POSTER_WIDTH + 24, EPISODE_POSTER_HEIGHT + 92);

    let poster_box = Box::new(Orientation::Vertical, 0);
    poster_box.set_size_request(EPISODE_POSTER_WIDTH, EPISODE_POSTER_HEIGHT);
    poster_box.set_halign(gtk4::Align::Center);
    poster_box.set_valign(gtk4::Align::Start);

    let pixbuf = file
        .episode_image_path
        .as_deref()
        .and_then(cached_path)
        .and_then(|cache_path| {
            GtkPixbuf::from_file_at_scale(
                &cache_path,
                EPISODE_POSTER_WIDTH,
                EPISODE_POSTER_HEIGHT,
                true,
            )
            .ok()
        });

    match pixbuf {
        Some(pixbuf) => {
            let picture = poster_picture(&pixbuf, EPISODE_POSTER_WIDTH, EPISODE_POSTER_HEIGHT);
            picture.add_css_class("episode-poster");
            poster_box.append(&picture);
        }
        None => {
            let placeholder = placeholder_widget(EPISODE_POSTER_WIDTH, EPISODE_POSTER_HEIGHT);
            placeholder.add_css_class("episode-poster");
            poster_box.append(&placeholder);
        }
    }
    card.append(&poster_box);

    let num_label = Label::new(Some(&episode_number));
    num_label.set_halign(gtk4::Align::Center);
    num_label.add_css_class("episode-number");
    card.append(&num_label);

    if !episode_label.is_empty() {
        let title_label = Label::new(Some(&episode_label));
        title_label.set_halign(gtk4::Align::Center);
        title_label.set_justify(gtk4::Justification::Center);
        title_label.set_wrap(true);
        title_label.set_max_width_chars(24);
        title_label.set_lines(2);
        title_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        title_label.add_css_class("episode-title");
        card.append(&title_label);
    }

    let play_btn = Button::with_label("▶");
    play_btn.add_css_class("episode-play-btn");
    play_btn.set_halign(gtk4::Align::Center);
    play_btn.set_tooltip_text(Some(&format!(
        "{} {}",
        t("button.play_episode"),
        episode_label
    )));
    let file_path = file.file_path.clone();
    let callback = on_play_episode.borrow().clone();
    if let Some(callback) = callback {
        let file_path = file_path.clone();
        play_btn.connect_clicked(move |_| callback(file_path.clone()));
    }
    card.append(&play_btn);

    let click = GestureClick::new();
    let callback = on_play_episode.borrow().clone();
    if let Some(callback) = callback {
        click.connect_pressed(move |_, _, _, _| callback(file_path.clone()));
    }
    card.add_controller(click);

    card
}

fn episode_texts(file: &MediaFile, translations: &TranslationsRef) -> (String, String) {
    let t = |key: &str| translations.borrow().t(key);
    let title = file.title.as_deref().unwrap_or("").trim();

    let episode_number = match (file.season, file.episode) {
        (Some(season), Some(episode)) => format!("S{:02}E{:02}", season, episode),
        (None, Some(episode)) => format!("E{:02}", episode),
        (Some(season), None) => format!("{}{:02}", t("episode.season_abbr"), season),
        (None, None) => String::new(),
    };

    let episode_label = match (file.season, file.episode) {
        (Some(season), Some(episode)) if !title.is_empty() => {
            format!("S{}E{} - {}", season, episode, title)
        }
        (Some(season), Some(episode)) => format!("S{}E{}", season, episode),
        (None, Some(episode)) if !title.is_empty() => {
            format!("{} {} - {}", t("episode.ep_word"), episode, title)
        }
        (None, Some(episode)) => format!("{} {}", t("episode.ep_word"), episode),
        (Some(season), None) if !title.is_empty() => {
            format!("{} {} - {}", t("episode.season_word"), season, title)
        }
        (Some(season), None) => format!("{} {}", t("episode.season_word"), season),
        (None, None) => title.to_string(),
    };

    (episode_label, episode_number)
}

fn episode_grid(cards: Vec<Box>) -> FlowBox {
    let grid = FlowBox::new();
    grid.add_css_class("episode-grid");
    grid.set_hexpand(true);
    grid.set_valign(gtk4::Align::Start);
    grid.set_max_children_per_line(6);
    grid.set_min_children_per_line(1);
    grid.set_row_spacing(14);
    grid.set_column_spacing(14);
    grid.set_homogeneous(false);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn media_file(season: Option<i32>, episode: Option<i32>, title: &str) -> MediaFile {
        MediaFile {
            id: 1,
            media_id: 1,
            season,
            episode,
            title: Some(title.to_string()),
            file_path: "/tmp/S01E01.mkv".to_string(),
            size_bytes: None,
            mtime_ms: None,
            indexed_at: String::new(),
            audio_stream_index: None,
            subtitle_stream_index: None,
            subtitles_disabled: false,
            streams_indexed_at: None,
            episode_image_path: None,
            episode_image_checked_at: None,
        }
    }

    #[test]
    fn season_numbers_ordena_y_elimina_duplicados() {
        let files = vec![
            media_file(Some(2), Some(1), "b"),
            media_file(Some(1), Some(1), "a"),
            media_file(Some(2), Some(2), "c"),
            media_file(None, Some(3), "d"),
        ];
        assert_eq!(season_numbers(&files), vec![1, 2]);
    }

    #[test]
    fn season_numbers_vacio_sin_temporadas() {
        let files = vec![media_file(None, Some(1), "a")];
        assert!(season_numbers(&files).is_empty());
    }
}
