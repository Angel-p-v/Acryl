//! Galería de medios virtualizada con GtkGridView.
//!
//! Usa `GtkGridView` + `GListStore` (con `StringObject`) + `SignalListItemFactory`
//! para que solo se rendericen las tarjetas visibles, eliminando las advertencias
//! de GTK al medir `GtkGrid` con miles de ítems y permitiendo scroll fluido con
//! bibliotecas grandes. A diferencia de `GtkListView` (un ítem por fila),
//! `GtkGridView` organiza los ítems en múltiples columnas como una rejilla.

use crate::core::poster_service::{PosterService, cached_path};
use crate::db::media_repo::Media;
use crate::ui::i18n::TranslationsRef;
use crate::ui::poster::{placeholder_widget, poster_picture};
use gtk4::gdk_pixbuf::Pixbuf as GtkPixbuf;
use gtk4::prelude::*;
use gtk4::{
    Box, GridView, Label, Orientation, ScrolledWindow, SignalListItemFactory, SingleSelection,
    StringObject, gio,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

/// Tamaño del póster en la galería (relación 2:3).
const CARD_POSTER_WIDTH: i32 = 200;
const CARD_POSTER_HEIGHT: i32 = 300;

/// Ancho y alto de la tarjeta completa (póster + padding + textos).
const CARD_WIDTH: i32 = CARD_POSTER_WIDTH + 16;
const CARD_HEIGHT: i32 = 404;

/// Límites de columnas por fila.
const MIN_COLUMNS: u32 = 1;
const MAX_COLUMNS: u32 = 8;

/// Callback de selección compartido.
type SelectCallback = Rc<dyn Fn(i64)>;
type SharedSelectCallback = Rc<std::cell::RefCell<Option<SelectCallback>>>;

/// Callback compartido para refresh o scroll (hilo principal).
type ViewCallback = Rc<std::cell::RefCell<Option<Rc<dyn Fn()>>>>;

/// Componente que muestra una galería de medios con pósters, virtualizada.
pub struct MediaList {
    content: Box,
    store: gio::ListStore,
    /// Medios completos, indexados por la misma posición que el `store`.
    media: Rc<RefCell<Vec<Media>>>,
    /// Callback para refrescar la galería cuando una descarga de póster termina.
    refresh_callback: ViewCallback,
    /// Callback para cargar más medios cuando el scroll llega al final.
    on_scroll_bottom: ViewCallback,
    /// Canal para señalar descargas de póster completadas (hilo secundario -> principal).
    download_tx: mpsc::Sender<()>,
    /// Receptor de señales de descarga completada (solo hilo principal).
    download_rx: mpsc::Receiver<()>,
    selection: SingleSelection,
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
        let media_ref = Rc::new(std::cell::RefCell::new(Vec::<Media>::new()));
        let on_select_ref = Rc::new(std::cell::RefCell::new(None as Option<SelectCallback>));
        let on_scroll_bottom_ref = ViewCallback::default();

        // connect_setup crea el widget vacío de cada fila y lo asigna al ListItem.
        factory.connect_setup(move |_, list_item| {
            let card = build_card_widget();
            <glib::Object as glib::object::ObjectExt>::set_property(list_item, "child", &card);
        });

        factory.connect_bind({
            let media_for_bind = Rc::clone(&media_ref);
            let translations_for_bind = translations_clone.clone();
            let on_select_for_bind = Rc::clone(&on_select_ref);
            move |_, list_item| {
                // Obtener el StringObject del modelo a través de la propiedad "item"
                // del GtkListItem, y usarlo para encontrar el Media correspondiente.
                let item_obj: glib::Object =
                    <glib::Object as glib::object::ObjectExt>::property(list_item, "item");
                let title: String =
                    <glib::Object as glib::object::ObjectExt>::property(&item_obj, "string");

                let media = media_for_bind.borrow();
                let item = media
                    .iter()
                    .find(|m| m.title == title)
                    .expect("medio no encontrado en la lista");

                // Obtener el widget child (Box) del ListItem a través de la propiedad "child".
                let child: Option<gtk4::Widget> =
                    <glib::Object as glib::object::ObjectExt>::property(list_item, "child");
                let widget = child.expect("el ListItem debe tener un child");
                let box_child = widget
                    .downcast_ref::<Box>()
                    .expect("el child debe ser un Box");

                bind_card(box_child, item, &translations_for_bind);

                /* Conectar un gestor de clic en la tarjeta para que la navegación a
                 * detalles siempre se dispare al pulsar, incluso si el usuario pulsa
                 * la misma tarjeta que ya estaba seleccionada (en cuyo caso
                 * SingleSelection no emite selection_changed porque la posición no
                 * cambia). */
                let on_select_cb = Rc::clone(&on_select_for_bind);
                let media_id = item.id;
                let click = gtk4::GestureClick::new();
                click.connect_pressed(move |_, _, _, _| {
                    if let Some(cb) = on_select_cb.borrow().as_ref() {
                        cb(media_id);
                    }
                });
                box_child.add_controller(click);
            }
        });

        // Selección: al hacer click en un ítem se dispara esta señal.
        let selection = SingleSelection::new(Some(store.clone()));
        // Deshabilitar autoselect: GTK4 selecciona el primer ítem por defecto,
        // lo que causa que una tarjeta quede resaltada al iniciar la app.
        selection.set_autoselect(false);
        selection.set_selected(gtk4::INVALID_LIST_POSITION);
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

        // GridView organiza los ítems en múltiples columnas (rejilla),
        // a diferencia de ListView que muestra un ítem por fila.
        let grid_view = GridView::new(Some(selection.clone()), Some(factory_for_view));
        grid_view.set_hexpand(true);
        grid_view.set_vexpand(true);
        grid_view.set_min_columns(MIN_COLUMNS);
        grid_view.set_max_columns(MAX_COLUMNS);

        let scrolled = ScrolledWindow::new();
        scrolled.set_hexpand(true);
        scrolled.set_vexpand(true);
        scrolled.set_hscrollbar_policy(gtk4::PolicyType::Never);
        scrolled.set_vscrollbar_policy(gtk4::PolicyType::Automatic);
        scrolled.set_child(Some(&grid_view));

        content.append(&scrolled);

        // Detectar scroll cerca del final para paginación infinita.
        let adjustment = scrolled.vadjustment();
        let on_scroll_bottom_for_scroll = Rc::clone(&on_scroll_bottom_ref);
        adjustment.connect_value_changed(move |adj| {
            let value = adj.value();
            let upper = adj.upper();
            let page_size = adj.page_size();
            if value + page_size >= upper - 1.0
                && let Some(cb) = on_scroll_bottom_for_scroll.borrow().as_ref()
            {
                cb();
            }
        });

        let (download_tx, download_rx) = mpsc::channel();

        Self {
            content,
            store,
            media: media_ref,
            refresh_callback: ViewCallback::default(),
            on_scroll_bottom: Rc::clone(&on_scroll_bottom_ref),
            download_tx,
            download_rx,
            selection,
            on_media_select: on_select_ref,
            scrolled: scrolled.clone(),
        }
    }

    /// Registra un callback que se invoca cuando una descarga de póster termina
    /// para refrescar la galería y mostrar las imágenes descargadas.
    pub fn set_refresh_callback<F>(&mut self, callback: F)
    where
        F: Fn() + 'static,
    {
        *self.refresh_callback.borrow_mut() = Some(Rc::new(callback));
    }

    /// Registra un callback que se invocará cuando el scroll llegue al final
    /// de la galería, para cargar más medios (paginación infinita).
    pub fn set_on_scroll_bottom<F>(&mut self, callback: F)
    where
        F: Fn() + 'static,
    {
        *self.on_scroll_bottom.borrow_mut() = Some(Rc::new(callback));
    }

    /// Inicia la descarga asíncrona de los pósters que aún no están en caché.
    /// Cada descarga se ejecuta en un hilo secundario para no bloquear la UI.
    /// Cuando termina, se invoca el `refresh_callback` para refrescar la
    /// galería y mostrar los pósters descargados.
    pub fn start_poster_downloads(&self) {
        let media = self.media.borrow().clone();
        let download_tx = self.download_tx.clone();

        for m in media {
            let Some(poster_url) = m.poster_path.as_deref() else {
                continue;
            };
            if cached_path(poster_url).is_some() {
                continue;
            }

            let url = poster_url.to_string();
            let tx = download_tx.clone();

            std::thread::spawn(move || {
                let service = PosterService::new();
                let runtime = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(_) => return,
                };
                let _ = runtime.block_on(service.download_async(&url));
                let _ = tx.send(());
            });
        }
    }

    /// Devuelve `true` si hay descargas de póster pendientes por procesar.
    /// Se debe llamar desde el hilo principal (por ejemplo, con
    /// `glib::timeout_add_local`) para refrescar la galería cuando terminen.
    pub fn check_downloads(&self) -> bool {
        self.download_rx.try_recv().is_ok()
    }

    /// Devuelve el `ScrolledWindow` que contiene la galería.
    pub fn scrolled_window(&self) -> &ScrolledWindow {
        &self.scrolled
    }

    pub fn widget(&self) -> &Box {
        &self.content
    }

    /// Limpia la selección actual de la galería, para que ninguna tarjeta
    /// quede con el tono de fondo de "seleccionada" al volver a esta vista.
    pub fn clear_selection(&self) {
        self.selection.set_selected(gtk4::INVALID_LIST_POSITION);
    }

    /// Reemplaza el contenido de la galería con la lista de medios dada.
    /// Se usa para la carga inicial o cuando cambia el filtro/búsqueda.
    pub fn set_media(&self, media: Vec<Media>) {
        self.store.remove_all();
        *self.media.borrow_mut() = media.clone();
        for m in &media {
            let obj = StringObject::new(&m.title);
            self.store.append(&obj);
        }
        // Limpiar la selección: al recargar el contenido, la selección
        // anterior ya no es válida y podría dejar una tarjeta resaltada.
        self.clear_selection();
        // Iniciar descargas asíncronas de pósters que aún no están en caché.
        self.start_poster_downloads();
    }

    /// Añade medios al final de la galería sin reemplazar los existentes.
    /// Se usa para paginación con scroll infinito.
    pub fn append_media(&self, media: &[Media]) {
        if media.is_empty() {
            return;
        }
        let mut current = self.media.borrow_mut();
        for m in media {
            current.push(m.clone());
            let obj = StringObject::new(&m.title);
            self.store.append(&obj);
        }
        // Iniciar descargas asíncronas de los nuevos pósters que faltan.
        drop(current);
        self.start_poster_downloads();
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

/// Crea una tarjeta vacía con la estructura base para ser rellenada en `bind_card`.
fn build_card_widget() -> Box {
    let card = Box::new(Orientation::Vertical, 6);
    card.add_css_class("media-card");
    card.set_halign(gtk4::Align::Start);
    card.set_valign(gtk4::Align::Start);
    card.set_size_request(CARD_WIDTH, CARD_HEIGHT);
    // Pequeña separación respecto al borde superior de la galería.
    card.set_margin_top(8);
    // Pequeña separación respecto a los bordes izquierdo y derecho de la galería.
    card.set_margin_start(8);
    card.set_margin_end(8);

    let poster_stack = Box::new(Orientation::Vertical, 0);
    poster_stack.set_halign(gtk4::Align::Start);
    poster_stack.set_valign(gtk4::Align::Start);
    poster_stack.set_size_request(CARD_POSTER_WIDTH, CARD_POSTER_HEIGHT);
    card.append(&poster_stack);

    let type_label = Label::new(None);
    type_label.set_halign(gtk4::Align::Start);
    type_label.add_css_class("card-type-label");
    card.append(&type_label);

    let title_label = Label::new(None);
    title_label.set_wrap(true);
    title_label.set_max_width_chars(20);
    title_label.set_justify(gtk4::Justification::Left);
    title_label.set_halign(gtk4::Align::Start);
    title_label.set_xalign(0.0);
    title_label.set_lines(2);
    title_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    title_label.add_css_class("poster-title");
    card.append(&title_label);

    let year_label = Label::new(None);
    year_label.add_css_class("poster-year");
    year_label.set_halign(gtk4::Align::Start);
    card.append(&year_label);

    card
}
