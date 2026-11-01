//! Diálogo para elegir el póster y el banner de un medio.
//!
//! Muestra las opciones disponibles (obtenidas de la API de metadatos) y
//! permite al usuario seleccionar una imagen diferente o eliminar la actual.
//!
//! `gtk4::Dialog` está deprecado desde GTK 4.10, pero se mantiene por
//! compatibilidad con la versión de GTK4 del proyecto.
#![allow(deprecated)]

use crate::core::metadata::ImageOption;
use crate::core::poster_service::{PosterService, cached_path};
use crate::ui::poster::poster_picture;
use gtk4::gdk_pixbuf::Pixbuf as GtkPixbuf;
use gtk4::prelude::*;
use gtk4::{Align, Box, Button, Dialog, FlowBox, Image, Label, Orientation, Window};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Tamaño de las miniaturas en el selector.
const THUMB_SIZE: i32 = 120;

/// Resultado de la selección del usuario.
pub struct SelectorResult {
    /// URL elegida para el póster, o `None` si se quiere eliminar el actual.
    pub poster_path: Option<String>,
    /// URL elegida para el banner, o `None` si se quiere eliminar el actual.
    pub banner_path: Option<String>,
}

/// Diálogo modal para elegir póster y banner.
pub struct PosterSelector {
    dialog: Dialog,
    /// Opción de póster actualmente seleccionada por el usuario.
    selected_poster: Rc<RefCell<Option<ImageOption>>>,
    /// Opción de banner actualmente seleccionada por el usuario.
    selected_banner: Rc<RefCell<Option<ImageOption>>>,
    /// Indica si el usuario eligió eliminar el póster (marca "Sin póster").
    remove_poster: Rc<Cell<bool>>,
    /// Indica si el usuario eligió eliminar el banner (marca "Sin banner").
    remove_banner: Rc<Cell<bool>>,
}

impl PosterSelector {
    /// Crea el diálogo con las opciones de póster y banner disponibles.
    ///
    /// Antes de construir las miniaturas, intenta descargar las imágenes que
    /// aún no están en caché para que el usuario vea una vista previa real en
    /// lugar de un marcador genérico.
    ///
    /// - `poster_options`: opciones de póster desde la API (puede estar vacío).
    /// - `banner_options`: opciones de banner desde la API (puede estar vacío).
    /// - `current_poster`: URL del póster actualmente asignado (si existe).
    /// - `current_banner`: URL del banner actualmente asignado (si existe).
    pub fn new(
        parent: &impl IsA<Window>,
        poster_options: &[ImageOption],
        banner_options: &[ImageOption],
        current_poster: Option<&str>,
        current_banner: Option<&str>,
    ) -> Self {
        let dialog = Dialog::new();
        dialog.set_title(Some("Elegir póster y banner"));
        dialog.set_modal(true);
        dialog.set_transient_for(Some(parent.upcast_ref::<Window>()));
        dialog.set_default_size(700, 500);

        let selected_poster = Rc::new(RefCell::new(None));
        let selected_banner = Rc::new(RefCell::new(None));
        let remove_poster = Rc::new(Cell::new(false));
        let remove_banner = Rc::new(Cell::new(false));

        // ---------- Descargar imágenes faltantes para poder mostrarlas ----------
        // Se descargan todas las opciones que no están en caché para que el
        // usuario vea la vista previa real de cada una al abrir el diálogo.
        let runtime = tokio::runtime::Runtime::new().ok();

        if let Some(rt) = &runtime {
            let service = PosterService::new();
            let all_urls: Vec<String> = poster_options
                .iter()
                .chain(banner_options.iter())
                .map(|o| o.url.clone())
                .collect();
            for url in all_urls {
                if cached_path(&url).is_none() {
                    let _ = rt.block_on(service.download_async(&url));
                }
            }
        }

        let content = dialog.content_area();
        content.set_orientation(Orientation::Vertical);
        content.set_spacing(12);
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(12);
        content.set_margin_end(12);

        // ---------- Sección: Póster ----------
        let poster_section = section_box("Póster");
        let poster_flow = thumbnail_flow();
        poster_section.append(&poster_flow);

        // Opción "Sin póster" (eliminar).
        let btn_remove_poster = thumbnail_button_remove("Sin póster");
        {
            let selected = selected_poster.clone();
            let remove = remove_poster.clone();
            btn_remove_poster.connect_clicked(move |_| {
                *selected.borrow_mut() = None;
                remove.set(true);
            });
        }
        poster_flow.append(&btn_remove_poster);

        // Opciones de póster desde la API.
        for opt in poster_options {
            let btn = thumbnail_button_image(opt);
            {
                let url = opt.url.clone();
                let selected = selected_poster.clone();
                let remove = remove_poster.clone();
                btn.connect_clicked(move |_| {
                    *selected.borrow_mut() = Some(ImageOption {
                        image_type: "poster".to_string(),
                        resolution: "original".to_string(),
                        url: url.clone(),
                    });
                    remove.set(false);
                });
            }
            // Si coincide con el póster actual, marcar como seleccionado por defecto.
            if let Some(cur) = current_poster
                && cur == opt.url
            {
                *selected_poster.borrow_mut() = Some(ImageOption {
                    image_type: "poster".to_string(),
                    resolution: "original".to_string(),
                    url: opt.url.clone(),
                });
            }
            poster_flow.append(&btn);
        }

        content.append(&poster_section);

        // ---------- Sección: Banner ----------
        let banner_section = section_box("Banner");
        let banner_flow = thumbnail_flow();
        banner_section.append(&banner_flow);

        // Opción "Sin banner" (eliminar).
        let btn_remove_banner = thumbnail_button_remove("Sin banner");
        {
            let selected = selected_banner.clone();
            let remove = remove_banner.clone();
            btn_remove_banner.connect_clicked(move |_| {
                *selected.borrow_mut() = None;
                remove.set(true);
            });
        }
        banner_flow.append(&btn_remove_banner);

        // Opciones de banner desde la API.
        for opt in banner_options {
            let btn = thumbnail_button_image(opt);
            {
                let url = opt.url.clone();
                let selected = selected_banner.clone();
                let remove = remove_banner.clone();
                btn.connect_clicked(move |_| {
                    *selected.borrow_mut() = Some(ImageOption {
                        image_type: "banner".to_string(),
                        resolution: "original".to_string(),
                        url: url.clone(),
                    });
                    remove.set(false);
                });
            }
            if let Some(cur) = current_banner
                && cur == opt.url
            {
                *selected_banner.borrow_mut() = Some(ImageOption {
                    image_type: "banner".to_string(),
                    resolution: "original".to_string(),
                    url: opt.url.clone(),
                });
            }
            banner_flow.append(&btn);
        }

        content.append(&banner_section);

        // ---------- Botones de acción ----------
        dialog.add_button("Cancelar", gtk4::ResponseType::Cancel);
        dialog.add_button("Aplicar", gtk4::ResponseType::Apply);

        Self {
            dialog,
            selected_poster,
            selected_banner,
            remove_poster,
            remove_banner,
        }
    }

    /// Ejecuta el diálogo de forma modal y llama al `callback` con el resultado.
    /// El diálogo se destruye automáticamente al cerrarse.
    pub fn run<F>(self, callback: F)
    where
        F: Fn(SelectorResult) + 'static,
    {
        let dialog = self.dialog;
        let selected_poster = self.selected_poster;
        let selected_banner = self.selected_banner;
        let remove_poster = self.remove_poster;
        let remove_banner = self.remove_banner;

        dialog.connect_response(move |dlg, response| {
            match response {
                gtk4::ResponseType::Apply => {
                    let poster_path = if remove_poster.get() {
                        None
                    } else {
                        selected_poster.borrow().as_ref().map(|opt| opt.url.clone())
                    };
                    let banner_path = if remove_banner.get() {
                        None
                    } else {
                        selected_banner.borrow().as_ref().map(|opt| opt.url.clone())
                    };
                    callback(SelectorResult {
                        poster_path,
                        banner_path,
                    });
                }
                _ => {
                    // Cancelar o cerrar: no hacer cambios.
                    callback(SelectorResult {
                        poster_path: None,
                        banner_path: None,
                    });
                }
            }
            dlg.close();
        });

        dialog.present();
    }
}

/// Crea un contenedor de sección con título.
fn section_box(title: &str) -> Box {
    let section = Box::new(Orientation::Vertical, 8);
    section.set_hexpand(true);
    section.set_vexpand(false);

    let label = Label::new(Some(title));
    label.set_halign(Align::Start);
    label.set_xalign(0.0);
    label.add_css_class("section-title");
    section.append(&label);

    section
}

/// Crea un FlowBox para organizar las miniaturas en rejilla.
fn thumbnail_flow() -> FlowBox {
    let flow = FlowBox::new();
    flow.add_css_class("thumbnail-flow");
    flow.set_hexpand(true);
    flow.set_vexpand(false);
    flow.set_max_children_per_line(5);
    flow.set_min_children_per_line(1);
    flow.set_row_spacing(10);
    flow.set_column_spacing(10);
    flow.set_homogeneous(true);
    flow.set_selection_mode(gtk4::SelectionMode::None);
    flow.set_margin_top(6);
    flow
}

/// Crea un botón con la miniatura de una opción de imagen.
fn thumbnail_button_image(opt: &ImageOption) -> Button {
    let btn = Button::new();
    btn.set_hexpand(false);
    btn.set_vexpand(false);
    btn.set_size_request(THUMB_SIZE, THUMB_SIZE + 40);
    btn.add_css_class("thumbnail-btn");

    let box_ = Box::new(Orientation::Vertical, 6);
    box_.set_halign(Align::Center);
    box_.set_valign(Align::Center);

    // Mostrar la miniatura descargada o un placeholder neutro.
    if let Some(cache_path) = cached_path(&opt.url) {
        if let Ok(pixbuf) = GtkPixbuf::from_file_at_scale(&cache_path, THUMB_SIZE, THUMB_SIZE, true)
        {
            let picture = poster_picture(&pixbuf, THUMB_SIZE, THUMB_SIZE);
            box_.append(&picture);
        } else {
            placeholder_thumbnail(&box_, &opt.url);
        }
    } else {
        placeholder_thumbnail(&box_, &opt.url);
    }

    let lbl = Label::new(Some(&format!("{}\n{}", opt.image_type, opt.resolution)));
    lbl.set_halign(Align::Center);
    lbl.set_xalign(0.5);
    lbl.set_wrap(true);
    lbl.set_max_width_chars(12);
    lbl.add_css_class("thumbnail-label");
    box_.append(&lbl);

    btn.set_child(Some(&box_));
    btn
}

/// Crea un botón para la opción de eliminar (papelera).
fn thumbnail_button_remove(text: &str) -> Button {
    let btn = Button::new();
    btn.set_hexpand(false);
    btn.set_vexpand(false);
    btn.set_size_request(THUMB_SIZE, THUMB_SIZE + 40);
    btn.add_css_class("thumbnail-btn");

    let box_ = Box::new(Orientation::Vertical, 6);
    box_.set_halign(Align::Center);
    box_.set_valign(Align::Center);

    let icon = Image::from_icon_name("user-trash-symbolic");
    icon.set_pixel_size(48);
    icon.set_halign(Align::Center);
    box_.append(&icon);

    let lbl = Label::new(Some(text));
    lbl.set_halign(Align::Center);
    lbl.set_xalign(0.5);
    lbl.set_wrap(true);
    lbl.set_max_width_chars(12);
    lbl.add_css_class("thumbnail-label");
    box_.append(&lbl);

    btn.set_child(Some(&box_));
    btn
}

/// Añade un placeholder neutro (sin icono de interrogación) cuando no se
/// puede mostrar la imagen. Solo muestra un rectángulo con el texto.
fn placeholder_thumbnail(container: &Box, text: &str) {
    let rect = Box::new(Orientation::Vertical, 0);
    rect.set_size_request(THUMB_SIZE, THUMB_SIZE);
    rect.set_halign(Align::Center);
    rect.set_valign(Align::Center);
    rect.add_css_class("thumbnail-placeholder");
    container.append(&rect);

    let lbl = Label::new(Some(text));
    lbl.set_halign(Align::Center);
    lbl.set_xalign(0.5);
    lbl.set_wrap(true);
    lbl.set_max_width_chars(12);
    lbl.add_css_class("thumbnail-label");
    container.append(&lbl);
}
