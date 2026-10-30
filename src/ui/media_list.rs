//! Componente que muestra una galería de medios con pósters.
//! Usa FlowBox con renderizado de pósters desde caché local.
//! Diseñado para ser eficiente con colecciones grandes mediante carga paginada.

use crate::core::poster_service::cached_path;
use crate::db::media_repo::Media;
use gtk4::gdk_pixbuf::Pixbuf as GtkPixbuf;
use gtk4::prelude::*;
use gtk4::{Box, FlowBox, GestureClick, Label, Orientation, ScrolledWindow};
use std::cell::RefCell;
use std::rc::Rc;

/// Tarjeta individual de un medio (póster + título + año).
struct MediaCard {
    widget: Box,
}

impl MediaCard {
    fn new(media: &Media, on_select: Rc<dyn Fn(i64)>) -> Self {
        let card = Box::new(Orientation::Vertical, 5);
        card.set_halign(gtk4::Align::Center);
        card.set_valign(gtk4::Align::Start);
        card.set_size_request(200, 300);
        card.set_margin_top(10);
        card.set_margin_bottom(10);
        card.set_margin_start(10);
        card.set_margin_end(10);

        // Imagen del póster
        let image = gtk4::Image::new();
        image.set_size_request(150, 200);
        image.set_halign(gtk4::Align::Center);

        // Cargar póster desde caché si está disponible
        #[allow(clippy::collapsible_if)]
        if let Some(poster_path) = &media.poster_path
            && let Some(cache_path) = cached_path(poster_path)
            && let Ok(pixbuf) = GtkPixbuf::from_file_at_scale(&cache_path, 150, 200, true)
        {
            image.set_from_pixbuf(Some(&pixbuf));
        }

        card.append(&image);

        // Título
        let title_label = Label::new(Some(&media.title));
        title_label.set_wrap(true);
        title_label.set_max_width_chars(20);
        title_label.set_halign(gtk4::Align::Center);
        title_label.add_css_class("card-title");
        card.append(&title_label);

        // Año si está disponible
        if let Some(year) = media.year {
            let year_label = Label::new(Some(&format!("{}", year)));
            year_label.add_css_class("card-year");
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
        let content = Box::new(Orientation::Vertical, 0);
        content.set_vexpand(true);
        content.set_hexpand(true);

        let flowbox = FlowBox::new();
        flowbox.set_hexpand(true);
        flowbox.set_vexpand(true);
        flowbox.set_valign(gtk4::Align::Start);
        flowbox.set_halign(gtk4::Align::Start);
        flowbox.set_max_children_per_line(6);
        flowbox.set_min_children_per_line(1);
        flowbox.set_row_spacing(16);
        flowbox.set_column_spacing(16);
        flowbox.set_homogeneous(false);

        let scrolled = ScrolledWindow::new();
        scrolled.set_hexpand(true);
        scrolled.set_vexpand(true);
        scrolled.set_hscrollbar_policy(gtk4::PolicyType::Automatic);
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
