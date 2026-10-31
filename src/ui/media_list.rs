//! Galería de medios con tarjetas al estilo Crunchyroll.
//!
//! Cada tarjeta muestra:
//! - Póster (relación 2:3) con badge de tipo en la esquina superior izquierda.
//! - Título y año debajo.
//! - Efecto hover con elevación.
//! - Todo el área es clickeable para abrir los detalles.
//!
//! La galería es responsive: el número de columnas por línea se ajusta
//! dinámicamente al ancho de la ventana.

use crate::core::poster_service::cached_path;
use crate::db::media_repo::Media;
use crate::ui::poster::{placeholder_widget, poster_picture};
use gtk4::gdk_pixbuf::Pixbuf as GtkPixbuf;
use gtk4::prelude::*;
use gtk4::{Box, FlowBox, GestureClick, Label, Orientation, ScrolledWindow};
use std::cell::RefCell;
use std::rc::Rc;

/// Tamaño del póster en la galería (relación 2:3).
const CARD_POSTER_WIDTH: i32 = 220;
const CARD_POSTER_HEIGHT: i32 = 330;

/// Badge de tipo mostrado en la esquina superior izquierda del póster.
struct TypeBadge {
    widget: Box,
}

impl TypeBadge {
    fn new(media_type: &str) -> Self {
        let label = match media_type {
            "anime" => "Anime",
            "series" => "Serie",
            "movie" => "Película",
            other => other,
        };

        let box_ = Box::new(Orientation::Horizontal, 0);
        box_.set_halign(gtk4::Align::Start);
        box_.set_valign(gtk4::Align::Start);
        box_.set_margin_start(10);
        box_.set_margin_top(10);

        let label_widget = Label::new(Some(label));
        label_widget.add_css_class("type-badge");
        box_.append(&label_widget);

        Self { widget: box_ }
    }
}

/// Tarjeta individual de un medio (póster + badge + título + año).
struct MediaCard {
    widget: Box,
}

impl MediaCard {
    fn new(media: &Media, on_select: Rc<dyn Fn(i64)>) -> Self {
        let card = Box::new(Orientation::Vertical, 10);
        card.add_css_class("media-card");
        card.set_halign(gtk4::Align::Center);
        card.set_valign(gtk4::Align::Start);
        card.set_size_request(CARD_POSTER_WIDTH + 32, CARD_POSTER_HEIGHT + 110);
        card.set_margin_top(12);
        card.set_margin_bottom(12);
        card.set_margin_start(12);
        card.set_margin_end(12);

        // Contenedor del póster con badge solapado arriba-izquierda.
        let poster_stack = Box::new(Orientation::Vertical, 0);
        poster_stack.set_hexpand(false);
        poster_stack.set_vexpand(false);
        poster_stack.set_halign(gtk4::Align::Center);
        poster_stack.set_valign(gtk4::Align::Start);
        poster_stack.set_size_request(CARD_POSTER_WIDTH, CARD_POSTER_HEIGHT);

        // Badge de tipo solapado sobre el póster.
        let badge = TypeBadge::new(&media.r#type);
        poster_stack.append(&badge.widget);

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

        // Título.
        let title_label = Label::new(Some(&media.title));
        title_label.set_wrap(true);
        title_label.set_max_width_chars(22);
        title_label.set_justify(gtk4::Justification::Center);
        title_label.set_halign(gtk4::Align::Center);
        title_label.set_valign(gtk4::Align::Start);
        title_label.add_css_class("poster-title");
        card.append(&title_label);

        // Año.
        if let Some(year) = media.year {
            let year_label = Label::new(Some(&format!("{}", year)));
            year_label.add_css_class("poster-year");
            year_label.set_halign(gtk4::Align::Center);
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
        flowbox.set_max_children_per_line(5);
        flowbox.set_min_children_per_line(1);
        flowbox.set_row_spacing(28);
        flowbox.set_column_spacing(24);
        flowbox.set_homogeneous(true);
        flowbox.set_selection_mode(gtk4::SelectionMode::None);
        flowbox.set_margin_top(20);
        flowbox.set_margin_bottom(20);
        flowbox.set_margin_start(20);
        flowbox.set_margin_end(20);

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
    /// resoluciones de pantalla.
    pub fn set_responsive_columns(&self, available_width: i32) {
        // Cada tarjeta ocupa ~284px (poster 220 + márgenes y padding).
        // Calculamos cuántas caben con un mínimo de 2 y un máximo de 8.
        let card_width = CARD_POSTER_WIDTH + 56; // 220 + 32 de márgenes laterales
        let columns = if available_width < 600 {
            2
        } else {
            (available_width / card_width).clamp(2, 8)
        };
        self.flowbox
            .set_max_children_per_line(columns.try_into().unwrap_or(2));
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
