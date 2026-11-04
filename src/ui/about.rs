//! Diálogo «Acerca de» de la aplicación.
//!
//! Usa `AdwAboutDialog` para mostrar la información del proyecto (nombre,
//! versión, autor, licencia y enlaces) de forma nativa sobre la ventana
//! principal.

use crate::ui::i18n::TranslationsRef;
use gtk4::prelude::*;
use libadwaita::AboutDialog;
use libadwaita::prelude::*;

/// URL del repositorio del proyecto.
const REPOSITORY_URL: &str = "https://github.com/Angel-p-v/app-media-manager";

/// Construye el diálogo «Acerca de» con los datos del proyecto.
pub fn build_about_dialog(translations: &TranslationsRef) -> AboutDialog {
    let comments = translations.borrow().t("about.description");
    let issue_url = format!("{}/issues", REPOSITORY_URL);

    AboutDialog::builder()
        .application_name("Acryl")
        .application_icon("acryl")
        .developer_name("Angel-p-v")
        .version(env!("CARGO_PKG_VERSION"))
        .comments(comments)
        .website(REPOSITORY_URL)
        .issue_url(issue_url)
        .license_type(gtk4::License::MitX11)
        .build()
}

/// Muestra el diálogo «Acerca de» sobre la ventana indicada.
pub fn show_about_dialog(parent: &impl IsA<gtk4::Widget>, translations: &TranslationsRef) {
    build_about_dialog(translations).present(Some(parent));
}
