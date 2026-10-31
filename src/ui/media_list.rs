//! Galería de medios con tarjetas al estilo Crunchyroll.
//!
//! Cada tarjeta muestra:
//! - Póster (relación 2:3) con badge de tipo en la esquina superior izquierda.
//! - Título y año debajo.
//! - Efecto hover con elevación.
//! - Todo el área es clickeable para abrir los detalles.
//!
//! La galería es responsive: el número de columnas por línea se ajusta
//! dinámicamente al ancho de la ventana. Las tarjetas se distribuyen desde la
//! izquierda con un espaciado pequeño, uniforme y sin centrarlas
//! individualmente.

use crate::core::poster_service::cached_path;
use crate::db::media_repo::Media;
use crate::ui::poster::{placeholder_widget, poster_picture};
use gtk4::gdk_pixbuf::Pixbuf as GtkPixbuf;
use gtk4::prelude::*;
use gtk4::{Box, FlowBox, GestureClick, Label, Orientation, ScrolledWindow};
use std::cell::RefCell;
use std::rc::Rc;

/// Tamaño del póster en la galería (relación 2:3).
const CARD_POSTER_WIDTH: i32 = 200;
const CARD_POSTER_HEIGHT: i32 = 300;

/// Ancho estimado de una tarjeta completa (póster + márgenes + padding).
const CARD_WIDTH: u32 = 248;

/// Espaciado uniforme entre tarjetas (horizontal y vertical).
const CARD_SPACING: i32 = 12;

/// Margen exterior de la galería respecto al área de contenido.
const GALLERY_MARGIN: i32 = 16;

/// Número mínimo y máximo de columnas por línea.
const MIN_COLUMNS: u32 = 2;
const MAX_COLUMNS: u32 = 8;

/// Tarjeta individual de un medio (póster + título + etiqueta de tipo).
/// Diseño al estilo Crunchyroll: póster grande con título y etiqueta debajo.
/// Las tarjetas no se centran individualmente; se colocan consecutivamente
/// desde la izquierda del área de contenido.
struct MediaCard {
    widget: Box,
}

impl MediaCard {
    fn new(media: &Media, on_select: Rc<dyn Fn(i64)>) -> Self {
        let card = Box::new(Orientation::Vertical, 8);
        card.add_css_class("media-card");
        // Alineación al inicio: las tarjetas se colocan consecutivamente desde
        // la izquierda, sin ocupar el espacio sobrante de forma individual.
        card.set_halign(gtk4::Align::Start);
        card.set_valign(gtk4::Align::Start);
        card.set_margin_top(CARD_SPACING);
        card.set_margin_bottom(CARD_SPACING);
        card.set_margin_start(CARD_SPACING);
        card.set_margin_end(CARD_SPACING);

        // Póster (ocupa la mayor parte de la tarjeta).
        let poster_stack = Box::new(Orientation::Vertical, 0);
        poster_stack.set_hexpand(false);
        poster_stack.set_vexpand(false);
        poster_stack.set_halign(gtk4::Align::Start);
        poster_stack.set_valign(gtk4::Align::Start);
        poster_stack.set_size_request(CARD_POSTER_WIDTH, CARD_POSTER_HEIGHT);

        // Póster.
        let pixbuf = media
            .poster_path
            .as_deref()
            .and_then(cached_path)
            .and_then(|cache_path| {
                GtkPixbuf::from_file_at_scale(
                    &cache_path,
                    CARD_POSTER_WIDTH,
                    CARD_POSTER_HEIGHT,
                    true,
                )
                .ok()
            });

        match pixbuf {
            Some(pixbuf) => {
                let picture = poster_picture(&pixbuf, CARD_POSTER_WIDTH, CARD_POSTER_HEIGHT);
                picture.add_css_class("media-poster");
                poster_stack.append(&picture);
            }
            None => {
                let placeholder = placeholder_widget(CARD_POSTER_WIDTH, CARD_POSTER_HEIGHT);
                placeholder.add_css_class("media-poster");
                poster_stack.append(&placeholder);
            }
        }

        card.append(&poster_stack);

        // Etiqueta de tipo (Anime / Serie / Película) debajo del póster.
        let type_label_text = match media.r#type.as_str() {
            "anime" => "Anime",
            "series" => "Serie",
            "movie" => "Película",
            other => other,
        };
        let type_label = Label::new(Some(type_label_text));
        type_label.set_halign(gtk4::Align::Start);
        type_label.set_valign(gtk4::Align::Start);
        type_label.add_css_class("card-type-label");
        card.append(&type_label);

        // Título.
        let title_label = Label::new(Some(&media.title));
        title_label.set_wrap(true);
        title_label.set_max_width_chars(18);
        title_label.set_justify(gtk4::Justification::Left);
        title_label.set_halign(gtk4::Align::Start);
        title_label.set_valign(gtk4::Align::Start);
        title_label.set_lines(2);
        title_label.add_css_class("poster-title");
        card.append(&title_label);

        // Año.
        if let Some(year) = media.year {
            let year_label = Label::new(Some(&format!("{}", year)));
            year_label.add_css_class("poster-year");
            year_label.set_halign(gtk4::Align::Start);
            card.append(&year_label);
        }

        // Todo el tarjetón es clickeable.
        let click = GestureClick::new();
        let media_id = media.id;
        let on_select_clone = on_select.clone();
        click.connect_pressed(move |_, _, _, _| {
            on_select_clone(media_id);
        });
        card.add_controller(click);

        Self { widget: card }
    }
}

/// Componente que muestra una galería de medios con pósters.
pub struct MediaList {
    content: Box,
    flowbox: FlowBox,
    media: Rc<RefCell<Vec<Media>>>,
    on_media_select: Option<Rc<dyn Fn(i64)>>,
}

impl MediaList {
    pub fn new() -> Self {
        crate::ui::poster::install_poster_css();

        let content = Box::new(Orientation::Vertical, 0);
        content.set_vexpand(true);
        content.set_hexpand(true);

        let flowbox = FlowBox::new();
        flowbox.set_hexpand(true);
        flowbox.set_vexpand(true);
        flowbox.set_valign(gtk4::Align::Start);
        flowbox.set_halign(gtk4::Align::Fill);
        // Las tarjetas se distribuyen desde la izquierda sin centrar cada una
        // individualmente ni repartir el espacio sobrentante entre ellas.
        flowbox.set_max_children_per_line(MIN_COLUMNS);
        flowbox.set_min_children_per_line(1);
        flowbox.set_row_spacing(CARD_SPACING as u32);
        flowbox.set_column_spacing(CARD_SPACING as u32);
        flowbox.set_homogeneous(true);
        flowbox.set_selection_mode(gtk4::SelectionMode::None);
        flowbox.set_margin_top(GALLERY_MARGIN);
        flowbox.set_margin_bottom(GALLERY_MARGIN);
        flowbox.set_margin_start(GALLERY_MARGIN);
        flowbox.set_margin_end(GALLERY_MARGIN);

        let scrolled = ScrolledWindow::new();
        scrolled.set_hexpand(true);
        scrolled.set_vexpand(true);
        scrolled.set_hscrollbar_policy(gtk4::PolicyType::Never);
        scrolled.set_vscrollbar_policy(gtk4::PolicyType::Automatic);
        scrolled.set_child(Some(&flowbox));

        content.append(&scrolled);

        Self {
            content,
            flowbox,
            media: Rc::new(RefCell::new(Vec::new())),
            on_media_select: None,
        }
    }

    pub fn widget(&self) -> &Box {
        &self.content
    }

    /// Ajusta el número máximo de columnas por línea según el ancho
    /// disponible de la galería, para que sea responsive en distintas
    /// resoluciones de pantalla. Las tarjetas siguen empezando desde la
    /// izquierda y ocupan progresivamente todo el ancho disponible.
    pub fn set_responsive_columns(&self, available_width: i32) {
        let usable_width = available_width.saturating_sub(2 * GALLERY_MARGIN) as u32;
        let columns = if usable_width < 600 {
            MIN_COLUMNS
        } else {
            (usable_width / CARD_WIDTH).clamp(MIN_COLUMNS, MAX_COLUMNS)
        };
        self.flowbox
            .set_max_children_per_line(columns);
        self.flowbox.set_min_children_per_line(1);
    }

    pub fn set_media(&mut self, media: Vec<Media>) {
        *self.media.borrow_mut() = media;
        self.refresh();
    }

    pub fn refresh(&self) {
        while let Some(child) = self.flowbox.first_child() {
            self.flowbox.remove(&child);
        }

        let media = self.media.borrow();
        if let Some(on_select) = &self.on_media_select {
            for m in media.iter() {
                let card = MediaCard::new(m, on_select.clone());
                self.flowbox.insert(&card.widget, -1);
            }
        }
    }

    pub fn on_select<F>(&mut self, f: F)
    where
        F: Fn(i64) + 'static,
    {
        self.on_media_select = Some(Rc::new(f));
        self.refresh();
    }
}

impl Default for MediaList {
    fn default() -> Self {
        Self::new()
    }
}
