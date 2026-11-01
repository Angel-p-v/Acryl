//! Galería de medios virtualizada con GtkListView.
//!
//! Usa `GtkListView` + `GListStore` (con `StringObject`) + `SignalListItemFactory`
//! para que solo se rendericen las tarjetas visibles, eliminando las advertencias
//! de GTK al medir `GtkGrid` con miles de ítems y permitiendo scroll fluido con
//! bibliotecas grandes.

use crate::core::poster_service::cached_path;
use crate::db::media_repo::Media;
use crate::ui::i18n::TranslationsRef;
use crate::ui::poster::{placeholder_widget, poster_picture};
use gtk4::gdk_pixbuf::Pixbuf as GtkPixbuf;
use gtk4::prelude::*;
use gtk4::{
    Box, Label, ListView, Orientation, ScrolledWindow, SignalListItemFactory, SingleSelection,
    StringObject, gio,
};
use std::rc::Rc;

/// Tamaño del póster en la galería (relación 2:3).
const CARD_POSTER_WIDTH: i32 = 200;
const CARD_POSTER_HEIGHT: i32 = 300;

/// Callback de selección compartido.
type SelectCallback = Rc<dyn Fn(i64)>;
type SharedSelectCallback = Rc<std::cell::RefCell<Option<SelectCallback>>>;

/// Componente que muestra una galería de medios con pósters, virtualizada.
pub struct MediaList {
    content: Box,
    store: gio::ListStore,
    /// Medios completos, indexados por la misma posición que el `store`.
    media: Rc<std::cell::RefCell<Vec<Media>>>,
    on_media_select: SharedSelectCallback,
    scrolled: ScrolledWindow,
}

impl MediaList {
    pub fn new(translations: TranslationsRef) -> Self {
        crate::ui::poster::install_poster_css();

        let content = Box::new(Orientation::Vertical, 0);
        content.set_vexpand(true);
        content.set_hexpand(true);

        // Modelo de datos: cada fila es un StringObject con el título.
        // Los Media completos se guardan en paralelo en `media`.
        let store = gio::ListStore::new::<StringObject>();

        // Factory que crea y vincula los widgets de cada fila.
        let factory = SignalListItemFactory::new();
        let factory_for_view = factory.clone();

        let translations_clone = translations.clone();
        let media_ref = Rc::new(std::cell::RefCell::new(Vec::new()));
        let on_select_ref = Rc::new(std::cell::RefCell::new(None as Option<SelectCallback>));
        factory.connect_bind({
            let media_for_bind = Rc::clone(&media_ref);
            let translations_for_bind = translations_clone.clone();
            move |_, list_item| {
                let list_item: &gtk4::ListItem = list_item
                    .downcast_ref::<gtk4::ListItem>()
                    .expect("el ítem debe ser un ListItem");
                let position = list_item.position() as usize;
                let media = media_for_bind.borrow();
                let item = media.get(position).expect("medio en posición");

                let child = list_item
                    .child()
                    .expect("el ListItem debe tener un child")
                    .downcast::<Box>()
                    .expect("el child debe ser un Box");

                bind_card(&child, item, &translations_for_bind);
            }
        });

        // Selección: al hacer click en un ítem se dispara esta señal.
        let selection = SingleSelection::new(Some(store.clone()));
        let on_select_sel = Rc::clone(&on_select_ref);
        let media_for_sel = Rc::clone(&media_ref);
        selection.connect_selection_changed(move |_selection, position, _n_items| {
            if position != gtk4::INVALID_LIST_POSITION
                && let Some(item) = media_for_sel.borrow().get(position as usize)
                && let Some(cb) = on_select_sel.borrow().as_ref()
            {
                cb(item.id);
            }
        });

        let list_view = ListView::new(Some(selection.clone()), Some(factory_for_view));
        list_view.set_hexpand(true);
        list_view.set_vexpand(true);

        let scrolled = ScrolledWindow::new();
        scrolled.set_hexpand(true);
        scrolled.set_vexpand(true);
        scrolled.set_hscrollbar_policy(gtk4::PolicyType::Never);
        scrolled.set_vscrollbar_policy(gtk4::PolicyType::Automatic);
        scrolled.set_child(Some(&list_view));

        content.append(&scrolled);

        Self {
            content,
            store,
            media: media_ref,
            on_media_select: on_select_ref,
            scrolled,
        }
    }

    /// Devuelve el `ScrolledWindow` que contiene la galería.
    pub fn scrolled_window(&self) -> &ScrolledWindow {
        &self.scrolled
    }

    pub fn widget(&self) -> &Box {
        &self.content
    }

    /// Reemplaza el contenido de la galería con la lista de medios dada.
    pub fn set_media(&self, media: Vec<Media>) {
        self.store.remove_all();
        *self.media.borrow_mut() = media.clone();
        for m in &media {
            let obj = StringObject::new(&m.title);
            self.store.append(&obj);
        }
    }

    /// Registra el callback invocado cuando el usuario selecciona un medio.
    pub fn on_select<F>(&mut self, f: F)
    where
        F: Fn(i64) + 'static,
    {
        *self.on_media_select.borrow_mut() = Some(Rc::new(f));
    }
}

/// Rellena una tarjeta ya creada con los datos de un `Media`.
fn bind_card(card: &Box, media: &Media, translations: &TranslationsRef) {
    // Limpiar hijos existentes para reutilizar el widget sin fugas.
    while let Some(child) = card.first_child() {
        card.remove(&child);
    }

    // Póster.
    let poster_stack = Box::new(Orientation::Vertical, 0);
    poster_stack.set_halign(gtk4::Align::Start);
    poster_stack.set_valign(gtk4::Align::Start);
    poster_stack.set_size_request(CARD_POSTER_WIDTH, CARD_POSTER_HEIGHT);

    let pixbuf = media
        .poster_path
        .as_deref()
        .and_then(cached_path)
        .and_then(|cache_path| {
            GtkPixbuf::from_file_at_scale(&cache_path, CARD_POSTER_WIDTH, CARD_POSTER_HEIGHT, true)
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

    // Etiqueta de tipo.
    let type_label_text = match media.r#type.as_str() {
        "anime" => translations.borrow().t("media.type.anime"),
        "series" => translations.borrow().t("media.type.series"),
        "movie" => translations.borrow().t("media.type.movie"),
        other => other.to_string(),
    };
    let type_label = Label::new(Some(&type_label_text));
    type_label.set_halign(gtk4::Align::Start);
    type_label.add_css_class("card-type-label");
    card.append(&type_label);

    // Título.
    let title_label = Label::new(Some(&media.title));
    title_label.set_wrap(true);
    title_label.set_max_width_chars(20);
    title_label.set_justify(gtk4::Justification::Left);
    title_label.set_halign(gtk4::Align::Start);
    title_label.set_xalign(0.0);
    title_label.set_lines(2);
    title_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    title_label.add_css_class("poster-title");
    card.append(&title_label);

    // Año.
    if let Some(year) = media.year {
        let year_label = Label::new(Some(&format!("{year}")));
        year_label.add_css_class("poster-year");
        year_label.set_halign(gtk4::Align::Start);
        card.append(&year_label);
    }
}

#[cfg(test)]
mod tests {}
