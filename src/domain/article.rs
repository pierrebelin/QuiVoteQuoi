//! Numero d'article cite par un texte libre (objet d'un scrutin, titre d'amendement).

use serde::Serialize;

/// Mentions latines qui suivent un numero d'article.
const LATIN: [&str; 12] = [
    "bis", "ter", "quater", "quinquies", "sexies", "septies", "octies", "nonies", "decies",
    "undecies", "duodecies", "terdecies",
];

/// Numero d'article tel que lu dans un texte: « 15 », « 1 », « 15 bis ».
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ArticleNumber(String);

impl ArticleNumber {
    /// Premier « article N » du texte; la position (avant, apres) n'est pas lue.
    pub fn in_text(text: &str) -> Option<Self> {
        let mut tokens = text.split_whitespace();
        while let Some(token) = tokens.next() {
            let word = token.rsplit(['\'', '\u{2019}']).next().unwrap_or(token);
            if !word.eq_ignore_ascii_case("article") {
                continue;
            }
            let number = tokens.next()?;
            let number = number.trim_end_matches([',', '.', ';']);
            let base = if number.eq_ignore_ascii_case("premier")
                || number.eq_ignore_ascii_case("1er")
            {
                "1".to_string()
            } else if !number.is_empty() && number.chars().all(|c| c.is_ascii_digit()) {
                number.to_string()
            } else {
                continue;
            };
            let latin = tokens
                .next()
                .map(|t| t.trim_end_matches([',', '.', ';']).to_lowercase())
                .filter(|t| LATIN.contains(&t.as_str()));
            return Some(Self(match latin {
                Some(l) => format!("{base} {l}"),
                None => base,
            }));
        }
        None
    }
}

impl std::fmt::Display for ArticleNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> Option<String> {
        ArticleNumber::in_text(text).map(|n| n.to_string())
    }

    #[test]
    fn should_read_cited_article_when_text_places_amendment_after_an_article() {
        assert_eq!(
            read("l'amendement n° 1376 de Mme Thomin après l'article 15 du projet de loi de finances"),
            Some("15".to_string())
        );
    }

    #[test]
    fn should_read_article_one_when_text_writes_premier_or_1er() {
        assert_eq!(
            read("l'amendement n° 4 de M. X à l'article premier du projet de loi"),
            Some("1".to_string())
        );
        assert_eq!(
            read("l'article 1er du projet de loi"),
            Some("1".to_string())
        );
    }

    #[test]
    fn should_keep_latin_mention_when_article_is_a_bis_one() {
        assert_eq!(
            read("l'article 15 bis du projet de loi"),
            Some("15 bis".to_string())
        );
    }
}
