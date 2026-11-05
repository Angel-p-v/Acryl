//! Vista de historial de reproducción.
//! Muestra las tarjetas de medios reproducidos con estilo visual completo:
//! tarjetas con fondo, borde y esquinas redondeadas, póster, título, tipo,
//! año, temporada/episodio y fecha de progreso.

use crate::db::{MediaRepository, close_db, open_db};
use crate::ui::i18n::TranslationsRef;
use crate::ui::media_list::bind_history_card;
use gtk4::prelude::*;
use gtk4::{Box, Button, FlowBox, GestureClick, Label, Orientation, ScrolledWindow};
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

/// Vista de historial de reproducción.
pub struct HistoryView {
    /// Página completa (se añade al Stack).
    pub page: Box,
    /// Botón «Volver».
    pub back_button: Button,
    /// Botón «Borrar historial».
    pub clear_button: Button,
    /// Rejilla de tarjetas.
    pub list: FlowBox,
    /// Ruta de la base de datos (para recargar al cambiar idioma).
    db_path: Option<String>,
    /// Referencia a la ventana principal (para callbacks y traducciones).
    this: Option<crate::ui::main_window::MainWindowRef>,
    /// Callback de clic en tarjeta (para recargar al cambiar idioma).
    on_card_click: Option<Rc<dyn Fn(i64)>>,
}

impl HistoryView {
    /// Crea una nueva vista de historial vacía.
    pub fn new(translations: TranslationsRef) -> Self {
        let t = move |key: &str| -> String { translations.borrow().t(key) };

        let page = Box::new(Orientation::Vertical, 0);
        page.set_hexpand(true);
        page.set_vexpand(true);

        // Cabecera.
        let header = Box::new(Orientation::Vertical, 8);
        header.set_halign(gtk4::Align::Fill);
        header.set_valign(gtk4::Align::Start);
        header.set_margin_top(24);
        header.set_margin_start(24);
        header.set_margin_end(24);

        let title = Label::new(Some(&t("history.title")));
        title.set_halign(gtk4::Align::Start);
        title.set_hexpand(true);
        title.set_xalign(0.0);
        title.add_css_class("page-title");
        header.append(&title);

        let subtitle = Label::new(Some(&t("history.subtitle")));
        subtitle.set_halign(gtk4::Align::Start);
        subtitle.set_xalign(0.0);
        subtitle.set_wrap(true);
        subtitle.add_css_class("dim-label");
        header.append(&subtitle);

        // Fila de acciones.
        let actions = Box::new(Orientation::Horizontal, 0);
        actions.set_hexpand(true);
        actions.set_halign(gtk4::Align::Fill);
        actions.set_valign(gtk4::Align::Start);
        actions.set_margin_top(12);

        let back_button = Button::with_label(&t("history.back"));
        back_button.add_css_class("flat-button");
        back_button.set_valign(gtk4::Align::Center);
        actions.append(&back_button);

        let clear_button = Button::with_label(&t("history.clear"));
        clear_button.add_css_class("flat-button");
        clear_button.add_css_class("button.danger");
        clear_button.set_valign(gtk4::Align::Center);
        clear_button.set_hexpand(true);
        clear_button.set_halign(gtk4::Align::End);
        actions.append(&clear_button);

        header.append(&actions);

        page.append(&header);

        // Rejilla de tarjetas.
        let list = FlowBox::new();
        list.add_css_class("history-grid");
        list.set_hexpand(true);
        list.set_valign(gtk4::Align::Start);
        list.set_homogeneous(false);
        list.set_selection_mode(gtk4::SelectionMode::None);
        list.set_row_spacing(0);
        list.set_column_spacing(0);
        list.set_margin_top(16);
        list.set_margin_bottom(0);
        list.set_margin_start(0);
        list.set_margin_end(0);
        list.set_min_children_per_line(1);
        list.set_max_children_per_line(12);

        let scrolled = ScrolledWindow::new();
        scrolled.set_hexpand(true);
        scrolled.set_vexpand(true);
        scrolled.set_hscrollbar_policy(gtk4::PolicyType::Never);
        scrolled.set_vscrollbar_policy(gtk4::PolicyType::Automatic);
        scrolled.set_child(Some(&list));
        page.append(&scrolled);

        Self {
            page,
            back_button,
            clear_button,
            list,
            db_path: None,
            this: None,
            on_card_click: None,
        }
    }

    /// Configura el contexto necesario para recargar el historial.
    /// Se llama después de crear MainWindow, ya que `this` no existe
    /// todavía en el momento de construir HistoryView.
    pub fn set_context(
        &mut self,
        db_path: &str,
        this: &crate::ui::main_window::MainWindowRef,
        on_card_click: impl Fn(i64) + 'static + Clone,
    ) {
        self.db_path = Some(db_path.to_string());
        self.this = Some(this.clone());
        self.on_card_click = Some(Rc::new(on_card_click));
    }

    /// Carga el historial de reproducción desde la BD y lo muestra en la rejilla.
    pub fn load(&self) {
        self.load_history();
    }

    /// Refresca los textos de la UI aplicando el idioma actual.
    /// Se llama después de cambiar el idioma para que las tarjetas
    /// muestren los textos traducidos sin reiniciar la aplicación.
    pub fn refresh_texts(&self) {
        self.load_history();
    }

    /// Carga el historial desde la BD y pinta las tarjetas con las
    /// traducciones actuales.
    fn load_history(&self) {
        let db_path = match &self.db_path {
            Some(p) => p.clone(),
            None => return,
        };
        let this = match &self.this {
            Some(t) => t.clone(),
            None => return,
        };
        let on_card_click = match &self.on_card_click {
            Some(c) => c.clone(),
            None => return,
        };

        let mut conn = match open_db(&db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error abriendo BD para historial: {}", e);
                return;
            }
        };
        let history = {
            let mut repo = MediaRepository::new(&mut conn);
            repo.list_playback_history(Some(100), Some(0))
                .unwrap_or_default()
        };
        close_db(conn);

        let translations: TranslationsRef = {
            let w = this.borrow();
            w.translations.clone()
        };

        // Vaciar filas existentes.
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }

        if history.is_empty() {
            let empty_box = Box::new(Orientation::Vertical, 8);
            empty_box.set_halign(gtk4::Align::Center);
            empty_box.set_valign(gtk4::Align::Center);
            empty_box.set_hexpand(true);
            empty_box.set_vexpand(true);
            empty_box.set_margin_top(48);

            let empty_label = Label::new(Some(&t(&translations, "history.empty")));
            empty_label.set_halign(gtk4::Align::Center);
            empty_label.set_xalign(0.5);
            empty_label.add_css_class("dim-label");

            empty_box.append(&empty_label);
            self.list.append(&empty_box);
            return;
        }

        let (download_tx, _download_rx) = mpsc::channel();
        let downloading_posters = Arc::new(Mutex::new(std::collections::HashSet::new()));

        for entry in history.iter() {
            let card = build_history_card_widget();
            bind_history_card(
                &card,
                entry,
                &translations,
                &download_tx,
                &downloading_posters,
            );

            // Temporada y episodio si es una serie/anime.
            if let (Some(season), Some(episode)) = (entry.season, entry.episode)
                && (entry.r#type == "series" || entry.r#type == "anime")
            {
                let ep_label_text = format!(
                    "S{:02}E{:02} – {}",
                    season,
                    episode,
                    entry.episode_title.as_deref().unwrap_or(&entry.title)
                );
                let ep_label = Label::new(Some(&ep_label_text));
                ep_label.set_halign(gtk4::Align::Start);
                ep_label.set_xalign(0.0);
                card.append(&ep_label);
            }

            // Fecha y progreso de reproducción.
            let progress = if entry.duration > 0.0 {
                let pct = (entry.position / entry.duration * 100.0) as u32;
                format!("{}%", pct)
            } else {
                "—".to_string()
            };
            let date_label = Label::new(Some(&format!("{} · {}", entry.played_at, progress)));
            date_label.set_halign(gtk4::Align::Start);
            date_label.set_xalign(0.0);
            card.append(&date_label);

            // Clic en la tarjeta.
            let click = GestureClick::new();
            let media_id = entry.media_id;
            let on_click = on_card_click.clone();
            click.connect_pressed(move |_, _, _, _| {
                on_click(media_id);
            });
            card.add_controller(click);

            self.list.append(&card);
        }
    }
}

/// Crea una tarjeta de historial con estilo visual completo (clase media-card).
fn build_history_card_widget() -> Box {
    let card = Box::new(Orientation::Vertical, 6);
    card.add_css_class("media-card");
    card.set_halign(gtk4::Align::Start);
    card.set_valign(gtk4::Align::Start);
    card.set_size_request(-1, 340);
    card.set_margin_top(0);
    card.set_margin_start(0);
    card.set_margin_end(0);

    let poster_stack = Box::new(Orientation::Vertical, 0);
    poster_stack.set_halign(gtk4::Align::Start);
    poster_stack.set_valign(gtk4::Align::Start);
    poster_stack.set_size_request(200, 300);
    card.append(&poster_stack);

    let type_label = Label::new(None);
    type_label.set_halign(gtk4::Align::Start);
    card.append(&type_label);

    let title_label = Label::new(None);
    title_label.set_wrap(true);
    title_label.set_max_width_chars(20);
    title_label.set_justify(gtk4::Justification::Left);
    title_label.set_halign(gtk4::Align::Start);
    title_label.set_xalign(0.0);
    title_label.set_lines(2);
    title_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    card.append(&title_label);

    let year_label = Label::new(None);
    year_label.set_halign(gtk4::Align::Start);
    card.append(&year_label);

    card
}

/// Traduce una clave de i18n (función auxiliar local).
fn t(translations: &TranslationsRef, key: &str) -> String {
    translations.borrow().t(key)
}
