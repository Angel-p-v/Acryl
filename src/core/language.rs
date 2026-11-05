//! Normalización de idiomas de pistas de audio y subtítulos.
//!
//! `ffprobe` y los distintos contenedores no usan un vocabulario uniforme para
//! el campo de idioma: pueden aparecer códigos ISO de dos o tres letras
//! (`ja`, `jpn`), nombres en inglés o incluso variantes con mayúsculas,
//! acentos o el códec entre paréntesis. Para que Ajustes muestre una única
//! entrada por idioma y para poder resolverla después contra las pistas reales
//! de cada archivo, todo se reduce a un código ISO 639-1 canónico.
//!
//! La clave persistida en la configuración es estable (`language:ja`) y no
//! depende de la pista concreta ni de su códec. Las claves antiguas del tipo
//! `embedded:audio:jpn:aac:japanese` se siguen aceptando y se normalizan a la
//! nueva clave cuando se cargan.

/// Identidad de idioma extraída de una clave de configuración o de pista.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageIdentity {
    /// Idioma reconocido, con su código ISO 639-1 canónico.
    Known(&'static str),
    /// La pista no declara un idioma reconocible.
    Unknown,
}

/// Devuelve el código ISO 639-1 canónico para un valor de idioma.
///
/// Acepta códigos de dos o tres letras, nombres en inglés o español y algunas
/// variantes habituales de `ffprobe`. Ignora el contenido entre paréntesis
/// (por ejemplo, `Japanese (AAC)`) y los separadores de variantes regionales.
pub fn canonical_code(value: &str) -> Option<&'static str> {
    let normalized = normalize(value);
    if normalized.is_empty() {
        return None;
    }

    if let Some(code) = match normalized.as_str() {
        "latin spanish" | "spanish latin" | "latino" | "latinoamericano" => Some("es"),
        "brazilian portuguese" | "portuguese brazil" => Some("pt"),
        "european portuguese" => Some("pt"),
        "norwegian bokmal" | "norwegian bokmål" | "bokmal" | "bokmål" => Some("no"),
        "norwegian nynorsk" | "nynorsk" => Some("no"),
        "chinese simplified" | "simplified chinese" => Some("zh"),
        "chinese traditional" | "traditional chinese" => Some("zh"),
        _ => None,
    } {
        return Some(code);
    }

    let token = normalized.split_whitespace().next().unwrap_or("");
    match token {
        "spa" | "es" | "espanol" | "español" | "spanish" | "castilian" | "castellano"
        | "espanola" | "española" => Some("es"),
        "eng" | "en" | "english" | "ingles" | "inglés" => Some("en"),
        "jpn" | "ja" | "jap" | "japanese" | "japones" | "japonés" | "japonesa" | "japonese"
        | "japonose" | "nihongo" => Some("ja"),
        "fra" | "fre" | "fr" | "french" | "francais" | "français" | "frances" | "francés" => {
            Some("fr")
        }
        "deu" | "ger" | "de" | "german" | "deutsch" | "aleman" | "alemán" => Some("de"),
        "ita" | "it" | "italian" | "italiano" => Some("it"),
        "por" | "pt" | "portuguese" | "portugues" | "portugués" => Some("pt"),
        "rus" | "ru" | "russian" | "ruso" => Some("ru"),
        "zho" | "chi" | "zh" | "chinese" | "mandarin" | "chino" => Some("zh"),
        "kor" | "ko" | "korean" | "coreano" => Some("ko"),
        "ara" | "ar" | "arabic" | "arabe" | "árabe" => Some("ar"),
        "hin" | "hi" | "hindi" => Some("hi"),
        "nld" | "dut" | "nl" | "dutch" | "nederlands" | "neerlandes" | "neerlandés" => Some("nl"),
        "swe" | "sv" | "swedish" | "svenska" | "sueco" => Some("sv"),
        "nor" | "no" | "nb" | "nn" | "norwegian" | "norsk" | "noruego" => Some("no"),
        "dan" | "da" | "danish" | "dansk" | "danes" | "danés" => Some("da"),
        "fin" | "fi" | "finnish" | "suomi" | "fines" | "finés" => Some("fi"),
        "pol" | "pl" | "polish" | "polski" | "polaco" => Some("pl"),
        "tur" | "tr" | "turkish" | "turkce" | "türkçe" | "turco" => Some("tr"),
        "ces" | "cze" | "cs" | "czech" | "cestina" | "čeština" | "checo" => Some("cs"),
        "ell" | "gre" | "el" | "greek" | "griego" => Some("el"),
        "heb" | "he" | "hebrew" | "hebreo" => Some("he"),
        "tha" | "th" | "thai" | "tailandes" | "tailandés" => Some("th"),
        "ukr" | "uk" | "ukrainian" | "ucraniano" => Some("uk"),
        "vie" | "vi" | "vietnamese" | "vietnamita" => Some("vi"),
        "ind" | "id" | "indonesian" | "indonesio" => Some("id"),
        "msa" | "may" | "ms" | "malay" | "malayo" => Some("ms"),
        "tgl" | "fil" | "tl" | "filipino" | "tagalog" => Some("tl"),
        "hun" | "hu" | "hungarian" | "magyar" | "hungaro" | "húngaro" => Some("hu"),
        "ron" | "rum" | "ro" | "romanian" | "romana" | "română" | "rumano" => Some("ro"),
        "fas" | "per" | "fa" | "persian" | "farsi" | "persa" => Some("fa"),
        "cat" | "ca" | "catalan" | "catalán" => Some("ca"),
        "eus" | "baq" | "eu" | "basque" | "euskara" | "vasco" => Some("eu"),
        "glg" | "gl" | "galician" | "galego" | "gallego" => Some("gl"),
        "afr" | "af" | "afrikaans" => Some("af"),
        "swa" | "sw" | "swahili" | "suajili" => Some("sw"),
        "bul" | "bg" | "bulgarian" | "bulgaro" | "búlgaro" => Some("bg"),
        "hrv" | "hr" | "croatian" | "croata" => Some("hr"),
        "srp" | "sr" | "serbian" | "serbio" => Some("sr"),
        "slk" | "slo" | "sk" | "slovak" | "eslovaco" => Some("sk"),
        "slv" | "sl" | "slovenian" | "esloveno" => Some("sl"),
        "ben" | "bn" | "bengali" | "bengalí" => Some("bn"),
        "tam" | "ta" | "tamil" => Some("ta"),
        "tel" | "te" | "telugu" => Some("te"),
        "mal" | "ml" | "malayalam" => Some("ml"),
        "urd" | "ur" | "urdu" => Some("ur"),
        _ => None,
    }
}

/// Resuelve el idioma usando primero el campo `language` y, si no basta, el
/// título de la pista. Esto cubre archivos cuyo idioma viene vacío o como
/// `und` pero cuyo título sí indica el idioma real.
pub fn canonical_code_from_hints(
    language: Option<&str>,
    title: Option<&str>,
) -> Option<&'static str> {
    language
        .and_then(canonical_code)
        .or_else(|| title.and_then(canonical_code))
}

/// Clave estable de configuración para un idioma.
///
/// `None` representa un idioma no reconocible (`language:und`).
pub fn language_key(code: Option<&str>) -> String {
    match code.and_then(canonical_code) {
        Some(code) => format!("language:{}", code),
        None => "language:und".to_string(),
    }
}

/// Extrae la identidad de idioma de una clave de pista o de configuración.
///
/// Acepta las claves nuevas (`language:ja`), las antiguas claves embebidas
/// (`embedded:audio:jpn:aac:japanese`) y códigos o nombres sueltos. Devuelve
/// `None` cuando la clave no describe un idioma (por ejemplo, `default:audio`
/// o una ruta externa).
pub fn identity_from_track_key(key: &str) -> Option<LanguageIdentity> {
    let key = key.trim();
    if key.is_empty() {
        return None;
    }

    if let Some(rest) = key.strip_prefix("language:") {
        return Some(match canonical_code(rest) {
            Some(code) => LanguageIdentity::Known(code),
            None => LanguageIdentity::Unknown,
        });
    }

    if key.starts_with("external:") {
        return None;
    }

    if let Some(rest) = key.strip_prefix("embedded:") {
        let parts: Vec<&str> = rest.split(':').collect();
        let language = parts.get(1).copied();
        let title = parts.get(3).copied();
        return Some(match canonical_code_from_hints(language, title) {
            Some(code) => LanguageIdentity::Known(code),
            None => LanguageIdentity::Unknown,
        });
    }

    canonical_code(key).map(LanguageIdentity::Known)
}

/// Convierte una clave antigua o nueva a la clave de idioma canónica.
///
/// Las claves que no representan un idioma (por ejemplo, una ruta externa o
/// `default:audio`) se conservan intactas para no perder preferencias
/// existentes.
pub fn normalize_track_key(key: &str) -> String {
    match identity_from_track_key(key) {
        Some(LanguageIdentity::Known(code)) => language_key(Some(code)),
        Some(LanguageIdentity::Unknown)
            if key.starts_with("language:") || key.starts_with("embedded:") =>
        {
            language_key(None)
        }
        _ => key.to_string(),
    }
}

/// Normaliza un valor para compararlo: minúsculas, sin paréntesis ni
/// separadores de variante regional.
fn normalize(value: &str) -> String {
    let lower = value.trim().to_lowercase();
    let cut = lower.find(['(', '[', '·']).unwrap_or(lower.len());
    lower[..cut]
        .replace(['_', '-'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconoce_codigos_y_nombres_del_mismo_idioma() {
        for value in ["jpn", "ja", "Japanese", "JAPONÉS", "japonese (AAC)"] {
            assert_eq!(canonical_code(value), Some("ja"), "valor: {value}");
        }
        for value in ["spa", "es-MX", "Spanish", "Español", "Castilian"] {
            assert_eq!(canonical_code(value), Some("es"), "valor: {value}");
        }
    }

    #[test]
    fn prioriza_el_campo_idioma_y_usa_el_titulo_como_respaldo() {
        assert_eq!(
            canonical_code_from_hints(Some("jpn"), Some("Japanese")),
            Some("ja")
        );
        assert_eq!(
            canonical_code_from_hints(None, Some("Japanese (AAC)")),
            Some("ja")
        );
        assert_eq!(canonical_code_from_hints(Some("und"), None), None);
    }

    #[test]
    fn las_claves_nuevas_y_antiguas_se_normalizan_igual() {
        assert_eq!(normalize_track_key("language:ja"), "language:ja");
        assert_eq!(
            normalize_track_key("embedded:audio:jpn:aac:japanese"),
            "language:ja"
        );
        assert_eq!(
            normalize_track_key("embedded:subtitle:spa:ass:spanish"),
            "language:es"
        );
        assert_eq!(
            normalize_track_key("embedded:audio:und:aac:unknown"),
            "language:und"
        );
    }

    #[test]
    fn las_claves_que_no_son_idioma_se_conservan() {
        assert_eq!(
            normalize_track_key("external:/tmp/video.es.srt"),
            "external:/tmp/video.es.srt"
        );
        assert_eq!(normalize_track_key("default:audio"), "default:audio");
    }

    #[test]
    fn identifica_idiomas_desconocidos() {
        assert_eq!(
            identity_from_track_key("language:und"),
            Some(LanguageIdentity::Unknown)
        );
        assert_eq!(
            identity_from_track_key("embedded:audio::aac:Unknown"),
            Some(LanguageIdentity::Unknown)
        );
        assert_eq!(identity_from_track_key("external:/tmp/a.srt"), None);
        assert_eq!(identity_from_track_key("default:subtitle"), None);
    }

    #[test]
    fn language_key_usa_und_para_desconocido() {
        assert_eq!(language_key(Some("ja")), "language:ja");
        assert_eq!(language_key(Some("jpn")), "language:ja");
        assert_eq!(language_key(None), "language:und");
    }
}
