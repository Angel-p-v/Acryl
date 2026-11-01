//! Galería de medios con tarjetas al estilo Crunchyroll.
//!
//! Cada tarjeta muestra:
//! - Póster (relación 2:3) con etiqueta de tipo.
//! - Título y año debajo.
//! - Efecto hover con elevación.
//! - Todo el área es clickeable para abrir los detalles.
//!
//! La galería usa un `GtkGrid` con columnas de tamaño uniforme y alineación al
//! inicio. El número de columnas se recalcula al cambiar el ancho disponible:
//! las tarjetas fluyen de izquierda a derecha, saltan de fila automáticamente
//! y mantienen un espaciado constante, sin centrarse individualmente ni
//! estirarse para rellenar el ancho restante.

use crate::core::poster_service::cached_path;
use crate::db::media_repo::Media;
use crate::ui::i18n::TranslationsRef;
use crate::ui::poster::{placeholder_widget, poster_picture};
use gtk4::gdk_pixbuf::Pixbuf as GtkPixbuf;
use gtk4::prelude::*;
use gtk4::{Box, GestureClick, Grid, Label, Orientation, ScrolledWindow, glib};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Tamaño del póster en la galería (relación 2:3).
const CARD_POSTER_WIDTH: i32 = 200;
const CARD_POSTER_HEIGHT: i32 = 300;

/// Ancho y alto de la tarjeta completa (póster + padding + textos). Todas las
/// tarjetas comparten dimensiones para que la rejilla quede regular.
const CARD_WIDTH: i32 = CARD_POSTER_WIDTH + 16;
const CARD_HEIGHT: i32 = 404;

/// Espaciado entre tarjetas (horizontal y vertical), en píxeles.
const CARD_SPACING: i32 = 16;

/// Margen exterior de la galería respecto al área de contenido.
const GALLERY_MARGIN: i32 = 24;

/// Límites de columnas por fila.
const MIN_COLUMNS: u32 = 1;
const MAX_COLUMNS: u32 = 8;

/// Callback de selección compartido por todas las tarjetas.
type SelectCallback = Rc<dyn Fn(i64)>;
type SharedSelectCallback = Rc<RefCell<Option<SelectCallback>>>;

/// Tarjeta individual de un medio (póster + tipo + título + año).
struct MediaCard {
    widget: Box,
}

impl MediaCard {
    fn new(media: &Media, on_select: Rc<dyn Fn(i64)>, translations: TranslationsRef) -> Self {
        let card = Box::new(Orientation::Vertical, 6);
        card.add_css_class("media-card");
        card.set_halign(gtk4::Align::Start);
        card.set_valign(gtk4::Align::Start);
        card.set_size_request(CARD_WIDTH, CARD_HEIGHT);

        // Póster con proporción fija 2:3.
        let poster_stack = Box::new(Orientation::Vertical, 0);
        poster_stack.set_halign(gtk4::Align::Start);
        poster_stack.set_valign(gtk4::Align::Start);
        poster_stack.set_size_request(CARD_POSTER_WIDTH, CARD_POSTER_HEIGHT);

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

        // Etiqueta de tipo (Anime / Serie / Película).
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

        // Título, limitado a dos líneas para no deformar la tarjeta.
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
            let year_label = Label::new(Some(&format!("{}", year)));
            year_label.add_css_class("poster-year");
            year_label.set_halign(gtk4::Align::Start);
            card.append(&year_label);
        }

        // Toda la tarjeta es clickeable.
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

/// Calcula cuántas tarjetas caben en una fila para el ancho disponible.
///
/// Se descuentan los márgenes exteriores de la galería y se divide por el
/// ancho de tarjeta más el espaciado. El resultado se limita al rango
/// soportado para evitar rejillas inutilizables en ventanas muy estrechas o
/// demasiado anchas.
fn columns_for_width(available_width: i32) -> u32 {
    let usable_width = available_width
        .saturating_sub(2 * GALLERY_MARGIN)
        .max(CARD_WIDTH);
    let fit = (usable_width + CARD_SPACING) / (CARD_WIDTH + CARD_SPACING);
    fit.clamp(MIN_COLUMNS as i32, MAX_COLUMNS as i32) as u32
}

/// Recoloca las tarjetas en la rejilla con el número de columnas indicado.
fn layout_cards(grid: &Grid, cards: &[Box], columns: u32) {
    while let Some(child) = grid.first_child() {
        grid.remove(&child);
    }

    let columns = columns.max(1);
    for (index, card) in cards.iter().enumerate() {
        let index = index as u32;
        grid.attach(
            card,
            (index % columns) as i32,
            (index / columns) as i32,
            1,
            1,
        );
    }
}

/// Componente que muestra una galería de medios con pósters.
pub struct MediaList {
    content: Box,
    grid: Grid,
    media: Rc<RefCell<Vec<Media>>>,
    cards: Rc<RefCell<Vec<Box>>>,
    on_media_select: SharedSelectCallback,
    translations: TranslationsRef,
    columns: Rc<Cell<u32>>,
}

impl MediaList {
    pub fn new(translations: TranslationsRef) -> Self {
        crate::ui::poster::install_poster_css();

        let content = Box::new(Orientation::Vertical, 0);
        content.set_vexpand(true);
        content.set_hexpand(true);

        // Rejilla de columnas uniformes, alineada al inicio. `halign(Start)`
        // evita que la rejilla se centre o reparta el espacio sobrante.
        let grid = Grid::new();
        grid.set_column_spacing(CARD_SPACING as u32);
        grid.set_row_spacing(CARD_SPACING as u32);
        grid.set_column_homogeneous(true);
        grid.set_row_homogeneous(false);
        grid.set_halign(gtk4::Align::Start);
        grid.set_valign(gtk4::Align::Start);
        grid.set_margin_top(GALLERY_MARGIN);
        grid.set_margin_bottom(GALLERY_MARGIN);
        grid.set_margin_start(GALLERY_MARGIN);
        grid.set_margin_end(GALLERY_MARGIN);

        let scrolled = ScrolledWindow::new();
        scrolled.set_hexpand(true);
        scrolled.set_vexpand(true);
        scrolled.set_hscrollbar_policy(gtk4::PolicyType::Never);
        scrolled.set_vscrollbar_policy(gtk4::PolicyType::Automatic);
        scrolled.set_child(Some(&grid));

        content.append(&scrolled);

        let columns = Rc::new(Cell::new(columns_for_width(900)));
        let cards: Rc<RefCell<Vec<Box>>> = Rc::new(RefCell::new(Vec::new()));

        // GTK4 no expone `size-allocate`; un tick callback compara el ancho
        // asignado y solo reconstruye la rejilla cuando cambia el número de
        // columnas, no en cada fotograma.
        {
            let grid_tick = grid.clone();
            let cards_tick = cards.clone();
            let columns_tick = columns.clone();
            scrolled.add_tick_callback(move |scrolled, _| {
                let width = scrolled.width();
                if width > 0 {
                    let new_columns = columns_for_width(width);
                    if new_columns != columns_tick.get() {
                        columns_tick.set(new_columns);
                        layout_cards(&grid_tick, &cards_tick.borrow(), new_columns);
                    }
                }
                glib::ControlFlow::Continue
            });
        }

        Self {
            content,
            grid,
            media: Rc::new(RefCell::new(Vec::new())),
            cards,
            on_media_select: Rc::new(RefCell::new(None)),
            translations,
            columns,
        }
    }

    pub fn widget(&self) -> &Box {
        &self.content
    }

    pub fn set_media(&self, media: Vec<Media>) {
        *self.media.borrow_mut() = media;
        self.refresh();
    }

    pub fn refresh(&self) {
        let mut cards = self.cards.borrow_mut();
        cards.clear();

        if let Some(on_select) = self.on_media_select.borrow().clone() {
            let media = self.media.borrow();
            cards.reserve(media.len());
            for item in media.iter() {
                let card = MediaCard::new(item, on_select.clone(), self.translations.clone());
                cards.push(card.widget);
            }
        }

        layout_cards(&self.grid, &cards, self.columns.get());
    }

    pub fn on_select<F>(&mut self, f: F)
    where
        F: Fn(i64) + 'static,
    {
        *self.on_media_select.borrow_mut() = Some(Rc::new(f));
        self.refresh();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calcula_columnas_segun_el_ancho() {
        // 1200 - 48 = 1152 usables; (1152 + 16) / 232 = 5.
        assert_eq!(columns_for_width(1200), 5);
        // 800 - 48 = 752; (752 + 16) / 232 = 3.
        assert_eq!(columns_for_width(800), 3);
        // 400 - 48 = 352; (352 + 16) / 232 = 1.
        assert_eq!(columns_for_width(400), 1);
    }

    #[test]
    fn columnas_respetan_los_limites() {
        assert_eq!(columns_for_width(4000), MAX_COLUMNS);
        assert_eq!(columns_for_width(0), MIN_COLUMNS);
    }
}
