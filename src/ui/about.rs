//! Diálogo «Acerca de» de la aplicación.
//!
//! Usa `AdwAboutDialog` para mostrar la información del proyecto (nombre,
//! versión, autor, licencia, enlaces e información de depuración) de forma
//! nativa sobre la ventana principal.

use crate::ui::i18n::TranslationsRef;
use gtk4::prelude::*;
use libadwaita::AboutDialog;
use libadwaita::prelude::*;

/// URL del repositorio del proyecto.
const REPOSITORY_URL: &str = "https://github.com/Angel-p-v/app-media-manager";

/// Autor y mantenedor principal del proyecto.
const DEVELOPER: &str = "Angel-p-v";

/// Aviso de copyright mostrado en el diálogo.
const COPYRIGHT: &str = "© 2026 Angel-p-v";

/// Construye el texto de depuración con las versiones en tiempo de ejecución.
///
/// Facilita el diagnóstico de errores al permitir al usuario copiar las
/// versiones de Acryl, GTK y Libadwaita junto con su sistema.
fn runtime_debug_info() -> String {
    format!(
        "Acryl {}\nGTK {}.{}.{}\nLibadwaita {}.{}.{}\nSistema: {} {}",
        env!("CARGO_PKG_VERSION"),
        gtk4::major_version(),
        gtk4::minor_version(),
        gtk4::micro_version(),
        libadwaita::major_version(),
        libadwaita::minor_version(),
        libadwaita::micro_version(),
        std::env::consts::OS,
        std::env::consts::ARCH,
    )
}

/// Construye el diálogo «Acerca de» con los datos del proyecto.
pub fn build_about_dialog(translations: &TranslationsRef) -> AboutDialog {
    let comments = translations.borrow().t("about.description");
    let issue_url = format!("{}/issues", REPOSITORY_URL);
    let support_url = format!("{}/discussions", REPOSITORY_URL);

    AboutDialog::builder()
        .application_name("Acryl")
        .application_icon("acryl")
        .developer_name(DEVELOPER)
        .version(env!("CARGO_PKG_VERSION"))
        .comments(comments)
        .copyright(COPYRIGHT)
        .developers(vec![DEVELOPER])
        .website(REPOSITORY_URL)
        .issue_url(issue_url)
        .support_url(support_url)
        .license_type(gtk4::License::MitX11)
        .debug_info(runtime_debug_info())
        .build()
}

/// Muestra el diálogo «Acerca de» sobre la ventana indicada.
pub fn show_about_dialog(parent: &impl IsA<gtk4::Widget>, translations: &TranslationsRef) {
    build_about_dialog(translations).present(Some(parent));
}
