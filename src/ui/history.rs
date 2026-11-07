//! Vista de historial de reproducción.
//! Muestra las tarjetas de medios reproducidos con estilo visual completo:
//! tarjetas con fondo, borde y esquinas redondeadas, póster, título, tipo,
//! año, temporada/episodio y fecha de progreso.
//! El cambio de idioma se aplica en caliente (sin reiniciar la aplicación).

use crate::core::progress::{compute_playback_progress, format_remaining};
use crate::db::{MediaRepository, close_db, open_db};
use crate::ui::i18n::TranslationsRef;
use crate::ui::media_list::bind_history_card;
use gtk4::prelude::*;
use gtk4::{Box, Button, FlowBox, GestureClick, Label, Orientation, ProgressBar, ScrolledWindow};
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

/// Petición de apertura de una entrada del historial.
#[derive(Debug, Clone)]
pub struct HistoryOpenRequest {
    /// Medio al que pertenece la entrada (serie, anime o película).
    pub media_id: i64,
    /// Ruta del archivo reproducido, si se conoce.
    pub file_path: Option<String>,
    /// `true` si el clic fue sobre el nombre de la serie/película: solo se
    /// abre la vista de detalles, sin reproducir automáticamente.
    pub details_only: bool,
}

/// Callback de clic en una tarjeta del historial.
type HistoryClickCallback = Rc<dyn Fn(HistoryOpenRequest)>;

/// Vista de historial de reproducción.
pub struct HistoryView {
    /// Página completa (se añade al Stack).
    pub page: Box,
    /// Título de la página.
    pub title: Label,
    /// Subtítulo de la página.
    pub subtitle: Label,
    /// Botón «Volver».
    pub back_button: Button,
    /// Botón «Borrar historial».
    pub clear_button: Button,
    /// Rejilla de tarjetas.
    pub list: FlowBox,
    /// Ruta de la base de datos (para recargar al cambiar idioma).
    db_path: Option<String>,
    /// Referencia a la ventana principal (para callbacks de clic).
    this: Option<crate::ui::main_window::MainWindowRef>,
    /// Referencia a las traducciones (para recargar al cambiar idioma).
    translations: TranslationsRef,
    /// Callback de clic en tarjeta (para recargar al cambiar idioma).
    on_card_click: Option<HistoryClickCallback>,
}

impl HistoryView {
    /// Crea una nueva vista de historial vacía.
    pub fn new(translations: TranslationsRef) -> Self {
        let translations_clone = translations.clone();
        let t = move |key: &str| -> String { translations_clone.borrow().t(key) };

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
            title,
            subtitle,
            back_button,
            clear_button,
            list,
            db_path: None,
            this: None,
            translations,
            on_card_click: None,
        }
    }

    /// Inicializa el contexto necesario para recargar el historial.
    /// Se llama después de crear MainWindow, ya que `this` no existe
    /// todavía en el momento de construir HistoryView.
    pub fn init(
        &mut self,
        db_path: &str,
        this: crate::ui::main_window::MainWindowRef,
        on_card_click: impl Fn(HistoryOpenRequest) + 'static + Clone,
    ) {
        self.db_path = Some(db_path.to_string());
        self.this = Some(this);
        self.on_card_click = Some(Rc::new(on_card_click));
    }

    /// Carga el historial de reproducción desde la BD y lo muestra en la rejilla.
    pub fn load(&self) {
        self.load_history();
    }

    /// Refresca los textos de la UI aplicando el idioma actual.
    /// Se llama después de cambiar el idioma para que la cabecera y las
    /// tarjetas muestren los textos traducidos sin reiniciar la aplicación.
    pub fn refresh_texts(&self) {
        let translations = self.translations.clone();
        self.title.set_label(&t(&translations, "history.title"));
        self.subtitle
            .set_label(&t(&translations, "history.subtitle"));
        self.back_button
            .set_label(&t(&translations, "history.back"));
        self.clear_button
            .set_label(&t(&translations, "history.clear"));
        self.load_history();
    }

    /// Carga el historial desde la BD y pinta las tarjetas con las
    /// traducciones actuales.
    fn load_history(&self) {
        let db_path = match &self.db_path {
            Some(p) => p.clone(),
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

        let translations = self.translations.clone();

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
            let title_label = bind_history_card(
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
                ep_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
                ep_label.set_max_width_chars(28);
                card.append(&ep_label);
            }

            // Fecha y progreso de reproducción.
            let completed = entry.completed_at.is_some();
            let playback_progress =
                compute_playback_progress(entry.position, entry.duration, completed);

            // Texto breve para la tarjeta: porcentaje o «Visto». El tiempo
            // restante se reserva para el tooltip para no ensanchar la tarjeta.
            let progress_label = if playback_progress.completed {
                t(&translations, "history.watched")
            } else {
                format!("{}%", playback_progress.percent)
            };
            let tooltip_label = if let Some(remaining) = playback_progress.remaining_seconds {
                format!(
                    "{} · {}",
                    progress_label,
                    t(&translations, "history.remaining_format").replacen(
                        "{}",
                        &format_remaining(remaining),
                        1
                    )
                )
            } else {
                progress_label.clone()
            };

            let date_label = Label::new(Some(&format!("{} · {}", entry.played_at, progress_label)));
            date_label.set_halign(gtk4::Align::Start);
            date_label.set_xalign(0.0);
            // Evitar que un texto largo ensanche la tarjeta del historial.
            date_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
            date_label.set_max_width_chars(28);
            card.append(&date_label);

            // Barra de progreso visual, fina y sin expansión horizontal para
            // que el FlowBox no estire la tarjeta.
            let progress_bar = ProgressBar::new();
            progress_bar.set_fraction(playback_progress.fraction);
            progress_bar.set_show_text(false);
            progress_bar.set_size_request(-1, 6);
            progress_bar.set_tooltip_text(Some(&tooltip_label));
            if playback_progress.completed {
                progress_bar.add_css_class("history-progress-completed");
            } else {
                progress_bar.add_css_class("history-progress");
            }
            card.append(&progress_bar);

            // Clic en la tarjeta: abre los detalles y reanuda la reproducción.
            let click = GestureClick::new();
            let media_id = entry.media_id;
            let file_path = entry.file_path.clone();
            let on_click = on_card_click.clone();
            click.connect_pressed(move |_, _, _, _| {
                on_click(HistoryOpenRequest {
                    media_id,
                    file_path: file_path.clone(),
                    details_only: false,
                });
            });
            card.add_controller(click);

            // Clic sobre el nombre de la serie/película: solo abre la vista de
            // detalles, sin reproducir. Se reclama la secuencia para que GTK
            // deniegue el gesto de la tarjeta (widget padre en la propagación)
            // y no se abra también la reproducción.
            let title_click = GestureClick::new();
            let media_id = entry.media_id;
            let file_path = entry.file_path.clone();
            let on_click = on_card_click.clone();
            title_click.connect_pressed(move |gesture, _, _, _| {
                gesture.set_state(gtk4::EventSequenceState::Claimed);
                on_click(HistoryOpenRequest {
                    media_id,
                    file_path: file_path.clone(),
                    details_only: true,
                });
            });
            title_label.add_controller(title_click);

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
