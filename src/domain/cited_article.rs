//! Articles de code et de loi qu'un article de texte modifie, et rédaction en
//! vigueur obtenue pour eux (SPEC-en-bref RM-07, RM-08).

/// Codes reconnus, au nom canonique de Légifrance (majuscule initiale). Un code
/// absent de cette liste n'est pas reconnu : le renvoi est ignoré, il ne bloque
/// pas (RM-08).
pub const KNOWN_CODES: [&str; 41] = [
    "Code civil",
    "Code pénal",
    "Code de la santé publique",
    "Code de la sécurité sociale",
    "Code du travail",
    "Code général des impôts",
    "Code de l'éducation",
    "Code de l'environnement",
    "Code général des collectivités territoriales",
    "Code de procédure pénale",
    "Code de la construction et de l'habitation",
    "Code de commerce",
    "Code monétaire et financier",
    "Code de l'action sociale et des familles",
    "Code de l'entrée et du séjour des étrangers et du droit d'asile",
    "Code électoral",
    "Code de la défense",
    "Code rural et de la pêche maritime",
    "Code de l'urbanisme",
    "Code de la route",
    "Code de l'énergie",
    "Code des transports",
    "Code de la consommation",
    "Code de la sécurité intérieure",
    "Code de justice administrative",
    "Code de procédure civile",
    "Code des assurances",
    "Code de la propriété intellectuelle",
    "Code de l'organisation judiciaire",
    "Code général de la fonction publique",
    "Code des postes et des communications électroniques",
    "Code des impositions sur les biens et services",
    "Code des juridictions financières",
    "Code de la recherche",
    "Code du cinéma et de l'image animée",
    "Code des pensions civiles et militaires de retraite",
    "Code de la mutualité",
    "Code des douanes",
    "Code du sport",
    "Code du tourisme",
    "Code général de la propriété des personnes publiques",
];

/// Mentions latines qui suivent un numéro d'article.
const LATIN: [&str; 26] = [
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
    "quaterdecies",
    "quindecies",
    "sexdecies",
    "septdecies",
    "octodecies",
    "novodecies",
    "vicies",
    "unvicies",
    "duovicies",
    "tervicies",
    "quatervicies",
    "quinvicies",
    "sexvicies",
    "septvicies",
];

/// Mots qui, avant « code », renvoient au dernier code nommé.
const SAME_CODE_MARKERS: [&str; 5] = ["même", "ce", "présent", "dudit", "ledit"];

/// Verbes qui, juste avant « un article », disent que le texte le crée.
const INSERTION_VERBS: [&str; 6] = [
    "inséré",
    "insérés",
    "rétabli",
    "rétablis",
    "ajouté",
    "ajoutés",
];

/// Verbes qui, après un numéro, disent que le texte rétablit l'article.
const RESTORATION_VERBS: [&str; 2] = ["rétabli", "rétablis"];

/// Mots qui peuvent séparer un numéro de son verbe de rétablissement.
const RESTORATION_FILLERS: [&str; 3] = ["est", "sont", "ainsi"];

/// Déterminants qui précèdent « article » quand le texte en crée un.
const DETERMINERS: [&str; 4] = ["un", "des", "deux", "trois"];

/// Mots au plus entre un verbe d'insertion suivi d'une virgule et le
/// déterminant de l'article inséré (« inséré, après l'article L. 1, un »).
const INTERPOSED_REACH: usize = 8;

/// Ponctuation qui ouvre un mot sans en faire partie.
const LEADING_PUNCTUATION: [char; 5] = ['(', '[', '«', '"', '“'];

/// Ponctuation qui ferme un mot sans en faire partie.
const TRAILING_PUNCTUATION: [char; 10] = [',', ';', ':', '.', ')', ']', '»', '"', '”', '!'];

/// Texte dont un article est cité.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CitedText {
    /// Un code, par son nom canonique (`Code de la santé publique`).
    Code(String),
    /// Une loi, par son numéro (`2024-322`).
    Law(String),
}

impl CitedText {
    /// Le texte comme on le nomme dans une phrase.
    pub fn label(&self) -> String {
        match self {
            CitedText::Code(name) => name.to_lowercase(),
            CitedText::Law(number) => format!("loi n° {number}"),
        }
    }
}

/// Un article cité par le contenu d'un article de texte (RM-07).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitedArticle {
    text: CitedText,
    number: String,
    cited_as: String,
    inserted: bool,
}

impl CitedArticle {
    pub fn new(text: CitedText, number: String, cited_as: String, inserted: bool) -> Self {
        Self {
            text,
            number,
            cited_as,
            inserted,
        }
    }

    pub fn text(&self) -> &CitedText {
        &self.text
    }

    /// Numéro normalisé : sans point ni espace avant le tiret (`L312-1`), la
    /// mention latine séparée par une espace (`6 nonies`).
    pub fn number(&self) -> &str {
        &self.number
    }

    /// Le numéro tel qu'il est écrit dans le texte (`L. 312-1`).
    pub fn cited_as(&self) -> &str {
        &self.cited_as
    }

    /// Vrai quand le texte insère ou rétablit l'article.
    pub fn inserted(&self) -> bool {
        self.inserted
    }

    /// Les articles cités par `content`, sans doublon, dans l'ordre
    /// d'apparition.
    pub fn cited_in(content: &str) -> Vec<CitedArticle> {
        let words = read_words(content);
        let mut found: Vec<CitedArticle> = Vec::new();
        let mut last_code: Option<&'static str> = None;
        // Le code qu'une phrase « Le code X est ainsi modifié : » ouvre, jusqu'à
        // la section suivante : ses articles sont cités sans préfixe (RM-07).
        let mut modified_code: Option<&'static str> = None;
        let mut at = 0;
        while at < words.len() {
            if is_article_keyword(&words[at]) {
                if let Some((readings, list_end)) = list_at(&words, at + 1) {
                    let designation = designation_at(&words, list_end);
                    let end = designation.as_ref().map_or(list_end, |(_, end)| *end);
                    let inserted_by_text =
                        introduces_insertion(&words[..at]) || restores(&words[end..]);
                    let designated = match designation.map(|(designation, _)| designation) {
                        Some(Designation::Code(mention)) => Some(
                            mention
                                .resolve(&mut last_code)
                                .map(|name| CitedText::Code(name.to_string())),
                        ),
                        Some(Designation::Law(number)) => Some(Some(CitedText::Law(number))),
                        None => None,
                    };
                    for reading in readings {
                        let text = match &designated {
                            Some(text) => text.clone(),
                            None if reading.prefixed => {
                                last_code.map(|name| CitedText::Code(name.to_string()))
                            }
                            None if names_a_text(&words[end..]) => None,
                            None => modified_code.map(|name| CitedText::Code(name.to_string())),
                        };
                        if let Some(text) = text {
                            record(
                                &mut found,
                                CitedArticle::new(
                                    text,
                                    reading.number,
                                    reading.cited_as,
                                    inserted_by_text,
                                ),
                            );
                        }
                    }
                    at = end;
                    continue;
                }
            } else if let Some((mention, end)) = code_mention_at(&words, at) {
                let named = mention.resolve(&mut last_code);
                if opens_modifications(&words[end..]) {
                    modified_code = named;
                }
                at = end;
                continue;
            } else if is_section_mark(&words[at]) {
                modified_code = None;
            }
            at += 1;
        }
        found
    }

    /// Vrai quand `raw` désigne le numéro de cet article, quelle que soit sa
    /// graphie (`L. 1110-5`, `L1110-5`).
    pub fn has_number(&self, raw: &str) -> bool {
        compact(raw) == compact(&self.number)
    }
}

/// Un doublon n'est pas gardé ; il marque l'article inséré si l'une de ses
/// mentions l'insère.
fn record(found: &mut Vec<CitedArticle>, article: CitedArticle) {
    match found
        .iter_mut()
        .find(|known| known.text == article.text && known.number == article.number)
    {
        Some(known) => known.inserted |= article.inserted,
        None => found.push(article),
    }
}

/// Un numéro comparable : sans espace ni point, en minuscules.
fn compact(number: &str) -> String {
    number
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '.')
        .map(|character| match character {
            '\u{2010}' | '\u{2011}' => '-',
            other => other,
        })
        .flat_map(char::to_lowercase)
        .collect()
}

/// Un mot du texte, lu sans la ponctuation qui l'entoure.
struct Word {
    /// Le mot, casse d'origine.
    bare: String,
    /// Le mot en minuscules, pour les comparaisons.
    lower: String,
    /// Premier signe de ponctuation qui suit le mot, s'il y en a un.
    after: Option<char>,
}

impl Word {
    fn read(token: &str) -> Option<Self> {
        let trimmed = token.trim_start_matches(LEADING_PUNCTUATION);
        let bare = trimmed.trim_end_matches(TRAILING_PUNCTUATION);
        if bare.is_empty() {
            return None;
        }
        Some(Self {
            bare: bare.to_string(),
            lower: bare.to_lowercase(),
            after: trimmed[bare.len()..].chars().next(),
        })
    }
}

/// Les mots du texte : apostrophes et traits d'union typographiques ramenés à
/// leur forme simple, espaces insécables comprises dans le découpage.
fn read_words(content: &str) -> Vec<Word> {
    let simplified: String = content
        .chars()
        .map(|character| match character {
            '\u{2018}' | '\u{2019}' => '\'',
            '\u{2010}' | '\u{2011}' => '-',
            other => other,
        })
        .collect();
    simplified
        .split_whitespace()
        .filter_map(Word::read)
        .collect()
}

/// « est ainsi modifié » : la suite énumère les modifications du texte nommé.
fn opens_modifications(after: &[Word]) -> bool {
    matches!(
        after,
        [is, thus, modified, ..]
            if is.lower == "est" && thus.lower == "ainsi" && modified.lower == "modifié"
    )
}

/// « de la présente loi », « du décret » : un numéro sans préfixe suivi d'un
/// complément appartient à ce texte, pas au code en cours de modification.
fn names_a_text(after: &[Word]) -> bool {
    after.first().is_some_and(|word| {
        matches!(word.lower.as_str(), "de" | "du" | "des") || word.lower.starts_with("d'")
    })
}

/// « II. » : une section numérotée en chiffres romains.
fn is_section_mark(word: &Word) -> bool {
    word.after == Some('.') && word.bare.chars().all(|c| matches!(c, 'I' | 'V' | 'X'))
}

fn is_article_keyword(word: &Word) -> bool {
    let last = word.lower.rsplit('\'').next().unwrap_or(&word.lower);
    last == "article" || last == "articles"
}

/// Des chiffres séparés par des tirets : `1110-5`, `1111-12-1`.
fn is_number(text: &str) -> bool {
    !text.is_empty()
        && text
            .split('-')
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
}

/// Le numéro d'une loi : `2024-322`.
fn is_law_number(text: &str) -> bool {
    is_number(text) && text.contains('-')
}

/// Le préfixe d'un numéro d'article de code, sous sa forme canonique et tel
/// qu'il est écrit : (`L`, `L.`), (`LO`, `L.O.`).
fn code_prefix(word: &Word) -> Option<(&'static str, String)> {
    let canonical = match (word.bare.as_str(), word.after) {
        ("L", Some('.')) => "L",
        ("R", Some('.')) => "R",
        ("D", Some('.')) => "D",
        ("LO", None | Some('.')) | ("L.O", Some('.')) => "LO",
        _ => return None,
    };
    let written = match word.after {
        Some('.') => format!("{}.", word.bare),
        _ => word.bare.clone(),
    };
    Some((canonical, written))
}

/// Un numéro d'article lu dans le texte.
struct Reading {
    /// Numéro normalisé.
    number: String,
    /// Numéro tel qu'il est écrit.
    cited_as: String,
    /// Le numéro porte un préfixe de code (`L.`, `R.`, `D.`, `LO`).
    prefixed: bool,
    /// Ponctuation qui suit le numéro.
    after: Option<char>,
}

fn reading_at(words: &[Word], at: usize) -> Option<(Reading, usize)> {
    let word = words.get(at)?;
    if let Some((prefix, written)) = code_prefix(word) {
        let digits = words.get(at + 1)?;
        if !is_number(&digits.bare) {
            return None;
        }
        let reading = Reading {
            number: format!("{prefix}{}", digits.bare),
            cited_as: format!("{written} {}", digits.bare),
            prefixed: true,
            after: digits.after,
        };
        return Some((reading, at + 2));
    }
    let base = if word.lower == "1er" || word.lower == "premier" {
        "1"
    } else if is_number(&word.bare) {
        &word.bare
    } else {
        return None;
    };
    let mut reading = Reading {
        number: base.to_string(),
        cited_as: word.bare.clone(),
        prefixed: false,
        after: word.after,
    };
    let mut next = at + 1;
    if reading.after.is_none() {
        if let Some(latin) = words
            .get(next)
            .filter(|word| LATIN.contains(&word.lower.as_str()))
        {
            reading.number = format!("{} {}", reading.number, latin.lower);
            reading.cited_as = format!("{} {}", reading.cited_as, latin.bare);
            reading.after = latin.after;
            next += 1;
            if reading.after.is_none() {
                if let Some(letter) = words.get(next).filter(|word| is_suffix_letter(word)) {
                    reading.number = format!("{} {}", reading.number, letter.bare);
                    reading.cited_as = format!("{} {}", reading.cited_as, letter.bare);
                    reading.after = letter.after;
                    next += 1;
                }
            }
        }
    }
    Some((reading, next))
}

/// La lettre qui suit une mention latine : `200 quater A`.
fn is_suffix_letter(word: &Word) -> bool {
    word.bare.len() == 1 && word.bare.chars().all(|c| c.is_ascii_uppercase())
}

/// Les numéros énumérés après « article » : `L. 1`, `L. 1 et L. 2`,
/// `L. 1, L. 2 et L. 3`.
fn list_at(words: &[Word], start: usize) -> Option<(Vec<Reading>, usize)> {
    let (first, mut end) = reading_at(words, start)?;
    let mut readings = vec![first];
    loop {
        let after = readings.last().and_then(|reading| reading.after);
        let separator = match after {
            Some(',') => 0,
            None if words
                .get(end)
                .is_some_and(|word| word.lower == "et" || word.lower == "ou") =>
            {
                1
            }
            _ => break,
        };
        let Some((reading, next)) = reading_at(words, end + separator) else {
            break;
        };
        readings.push(reading);
        end = next;
    }
    Some((readings, end))
}

/// Un code nommé dans le texte.
enum CodeMention {
    /// Un code de `KNOWN_CODES`.
    Named(&'static str),
    /// « le même code », « ce code », « dudit code ».
    Same,
    /// Un code hors de `KNOWN_CODES`.
    Unknown,
}

impl CodeMention {
    /// Met à jour le dernier code nommé et rend le code désigné, s'il est
    /// reconnu. Un code inconnu remet le dernier code à rien.
    fn resolve(&self, last_code: &mut Option<&'static str>) -> Option<&'static str> {
        match self {
            CodeMention::Named(name) => {
                *last_code = Some(name);
                Some(name)
            }
            CodeMention::Same => *last_code,
            CodeMention::Unknown => {
                *last_code = None;
                None
            }
        }
    }
}

/// Ce que désigne la suite d'un numéro d'article.
enum Designation {
    Code(CodeMention),
    Law(String),
}

fn code_mention_at(words: &[Word], at: usize) -> Option<(CodeMention, usize)> {
    let word = words.get(at)?;
    if SAME_CODE_MARKERS.contains(&word.lower.as_str())
        && words.get(at + 1).is_some_and(|next| next.lower == "code")
    {
        return Some((CodeMention::Same, at + 2));
    }
    if word.lower != "code" {
        return None;
    }
    match known_code_at(words, at) {
        Some((name, length)) => Some((CodeMention::Named(name), at + length)),
        None => Some((CodeMention::Unknown, at + 1)),
    }
}

/// Le plus long code de `KNOWN_CODES` écrit à partir de `at`.
fn known_code_at(words: &[Word], at: usize) -> Option<(&'static str, usize)> {
    KNOWN_CODES
        .iter()
        .filter_map(|name| {
            let lowered = name.to_lowercase();
            let parts: Vec<&str> = lowered.split_whitespace().collect();
            let candidate = words.get(at..at + parts.len())?;
            let same = candidate
                .iter()
                .zip(&parts)
                .all(|(word, part)| word.lower == *part);
            same.then_some((*name, parts.len()))
        })
        .max_by_key(|(_, length)| *length)
}

fn law_at(words: &[Word], at: usize) -> Option<(String, usize)> {
    if words.get(at)?.lower != "loi" {
        return None;
    }
    let mut next = at + 1;
    if words
        .get(next)
        .is_some_and(|word| word.lower == "organique")
    {
        next += 1;
    }
    let marker = &words.get(next)?.lower;
    let (number, end) = match ["n°", "nº"]
        .iter()
        .find_map(|prefix| marker.strip_prefix(prefix))
    {
        Some("") => (words.get(next + 1)?.bare.as_str(), next + 2),
        Some(fused) => (fused, next + 1),
        None => return None,
    };
    is_law_number(number).then(|| (number.to_string(), end))
}

/// Le code ou la loi que désigne ce qui suit les numéros : « du code de la
/// santé publique », « du même code », « de la loi n° 2024-322 ».
fn designation_at(words: &[Word], at: usize) -> Option<(Designation, usize)> {
    let mut start = at;
    if words
        .get(start)
        .is_some_and(|word| word.lower == "du" || word.lower == "de")
    {
        start += 1;
    }
    if words.get(start).is_some_and(|word| word.lower == "la") {
        start += 1;
    }
    if let Some((mention, end)) = code_mention_at(words, start) {
        return Some((Designation::Code(mention), end));
    }
    law_at(words, start).map(|(number, end)| (Designation::Law(number), end))
}

/// « il est inséré un article », « est complété par un article » : les mots
/// qui précèdent « article » disent que le texte le crée.
fn introduces_insertion(before: &[Word]) -> bool {
    let words: Vec<&str> = before.iter().map(|word| word.lower.as_str()).collect();
    let inserted = matches!(
        words.as_slice(),
        [.., verb, determiner]
            if INSERTION_VERBS.contains(verb) && DETERMINERS.contains(determiner)
    );
    let completed = matches!(
        words.as_slice(),
        [.., verb, "par", determiner]
            if verb.starts_with("complété") && DETERMINERS.contains(determiner)
    );
    // « il est inséré, après l'article X, un article Y » : le renvoi intercalé
    // est encadré de virgules entre le verbe et le déterminant.
    let interposed = matches!(
        before,
        [.., reference_end, determiner]
            if DETERMINERS.contains(&determiner.lower.as_str())
                && reference_end.after == Some(',')
    ) && before
        .iter()
        .rev()
        .skip(2)
        .take(INTERPOSED_REACH)
        .any(|word| INSERTION_VERBS.contains(&word.lower.as_str()) && word.after == Some(','));
    inserted || completed || interposed
}

/// « est ainsi rétabli » : les mots qui suivent le numéro disent que le texte
/// rétablit l'article.
fn restores(after: &[Word]) -> bool {
    after
        .iter()
        .find(|word| !RESTORATION_FILLERS.contains(&word.lower.as_str()))
        .is_some_and(|word| RESTORATION_VERBS.contains(&word.lower.as_str()))
}

/// Rédaction d'un article cité à la date de la version (RM-07).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CitedWording {
    /// Rédaction en vigueur, avec le lien vers le texte officiel.
    InForce { text: String, official_url: String },
    /// L'article est créé par le texte : il n'a pas de rédaction.
    Created,
    /// La source ne connaît pas cet article : renvoi non reconnu, il ne bloque
    /// pas (RM-08).
    NotFound,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code(name: &str) -> CitedText {
        CitedText::Code(name.to_string())
    }

    fn cited(text: CitedText, number: &str, cited_as: &str, inserted: bool) -> CitedArticle {
        CitedArticle::new(text, number.to_string(), cited_as.to_string(), inserted)
    }

    #[test]
    fn should_cite_law_article_when_text_names_article_of_a_numbered_law() {
        let content = "Le III de l'article 51 de la loi n° 2024\u{2011}322 du 9 avril 2024 de finances pour 2024 est abrogée.";

        let found = CitedArticle::cited_in(content);

        assert_eq!(
            found,
            vec![cited(
                CitedText::Law("2024-322".to_string()),
                "51",
                "51",
                false
            )]
        );
        assert_eq!(found[0].text().label(), "loi n° 2024-322");
    }

    #[test]
    fn should_cite_code_article_when_text_names_article_of_a_known_code() {
        let content = "L'article L. 1110-5 du code de la santé publique est complété par un alinéa ainsi rédigé.";

        let found = CitedArticle::cited_in(content);

        assert_eq!(
            found,
            vec![cited(
                code("Code de la santé publique"),
                "L1110-5",
                "L. 1110-5",
                false
            )]
        );
        assert_eq!(found[0].text().label(), "code de la santé publique");
    }

    #[test]
    fn should_attach_prefixed_number_to_last_named_code_when_no_text_follows() {
        let content = "Le code de la santé publique est ainsi modifié :\n1° L'article L. 1110-5 est complété par un alinéa ainsi rédigé ;\n2° L'article R. 1110-1 est abrogé.";

        let found = CitedArticle::cited_in(content);

        assert_eq!(
            found,
            vec![
                cited(
                    code("Code de la santé publique"),
                    "L1110-5",
                    "L. 1110-5",
                    false
                ),
                cited(
                    code("Code de la santé publique"),
                    "R1110-1",
                    "R. 1110-1",
                    false
                ),
            ]
        );
    }

    #[test]
    fn should_attach_to_last_named_code_when_text_says_meme_code() {
        let content = "Le code général des impôts est ainsi modifié : 1° À l'article 200 quater du même code, le montant est remplacé ; 2° À l'article 244 de ce code, le taux est réduit ; 3° À l'article 790 dudit code, les mots sont supprimés.";

        let found = CitedArticle::cited_in(content);

        let tax = code("Code général des impôts");
        assert_eq!(
            found,
            vec![
                cited(tax.clone(), "200 quater", "200 quater", false),
                cited(tax.clone(), "244", "244", false),
                cited(tax, "790", "790", false),
            ]
        );
    }

    #[test]
    fn should_cite_each_article_when_text_lists_several() {
        let content = "Le code de la santé publique est ainsi modifié : aux articles L. 1 et L. 2 du même code, les mots sont supprimés.";
        let with_commas =
            "Aux articles L. 1, L. 2 et L. 3 du code de l'éducation, les mots sont supprimés.";
        let repeated = "Le code civil est ainsi modifié : à l'article L. 1, les mots sont supprimés ; à l'article L. 2 du même code, ils sont rétablis ; l'article L. 1 est abrogé.";

        let santé = code("Code de la santé publique");
        let education = code("Code de l'éducation");
        let civil = code("Code civil");
        assert_eq!(
            CitedArticle::cited_in(content),
            vec![
                cited(santé.clone(), "L1", "L. 1", false),
                cited(santé, "L2", "L. 2", false),
            ]
        );
        assert_eq!(
            CitedArticle::cited_in(with_commas),
            vec![
                cited(education.clone(), "L1", "L. 1", false),
                cited(education.clone(), "L2", "L. 2", false),
                cited(education, "L3", "L. 3", false),
            ]
        );
        assert_eq!(
            CitedArticle::cited_in(repeated),
            vec![
                cited(civil.clone(), "L1", "L. 1", false),
                cited(civil, "L2", "L. 2", false),
            ]
        );
    }

    #[test]
    fn should_ignore_unknown_code_when_name_is_not_listed() {
        let content = "L'article L. 1 du code du patrimoine est abrogé ; l'article L. 2 du code civil est abrogé.";

        assert_eq!(
            CitedArticle::cited_in(content),
            vec![cited(code("Code civil"), "L2", "L. 2", false)]
        );
    }

    #[test]
    fn should_normalise_number_without_dot_when_number_has_a_letter_prefix() {
        let content = "Le code de commerce est ainsi modifié : l'article L. 312-1, l'article R. 2-1, l'article D. 5, l'article LO 135 et l'article L.O. 136 sont abrogés ; l'article 6 nonies du code général des impôts, l'article 200 quater A du même code et l'article 1er de la loi n° 78-17 aussi.";

        let found = CitedArticle::cited_in(content);

        let numbers: Vec<(&str, &str)> = found
            .iter()
            .map(|article| (article.number(), article.cited_as()))
            .collect();
        assert_eq!(
            numbers,
            vec![
                ("L312-1", "L. 312-1"),
                ("R2-1", "R. 2-1"),
                ("D5", "D. 5"),
                ("LO135", "LO 135"),
                ("LO136", "L.O. 136"),
                ("6 nonies", "6 nonies"),
                ("200 quater A", "200 quater A"),
                ("1", "1er"),
            ]
        );
        let first = &found[0];
        assert!(first.has_number("L312-1"));
        assert!(first.has_number("L. 312-1"));
        assert!(first.has_number("l312-1"));
        assert!(!first.has_number("L312-2"));
        assert!(found[5].has_number("6 nonies"));
        assert!(!found[5].has_number("6 ter"));
    }

    #[test]
    fn should_ignore_meme_code_when_last_named_code_is_unknown() {
        let after_unknown = "Le code du patrimoine est ainsi modifié : 1° À l'article L. 1 du même code ; 2° L'article L. 2 est abrogé.";
        let reset_by_unknown = "Le code civil est ainsi modifié : à l'article L. 1, les mots sont supprimés ; le code du patrimoine est ainsi modifié : à l'article L. 2 du même code, les mots sont supprimés.";

        assert_eq!(CitedArticle::cited_in(after_unknown), Vec::new());
        assert_eq!(
            CitedArticle::cited_in(reset_by_unknown),
            vec![cited(code("Code civil"), "L1", "L. 1", false)]
        );
    }

    #[test]
    fn should_mark_article_inserted_when_insertion_verb_precedes_an_interposed_reference() {
        let content = "Le code de la santé publique est ainsi modifié : il est inséré, après l'article L. 1111-12, un article L. 1111-12-1 ainsi rédigé :";

        let santé = code("Code de la santé publique");
        assert_eq!(
            CitedArticle::cited_in(content),
            vec![
                cited(santé.clone(), "L1111-12", "L. 1111-12", false),
                cited(santé, "L1111-12-1", "L. 1111-12-1", true),
            ]
        );
    }

    #[test]
    fn should_mark_article_inserted_when_text_inserts_it() {
        let inserted = "Le code de la santé publique est ainsi modifié : après l'article L. 1111-12, il est inséré un article L. 1111-12-1 ainsi rédigé :";
        let completed = "Le code de la santé publique est ainsi modifié : l'article L. 1110-5 est complété par un article L. 1110-5-1 ainsi rédigé :";
        let restored = "Le code de la santé publique est ainsi modifié : 1° L'article L. 1111-3 est ainsi rétabli ; 2° L'article L. 1111-4 est abrogé.";

        let santé = code("Code de la santé publique");
        assert_eq!(
            CitedArticle::cited_in(inserted),
            vec![
                cited(santé.clone(), "L1111-12", "L. 1111-12", false),
                cited(santé.clone(), "L1111-12-1", "L. 1111-12-1", true),
            ]
        );
        assert_eq!(
            CitedArticle::cited_in(completed),
            vec![
                cited(santé.clone(), "L1110-5", "L. 1110-5", false),
                cited(santé.clone(), "L1110-5-1", "L. 1110-5-1", true),
            ]
        );
        assert_eq!(
            CitedArticle::cited_in(restored),
            vec![
                cited(santé.clone(), "L1111-3", "L. 1111-3", true),
                cited(santé, "L1111-4", "L. 1111-4", false),
            ]
        );
    }

    #[test]
    fn should_cite_article_of_the_code_des_impositions_when_text_names_it() {
        let content = "Le 2° de l'article L. 425-12 du code des impositions sur les biens et services est remplacé.";

        assert_eq!(
            CitedArticle::cited_in(content),
            vec![cited(
                code("Code des impositions sur les biens et services"),
                "L425-12",
                "L. 425-12",
                false
            )]
        );
    }

    #[test]
    fn should_attach_bare_numbers_to_the_code_being_modified() {
        let content = "Le code général des impôts est ainsi modifié : 1° Au premier alinéa du 1 des articles 39 bis A, 39 bis B et 199 unvicies et au VIII de l'article 244 quater O, l'année est remplacée.";
        let cgi = || code("Code général des impôts");

        assert_eq!(
            CitedArticle::cited_in(content),
            vec![
                cited(cgi(), "39 bis A", "39 bis A", false),
                cited(cgi(), "39 bis B", "39 bis B", false),
                cited(cgi(), "199 unvicies", "199 unvicies", false),
                cited(cgi(), "244 quater O", "244 quater O", false),
            ]
        );
    }

    #[test]
    fn should_not_attach_bare_number_when_the_code_is_only_mentioned() {
        let content = "Le code général des impôts s'applique. L'article 5 entre en vigueur en 2027.";

        assert_eq!(CitedArticle::cited_in(content), vec![]);
    }

    #[test]
    fn should_not_attach_bare_number_to_the_modified_code_when_it_names_another_text() {
        let content = "Le code général des impôts est ainsi modifié : au sens de l'article 12 de la présente loi, le taux est remplacé.";

        assert_eq!(CitedArticle::cited_in(content), vec![]);
    }

    #[test]
    fn should_stop_attaching_bare_numbers_at_the_next_section() {
        let content = "I. – Le code général des impôts est ainsi modifié : à l'article 39, le mot est supprimé. II. – L'article 5 s'applique en 2027.";

        assert_eq!(
            CitedArticle::cited_in(content),
            vec![cited(code("Code général des impôts"), "39", "39", false)]
        );
    }
}
