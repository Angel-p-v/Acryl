//! Componente Poster: renderiza pósters con gdk-pixbuf.

use gtk4::gdk_pixbuf::Pixbuf as GtkPixbuf;
use gtk4::prelude::*;
use gtk4::{Box, Image, Label, Orientation};
use std::path::Path;

pub struct Poster {
    box_: Box,
    image: Image,
    label: Label,
}

impl Poster {
    pub fn new(title: &str, poster_path: &Option<String>) -> Self {
        let box_ = Box::new(Orientation::Vertical, 5);
        box_.set_margin_top(5);
        box_.set_margin_bottom(5);
        box_.set_margin_start(5);
        box_.set_margin_end(5);

        let image = Image::new();
        image.set_size_request(150, 200);

        if let Some(path) = poster_path
            && Path::new(path).exists()
            && let Ok(pixbuf) = GtkPixbuf::from_file_at_scale(path, 150, 200, true)
        {
            image.set_from_pixbuf(Some(&pixbuf));
        }

        let label = Label::new(Some(title));
        label.set_wrap(true);
        label.set_max_width_chars(15);

        box_.append(&image);
        box_.append(&label);

        Self { box_, image, label }
    }

    pub fn widget(&self) -> &Box {
        &self.box_
    }

    pub fn set_title(&self, title: &str) {
        self.label.set_label(title);
    }

    pub fn set_poster(&self, poster_path: Option<&str>) {
        if let Some(path) = poster_path
            && Path::new(path).exists()
            && let Ok(pixbuf) = GtkPixbuf::from_file_at_scale(path, 150, 200, true)
        {
            self.image.set_from_pixbuf(Some(&pixbuf));
        }
    }
}
