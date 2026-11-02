//! Diálogo para elegir el póster y el banner de un medio.
//!
//! Muestra las opciones disponibles (obtenidas de la API de metadatos) y
//! permite al usuario seleccionar una imagen diferente o eliminar la actual.
//!
//! Las miniaturas se muestran inmediatamente con un marcador neutro. Las
//! imágenes que no están en caché se descargan en segundo plano; cada miniatura
//! se actualiza individualmente cuando su descarga termina, sin bloquear la UI.
//!
//! `gtk4::Dialog` está deprecado desde GTK 4.10, pero se mantiene por
//! compatibilidad con la versión de GTK4 del proyecto.
#![allow(deprecated)]

use crate::core::metadata::ImageOption;
use crate::core::poster_service::{PosterService, cached_path};
use crate::ui::poster::poster_picture;
use gtk4::gdk_pixbuf::{InterpType, Pixbuf as GtkPixbuf};
use gtk4::prelude::*;
use gtk4::{Align, Box, Button, Dialog, FlowBox, Image, Label, Orientation, Window};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::mpsc;

/// Tamaño de las miniaturas en el selector.
const THUMB_SIZE: i32 = 120;

/// Cambio que el usuario quiere aplicar a una imagen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageChoice {
    /// Mantener la imagen actual sin cambios.
    Keep,
    /// Usar la imagen indicada.
    Set(String),
    /// Eliminar la imagen actual.
    Remove,
}

/// Resultado de la selección del usuario.
pub struct SelectorResult {
    /// Cambio elegido para el póster.
    pub poster: ImageChoice,
    /// Cambio elegido para el banner.
    pub banner: ImageChoice,
}

/// Entrada de una miniatura registrada para actualizarla cuando termine su
/// descarga y para eliminar duplicados visuales.
#[derive(Clone)]
struct ThumbEntry {
    button: Button,
    flow: FlowBox,
    provider: String,
    seen_fingerprints: Rc<RefCell<HashSet<u64>>>,
}

/// Diálogo modal para elegir póster y banner.
pub struct PosterSelector {
    dialog: Dialog,
    /// Cambio elegido para el póster.
    poster_choice: Rc<RefCell<ImageChoice>>,
    /// Cambio elegido para el banner.
    banner_choice: Rc<RefCell<ImageChoice>>,
}

impl PosterSelector {
    /// Crea el diálogo con las opciones de póster y banner disponibles.
    ///
    /// Las miniaturas se muestran inmediatamente con un marcador neutro. Las
    /// imágenes que no están en caché se descargan en un hilo secundario; al
    /// terminar cada descarga, se actualiza la miniatura correspondiente desde
    /// el hilo principal de GTK mediante `glib::timeout_add_local`.
    ///
    /// - `poster_options`: opciones de póster desde la API (puede estar vacío).
    /// - `banner_options`: opciones de banner desde la API (puede estar vacío).
    pub fn new(
        parent: &impl IsA<Window>,
        poster_options: &[ImageOption],
        banner_options: &[ImageOption],
    ) -> Self {
        let dialog = Dialog::new();
        dialog.set_title(Some("Elegir póster y banner"));
        dialog.set_modal(true);
        dialog.set_transient_for(Some(parent.upcast_ref::<Window>()));
        // Tamaño compacto: suficiente para 3-4 miniaturas por fila.
        dialog.set_default_size(720, 520);
        dialog.set_resizable(true);

        let poster_choice = Rc::new(RefCell::new(ImageChoice::Keep));
        let banner_choice = Rc::new(RefCell::new(ImageChoice::Keep));
        let url_to_entries: Rc<RefCell<HashMap<String, Vec<ThumbEntry>>>> =
            Rc::new(RefCell::new(HashMap::new()));
        let poster_fingerprints: Rc<RefCell<HashSet<u64>>> = Rc::new(RefCell::new(HashSet::new()));
        let banner_fingerprints: Rc<RefCell<HashSet<u64>>> = Rc::new(RefCell::new(HashSet::new()));
        let content = dialog.content_area();
        content.set_orientation(Orientation::Vertical);
        content.set_spacing(8);
        content.set_margin_top(8);
        content.set_margin_bottom(8);
        content.set_margin_start(8);
        content.set_margin_end(8);

        // ---------- Sección: Póster ----------
        let poster_section = section_box("Póster");
        let poster_flow = thumbnail_flow();
        poster_section.append(&poster_flow);

        // Opción "Sin póster" (eliminar).
        let btn_remove_poster = thumbnail_button_remove("Sin póster");
        {
            let choice = poster_choice.clone();
            btn_remove_poster.connect_clicked(move |_| {
                *choice.borrow_mut() = ImageChoice::Remove;
            });
        }
        poster_flow.append(&btn_remove_poster);

        // Opciones de póster desde la API.
        for opt in poster_options {
            let pixbuf = load_thumbnail(&opt.url);
            if let Some(fingerprint) = pixbuf.as_ref().and_then(image_fingerprint)
                && is_duplicate_fingerprint(&mut poster_fingerprints.borrow_mut(), fingerprint)
            {
                continue;
            }

            let btn = thumbnail_button(opt, pixbuf.as_ref());
            // Registrar la URL para poder actualizar el botón después y eliminar
            // duplicados visuales que lleguen desde otra fuente.
            url_to_entries
                .borrow_mut()
                .entry(opt.url.clone())
                .or_default()
                .push(ThumbEntry {
                    button: btn.clone(),
                    flow: poster_flow.clone(),
                    provider: opt.provider.clone(),
                    seen_fingerprints: poster_fingerprints.clone(),
                });

            {
                let url = opt.url.clone();
                let choice = poster_choice.clone();
                btn.connect_clicked(move |_| {
                    *choice.borrow_mut() = ImageChoice::Set(url.clone());
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
            let choice = banner_choice.clone();
            btn_remove_banner.connect_clicked(move |_| {
                *choice.borrow_mut() = ImageChoice::Remove;
            });
        }
        banner_flow.append(&btn_remove_banner);

        // Opciones de banner desde la API.
        for opt in banner_options {
            let pixbuf = load_thumbnail(&opt.url);
            if let Some(fingerprint) = pixbuf.as_ref().and_then(image_fingerprint)
                && is_duplicate_fingerprint(&mut banner_fingerprints.borrow_mut(), fingerprint)
            {
                continue;
            }

            let btn = thumbnail_button(opt, pixbuf.as_ref());
            url_to_entries
                .borrow_mut()
                .entry(opt.url.clone())
                .or_default()
                .push(ThumbEntry {
                    button: btn.clone(),
                    flow: banner_flow.clone(),
                    provider: opt.provider.clone(),
                    seen_fingerprints: banner_fingerprints.clone(),
                });

            {
                let url = opt.url.clone();
                let choice = banner_choice.clone();
                btn.connect_clicked(move |_| {
                    *choice.borrow_mut() = ImageChoice::Set(url.clone());
                });
            }
            banner_flow.append(&btn);
        }

        content.append(&banner_section);

        // ---------- Botones de acción ----------
        dialog.add_button("Cancelar", gtk4::ResponseType::Cancel);
        dialog.add_button("Aplicar", gtk4::ResponseType::Apply);

        // ---------- Descarga en segundo plano ----------
        // Recopilar URLs que faltan en caché (sin repetir la misma URL).
        let mut missing_urls: Vec<String> = poster_options
            .iter()
            .chain(banner_options.iter())
            .filter(|o| cached_path(&o.url).is_none())
            .map(|o| o.url.clone())
            .collect();
        missing_urls.sort_unstable();
        missing_urls.dedup();

        if !missing_urls.is_empty() {
            let (tx, rx) = mpsc::channel::<String>();
            let service = PosterService::new();
            let urls_for_thread = missing_urls.clone();

            std::thread::spawn(move || {
                let rt = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(_) => return,
                };
                for url in urls_for_thread {
                    let _ = rt.block_on(service.download_async(&url));
                    let _ = tx.send(url);
                }
            });

            // Revisar el canal desde el hilo principal para actualizar miniaturas.
            let url_to_entries_for_idle = url_to_entries.clone();
            glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
                match rx.try_recv() {
                    Ok(url) => {
                        let entries = url_to_entries_for_idle
                            .borrow()
                            .get(&url)
                            .cloned()
                            .unwrap_or_default();
                        if let Some(cache_path) = cached_path(&url)
                            && let Ok(pixbuf) = GtkPixbuf::from_file_at_scale(
                                &cache_path,
                                THUMB_SIZE,
                                THUMB_SIZE,
                                true,
                            )
                        {
                            let fingerprint = image_fingerprint(&pixbuf);
                            for entry in entries {
                                if let Some(fingerprint) = fingerprint
                                    && is_duplicate_fingerprint(
                                        &mut entry.seen_fingerprints.borrow_mut(),
                                        fingerprint,
                                    )
                                {
                                    // Misma imagen que otra opción ya visible.
                                    entry.flow.remove(&entry.button);
                                    continue;
                                }
                                set_thumbnail(&entry.button, &pixbuf, &entry.provider, &url);
                            }
                        }
                        glib::ControlFlow::Continue
                    }
                    Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
                }
            });
        }

        Self {
            dialog,
            poster_choice,
            banner_choice,
        }
    }

    /// Ejecuta el diálogo de forma modal y llama al `callback` con el resultado.
    /// El diálogo se destruye automáticamente al cerrarse.
    pub fn run<F>(self, callback: F)
    where
        F: Fn(SelectorResult) + 'static,
    {
        let dialog = self.dialog;
        let poster_choice = self.poster_choice;
        let banner_choice = self.banner_choice;

        dialog.connect_response(move |dlg, response| {
            let result = match response {
                gtk4::ResponseType::Apply => SelectorResult {
                    poster: poster_choice.borrow().clone(),
                    banner: banner_choice.borrow().clone(),
                },
                _ => {
                    // Cancelar o cerrar: no hacer cambios.
                    SelectorResult {
                        poster: ImageChoice::Keep,
                        banner: ImageChoice::Keep,
                    }
                }
            };
            callback(result);
            dlg.close();
        });

        dialog.present();
    }
}

/// Crea un contenedor de sección con título.
fn section_box(title: &str) -> Box {
    let section = Box::new(Orientation::Vertical, 6);
    section.set_hexpand(false);
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
    flow.set_hexpand(false);
    flow.set_vexpand(false);
    flow.set_max_children_per_line(4);
    flow.set_min_children_per_line(1);
    flow.set_row_spacing(8);
    flow.set_column_spacing(8);
    flow.set_homogeneous(false);
    flow.set_selection_mode(gtk4::SelectionMode::None);
    flow.set_margin_top(4);
    flow
}

/// Carga la miniatura de una URL si ya está en caché.
fn load_thumbnail(url: &str) -> Option<GtkPixbuf> {
    let cache_path = cached_path(url)?;
    GtkPixbuf::from_file_at_scale(&cache_path, THUMB_SIZE, THUMB_SIZE, true).ok()
}

/// Crea un botón con la miniatura de una opción de imagen. Si `pixbuf` es
/// `None`, se muestra un marcador neutro hasta que termine la descarga.
fn thumbnail_button(opt: &ImageOption, pixbuf: Option<&GtkPixbuf>) -> Button {
    let btn = Button::new();
    btn.set_hexpand(false);
    btn.set_vexpand(false);
    btn.set_size_request(THUMB_SIZE + 20, THUMB_SIZE + 60);
    btn.add_css_class("thumbnail-btn");
    btn.set_tooltip_text(Some(&opt.url));

    let box_ = Box::new(Orientation::Vertical, 4);
    box_.set_halign(Align::Center);
    box_.set_valign(Align::Center);

    if let Some(pixbuf) = pixbuf {
        let picture = poster_picture(pixbuf, THUMB_SIZE, THUMB_SIZE);
        picture.set_hexpand(true);
        picture.set_vexpand(true);
        box_.append(&picture);
    } else {
        placeholder_thumbnail(&box_);
    }

    let lbl = Label::new(Some(&opt.provider));
    lbl.set_halign(Align::Center);
    lbl.set_xalign(0.5);
    lbl.set_wrap(true);
    lbl.set_max_width_chars(10);
    lbl.add_css_class("thumbnail-label");
    box_.append(&lbl);

    btn.set_child(Some(&box_));
    btn
}

/// Sustituye el marcador neutro de un botón por la imagen descargada.
fn set_thumbnail(btn: &Button, pixbuf: &GtkPixbuf, provider: &str, url: &str) {
    let box_ = Box::new(Orientation::Vertical, 4);
    box_.set_halign(Align::Center);
    box_.set_valign(Align::Center);

    let picture = poster_picture(pixbuf, THUMB_SIZE, THUMB_SIZE);
    box_.append(&picture);

    let lbl = Label::new(Some(provider));
    lbl.set_halign(Align::Center);
    lbl.set_xalign(0.5);
    lbl.set_wrap(true);
    lbl.set_max_width_chars(10);
    lbl.add_css_class("thumbnail-label");
    box_.append(&lbl);

    btn.set_child(Some(&box_));
    btn.set_tooltip_text(Some(url));
}

/// Calcula una huella visual (dHash de 64 bits) para detectar la misma imagen
/// aunque provenga de proveedores distintos o con otra resolución.
fn image_fingerprint(pixbuf: &GtkPixbuf) -> Option<u64> {
    const HASH_COLUMNS: usize = 9;
    const HASH_ROWS: usize = 8;

    let scaled =
        pixbuf.scale_simple(HASH_COLUMNS as i32, HASH_ROWS as i32, InterpType::Bilinear)?;
    let bytes = scaled.read_pixel_bytes();
    let data = bytes.as_ref();
    let channels = scaled.n_channels() as usize;
    let stride = scaled.rowstride() as usize;
    if channels < 3 {
        return None;
    }

    let mut luminance = [[0u32; HASH_COLUMNS]; HASH_ROWS];
    for (y, row) in luminance.iter_mut().enumerate() {
        for (x, value) in row.iter_mut().enumerate() {
            let offset = y * stride + x * channels;
            if offset + 2 >= data.len() {
                return None;
            }
            let r = u32::from(data[offset]);
            let g = u32::from(data[offset + 1]);
            let b = u32::from(data[offset + 2]);
            *value = (r * 299 + g * 587 + b * 114) / 1000;
        }
    }

    let mut hash = 0u64;
    let mut bit = 0;
    for row in &luminance {
        for x in 0..(HASH_COLUMNS - 1) {
            if row[x] > row[x + 1] {
                hash |= 1u64 << bit;
            }
            bit += 1;
        }
    }

    Some(hash)
}

/// Indica si la huella se parece a otra ya vista (misma imagen con pequeñas
/// diferencias de compresión, recorte o tamaño).
///
/// Los proveedores sirven la misma carátula con recompresiones y recortes
/// distintos, así que se toleran hasta 5 bits de diferencia sobre 64. Un par
/// de pósteres realmente diferentes queda muy por encima de ese umbral.
fn is_duplicate_fingerprint(seen: &mut HashSet<u64>, fingerprint: u64) -> bool {
    const MAX_DISTANCE: u32 = 5;

    if seen
        .iter()
        .any(|existing| (existing ^ fingerprint).count_ones() <= MAX_DISTANCE)
    {
        return true;
    }
    seen.insert(fingerprint);
    false
}

/// Crea un botón para la opción de eliminar (papelera).
fn thumbnail_button_remove(text: &str) -> Button {
    let btn = Button::new();
    btn.set_hexpand(false);
    btn.set_vexpand(false);
    btn.set_size_request(THUMB_SIZE + 20, THUMB_SIZE + 60);
    btn.add_css_class("thumbnail-btn");

    let box_ = Box::new(Orientation::Vertical, 4);
    box_.set_halign(Align::Center);
    box_.set_valign(Align::Center);

    let icon = Image::from_icon_name("user-trash-symbolic");
    icon.set_pixel_size(40);
    icon.set_halign(Align::Center);
    box_.append(&icon);

    let lbl = Label::new(Some(text));
    lbl.set_halign(Align::Center);
    lbl.set_xalign(0.5);
    lbl.set_wrap(true);
    lbl.set_max_width_chars(10);
    lbl.add_css_class("thumbnail-label");
    box_.append(&lbl);

    btn.set_child(Some(&box_));
    btn
}

/// Añade un placeholder neutro (sin icono de interrogación) cuando no se
/// puede mostrar la imagen. Muestra un rectángulo con borde para que se
/// vea claramente el espacio de la miniatura.
fn placeholder_thumbnail(container: &Box) {
    let rect = Box::new(Orientation::Vertical, 0);
    rect.set_size_request(THUMB_SIZE, THUMB_SIZE);
    rect.set_halign(Align::Center);
    rect.set_valign(Align::Center);
    rect.add_css_class("thumbnail-placeholder");
    container.append(&rect);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_detecta_duplicados_cercanos() {
        let mut seen = HashSet::new();

        assert!(!is_duplicate_fingerprint(&mut seen, 0b1010));
        assert!(is_duplicate_fingerprint(&mut seen, 0b1010));
        assert!(is_duplicate_fingerprint(&mut seen, 0b1011));
        assert!(!is_duplicate_fingerprint(&mut seen, u64::MAX));
    }

    #[test]
    fn fingerprint_tolera_recompresiones_de_hasta_cinco_bits() {
        let mut seen = HashSet::new();
        seen.insert(0u64);

        assert!(
            !is_duplicate_fingerprint(&mut seen, 0b111111),
            "seis bits de diferencia siguen siendo otra imagen"
        );
        assert!(
            is_duplicate_fingerprint(&mut seen, 0b11111),
            "cinco bits de diferencia se consideran la misma imagen"
        );
    }
}
