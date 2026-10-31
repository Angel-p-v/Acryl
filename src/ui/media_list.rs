//! Componente que muestra una galería de medios con pósters.
//! Usa FlowBox con renderizado de pósters desde caché local.
//! Diseñado para ser eficiente con colecciones grandes mediante carga paginada.

use crate::core::poster_service::cached_path;
use crate::db::media_repo::Media;
use crate::ui::poster::{placeholder_widget, poster_picture};
use gtk4::gdk_pixbuf::Pixbuf as GtkPixbuf;
use gtk4::prelude::*;
use gtk4::{Box, FlowBox, GestureClick, Label, Orientation, ScrolledWindow};
use std::cell::RefCell;
use std::rc::Rc;

/// Tamaño del póster en la galería (proporción 2:3, más grande y alto).
const CARD_POSTER_WIDTH: i32 = 260;
const CARD_POSTER_HEIGHT: i32 = 390;

/// Tarjeta individual de un medio (póster + título + año).
struct MediaCard {
    widget: Box,
}

impl MediaCard {
    fn new(media: &Media, on_select: Rc<dyn Fn(i64)>) -> Self {
        let card = Box::new(Orientation::Vertical, 8);
        card.add_css_class("poster-card");
        card.set_halign(gtk4::Align::Center);
        card.set_valign(gtk4::Align::Start);
        card.set_size_request(CARD_POSTER_WIDTH + 40, CARD_POSTER_HEIGHT + 80);
        card.set_margin_top(8);
        card.set_margin_bottom(8);
        card.set_margin_start(8);
        card.set_margin_end(8);

        // Cargar póster desde caché si está disponible.
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
                card.append(&picture);
            }
            None => {
                let placeholder = placeholder_widget(CARD_POSTER_WIDTH, CARD_POSTER_HEIGHT);
                card.append(&placeholder);
            }
        }

        // Título
        let title_label = Label::new(Some(&media.title));
        title_label.set_wrap(true);
        title_label.set_max_width_chars(22);
        title_label.set_justify(gtk4::Justification::Center);
        title_label.set_halign(gtk4::Align::Center);
        title_label.add_css_class("poster-title");
        card.append(&title_label);

        // Año si está disponible
        if let Some(year) = media.year {
            let year_label = Label::new(Some(&format!("{}", year)));
            year_label.add_css_class("poster-year");
            year_label.set_halign(gtk4::Align::Center);
            card.append(&year_label);
        }

        // Todo el tarjetón es clickeable
        let click = GestureClick::new();
        let media_id = media.id;
        click.connect_pressed(move |_, _, _, _| {
            on_select(media_id);
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
        flowbox.set_max_children_per_line(4);
        flowbox.set_min_children_per_line(1);
        flowbox.set_row_spacing(24);
        flowbox.set_column_spacing(24);
        flowbox.set_homogeneous(true);
        flowbox.set_selection_mode(gtk4::SelectionMode::None);
        flowbox.set_margin_top(12);
        flowbox.set_margin_bottom(12);
        flowbox.set_margin_start(12);
        flowbox.set_margin_end(12);

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

    pub fn set_media(&mut self, media: Vec<Media>) {
        *self.media.borrow_mut() = media;
        self.refresh();
    }

    pub fn refresh(&self) {
        // Remover todos los hijos existentes del FlowBox
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
