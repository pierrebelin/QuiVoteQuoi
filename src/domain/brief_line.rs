//! Contrôle d'une ligne « En bref » proposée par le modèle (SPEC-en-bref
//! RM-10, RM-11, RM-12).
//!
//! Le code décide seul si une ligne est publiable : longueur (RM-10), nombres
//! présents dans les sources (RM-11), absence d'intention et de jugement
//! (RM-12). Une ligne sur plusieurs paragraphes est refusée avec elles.
//! Fonction pure : aucune entrée-sortie, aucune horloge, aucune dépendance de
//! lecture de texte (les nombres sont lus à la main).

/// RM-10 : nombre maximal de mots d'une ligne.
pub const MAX_LINE_WORDS: usize = 25;

/// RM-12 : termes qui prêtent une intention (« vise à », « afin de ») ou
/// portent un jugement (« important », « historique »). Chaque forme est écrite
/// en toutes lettres : genre, nombre, conjugaison. Comparaison sans casse, sur
/// mots entiers.
pub const FORBIDDEN_WORDING: &[&str] = &[
    "vise à",
    "visent à",
    "visant à",
    "a pour objectif",
    "ont pour objectif",
    "a pour but",
    "ont pour but",
    "afin de",
    // « afin d'éviter » : l'apostrophe borne le mot.
    "afin d",
    "important",
    "importante",
    "importants",
    "importantes",
    "significatif",
    "significatifs",
    "significative",
    "significatives",
    "historique",
    "historiques",
    "ambitieux",
    "ambitieuse",
    "ambitieuses",
    "efficace",
    "efficaces",
    "injuste",
    "injustes",
];

/// Mots qui, placés avant un nombre, le qualifient (« article 13 »).
const REFERRAL_WORDS: &[&str] = &["article", "alinéa", "n°", "numéro"];

/// Mots vides sautés quand on cherche le mot qui suit un nombre.
const STOP_WORDS: &[&str] = &[
    "de", "d'", "du", "des", "la", "le", "les", "l'", "à", "au", "aux", "en", "et",
];

/// Séparateurs admis entre deux groupes de chiffres : espace, espace
/// insécable, espace fine insécable, espace fine.
const GROUP_SEPARATORS: &[char] = &[' ', '\u{a0}', '\u{202f}', '\u{2009}'];

/// Suffixes d'un ordinal écrit en chiffres (« 1er », « 2e », « 2ème »).
const ORDINAL_SUFFIXES: &[&str] = &["er", "re", "ère", "ere", "e", "ème", "eme"];

/// Nombres en lettres d'un seul mot, avec leur valeur.
const CARDINALS: &[(&str, u64)] = &[
    ("un", 1),
    ("une", 1),
    ("deux", 2),
    ("trois", 3),
    ("quatre", 4),
    ("cinq", 5),
    ("six", 6),
    ("sept", 7),
    ("huit", 8),
    ("neuf", 9),
    ("dix", 10),
    ("onze", 11),
    ("douze", 12),
    ("treize", 13),
    ("quatorze", 14),
    ("quinze", 15),
    ("seize", 16),
    ("vingt", 20),
    ("vingts", 20),
    ("trente", 30),
    ("quarante", 40),
    ("cinquante", 50),
    ("soixante", 60),
    ("cent", 100),
    ("cents", 100),
    ("mille", 1000),
];

/// Motif de refus d'une ligne proposée par le modèle.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum BriefLineError {
    #[error("brief line is empty")]
    Empty,
    #[error("brief line must be a single paragraph")]
    MultipleParagraphs,
    #[error("brief line is too long: {0} words")]
    TooLong(usize),
    #[error("brief line states a number absent from the sources: {0}")]
    UnsourcedNumber(String),
    #[error("brief line uses forbidden wording: {0}")]
    ForbiddenWording(String),
}

impl BriefLineError {
    /// Code stable du motif de refus, stocké avec son texte : il n'est jamais
    /// relu en énumération.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::MultipleParagraphs => "multiple_paragraphs",
            Self::TooLong(_) => "too_long",
            Self::ForbiddenWording(_) => "forbidden_wording",
            Self::UnsourcedNumber(_) => "unsourced_number",
        }
    }
}

/// Ligne « En bref » conforme : seule porte d'entrée d'une ligne publiée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BriefLine(String);

impl BriefLine {
    /// Contrôle une ligne brute du modèle contre les sources de l'article.
    ///
    /// Ordre des contrôles : vide, plusieurs paragraphes, longueur (RM-10),
    /// vocabulaire interdit (RM-12), nombres (RM-11).
    pub fn check(raw: &str, sources: &[&str]) -> Result<Self, BriefLineError> {
        let text = raw.trim();
        if text.is_empty() {
            return Err(BriefLineError::Empty);
        }
        if text.contains('\n') || text.contains('\r') {
            return Err(BriefLineError::MultipleParagraphs);
        }
        let words = word_count(text);
        if words > MAX_LINE_WORDS {
            return Err(BriefLineError::TooLong(words));
        }
        if let Some(term) = forbidden_term(text) {
            return Err(BriefLineError::ForbiddenWording(term.to_string()));
        }
        let sourced: Vec<Quantity> = sources
            .iter()
            .flat_map(|source| source_quantities(source))
            .collect();
        if let Some(unsourced) = quantities(text)
            .into_iter()
            .find(|quantity| !sourced.contains(quantity))
        {
            return Err(BriefLineError::UnsourcedNumber(unsourced.describe()));
        }
        Ok(Self(text.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// RM-10 : nombre de mots d'un texte. Un mot est une suite de caractères sans
/// espace : un mot élidé (« l'article ») compte pour un.
pub fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

/// RM-12 : premier terme interdit de la liste que le texte contient, sans
/// casse, sur mots entiers, quelle que soit la forme des espaces entre les mots
/// du terme.
fn forbidden_term(text: &str) -> Option<&'static str> {
    let lowered = text.to_lowercase();
    let normalized = lowered.split_whitespace().collect::<Vec<_>>().join(" ");
    FORBIDDEN_WORDING
        .iter()
        .copied()
        .find(|term| contains_whole_words(&normalized, term))
}

fn contains_whole_words(text: &str, term: &str) -> bool {
    text.match_indices(term).any(|(start, found)| {
        let before = text[..start].chars().next_back();
        let after = text[start + found.len()..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

/// RM-11 : un nombre lu dans un texte avec le mot qui le qualifie.
///
/// `value` est la valeur normalisée (`1500`, `10.5`), `qualifier` le mot
/// qualifiant normalisé (minuscules, `s` final retiré, `%` et `€` gardés).
/// « 3 mois » et « trois mois » donnent la même quantité ; « l'article 13 » et
/// « 13 jours » deux quantités différentes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quantity {
    pub value: String,
    pub qualifier: String,
}

impl Quantity {
    /// Texte lu, pour le motif de refus : valeur puis mot qualifiant.
    fn describe(&self) -> String {
        if self.qualifier.is_empty() {
            self.value.clone()
        } else {
            format!("{} {}", self.value, self.qualifier)
        }
    }
}

/// RM-11 : quantités d'une ligne. « un » et « une » y sont des articles
/// indéfinis, jamais contrôlés.
pub fn quantities(text: &str) -> Vec<Quantity> {
    read_quantities(text)
        .into_iter()
        .filter(|reading| !reading.indefinite)
        .map(|reading| reading.quantity)
        .collect()
}

/// RM-11 : quantités des sources. « un » et « une » y valent 1.
pub fn source_quantities(text: &str) -> Vec<Quantity> {
    read_quantities(text)
        .into_iter()
        .map(|reading| reading.quantity)
        .collect()
}

struct Reading {
    quantity: Quantity,
    /// Le nombre est un seul « un » ou « une » : article indéfini dans une ligne.
    indefinite: bool,
}

/// Un morceau de texte lu : mot, nombre en lettres (avant assemblage), nombre,
/// ou toute autre ponctuation (qui borne la recherche du mot qualifiant).
#[derive(Debug, Clone)]
enum Item {
    Word(String),
    Atom(Atom),
    Number(Number),
    Break,
}

/// Un mot de nombre en lettres (« vingt », « cents », « deuxième »).
#[derive(Debug, Clone, Copy)]
struct Atom {
    value: u64,
    ordinal: bool,
    indefinite: bool,
}

#[derive(Debug, Clone)]
struct Number {
    value: String,
    year: bool,
    indefinite: bool,
}

fn read_quantities(text: &str) -> Vec<Reading> {
    let items = read_letter_numbers(merge_pour_cent(tokenize(text)));
    items
        .iter()
        .enumerate()
        .filter_map(|(at, item)| match item {
            Item::Number(number) => Some(Reading {
                quantity: Quantity {
                    value: number.value.clone(),
                    qualifier: qualifier(&items, at, number.year),
                },
                indefinite: number.indefinite,
            }),
            _ => None,
        })
        .collect()
}

fn tokenize(text: &str) -> Vec<Item> {
    let chars: Vec<char> = text.to_lowercase().replace('’', "'").chars().collect();
    let mut items = Vec::new();
    let mut at = 0;
    while at < chars.len() {
        let c = chars[at];
        if c.is_ascii_digit() {
            at = read_digits(&chars, at, &mut items);
        } else if is_word_char(c) {
            at = read_word(&chars, at, &mut items);
        } else {
            if c == '%' || c == '€' {
                items.push(Item::Word(c.to_string()));
            } else if !c.is_whitespace() {
                items.push(Item::Break);
            }
            at += 1;
        }
    }
    items
}

fn is_word_char(c: char) -> bool {
    c.is_alphabetic() || c == '°'
}

fn digit_run(chars: &[char], from: usize) -> usize {
    chars
        .iter()
        .skip(from)
        .take_while(|c| c.is_ascii_digit())
        .count()
}

fn read_word(chars: &[char], start: usize, items: &mut Vec<Item>) -> usize {
    let mut end = start;
    let mut token = String::new();
    while let Some(&c) = chars.get(end) {
        let joins_parts = c == '-' && chars.get(end + 1).is_some_and(|next| is_word_char(*next));
        if !is_word_char(c) && !joins_parts {
            break;
        }
        token.push(c);
        end += 1;
    }
    let elided = chars.get(end) == Some(&'\'')
        && chars.get(end + 1).is_some_and(|next| next.is_alphabetic());
    if elided {
        token.push('\'');
        end += 1;
    }
    push_word(token, items);
    end
}

/// Un mot composé de tirets n'est un nombre que si toutes ses parties sont des
/// mots de nombre (ou « et ») : « vingt-cinq » oui, « cinq-mâts » non.
fn push_word(token: String, items: &mut Vec<Item>) {
    let pieces: Option<Vec<Item>> = token
        .split('-')
        .map(|part| {
            if part == "et" {
                Some(Item::Word(part.to_string()))
            } else {
                atom_of(part).map(Item::Atom)
            }
        })
        .collect();
    match pieces {
        Some(pieces) if pieces.iter().any(|piece| matches!(piece, Item::Atom(_))) => {
            items.extend(pieces);
        }
        _ => items.push(Item::Word(token)),
    }
}

fn atom_of(word: &str) -> Option<Atom> {
    if matches!(word, "premier" | "première") {
        return Some(Atom {
            value: 1,
            ordinal: true,
            indefinite: false,
        });
    }
    if let Some(&(_, value)) = CARDINALS.iter().find(|(name, _)| *name == word) {
        return Some(Atom {
            value,
            ordinal: false,
            indefinite: matches!(word, "un" | "une"),
        });
    }
    let stem = word.strip_suffix("ième")?;
    CARDINALS
        .iter()
        .find(|(name, _)| ordinal_stem(name) == stem)
        .map(|&(_, value)| Atom {
            value,
            ordinal: true,
            indefinite: false,
        })
}

/// Radical d'un nombre devant « ième » : « quatre » → « quatr », « cinq » →
/// « cinqu », « neuf » → « neuv ».
fn ordinal_stem(name: &str) -> &str {
    match name {
        "cinq" => "cinqu",
        "neuf" => "neuv",
        _ => name.strip_suffix('e').unwrap_or(name),
    }
}

fn read_digits(chars: &[char], start: usize, items: &mut Vec<Item>) -> usize {
    let mut end = start + digit_run(chars, start);
    let first_group_len = end - start;
    let mut integer: String = chars[start..end].iter().collect();
    let mut grouped = false;
    if first_group_len <= 3 {
        while chars.get(end).is_some_and(|c| GROUP_SEPARATORS.contains(c))
            && digit_run(chars, end + 1) == 3
        {
            integer.extend(&chars[end + 1..end + 4]);
            end += 4;
            grouped = true;
        }
    }
    let mut fraction = String::new();
    if chars.get(end) == Some(&',') {
        let length = digit_run(chars, end + 1);
        if length > 0 {
            fraction.extend(&chars[end + 1..end + 1 + length]);
            end += 1 + length;
        }
    }
    let suffix_len = chars
        .iter()
        .skip(end)
        .take_while(|c| c.is_alphabetic())
        .count();
    let suffix: String = chars[end..end + suffix_len].iter().collect();
    if ORDINAL_SUFFIXES.contains(&suffix.as_str()) {
        end += suffix_len;
    }

    let year = !grouped
        && fraction.is_empty()
        && first_group_len == 4
        && matches!(integer.parse::<u32>(), Ok(1800..=2100));
    let integer = integer.trim_start_matches('0');
    let integer = if integer.is_empty() { "0" } else { integer };
    let fraction = fraction.trim_end_matches('0');
    let value = if fraction.is_empty() {
        integer.to_string()
    } else {
        format!("{integer}.{fraction}")
    };
    items.push(Item::Number(Number {
        value,
        year,
        indefinite: false,
    }));
    end
}

/// « pour cent » vaut « % ».
fn merge_pour_cent(items: Vec<Item>) -> Vec<Item> {
    let mut merged = Vec::with_capacity(items.len());
    let mut rest = items.into_iter().peekable();
    while let Some(item) = rest.next() {
        let is_pour = matches!(&item, Item::Word(word) if word == "pour");
        let cent_follows =
            matches!(rest.peek(), Some(Item::Atom(atom)) if atom.value == 100 && !atom.ordinal);
        if is_pour && cent_follows {
            rest.next();
            merged.push(Item::Word("%".to_string()));
        } else {
            merged.push(item);
        }
    }
    merged
}

/// Assemble les mots de nombre consécutifs en un seul nombre.
fn read_letter_numbers(items: Vec<Item>) -> Vec<Item> {
    let mut out = Vec::with_capacity(items.len());
    let mut at = 0;
    while at < items.len() {
        let Item::Atom(first) = &items[at] else {
            out.push(items[at].clone());
            at += 1;
            continue;
        };
        let mut number = LetterNumber::start(*first);
        at += 1;
        loop {
            match (items.get(at), items.get(at + 1)) {
                (Some(Item::Atom(atom)), _) if number.accepts(*atom) => {
                    number.push(*atom);
                    at += 1;
                }
                (Some(Item::Word(word)), Some(Item::Atom(atom)))
                    if word == "et" && number.accepts_after_et(*atom) =>
                {
                    number.push(*atom);
                    at += 2;
                }
                _ => break,
            }
        }
        out.push(Item::Number(number.finish()));
    }
    out
}

/// Nombre en lettres en cours de lecture : milliers faits (`total`) et
/// reste inférieur à mille (`current`).
struct LetterNumber {
    total: u64,
    current: u64,
    last: u64,
    done: bool,
    count: usize,
    indefinite: bool,
}

impl LetterNumber {
    fn start(first: Atom) -> Self {
        let mut number = Self {
            total: 0,
            current: 0,
            last: 0,
            done: false,
            count: 0,
            indefinite: first.indefinite,
        };
        number.push(first);
        number
    }

    /// Le mot suivant prolonge-t-il le nombre (« vingt » + « cinq ») ou en
    /// commence-t-il un autre (« trois » + « quatre ») ?
    fn accepts(&self, atom: Atom) -> bool {
        if self.done {
            return false;
        }
        let rest = self.current % 100;
        match atom.value {
            1..=9 => rest.is_multiple_of(10),
            10..=16 => matches!(rest, 0 | 60 | 80),
            20 => rest == 0 || rest == 4,
            30 | 40 | 50 | 60 => rest == 0,
            100 => self.current < 10,
            1000 => self.total == 0,
            _ => false,
        }
    }

    /// « vingt et un », « soixante et onze » : « et » ne relie que les dizaines
    /// de vingt à soixante à « un » ou « onze ».
    fn accepts_after_et(&self, atom: Atom) -> bool {
        matches!(self.last, 20 | 30 | 40 | 50 | 60)
            && self.current % 100 == self.last
            && matches!(atom.value, 1 | 11)
            && self.accepts(atom)
    }

    fn push(&mut self, atom: Atom) {
        match atom.value {
            20 if self.current % 100 == 4 => self.current += 76,
            100 => self.current = self.current.max(1) * 100,
            1000 => {
                self.total = self.current.max(1) * 1000;
                self.current = 0;
            }
            value => self.current += value,
        }
        self.last = atom.value;
        self.done = atom.ordinal;
        self.count += 1;
    }

    /// Une année se lit en lettres comme en chiffres, sauf un compte rond de
    /// centaines : « deux mille euros » est un montant.
    fn finish(self) -> Number {
        let value = self.total + self.current;
        Number {
            value: value.to_string(),
            year: (1800..=2100).contains(&value) && !value.is_multiple_of(100),
            indefinite: self.count == 1 && self.indefinite,
        }
    }
}

/// Mot qualifiant du nombre en `at` : mot de renvoi qui le précède ; sinon
/// « année » pour une année ; sinon mot qui le suit (mots vides sautés) ; à
/// défaut mot qui le précède.
fn qualifier(items: &[Item], at: usize, year: bool) -> String {
    let previous = match at.checked_sub(1).and_then(|before| items.get(before)) {
        Some(Item::Word(word)) => Some(normalize_word(word)),
        _ => None,
    };
    if let Some(word) = &previous {
        if REFERRAL_WORDS.contains(&word.as_str()) {
            return word.clone();
        }
    }
    if year {
        return "année".to_string();
    }
    let next = items[at + 1..]
        .iter()
        .find(|item| !matches!(item, Item::Word(word) if STOP_WORDS.contains(&word.as_str())));
    if let Some(Item::Word(word)) = next {
        return normalize_word(word);
    }
    previous.unwrap_or_default()
}

fn normalize_word(word: &str) -> String {
    match word.strip_suffix('s') {
        Some(stem) if !stem.is_empty() => stem.to_string(),
        _ => word.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(count: usize) -> String {
        vec!["mot"; count].join(" ")
    }

    #[test]
    fn should_accept_line_when_it_has_25_words() {
        let line = BriefLine::check(&words(25), &[]).expect("25 words are accepted");

        assert_eq!(line.as_str(), words(25));
    }

    #[test]
    fn should_reject_line_as_too_long_when_it_has_26_words() {
        assert_eq!(
            BriefLine::check(&words(26), &[]),
            Err(BriefLineError::TooLong(26))
        );
    }

    #[test]
    fn should_count_elided_word_once_when_line_writes_l_article() {
        assert_eq!(word_count("l'article"), 1);
        assert_eq!(word_count("l'article du code"), 3);

        let line = format!("l'article {}", words(24));
        assert!(BriefLine::check(&line, &[]).is_ok());

        let line = format!("l'article {}", words(25));
        assert_eq!(
            BriefLine::check(&line, &[]),
            Err(BriefLineError::TooLong(26))
        );
    }

    #[test]
    fn should_reject_line_when_it_spans_two_paragraphs() {
        assert_eq!(
            BriefLine::check("Première phrase.\n\nSeconde phrase.", &[]),
            Err(BriefLineError::MultipleParagraphs)
        );
        assert_eq!(
            BriefLine::check("Première phrase.\r\nSeconde phrase.", &[]),
            Err(BriefLineError::MultipleParagraphs)
        );
    }

    #[test]
    fn should_reject_line_when_it_is_blank() {
        assert_eq!(BriefLine::check("", &[]), Err(BriefLineError::Empty));
        assert_eq!(BriefLine::check("  \n\t ", &[]), Err(BriefLineError::Empty));
    }

    fn quantity(value: &str, qualifier: &str) -> Quantity {
        Quantity {
            value: value.to_string(),
            qualifier: qualifier.to_string(),
        }
    }

    #[test]
    fn should_accept_number_in_digits_when_sources_write_it_in_letters_with_same_unit() {
        let line = BriefLine::check(
            "Le délai passe à 3 mois.",
            &["Le recours est ouvert dans un délai de trois mois."],
        )
        .expect("3 mois is sourced by trois mois");

        assert_eq!(line.as_str(), "Le délai passe à 3 mois.");
    }

    #[test]
    fn should_accept_number_in_letters_when_sources_write_it_in_digits() {
        let line = BriefLine::check(
            "Le délai est de trois mois.",
            &["Le recours est ouvert dans un délai de 3 mois."],
        )
        .expect("trois mois is sourced by 3 mois");

        assert_eq!(line.as_str(), "Le délai est de trois mois.");
    }

    #[test]
    fn should_reject_number_when_sources_only_cite_it_as_an_article_number() {
        assert_eq!(
            BriefLine::check(
                "Le délai passe à 13 jours.",
                &["L'article 13 du code civil est modifié."],
            ),
            Err(BriefLineError::UnsourcedNumber("13 jour".to_string()))
        );
    }

    #[test]
    fn should_reject_number_when_line_converts_the_unit() {
        assert_eq!(
            BriefLine::check(
                "Le délai est de 90 jours.",
                &["Le recours est ouvert dans un délai de trois mois."],
            ),
            Err(BriefLineError::UnsourcedNumber("90 jour".to_string()))
        );
    }

    #[test]
    fn should_accept_article_number_when_sources_cite_the_same_article() {
        let line = BriefLine::check(
            "Il modifie l'article 13 du code civil.",
            &["L'article 13 du code civil est modifié."],
        )
        .expect("article 13 is sourced by article 13");

        assert_eq!(line.as_str(), "Il modifie l'article 13 du code civil.");
    }

    #[test]
    fn should_read_compound_numbers_in_letters_when_written_with_hyphens_or_et() {
        let cases = [
            ("vingt-cinq jours", "25", "jour"),
            ("vingt et un jours", "21", "jour"),
            ("vingt-et-un jours", "21", "jour"),
            ("soixante et onze jours", "71", "jour"),
            ("soixante-dix jours", "70", "jour"),
            ("quatre-vingts jours", "80", "jour"),
            ("quatre-vingt-dix jours", "90", "jour"),
            ("quatre-vingt-onze jours", "91", "jour"),
            ("deux cents jours", "200", "jour"),
            ("cent trois euros", "103", "euro"),
            ("trois mille euros", "3000", "euro"),
            ("mille deux cents euros", "1200", "euro"),
            ("deux cent mille euros", "200000", "euro"),
        ];

        for (text, value, qualifier) in cases {
            assert_eq!(quantities(text), vec![quantity(value, qualifier)], "{text}");
        }
        assert_eq!(
            quantities("trois et quatre jours"),
            vec![quantity("3", ""), quantity("4", "jour")],
            "et between two plain numbers does not join them"
        );
    }

    #[test]
    fn should_not_read_number_when_hyphenated_word_mixes_number_and_other_words() {
        assert!(quantities("un cinq-mâts").is_empty());
        assert!(quantities("le porte-parole du groupe").is_empty());
    }

    #[test]
    fn should_read_thousands_separated_by_narrow_spaces_when_written_in_digits() {
        for separator in [' ', '\u{a0}', '\u{202f}', '\u{2009}'] {
            let text = format!("1{separator}500 euros");
            assert_eq!(quantities(&text), vec![quantity("1500", "euro")], "{text}");
        }
        assert_eq!(
            quantities("12\u{202f}500\u{202f}000 euros"),
            vec![quantity("12500000", "euro")]
        );
        assert_eq!(
            quantities("3 mois 500 euros"),
            vec![quantity("3", "moi"), quantity("500", "euro")],
            "a group of three digits after a word is not a thousands group"
        );

        let line = BriefLine::check(
            "Une amende de 1\u{202f}500 euros est créée.",
            &["Elle est punie de 1\u{a0}500 euros d'amende."],
        )
        .expect("1 500 euros is sourced whatever the space");
        assert_eq!(line.as_str(), "Une amende de 1\u{202f}500 euros est créée.");
    }

    #[test]
    fn should_read_decimal_comma_when_written_in_digits() {
        assert_eq!(quantities("10,5 %"), vec![quantity("10.5", "%")]);
        assert_eq!(quantities("10,50 %"), vec![quantity("10.5", "%")]);
        assert_eq!(quantities("7 €"), vec![quantity("7", "€")]);
        assert_eq!(quantities("007 jours"), vec![quantity("7", "jour")]);
    }

    #[test]
    fn should_accept_year_when_sources_contain_the_same_year_in_a_date() {
        let sources = ["Il entre en vigueur au 1er janvier 2027."];
        assert_eq!(
            source_quantities(sources[0]),
            vec![quantity("1", "janvier"), quantity("2027", "année")]
        );

        let line = BriefLine::check("Il s'applique dès 2027.", &sources)
            .expect("2027 is sourced by the date");

        assert_eq!(line.as_str(), "Il s'applique dès 2027.");
    }

    #[test]
    fn should_read_year_only_between_1800_and_2100_written_without_separator() {
        assert_eq!(quantities("en 1800"), vec![quantity("1800", "année")]);
        assert_eq!(quantities("en 2100"), vec![quantity("2100", "année")]);
        assert_eq!(quantities("en 1799"), vec![quantity("1799", "en")]);
        assert_eq!(quantities("en 2101"), vec![quantity("2101", "en")]);
        assert_eq!(quantities("2 000 euros"), vec![quantity("2000", "euro")]);
    }

    #[test]
    fn should_qualify_number_by_referral_word_then_next_word_then_previous_word() {
        let cases = [
            ("articles 13", "13", "article"),
            ("à l'alinéa 2", "2", "alinéa"),
            ("le n° 5 du texte", "5", "n°"),
            ("numéro 7", "7", "numéro"),
            ("un délai de 3 jours", "3", "jour"),
            ("entre 5 des communes", "5", "commune"),
            ("fixé à 13.", "13", "à"),
            ("fixé à 13, puis", "13", "à"),
        ];

        for (text, value, qualifier) in cases {
            assert_eq!(quantities(text), vec![quantity(value, qualifier)], "{text}");
        }
    }

    #[test]
    fn should_not_check_indefinite_article_when_line_writes_un_or_une() {
        let line = BriefLine::check("Une amende et un délai sont créés.", &[])
            .expect("un and une are articles in a line");
        assert_eq!(line.as_str(), "Une amende et un délai sont créés.");

        assert!(quantities("un délai").is_empty());
        assert_eq!(source_quantities("un délai"), vec![quantity("1", "délai")]);
        assert_eq!(
            source_quantities("une amende"),
            vec![quantity("1", "amende")]
        );
    }

    #[test]
    fn should_accept_percentage_when_sources_write_pour_cent() {
        let line = BriefLine::check(
            "La hausse est de 3 %.",
            &["Une hausse de trois pour cent est prévue."],
        )
        .expect("3 % is sourced by trois pour cent");

        assert_eq!(line.as_str(), "La hausse est de 3 %.");
        assert_eq!(quantities("3 pour cent"), vec![quantity("3", "%")]);
        assert_eq!(quantities("3%"), vec![quantity("3", "%")]);
    }

    #[test]
    fn should_read_ordinal_when_line_writes_deuxieme_alinea() {
        let line = BriefLine::check(
            "Le deuxième alinéa est supprimé.",
            &["Le 2ème alinéa de l'article est supprimé."],
        )
        .expect("deuxième alinéa is sourced by 2ème alinéa");
        assert_eq!(line.as_str(), "Le deuxième alinéa est supprimé.");

        let cases = [
            ("le 1er janvier", "1", "janvier"),
            ("la 1re lecture", "1", "lecture"),
            ("la première lecture", "1", "lecture"),
            ("le premier alinéa", "1", "alinéa"),
            ("le 2e alinéa", "2", "alinéa"),
            ("le 2ème alinéa", "2", "alinéa"),
            ("le deuxième alinéa", "2", "alinéa"),
            ("le quatrième alinéa", "4", "alinéa"),
            ("le cinquième alinéa", "5", "alinéa"),
            ("le neuvième alinéa", "9", "alinéa"),
            ("le dix-huitième alinéa", "18", "alinéa"),
            ("le vingt et unième jour", "21", "jour"),
            ("le soixante-dixième jour", "70", "jour"),
        ];
        for (text, value, qualifier) in cases {
            assert_eq!(quantities(text), vec![quantity(value, qualifier)], "{text}");
        }
    }

    #[test]
    fn should_treat_typographic_apostrophe_as_elision_when_counting_and_reading() {
        let line = format!("l’article {}", words(24));
        assert!(BriefLine::check(&line, &[]).is_ok());
        let line = format!("l’article {}", words(25));
        assert_eq!(
            BriefLine::check(&line, &[]),
            Err(BriefLineError::TooLong(26))
        );

        assert_eq!(quantities("3 d’entre eux"), vec![quantity("3", "entre")]);
        assert_eq!(quantities("l’article 13"), vec![quantity("13", "article")]);
        let line = BriefLine::check("Au moins 3 d’entre eux.", &["Au moins 3 d'entre eux."])
            .expect("d’ and d' are the same elision");
        assert_eq!(line.as_str(), "Au moins 3 d’entre eux.");
    }

    #[test]
    fn should_reject_line_when_it_lends_an_intention_with_vise_a() {
        let cases = [
            ("Cet article vise à réduire le délai.", "vise à"),
            ("Un texte visant à réduire le délai est adopté.", "visant à"),
            ("Il a pour objectif de réduire le délai.", "a pour objectif"),
            ("Il a pour but de réduire le délai.", "a pour but"),
            (
                "Le délai change afin de protéger les locataires.",
                "afin de",
            ),
        ];

        for (text, term) in cases {
            assert_eq!(
                BriefLine::check(text, &[]),
                Err(BriefLineError::ForbiddenWording(term.to_string())),
                "{text}"
            );
        }
    }

    #[test]
    fn should_reject_line_when_it_carries_a_judgement_word() {
        let cases = [
            ("Cette réforme historique change le délai.", "historique"),
            ("Le délai subit un changement important.", "important"),
            ("Le délai subit un changement significatif.", "significatif"),
            ("Cette réforme ambitieux change le délai.", "ambitieux"),
            ("Ce délai efficace remplace l'ancien.", "efficace"),
            ("Ce délai injuste remplace l'ancien.", "injuste"),
        ];

        for (text, term) in cases {
            assert_eq!(
                BriefLine::check(text, &[]),
                Err(BriefLineError::ForbiddenWording(term.to_string())),
                "{text}"
            );
        }
    }

    #[test]
    fn should_ignore_case_when_matching_forbidden_wording() {
        assert_eq!(
            BriefLine::check("Afin De protéger les locataires, le délai change.", &[]),
            Err(BriefLineError::ForbiddenWording("afin de".to_string()))
        );
        assert_eq!(
            BriefLine::check("Ce texte VISE À réduire le délai.", &[]),
            Err(BriefLineError::ForbiddenWording("vise à".to_string()))
        );
        assert_eq!(
            BriefLine::check("Une réforme HISTORIQUE change le délai.", &[]),
            Err(BriefLineError::ForbiddenWording("historique".to_string()))
        );
    }

    #[test]
    fn should_reject_feminine_plural_and_conjugated_forms_when_line_uses_them() {
        let cases = [
            ("Ces mesures visent à réduire le délai.", "visent à"),
            (
                "Ces mesures ont pour objectif de réduire le délai.",
                "ont pour objectif",
            ),
            (
                "Ces mesures ont pour but de réduire le délai.",
                "ont pour but",
            ),
            ("Une réforme importante change le délai.", "importante"),
            (
                "Des changements importants touchent le délai.",
                "importants",
            ),
            ("Des réformes importantes touchent le délai.", "importantes"),
            (
                "Des changements significatifs touchent le délai.",
                "significatifs",
            ),
            (
                "Une évolution significative touche le délai.",
                "significative",
            ),
            (
                "Des évolutions significatives touchent le délai.",
                "significatives",
            ),
            ("Des réformes historiques touchent le délai.", "historiques"),
            ("Une réforme ambitieuse change le délai.", "ambitieuse"),
            ("Des réformes ambitieuses changent le délai.", "ambitieuses"),
            ("Des délais efficaces remplacent les anciens.", "efficaces"),
            ("Des délais injustes remplacent les anciens.", "injustes"),
        ];

        for (text, term) in cases {
            assert_eq!(
                BriefLine::check(text, &[]),
                Err(BriefLineError::ForbiddenWording(term.to_string())),
                "{text}"
            );
        }
    }

    #[test]
    fn should_match_whole_words_only_when_a_forbidden_term_is_inside_another_word() {
        for text in [
            "La devise à utiliser change.",
            "Le texte a été adopté historiquement tôt.",
            "L'importance du délai change.",
        ] {
            let line = BriefLine::check(text, &[]).expect(text);
            assert_eq!(line.as_str(), text);
        }

        assert_eq!(
            BriefLine::check("Il ne retient que l’important.", &[]),
            Err(BriefLineError::ForbiddenWording("important".to_string()))
        );
    }

    #[test]
    fn should_match_forbidden_term_when_words_are_separated_by_other_whitespace() {
        assert_eq!(
            BriefLine::check("Ce texte vise\u{a0}à réduire le délai.", &[]),
            Err(BriefLineError::ForbiddenWording("vise à".to_string()))
        );
        assert_eq!(
            BriefLine::check("Ce texte a  pour\tobjectif de réduire le délai.", &[]),
            Err(BriefLineError::ForbiddenWording(
                "a pour objectif".to_string()
            ))
        );
    }

    #[test]
    fn should_give_stable_code_for_each_rejection_when_stored() {
        assert_eq!(BriefLineError::Empty.as_str(), "empty");
        assert_eq!(
            BriefLineError::MultipleParagraphs.as_str(),
            "multiple_paragraphs"
        );
        assert_eq!(BriefLineError::TooLong(26).as_str(), "too_long");
        assert_eq!(
            BriefLineError::ForbiddenWording("vise à".to_string()).as_str(),
            "forbidden_wording"
        );
        assert_eq!(
            BriefLineError::UnsourcedNumber("13 jour".to_string()).as_str(),
            "unsourced_number"
        );
    }

    #[test]
    fn should_accept_year_in_letters_when_a_noun_follows_it() {
        let sources = ["Les affectations sont confirmées pour l'année 2027."];

        for line in [
            "Il confirme pour l'année deux mille vingt-sept les affectations.",
            "Il confirme en deux mille vingt-sept les affectations.",
        ] {
            assert!(BriefLine::check(line, &sources).is_ok(), "{line}");
        }
    }

    #[test]
    fn should_refuse_year_in_letters_when_sources_state_another_year() {
        assert_eq!(
            BriefLine::check(
                "Il confirme en deux mille vingt-huit les affectations.",
                &["Les affectations sont confirmées pour l'année 2027."],
            ),
            Err(BriefLineError::UnsourcedNumber("2028 année".to_string()))
        );
    }

    #[test]
    fn should_not_read_a_round_amount_in_letters_as_a_year() {
        assert_eq!(
            quantities("Une aide de deux mille euros."),
            vec![Quantity {
                value: "2000".to_string(),
                qualifier: "euro".to_string(),
            }]
        );
    }
}
