//! Lecture du fichier « Crédits AE et CP votés » de budget.gouv.fr (xls).
//!
//! Une feuille, une ligne par programme × action × sous-action × catégorie.
//! On garde les crédits de paiement totaux de la LFI (`CP (T2 + Hors T2) LFI
//! <année>`), sommés par programme et, dans le programme, par action et
//! catégorie de dépense (les sous-actions sont fondues dans leur action). Le
//! titre est le premier chiffre de la catégorie : le fichier le donne aussi
//! (`Code Titre`), et un désaccord fait échouer la lecture. Les colonnes sont trouvées par leur
//! libellé : un fichier qui en change échoue bruyamment au lieu d'importer la
//! mauvaise colonne.
//!
//! Le fichier ne liste que les programmes dotés : ceux à 0 € viennent de la
//! nomenclature (`nomenclature_workbook`).

use std::collections::BTreeMap;
use std::path::Path;

use calamine::{open_workbook_auto, Data, Reader};

use crate::application::use_cases::import_budget_exercise::BudgetLine;
use crate::domain::budget::{
    AccountKind, ActionCode, BudgetYear, CategoryCredit, Euros, ProgrammeAction, ProgrammeNumber,
    SpendingCategory,
};

const MISSION: &str = "Mission";
const MISSION_TYPE: &str = "Type Mission";
const PROGRAMME: &str = "Programme";
const PROGRAMME_NAME: &str = "Libellé Programme";
const ACTION: &str = "Action";
const ACTION_NAME: &str = "Libellé Action";
const CATEGORY: &str = "Catégorie";
const TITLE: &str = "Code Titre";

/// Type de mission du fichier → nature du compte. Les autres types (budgets
/// annexes `BA`, comptes d'affectation spéciale `CAS`) sont écartés et comptés.
const GENERAL_BUDGET_TYPE: &str = "BG";
const FINANCIAL_ASSISTANCE_TYPE: &str = "CCF";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum WorkbookError {
    #[error("workbook unreadable: {0}")]
    Unreadable(String),
    #[error("workbook has no sheet")]
    NoSheet,
    #[error("column {0:?} missing from the header row")]
    MissingColumn(String),
    #[error("row {row}: {reason}")]
    InvalidRow { row: usize, reason: String },
    #[error("programme {number} named both {first:?} and {second:?}")]
    ConflictingProgrammeName {
        number: u16,
        first: String,
        second: String,
    },
    #[error("action {action} of programme {number} named both {first:?} and {second:?}")]
    ConflictingActionName {
        number: u16,
        action: String,
        first: String,
        second: String,
    },
}

/// Cellule réduite à ce que la lecture distingue.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Empty,
    Text(String),
    Number(f64),
}

/// Octets 0x80 à 0x9F de cp1252 et leurs caractères : seuls écarts entre
/// cp1252 et Latin-1, nécessaires pour retrouver l'octet d'origine.
const CP1252_HIGH: [(char, u8); 27] = [
    ('€', 0x80),
    ('‚', 0x82),
    ('ƒ', 0x83),
    ('„', 0x84),
    ('…', 0x85),
    ('†', 0x86),
    ('‡', 0x87),
    ('ˆ', 0x88),
    ('‰', 0x89),
    ('Š', 0x8A),
    ('‹', 0x8B),
    ('Œ', 0x8C),
    ('Ž', 0x8E),
    ('‘', 0x91),
    ('’', 0x92),
    ('“', 0x93),
    ('”', 0x94),
    ('•', 0x95),
    ('–', 0x96),
    ('—', 0x97),
    ('˜', 0x98),
    ('™', 0x99),
    ('š', 0x9A),
    ('›', 0x9B),
    ('œ', 0x9C),
    ('ž', 0x9E),
    ('Ÿ', 0x9F),
];

fn cp1252_byte(character: char) -> Option<u8> {
    let code = u32::from(character);
    if code < 0x80 || (0xA0..=0xFF).contains(&code) {
        return u8::try_from(code).ok();
    }
    CP1252_HIGH
        .iter()
        .find(|(known, _)| *known == character)
        .map(|(_, byte)| *byte)
}

/// Certaines chaînes de ces fichiers sont stockées en UTF-16 mais marquées
/// 8 bits : la lecture cp1252 intercale alors un caractère nul après chaque
/// lettre (`A\0n\0n\0é\0e\0`). Une chaîne qui contient un nul est relue
/// en UTF-16LE à partir de ses octets d'origine ; si c'est impossible, la
/// lecture échoue plutôt que d'importer un nom altéré.
pub(crate) fn repaired(raw: &str) -> Result<String, String> {
    if !raw.contains('\0') {
        return Ok(raw.to_string());
    }
    let bytes = raw
        .chars()
        .map(|character| cp1252_byte(character).ok_or(character))
        .collect::<Result<Vec<u8>, char>>()
        .map_err(|character| format!("{raw:?}: {character:?} is not a cp1252 byte"))?;
    if bytes.len() % 2 != 0 {
        return Err(format!("{raw:?}: odd byte count for UTF-16"));
    }
    let units = bytes
        .chunks(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]));
    char::decode_utf16(units)
        .collect::<Result<String, _>>()
        .map_err(|error| format!("{raw:?}: {error}"))
}

fn cell(data: &Data) -> Result<Cell, String> {
    Ok(match data {
        Data::Empty => Cell::Empty,
        Data::String(text) => Cell::Text(repaired(text)?),
        Data::Float(value) => Cell::Number(*value),
        Data::Int(value) => Cell::Number(*value as f64),
        other => Cell::Text(other.to_string()),
    })
}

/// Lignes lues, une par programme, et ce qui a été écarté.
#[derive(Debug)]
pub struct CreditLines {
    pub lines: Vec<BudgetLine>,
    /// Types de mission écartés (`BA`, `CAS`…) et leur nombre de lignes.
    pub discarded_types: BTreeMap<String, usize>,
}

pub fn read_credit_lines(path: &Path, year: BudgetYear) -> Result<CreditLines, WorkbookError> {
    let rows = first_sheet_rows(path)?;
    parse_credit_rows(&rows, year)
}

/// Lignes de la première feuille, en-tête compris.
pub(crate) fn first_sheet_rows(path: &Path) -> Result<Vec<Vec<Cell>>, WorkbookError> {
    let mut workbook =
        open_workbook_auto(path).map_err(|error| WorkbookError::Unreadable(error.to_string()))?;
    let range = workbook
        .worksheet_range_at(0)
        .ok_or(WorkbookError::NoSheet)?
        .map_err(|error| WorkbookError::Unreadable(error.to_string()))?;
    range
        .rows()
        .map(|row| row.iter().map(cell).collect::<Result<Vec<_>, _>>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(WorkbookError::Unreadable)
}

/// Compare deux libellés d'en-tête sans tenir compte des blancs répétés.
pub(crate) fn same_label(left: &str, right: &str) -> bool {
    let squeeze = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    squeeze(left) == squeeze(right)
}

pub(crate) fn column(header: &[Cell], label: &str) -> Result<usize, WorkbookError> {
    header
        .iter()
        .position(|cell| matches!(cell, Cell::Text(text) if same_label(text, label)))
        .ok_or_else(|| WorkbookError::MissingColumn(label.to_string()))
}

pub(crate) fn text(
    row: &[Cell],
    index: usize,
    line: usize,
    what: &str,
) -> Result<String, WorkbookError> {
    match row.get(index) {
        Some(Cell::Text(text)) if !text.trim().is_empty() => Ok(text.trim().to_string()),
        Some(Cell::Number(value)) => Ok(value.to_string()),
        _ => Err(WorkbookError::InvalidRow {
            row: line,
            reason: format!("{what} is blank"),
        }),
    }
}

/// Nombre entier d'une cellule (le xls stocke des flottants : 101.0).
pub(crate) fn whole(
    row: &[Cell],
    index: usize,
    line: usize,
    what: &str,
) -> Result<i64, WorkbookError> {
    let invalid = |reason: String| WorkbookError::InvalidRow { row: line, reason };
    let value = match row.get(index) {
        Some(Cell::Number(value)) => *value,
        Some(Cell::Text(text)) => text
            .trim()
            .parse::<f64>()
            .map_err(|_| invalid(format!("{what} {text:?} is not a number")))?,
        _ => return Err(invalid(format!("{what} is blank"))),
    };
    if value.fract() != 0.0 || !value.is_finite() {
        return Err(invalid(format!("{what} {value} is not a whole number")));
    }
    Ok(value as i64)
}

pub(crate) fn programme_number(
    row: &[Cell],
    index: usize,
    line: usize,
) -> Result<ProgrammeNumber, WorkbookError> {
    let raw = whole(row, index, line, "programme")?;
    u16::try_from(raw)
        .ok()
        .and_then(|number| ProgrammeNumber::new(number).ok())
        .ok_or_else(|| WorkbookError::InvalidRow {
            row: line,
            reason: format!("programme {raw} is out of range"),
        })
}

/// Programme en cours de lecture : nature du compte, nom, CP cumulés, et par
/// action son nom et ses CP par catégorie.
struct ProgrammeDraft {
    kind: AccountKind,
    name: String,
    cp: i64,
    actions: BTreeMap<String, (String, BTreeMap<u8, i64>)>,
}

/// Catégorie d'une ligne, contrôlée contre le titre que le fichier lui donne.
fn category(
    row: &[Cell],
    category_at: usize,
    title_at: usize,
    line: usize,
) -> Result<SpendingCategory, WorkbookError> {
    let invalid = |reason: String| WorkbookError::InvalidRow { row: line, reason };
    let raw = whole(row, category_at, line, CATEGORY)?;
    let category = u8::try_from(raw)
        .ok()
        .and_then(|code| SpendingCategory::new(code).ok())
        .ok_or_else(|| invalid(format!("category {raw} is unknown")))?;
    let title = whole(row, title_at, line, TITLE)?;
    if title != i64::from(category.nature().title()) {
        return Err(invalid(format!(
            "category {category} belongs to title {}, the file says {title}",
            category.nature().title()
        )));
    }
    Ok(category)
}

pub fn parse_credit_rows(
    rows: &[Vec<Cell>],
    year: BudgetYear,
) -> Result<CreditLines, WorkbookError> {
    let header = rows.first().ok_or(WorkbookError::NoSheet)?;
    let cp_label = format!("CP (T2 + Hors T2) LFI {}", year.get());
    let mission_at = column(header, MISSION)?;
    let type_at = column(header, MISSION_TYPE)?;
    let programme_at = column(header, PROGRAMME)?;
    let name_at = column(header, PROGRAMME_NAME)?;
    let action_at = column(header, ACTION)?;
    let action_name_at = column(header, ACTION_NAME)?;
    let category_at = column(header, CATEGORY)?;
    let title_at = column(header, TITLE)?;
    let cp_at = column(header, &cp_label)?;

    // Clé : (nature, mission, numéro).
    let mut programmes: BTreeMap<(&'static str, String, u16), ProgrammeDraft> = BTreeMap::new();
    let mut discarded_types: BTreeMap<String, usize> = BTreeMap::new();

    for (offset, row) in rows.iter().enumerate().skip(1) {
        let line = offset + 1;
        if row.iter().all(|cell| *cell == Cell::Empty) {
            continue;
        }
        let kind = match text(row, type_at, line, MISSION_TYPE)?.as_str() {
            GENERAL_BUDGET_TYPE => AccountKind::GeneralBudget,
            FINANCIAL_ASSISTANCE_TYPE => AccountKind::FinancialAssistance,
            other => {
                *discarded_types.entry(other.to_string()).or_default() += 1;
                continue;
            }
        };
        let mission = text(row, mission_at, line, MISSION)?;
        let number = programme_number(row, programme_at, line)?;
        let name = text(row, name_at, line, PROGRAMME_NAME)?;
        let action = text(row, action_at, line, ACTION)?;
        let action_name = text(row, action_name_at, line, ACTION_NAME)?;
        let category = category(row, category_at, title_at, line)?;
        let cp = whole(row, cp_at, line, &cp_label)?;

        let draft = programmes
            .entry((kind.as_str(), mission, number.get()))
            .or_insert_with(|| ProgrammeDraft {
                kind,
                name: name.clone(),
                cp: 0,
                actions: BTreeMap::new(),
            });
        if draft.name != name {
            return Err(WorkbookError::ConflictingProgrammeName {
                number: number.get(),
                first: draft.name.clone(),
                second: name,
            });
        }
        draft.cp += cp;
        let (known_name, credits) = draft
            .actions
            .entry(action.clone())
            .or_insert_with(|| (action_name.clone(), BTreeMap::new()));
        if *known_name != action_name {
            return Err(WorkbookError::ConflictingActionName {
                number: number.get(),
                action,
                first: known_name.clone(),
                second: action_name,
            });
        }
        *credits.entry(category.code()).or_default() += cp;
    }

    let lines = programmes
        .into_iter()
        .map(|((_, mission, number), draft)| line_of(mission, number, draft))
        .collect::<Result<Vec<_>, WorkbookError>>()?;

    Ok(CreditLines {
        lines,
        discarded_types,
    })
}

fn line_of(
    mission: String,
    number: u16,
    draft: ProgrammeDraft,
) -> Result<BudgetLine, WorkbookError> {
    let invalid = |reason: String| WorkbookError::InvalidRow { row: 0, reason };
    let amount = |cp: i64| {
        Euros::new(cp).map_err(|error| invalid(format!("programme {number} totals {cp}: {error}")))
    };
    let actions = draft
        .actions
        .into_iter()
        .map(|(code, (name, credits))| {
            let credits = credits
                .into_iter()
                .map(|(category, cp)| {
                    Ok(CategoryCredit {
                        category: SpendingCategory::new(category)
                            .map_err(|error| invalid(error.to_string()))?,
                        cp: amount(cp)?,
                    })
                })
                .collect::<Result<Vec<_>, WorkbookError>>()?;
            let code = ActionCode::new(code).map_err(|error| invalid(error.to_string()))?;
            ProgrammeAction::new(code, name, credits).map_err(|error| invalid(error.to_string()))
        })
        .collect::<Result<Vec<_>, WorkbookError>>()?;
    Ok(BudgetLine {
        account_kind: draft.kind,
        mission,
        programme_number: ProgrammeNumber::new(number)
            .map_err(|error| invalid(error.to_string()))?,
        programme_name: draft.name,
        cp: amount(draft.cp)?,
        actions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(text: &str) -> Cell {
        Cell::Text(text.to_string())
    }

    fn n(value: f64) -> Cell {
        Cell::Number(value)
    }

    fn header() -> Vec<Cell> {
        vec![
            t("Mission"),
            t("Type Mission"),
            t("Programme"),
            t("Libellé Programme"),
            t("Action"),
            t("Libellé Action"),
            t("Catégorie"),
            t("Code Titre"),
            t("CP (T2 + Hors T2) LFI  2026"),
        ]
    }

    /// Ligne de l'action 01 en fonctionnement (catégorie 31, titre 3).
    fn row(mission: &str, kind: &str, number: f64, name: &str, cp: f64) -> Vec<Cell> {
        detailed(
            mission,
            kind,
            number,
            name,
            ("01", "Action"),
            (31.0, "3"),
            cp,
        )
    }

    fn detailed(
        mission: &str,
        kind: &str,
        number: f64,
        name: &str,
        (action, action_name): (&str, &str),
        (category, title): (f64, &str),
        cp: f64,
    ) -> Vec<Cell> {
        vec![
            t(mission),
            t(kind),
            n(number),
            t(name),
            t(action),
            t(action_name),
            n(category),
            t(title),
            n(cp),
        ]
    }

    fn year() -> BudgetYear {
        BudgetYear::new(2026).unwrap()
    }

    #[test]
    fn rows_of_one_programme_are_summed_into_a_single_line() {
        let rows = vec![
            header(),
            row(
                "Justice",
                "BG",
                101.0,
                "Accès au droit et à la justice",
                35000.0,
            ),
            row(
                "Justice",
                "BG",
                101.0,
                "Accès au droit et à la justice",
                712234633.0,
            ),
        ];

        let read = parse_credit_rows(&rows, year()).unwrap();

        assert_eq!(read.lines.len(), 1);
        assert_eq!(read.lines[0].cp.get(), 712269633);
        assert_eq!(read.lines[0].programme_number.get(), 101);
        assert_eq!(read.lines[0].account_kind, AccountKind::GeneralBudget);
    }

    #[test]
    fn annex_budgets_and_allocation_accounts_are_discarded_and_counted() {
        let rows = vec![
            header(),
            row(
                "Contrôle et exploitation aériens",
                "BA",
                612.0,
                "Navigation aérienne",
                1.0,
            ),
            row(
                "Participations financières",
                "CAS",
                731.0,
                "Opérations",
                1.0,
            ),
            row(
                "Avances à l'audiovisuel public",
                "CCF",
                841.0,
                "France Télévisions",
                5.0,
            ),
        ];

        let read = parse_credit_rows(&rows, year()).unwrap();

        assert_eq!(read.lines.len(), 1);
        assert_eq!(read.lines[0].account_kind, AccountKind::FinancialAssistance);
        assert_eq!(read.discarded_types.get("BA"), Some(&1));
        assert_eq!(read.discarded_types.get("CAS"), Some(&1));
    }

    #[test]
    fn the_credit_column_of_another_year_is_refused_rather_than_read() {
        let rows = vec![header()];

        let error = parse_credit_rows(&rows, BudgetYear::new(2027).unwrap()).unwrap_err();

        assert_eq!(
            error,
            WorkbookError::MissingColumn("CP (T2 + Hors T2) LFI 2027".to_string())
        );
    }

    #[test]
    fn a_fractional_amount_is_refused() {
        let rows = vec![header(), row("Justice", "BG", 101.0, "Accès", 10.5)];

        let error = parse_credit_rows(&rows, year()).unwrap_err();

        assert!(matches!(error, WorkbookError::InvalidRow { row: 2, .. }));
    }

    #[test]
    fn a_utf16_string_read_as_cp1252_is_decoded_again() {
        assert_eq!(repaired("A\0n\0n\0é\0e\0").unwrap(), "Année");
        // ’ (U+2019) = octets 0x19 0x20, lus en cp1252 comme U+0019 puis une espace.
        assert_eq!(repaired("l\0\u{19} ").unwrap(), "l\u{2019}");
        assert_eq!(repaired("Justice").unwrap(), "Justice");
    }

    #[test]
    fn a_string_that_cannot_be_utf16_is_refused() {
        assert!(repaired("A\0n").is_err());
    }

    #[test]
    fn one_number_with_two_names_is_refused() {
        let rows = vec![
            header(),
            row("Justice", "BG", 101.0, "Accès", 1.0),
            row("Justice", "BG", 101.0, "Autre nom", 1.0),
        ];

        let error = parse_credit_rows(&rows, year()).unwrap_err();

        assert!(matches!(
            error,
            WorkbookError::ConflictingProgrammeName { number: 101, .. }
        ));
    }

    #[test]
    fn credits_are_kept_by_action_and_category_with_sub_actions_merged() {
        let rows = vec![
            header(),
            detailed(
                "Justice",
                "BG",
                101.0,
                "Accès",
                ("01", "Aide juridictionnelle"),
                (61.0, "6"),
                700.0,
            ),
            detailed(
                "Justice",
                "BG",
                101.0,
                "Accès",
                ("01", "Aide juridictionnelle"),
                (61.0, "6"),
                12.0,
            ),
            detailed(
                "Justice",
                "BG",
                101.0,
                "Accès",
                ("01", "Aide juridictionnelle"),
                (31.0, "3"),
                3.0,
            ),
            detailed(
                "Justice",
                "BG",
                101.0,
                "Accès",
                ("03", "Aide aux victimes"),
                (64.0, "6"),
                30.0,
            ),
        ];

        let read = parse_credit_rows(&rows, year()).unwrap();

        let programme = &read.lines[0];
        assert_eq!(programme.cp.get(), 745);
        let actions: Vec<(&str, i64, usize)> = programme
            .actions
            .iter()
            .map(|action| {
                (
                    action.code().as_str(),
                    action.total().get(),
                    action.credits().len(),
                )
            })
            .collect();
        assert_eq!(actions, vec![("01", 715, 2), ("03", 30, 1)]);
        assert_eq!(programme.actions[0].name(), "Aide juridictionnelle");
    }

    #[test]
    fn a_title_that_contradicts_the_category_is_refused() {
        let rows = vec![
            header(),
            detailed(
                "Justice",
                "BG",
                101.0,
                "Accès",
                ("01", "Aide"),
                (61.0, "3"),
                1.0,
            ),
        ];

        let error = parse_credit_rows(&rows, year()).unwrap_err();

        assert!(matches!(error, WorkbookError::InvalidRow { row: 2, .. }));
    }

    #[test]
    fn a_category_outside_the_nomenclature_is_refused() {
        let rows = vec![
            header(),
            detailed(
                "Justice",
                "BG",
                101.0,
                "Accès",
                ("01", "Aide"),
                (66.0, "6"),
                1.0,
            ),
        ];

        let error = parse_credit_rows(&rows, year()).unwrap_err();

        assert!(matches!(error, WorkbookError::InvalidRow { row: 2, .. }));
    }

    #[test]
    fn one_action_with_two_names_is_refused() {
        let rows = vec![
            header(),
            detailed(
                "Justice",
                "BG",
                101.0,
                "Accès",
                ("01", "Aide"),
                (61.0, "6"),
                1.0,
            ),
            detailed(
                "Justice",
                "BG",
                101.0,
                "Accès",
                ("01", "Autre"),
                (61.0, "6"),
                1.0,
            ),
        ];

        let error = parse_credit_rows(&rows, year()).unwrap_err();

        assert!(matches!(
            error,
            WorkbookError::ConflictingActionName { number: 101, .. }
        ));
    }
}
