//! Contenu des articles d'une version : numéro lu dans l'en-tête, empreinte
//! stable d'un article à l'autre, article supprimé (SPEC-en-bref RM-03,
//! RM-04, RM-06).
//!
//! Le découpage du HTML est une affaire d'infrastructure ; ce module garde les
//! règles de lecture qui ne dépendent pas du format : ce qu'est un en-tête
//! d'article, ce qui compte comme « le même contenu ».

/// Mentions latines qui suivent un numéro d'article.
const LATIN: [&str; 12] = [
    "bis",
    "ter",
    "quater",
    "quinquies",
    "sexies",
    "septies",
    "octies",
    "nonies",
    "decies",
    "undecies",
    "duodecies",
    "terdecies",
];

/// Ponctuation qui peut suivre un mot de l'en-tête sans en faire partie.
const TRAILING_PUNCTUATION: [char; 4] = [':', '.', ',', ';'];

/// Marqueurs de navette : ils disent où en est l'article, pas ce qu'il dit.
/// Comparés en minuscules, entre parenthèses (RM-03).
const SHUTTLE_MARKERS: [&str; 6] = [
    "nouveau",
    "nouvelle",
    "nouveaux",
    "nouvelles",
    "non modifié",
    "non modifiée",
];

/// Plus grand numéro d'alinéa retiré d'un début de ligne : « (1) », « (12) ».
const MAX_PARAGRAPH_NUMBER_DIGITS: usize = 3;

/// RM-04 : le numéro d'un article, lu dans son en-tête.
///
/// « Article 1er » et « Article premier » donnent `1er`, « Article 19 bis
/// (nouveau) » donne `19 bis`, « ARTICLE 8 : intitulé » donne `8` : le marqueur
/// de navette et l'intitulé rédigé par les auteurs ne sont pas le numéro. Une
/// phrase qui cite un article (« Article 2 de la loi est abrogé ») n'est pas un
/// en-tête. Seule la première ligne est lue : l'intitulé d'un projet de loi de
/// finances tient sur la ligne suivante.
pub fn article_label(heading: &str) -> Option<String> {
    let line = heading.lines().find(|line| !line.trim().is_empty())?;
    let tokens: Vec<&str> = line.split_whitespace().collect();
    let (keyword, rest) = tokens.split_first()?;
    if !keyword.eq_ignore_ascii_case("article") {
        return None;
    }
    let (number_token, mut rest) = rest.split_first()?;
    let number = number_token.trim_end_matches(TRAILING_PUNCTUATION);
    let mut ended = number.len() != number_token.len();
    let mut label = base_label(number)?;

    for accepts in [Suffix::Latin, Suffix::Letter] {
        if ended {
            break;
        }
        let Some((token, after)) = rest.split_first() else {
            break;
        };
        let word = token.trim_end_matches(match accepts {
            Suffix::Latin => TRAILING_PUNCTUATION.as_slice(),
            // Un « L. » de code n'est pas une lettre d'article.
            Suffix::Letter => [':', ',', ';'].as_slice(),
        });
        if let Some(suffix) = accepts.read(word) {
            label.push(' ');
            label.push_str(&suffix);
            ended = word.len() != token.len();
            rest = after;
        }
    }

    let heading_continues = rest
        .first()
        .is_none_or(|next| next.starts_with(['(', ':', '–', '—', '-']));
    (ended || heading_continues).then_some(label)
}

/// Ce qui peut suivre le numéro d'un article dans son en-tête.
#[derive(Clone, Copy)]
enum Suffix {
    /// « bis », « ter »...
    Latin,
    /// « A », « B » : « Article 2 bis A ».
    Letter,
}

impl Suffix {
    fn read(self, word: &str) -> Option<String> {
        match self {
            Suffix::Latin => {
                let lowered = word.to_lowercase();
                LATIN.contains(&lowered.as_str()).then_some(lowered)
            }
            Suffix::Letter => {
                let mut chars = word.chars();
                match (chars.next(), chars.next()) {
                    (Some(letter), None) if letter.is_ascii_uppercase() => Some(letter.to_string()),
                    _ => None,
                }
            }
        }
    }
}

fn base_label(word: &str) -> Option<String> {
    if word.eq_ignore_ascii_case("premier") || word.eq_ignore_ascii_case("1er") {
        return Some("1er".to_string());
    }
    if word.eq_ignore_ascii_case("liminaire") || word.eq_ignore_ascii_case("unique") {
        return Some(word.to_lowercase());
    }
    (!word.is_empty() && word.len() <= 4 && word.chars().all(|c| c.is_ascii_digit()))
        .then(|| word.to_string())
}

/// RM-03 : le contenu d'un article ramené à ce qui le distingue d'un autre.
///
/// Sont neutralisés les marqueurs de navette (« (nouveau) »), les numéros
/// d'alinéa en début de ligne (« (1) »), les guillemets, les apostrophes
/// typographiques, les traits d'union insécables, la casse, les espaces et
/// l'espace avant `:` `;` `!` `?`. Deux versions qui ne diffèrent que par ces
/// détails donnent le même texte ; un mot changé en donne un autre. Le numéro
/// de l'article n'en fait pas partie : il n'est pas dans le contenu.
pub fn normalized_content(content: &str) -> String {
    let without_numbers = content
        .lines()
        .map(strip_paragraph_number)
        .collect::<Vec<_>>()
        .join("\n");
    let without_markers = strip_shuttle_markers(&without_numbers.to_lowercase());

    let mut cleaned = String::with_capacity(without_markers.len());
    for character in without_markers.chars() {
        match character {
            '«' | '»' | '“' | '”' | '"' => cleaned.push(' '),
            '’' | '‘' => cleaned.push('\''),
            '\u{2010}' | '\u{2011}' => cleaned.push('-'),
            other => cleaned.push(other),
        }
    }

    let mut spaced = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    for punctuation in [" :", " ;", " !", " ?"] {
        spaced = spaced.replace(punctuation, &punctuation[1..]);
    }
    spaced
}

/// Retire « (n) » en début de ligne : le numéro d'alinéa dit la place, pas le
/// contenu.
fn strip_paragraph_number(line: &str) -> &str {
    let trimmed = line.trim_start();
    let Some(inner) = trimmed.strip_prefix('(') else {
        return line;
    };
    let digits = inner.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 || digits > MAX_PARAGRAPH_NUMBER_DIGITS {
        return line;
    }
    match inner[digits..].strip_prefix(')') {
        Some(after) => after.trim_start(),
        None => line,
    }
}

/// Retire les parenthèses qui ne contiennent qu'un marqueur de navette. Le texte
/// est déjà en minuscules.
fn strip_shuttle_markers(text: &str) -> String {
    let mut kept = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('(') {
        kept.push_str(&rest[..open]);
        let from_open = &rest[open..];
        match from_open.find(')') {
            Some(close) if SHUTTLE_MARKERS.contains(&from_open[1..close].trim()) => {
                rest = &from_open[close + 1..];
            }
            _ => {
                kept.push('(');
                rest = &from_open[1..];
            }
        }
    }
    kept.push_str(rest);
    kept
}

/// RM-06 : un article dont le contenu n'est que « (Supprimé) » est supprimé à
/// ce stade de la navette.
pub fn is_deleted(content: &str) -> bool {
    let lowered = content.to_lowercase();
    let core = lowered.trim_matches(|c: char| c.is_whitespace() || matches!(c, '(' | ')' | '.'));
    matches!(core, "supprimé" | "supprimée")
}

/// Identité du contenu normalisé d'un article (RM-03) : deux articles de même
/// empreinte ont le même contenu. Valeur opaque pour le domaine : le hachage
/// est calculé par l'adaptateur qui lit le texte.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArticleFingerprint(String);

impl ArticleFingerprint {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Champs d'un article d'une version.
#[derive(Debug, Clone)]
pub struct NewVersionArticle {
    pub label: String,
    pub division: Option<String>,
    pub rank: u16,
    pub content: String,
    pub fingerprint: ArticleFingerprint,
}

/// Article du dispositif d'une version (RM-04), rangé sous sa division.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionArticle {
    label: String,
    division: Option<String>,
    rank: u16,
    content: String,
    fingerprint: ArticleFingerprint,
}

impl VersionArticle {
    pub fn new(fields: NewVersionArticle) -> Self {
        Self {
            label: fields.label,
            division: fields.division,
            rank: fields.rank,
            content: fields.content,
            fingerprint: fields.fingerprint,
        }
    }

    /// Numéro tel que l'en-tête l'écrit (`1er`, `19 bis`, `liminaire`).
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Division sous laquelle l'article est rangé, si le texte en a.
    pub fn division(&self) -> Option<&str> {
        self.division.as_deref()
    }

    /// Place de l'article dans la version, à partir de 1.
    pub fn rank(&self) -> u16 {
        self.rank
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn fingerprint(&self) -> &ArticleFingerprint {
        &self.fingerprint
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label(heading: &str) -> Option<String> {
        article_label(heading)
    }

    #[test]
    fn should_read_label_when_heading_writes_article_premier_or_1er() {
        assert_eq!(label("Article 1er"), Some("1er".to_string()));
        assert_eq!(label("Article premier"), Some("1er".to_string()));
        assert_eq!(label("ARTICLE 1ER"), Some("1er".to_string()));
        assert_eq!(label("Article 2"), Some("2".to_string()));
        assert_eq!(label("Article unique"), Some("unique".to_string()));
    }

    #[test]
    fn should_keep_latin_mention_and_drop_marker_when_heading_is_19_bis_nouveau() {
        assert_eq!(
            label("Article 19 bis (nouveau)"),
            Some("19 bis".to_string())
        );
        assert_eq!(
            label("Article 2 bis A (nouveau)"),
            Some("2 bis A".to_string())
        );
        assert_eq!(label("Article 1er A"), Some("1er A".to_string()));
        assert_eq!(label("Article 4 TER"), Some("4 ter".to_string()));
    }

    #[test]
    fn should_drop_authors_heading_when_budget_article_has_one() {
        assert_eq!(
            label("ARTICLE liminaire : : Prévisions de solde structurel"),
            Some("liminaire".to_string())
        );
        assert_eq!(
            label("ARTICLE 8 : Soutien au logement social"),
            Some("8".to_string())
        );
        assert_eq!(
            label("ARTICLE 22 : : \nRenforcement de la taxe"),
            Some("22".to_string())
        );
        assert_eq!(
            label("ARTICLE 23 \nInstauration d'un prélèvement"),
            Some("23".to_string()),
            "an intitulé on its own line follows the heading"
        );
        assert_eq!(
            label("ARTICLE 23 Instauration d'un prélèvement"),
            None,
            "an intitulé on the heading line, with no separator, is a sentence"
        );
        assert_eq!(
            label("Article 4 – Mesure temporaire"),
            Some("4".to_string())
        );
        assert_eq!(label("Article 9: Soutien"), Some("9".to_string()));
    }

    #[test]
    fn should_refuse_label_when_block_is_a_sentence_citing_an_article() {
        for block in [
            "Article 2 de la loi du 9 avril 2024 est abrogé.",
            "Article 3 est ainsi rédigé",
            "L’article 2 est abrogé.",
            "Article L. 1111-12 du code de la santé publique",
            "Article",
            "Article deux",
            "",
            "Les articles 1er et 2",
        ] {
            assert_eq!(label(block), None, "{block:?} is not a heading");
        }
    }

    #[test]
    fn should_normalise_same_content_when_only_shuttle_markers_differ() {
        let second_reading = "4° (nouveau) Le médecin évalue la demande.";
        let third_reading = "4° Le médecin évalue la demande.";

        assert_eq!(
            normalized_content(second_reading),
            normalized_content(third_reading)
        );
        assert_eq!(
            normalized_content("Le texte (Non modifié) est adopté."),
            normalized_content("Le texte est adopté.")
        );
    }

    #[test]
    fn should_keep_parenthesis_when_it_is_not_a_shuttle_marker() {
        assert_ne!(
            normalized_content("Le seuil (2025) est maintenu."),
            normalized_content("Le seuil est maintenu.")
        );
    }

    #[test]
    fn should_normalise_same_content_when_only_paragraph_numbers_quotes_and_spaces_differ() {
        let numbered =
            "(1) Le droit à l’aide à mourir est reconnu ;\n(2)  « Nul ne peut   être contraint » !";
        let plain = "Le droit à l'aide à mourir est reconnu; Nul ne peut être contraint!";

        assert_eq!(normalized_content(numbered), normalized_content(plain));
        assert_eq!(
            normalized_content("Article L. 1110\u{2011}5 du code"),
            normalized_content("article L. 1110-5   du  code")
        );
        assert_eq!(
            normalized_content("deux\u{a0}points\u{202f}: oui"),
            normalized_content("Deux points: oui")
        );
    }

    #[test]
    fn should_normalise_different_content_when_a_word_changes() {
        assert_ne!(
            normalized_content("Le médecin évalue la demande."),
            normalized_content("Le médecin refuse la demande.")
        );
        assert_ne!(
            normalized_content("Un délai de trois mois"),
            normalized_content("Un délai de six mois")
        );
    }

    #[test]
    fn should_detect_deleted_article_when_content_is_only_supprime() {
        assert!(is_deleted("(Supprimé)"));
        assert!(is_deleted("  (supprimé)\n"));
        assert!(is_deleted("(Supprimée)"));
        assert!(!is_deleted("(Supprimé) puis un texte qui reste"));
        assert!(!is_deleted("Le texte"));
        assert!(!is_deleted(""));
    }

    #[test]
    fn should_expose_every_field_of_an_article_when_built() {
        let article = VersionArticle::new(NewVersionArticle {
            label: "19 bis".to_string(),
            division: Some("Chapitre II".to_string()),
            rank: 7,
            content: "Le texte.".to_string(),
            fingerprint: ArticleFingerprint::new("sha256:abc".to_string()),
        });

        assert_eq!(article.label(), "19 bis");
        assert_eq!(article.division(), Some("Chapitre II"));
        assert_eq!(article.rank(), 7);
        assert_eq!(article.content(), "Le texte.");
        assert_eq!(article.fingerprint().as_str(), "sha256:abc");
    }
}
