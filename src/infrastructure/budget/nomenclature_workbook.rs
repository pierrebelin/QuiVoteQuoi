//! Lecture de la « Nomenclature par destination » de budget.gouv.fr (xls) et
//! complément des lignes de crédits.
//!
//! Le fichier des crédits ne liste que les programmes dotés. Or la loi en
//! ouvre aussi à 0 € (état B : « Haute Cour », « Restitution des biens mal
//! acquis »…), et la page doit tous les montrer (README §2). La nomenclature
//! du même exercice donne chaque programme, avec son numéro : ceux qu'elle a
//! en plus entrent à 0 €. Un programme doté qu'elle ignorerait, ou rangé dans
//! une autre mission, arrête l'import : les deux fichiers ne décrivent alors
//! pas le même budget.

use std::collections::BTreeMap;
use std::path::Path;

use crate::application::use_cases::import_budget_exercise::BudgetLine;
use crate::domain::budget::{AccountKind, BudgetYear, Euros, ProgrammeNumber};

use super::lfi_workbook::{
    column, first_sheet_rows, programme_number, text, whole, Cell, WorkbookError,
};

const YEAR: &str = "Année";
const LINE_TYPE: &str = "Type de ligne";
const BUDGET_TYPE: &str = "Type de budget";
const CODE: &str = "Code (programme, action ou sous-action)";
const FULL_NAME: &str = "Libellé complet";
const MISSION: &str = "Mission";

/// Seules les lignes de ce type décrivent un programme (les autres : actions, sous-actions).
const PROGRAMME_LINE: &str = "Programme";
const GENERAL_BUDGET_TYPE: &str = "BG";
const FINANCIAL_ASSISTANCE_TYPE: &str = "CCF";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NomenclatureProgramme {
    pub account_kind: AccountKind,
    pub mission: String,
    pub number: u16,
    pub name: String,
}

pub fn read_nomenclature(
    path: &Path,
    year: BudgetYear,
) -> Result<Vec<NomenclatureProgramme>, WorkbookError> {
    let rows = first_sheet_rows(path)?;
    parse_nomenclature_rows(&rows, year)
}

/// « MEB Engagements financiers de l'État » → « Engagements financiers de l'État ».
fn without_mission_code(raw: &str) -> String {
    match raw.split_once(' ') {
        Some((code, name))
            if code.len() == 3
                && code.starts_with('M')
                && code.chars().all(|c| c.is_ascii_uppercase()) =>
        {
            name.trim().to_string()
        }
        _ => raw.trim().to_string(),
    }
}

pub fn parse_nomenclature_rows(
    rows: &[Vec<Cell>],
    year: BudgetYear,
) -> Result<Vec<NomenclatureProgramme>, WorkbookError> {
    let header = rows.first().ok_or(WorkbookError::NoSheet)?;
    let year_at = column(header, YEAR)?;
    let line_type_at = column(header, LINE_TYPE)?;
    let budget_type_at = column(header, BUDGET_TYPE)?;
    let code_at = column(header, CODE)?;
    let name_at = column(header, FULL_NAME)?;
    let mission_at = column(header, MISSION)?;

    let mut programmes = Vec::new();
    for (offset, row) in rows.iter().enumerate().skip(1) {
        let line = offset + 1;
        if !matches!(row.get(line_type_at), Some(Cell::Text(kind)) if kind.trim() == PROGRAMME_LINE)
        {
            continue;
        }
        let account_kind = match text(row, budget_type_at, line, BUDGET_TYPE)?.as_str() {
            GENERAL_BUDGET_TYPE => AccountKind::GeneralBudget,
            FINANCIAL_ASSISTANCE_TYPE => AccountKind::FinancialAssistance,
            _ => continue,
        };
        let row_year = whole(row, year_at, line, YEAR)?;
        if row_year != i64::from(year.get()) {
            return Err(WorkbookError::InvalidRow {
                row: line,
                reason: format!(
                    "nomenclature of {row_year}, the manifest is for {}",
                    year.get()
                ),
            });
        }
        programmes.push(NomenclatureProgramme {
            account_kind,
            mission: without_mission_code(&text(row, mission_at, line, MISSION)?),
            number: programme_number(row, code_at, line)?.get(),
            name: text(row, name_at, line, FULL_NAME)?,
        });
    }
    Ok(programmes)
}

/// Lignes de crédits complétées des programmes de la nomenclature absents du
/// fichier des crédits, à 0 €. Les noms des programmes dotés restent ceux du
/// fichier des crédits.
pub fn complete_with_nomenclature(
    credits: Vec<BudgetLine>,
    nomenclature: &[NomenclatureProgramme],
) -> Result<Vec<BudgetLine>, WorkbookError> {
    let listed: BTreeMap<(&'static str, u16), &NomenclatureProgramme> = nomenclature
        .iter()
        .map(|programme| {
            (
                (programme.account_kind.as_str(), programme.number),
                programme,
            )
        })
        .collect();

    for line in &credits {
        let key = (line.account_kind.as_str(), line.programme_number.get());
        let Some(listed) = listed.get(&key) else {
            return Err(WorkbookError::InvalidRow {
                row: 0,
                reason: format!(
                    "programme {} has credits but is missing from the nomenclature",
                    key.1
                ),
            });
        };
        if listed.mission != line.mission {
            return Err(WorkbookError::InvalidRow {
                row: 0,
                reason: format!(
                    "programme {} is under {:?} in the credits and {:?} in the nomenclature",
                    key.1, line.mission, listed.mission
                ),
            });
        }
    }

    let credited: Vec<(&'static str, u16)> = credits
        .iter()
        .map(|line| (line.account_kind.as_str(), line.programme_number.get()))
        .collect();
    let mut lines = credits;
    for (key, programme) in listed {
        if credited.contains(&key) {
            continue;
        }
        let invalid = |reason: String| WorkbookError::InvalidRow { row: 0, reason };
        lines.push(BudgetLine {
            account_kind: programme.account_kind,
            mission: programme.mission.clone(),
            programme_number: ProgrammeNumber::new(programme.number)
                .map_err(|error| invalid(error.to_string()))?,
            programme_name: programme.name.clone(),
            cp: Euros::new(0).map_err(|error| invalid(error.to_string()))?,
        });
    }
    Ok(lines)
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
            t("Année"),
            t("Type de ligne"),
            t("Type de budget"),
            t("Code (programme, action ou sous-action)"),
            t("Libellé complet"),
            t("Mission"),
        ]
    }

    fn year() -> BudgetYear {
        BudgetYear::new(2026).unwrap()
    }

    fn programme(
        kind: AccountKind,
        mission: &str,
        number: u16,
        name: &str,
    ) -> NomenclatureProgramme {
        NomenclatureProgramme {
            account_kind: kind,
            mission: mission.to_string(),
            number,
            name: name.to_string(),
        }
    }

    fn credit(mission: &str, number: u16, cp: i64) -> BudgetLine {
        BudgetLine {
            account_kind: AccountKind::GeneralBudget,
            mission: mission.to_string(),
            programme_number: ProgrammeNumber::new(number).unwrap(),
            programme_name: format!("Programme {number}"),
            cp: Euros::new(cp).unwrap(),
        }
    }

    #[test]
    fn only_programme_lines_are_kept_and_the_mission_code_is_dropped() {
        let rows = vec![
            header(),
            vec![
                n(2026.0),
                t("Programme"),
                t("BG"),
                n(532.0),
                t("Haute Cour"),
                t("MPB Pouvoirs publics"),
            ],
            vec![
                n(2026.0),
                t("Action apparaissant au PAP/RAP"),
                t("BG"),
                t("0532-01"),
                t("Action"),
                t("MPB Pouvoirs publics"),
            ],
            vec![
                n(2026.0),
                t("Programme"),
                t("CAS"),
                n(731.0),
                t("Opérations"),
                t("MZZ Participations"),
            ],
        ];

        let read = parse_nomenclature_rows(&rows, year()).unwrap();

        assert_eq!(
            read,
            vec![programme(
                AccountKind::GeneralBudget,
                "Pouvoirs publics",
                532,
                "Haute Cour"
            )]
        );
    }

    #[test]
    fn a_nomenclature_of_another_year_is_refused() {
        let rows = vec![
            header(),
            vec![
                n(2025.0),
                t("Programme"),
                t("BG"),
                n(532.0),
                t("Haute Cour"),
                t("MPB Pouvoirs publics"),
            ],
        ];

        let error = parse_nomenclature_rows(&rows, year()).unwrap_err();

        assert!(matches!(error, WorkbookError::InvalidRow { row: 2, .. }));
    }

    #[test]
    fn programmes_missing_from_the_credits_enter_at_zero() {
        let nomenclature = vec![
            programme(
                AccountKind::GeneralBudget,
                "Pouvoirs publics",
                511,
                "Présidence",
            ),
            programme(
                AccountKind::GeneralBudget,
                "Pouvoirs publics",
                532,
                "Haute Cour",
            ),
        ];

        let lines =
            complete_with_nomenclature(vec![credit("Pouvoirs publics", 511, 100)], &nomenclature)
                .unwrap();

        assert_eq!(lines.len(), 2);
        let haute_cour = lines
            .iter()
            .find(|line| line.programme_number.get() == 532)
            .unwrap();
        assert_eq!(haute_cour.cp.get(), 0);
        assert_eq!(haute_cour.programme_name, "Haute Cour");
        let presidence = lines
            .iter()
            .find(|line| line.programme_number.get() == 511)
            .unwrap();
        assert_eq!(presidence.programme_name, "Programme 511");
    }

    #[test]
    fn a_credited_programme_unknown_to_the_nomenclature_is_refused() {
        let error = complete_with_nomenclature(vec![credit("Justice", 101, 1)], &[]).unwrap_err();

        assert!(matches!(error, WorkbookError::InvalidRow { .. }));
    }

    #[test]
    fn a_programme_filed_under_another_mission_is_refused() {
        let nomenclature = vec![programme(
            AccountKind::GeneralBudget,
            "Culture",
            101,
            "Accès",
        )];

        let error =
            complete_with_nomenclature(vec![credit("Justice", 101, 1)], &nomenclature).unwrap_err();

        assert!(matches!(error, WorkbookError::InvalidRow { .. }));
    }
}
