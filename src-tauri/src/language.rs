//! Which of Voxtral's languages a text is in. Used to pin the language once
//! per recording, so short, unclear pieces can't drift into another one.

/// Voxtral Mini's languages (ISO 639-1).
pub const SUPPORTED: [&str; 8] = ["de", "en", "fr", "es", "it", "pt", "nl", "hi"];

/// Below this, guesses are unreliable.
const MIN_CHARS: usize = 20;

pub fn detect(text: &str) -> Option<&'static str> {
    if text.trim().chars().count() < MIN_CHARS {
        return None;
    }
    let code = match whatlang::detect_lang(text)? {
        whatlang::Lang::Deu => "de",
        whatlang::Lang::Eng => "en",
        whatlang::Lang::Fra => "fr",
        whatlang::Lang::Spa => "es",
        whatlang::Lang::Ita => "it",
        whatlang::Lang::Por => "pt",
        whatlang::Lang::Nld => "nl",
        whatlang::Lang::Hin => "hi",
        _ => return None,
    };
    debug_assert!(SUPPORTED.contains(&code));
    Some(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_the_supported_languages() {
        let samples = [
            ("de", "Guten Morgen und herzlich willkommen zu unserer Runde. Wir sprechen heute über das neue Projekt."),
            ("en", "Good morning and welcome to our meeting. Today we are talking about the new project in town."),
            ("fr", "Bonjour et bienvenue à notre réunion. Aujourd'hui nous parlons du nouveau projet de la ville."),
            ("es", "Buenos días y bienvenidos a nuestra reunión. Hoy hablamos del nuevo proyecto de la ciudad."),
            ("it", "Buongiorno e benvenuti alla nostra riunione. Oggi parliamo del nuovo progetto della città."),
            ("nl", "Goedemorgen en welkom bij onze vergadering. Vandaag praten we over het nieuwe project in de stad."),
        ];
        for (code, text) in samples {
            assert_eq!(detect(text), Some(code), "{text}");
        }
    }

    #[test]
    fn short_or_unsupported_text_gives_no_answer() {
        assert_eq!(detect("Hallo"), None);
        assert_eq!(detect(""), None);
        assert_eq!(detect("Dzień dobry i witamy na naszym spotkaniu. Dzisiaj rozmawiamy o nowym projekcie."), None);
    }
}
