//! Sistema de internacionalización (i18n) de la interfaz.
//!
//! Carga diccionarios de traducción embebidos en tiempo de compilación y
//! permite cambiar de idioma en caliente sin reiniciar la aplicación. Las
//! claves son en inglés (idioma base) y los valores son las traducciones.
//!
//! Uso:
//!   let t = translations.borrow();
//!   label.set_label(t.t("settings.title"));

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Diccionario de traducciones cargado en memoria.
#[derive(Debug, Clone, Default)]
pub struct Translations {
    strings: HashMap<String, String>,
}

impl Translations {
    /// Carga las traducciones para `language` desde los archivos embebidos.
    /// El español (`es`) es el idioma por defecto cuando no existe un archivo
    /// para el idioma solicitado o falla la carga.
    pub fn load(language: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let json = match language {
            "en" => include_str!("./locales/en.json"),
            _ => include_str!("./locales/es.json"),
        };
        let strings: HashMap<String, String> = serde_json::from_str(json)?;
        Ok(Self { strings })
    }

    /// Devuelve el texto traducido para `key`, o la propia `key` si no hay
    /// traducción disponible (fallback seguro al idioma base).
    pub fn t(&self, key: &str) -> String {
        self.strings
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    /// Recarga el diccionario para otro idioma.
    pub fn set_language(&mut self, language: &str) -> Result<(), Box<dyn std::error::Error>> {
        *self = Self::load(language)?;
        Ok(())
    }
}

/// Referencia compartida y mutable a las traducciones. Permite cambiar de
/// idioma en caliente y que todos los componentes de la UI vean el cambio.
pub type TranslationsRef = Rc<RefCell<Translations>>;

/// Crea una nueva referencia a traducciones cargando el idioma dado.
pub fn load_translations(language: &str) -> Result<TranslationsRef, Box<dyn std::error::Error>> {
    Ok(Rc::new(RefCell::new(Translations::load(language)?)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carga_espanol_por_defecto() {
        let translations = Translations::load("es").expect("cargar español");
        assert_eq!(translations.t("settings.title"), "Ajustes");
        assert_eq!(translations.t("media.type.movie"), "Película");
        assert_eq!(translations.t("media.type.series"), "Serie");
        assert_eq!(translations.t("media.type.anime"), "Anime");
    }

    #[test]
    fn carga_ingles() {
        let translations = Translations::load("en").expect("cargar inglés");
        assert_eq!(translations.t("settings.title"), "Settings");
        assert_eq!(translations.t("media.type.movie"), "Movie");
        assert_eq!(translations.t("media.type.series"), "Series");
        assert_eq!(translations.t("media.type.anime"), "Anime");
    }

    #[test]
    fn idioma_desconocido_usa_espanol_como_fallback() {
        let translations = Translations::load("fr").expect("cargar francés (fallback)");
        assert_eq!(translations.t("settings.title"), "Ajustes");
    }

    #[test]
    fn clave_inexistente_devuelve_la_clave_misma() {
        let translations = Translations::load("es").expect("cargar español");
        assert_eq!(translations.t("no.existe"), "no.existe");
    }

    #[test]
    fn cambiar_idioma_en_caliente() {
        let mut translations = Translations::load("es").expect("cargar español");
        assert_eq!(translations.t("settings.title"), "Ajustes");
        translations.set_language("en").expect("cambiar a inglés");
        assert_eq!(translations.t("settings.title"), "Settings");
    }

    #[test]
    fn diccionarios_es_y_en_tienen_las_mismas_claves() {
        let es = Translations::load("es").expect("cargar español");
        let en = Translations::load("en").expect("cargar inglés");

        let mut es_keys: Vec<&str> = es.strings.keys().map(String::as_str).collect();
        let mut en_keys: Vec<&str> = en.strings.keys().map(String::as_str).collect();
        es_keys.sort_unstable();
        en_keys.sort_unstable();

        assert_eq!(
            es_keys, en_keys,
            "los diccionarios es y en deben tener las mismas claves"
        );
    }
}
