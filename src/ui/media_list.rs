//! Componente que muestra una lista de medios con sus pósters.

use crate::db::media_repo::Media;
use gtk4::prelude::*;
use gtk4::{Box, Label, Orientation, ScrolledWindow};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::poster::Poster;

pub struct MediaList {
    box_: Box,
    media: Rc<RefCell<Vec<Media>>>,
    on_media_select: Option<Rc<dyn Fn(i64)>>,
}

impl MediaList {
    pub fn new() -> Self {
        let box_ = Box::new(Orientation::Vertical, 10);
        box_.set_margin_top(10);
        box_.set_margin_bottom(10);
        box_.set_margin_start(10);
        box_.set_margin_end(10);

        Self {
            box_,
            media: Rc::new(RefCell::new(Vec::new())),
            on_media_select: None,
        }
    }

    pub fn widget(&self) -> &Box {
        &self.box_
    }

    pub fn set_media(&mut self, media: Vec<Media>) {
        *self.media.borrow_mut() = media;
        self.refresh();
    }

    pub fn refresh(&self) {
        while let Some(child) = self.box_.first_child() {
            self.box_.remove(&child);
        }

        let scrolled = ScrolledWindow::new();
        scrolled.set_hscrollbar_policy(gtk4::PolicyType::Never);
        scrolled.set_vscrollbar_policy(gtk4::PolicyType::Automatic);
        scrolled.set_vexpand(true);
        self.box_.append(&scrolled);

        let media = self.media.borrow();
        for m in media.iter() {
            let card = create_media_card(m.clone(), self.on_media_select.clone());
            scrolled.add(&card);
        }

        scrolled.set_visible(true);
    }

    pub fn on_select<F>(&mut self, f: F)
    where
        F: Fn(i64) + 'static,
    {
        self.on_media_select = Some(Rc::new(f));
        self.refresh();
    }
}

fn create_media_card(media: Media, on_select: Option<Rc<dyn Fn(i64)>>) -> gtk4::Box {
    let card = Box::new(Orientation::Vertical, 5);
    card.set_margin_top(5);
    card.set_margin_bottom(5);
    card.set_margin_start(5);
    card.set_margin_end(5);
    card.set_size_request(150, 220);

    let poster = Poster::new(&media.title, &media.poster_path);
    card.append(&poster.widget());

    let label = Label::new(Some(&media.title));
    label.set_wrap(true);
    label.set_max_width_chars(15);
    card.append(&label);

    let btn = gtk4::Button::new();
    btn.set_hexpand(true);
    btn.set_vexpand(true);
    btn.add_css_class("flat");

    let media_id = media.id;
    if let Some(on_select) = &on_select {
        let on_select = on_select.clone();
        btn.connect_clicked(move |_| {
            on_select(media_id);
        });
    }

    card.append(&btn);

    card
}
