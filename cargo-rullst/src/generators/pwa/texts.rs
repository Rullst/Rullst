//! The visible texts of the generated offline page and update notice.
//!
//! They come in the languages Rullst's account mail writes (English,
//! Brazilian Portuguese and Spanish), chosen by the primary subtag of
//! `--lang`; another language gets English, and the report says to
//! translate it.

/// One language's texts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Texts {
    pub offline_title: &'static str,
    pub offline_heading: &'static str,
    pub offline_message: &'static str,
    pub offline_retry: &'static str,
    pub update_message: &'static str,
    pub update_reload: &'static str,
}

const EN: Texts = Texts {
    offline_title: "Offline",
    offline_heading: "You are offline",
    offline_message: "This page needs a connection. Check your network and try again.",
    offline_retry: "Try again",
    update_message: "A new version of this app is available.",
    update_reload: "Reload",
};

const PT: Texts = Texts {
    offline_title: "Sem conexão",
    offline_heading: "Você está sem conexão",
    offline_message: "Esta página precisa de conexão. Verifique a sua rede e tente de novo.",
    offline_retry: "Tentar de novo",
    update_message: "Uma nova versão deste aplicativo está disponível.",
    update_reload: "Recarregar",
};

const ES: Texts = Texts {
    offline_title: "Sin conexión",
    offline_heading: "Estás sin conexión",
    offline_message: "Esta página necesita conexión. Comprueba tu red e inténtalo de nuevo.",
    offline_retry: "Reintentar",
    update_message: "Hay una nueva versión de esta aplicación.",
    update_reload: "Recargar",
};

/// The English texts, which the templates hold.
pub(super) const ENGLISH: Texts = EN;

/// The texts for the language tag `lang`, and whether they are in that
/// language (`false`: English for a language without texts).
pub(super) fn for_lang(lang: &str) -> (Texts, bool) {
    let primary = lang
        .split('-')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    match primary.as_str() {
        "en" => (EN, true),
        "pt" => (PT, true),
        "es" => (ES, true),
        _ => (EN, false),
    }
}

/// `text` as the body of a single-quoted JavaScript string: ASCII only, so
/// the file reads the same in any encoding a server declares.
pub(super) fn javascript(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '\'' => escaped.push_str("\\'"),
            '\\' => escaped.push_str("\\\\"),
            ' '..='~' => escaped.push(character),
            other => {
                let mut units = [0; 2];
                for unit in other.encode_utf16(&mut units) {
                    escaped.push_str(&format!("\\u{unit:04x}"));
                }
            }
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn languages_are_chosen_by_their_primary_subtag() {
        assert_eq!(for_lang("pt-BR"), (PT, true));
        assert_eq!(for_lang("PT"), (PT, true));
        assert_eq!(for_lang("es-419"), (ES, true));
        assert_eq!(for_lang("en-GB"), (EN, true));
        assert_eq!(for_lang("fr"), (EN, false));
        assert_eq!(ENGLISH, EN);
        for texts in [EN, PT, ES] {
            for text in [
                texts.offline_title,
                texts.offline_heading,
                texts.offline_message,
                texts.offline_retry,
                texts.update_message,
                texts.update_reload,
            ] {
                // They go into Rust and JavaScript string literals as they are.
                assert!(
                    !text.contains(['"', '\'', '\\', '{', '}', '<', '>', '&']),
                    "{text}"
                );
            }
        }
    }

    #[test]
    fn javascript_strings_are_ascii() {
        assert_eq!(javascript("Versão"), "Vers\\u00e3o");
        assert_eq!(javascript("it's \\ ok"), "it\\'s \\\\ ok");
        assert_eq!(javascript("😀"), "\\ud83d\\ude00");
        assert!(javascript(PT.update_message).is_ascii());
    }
}
