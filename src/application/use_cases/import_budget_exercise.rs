use std::collections::HashMap;

use crate::application::ports::budget_repository::{BudgetRepository, RepositoryError};
use crate::domain::budget::{
    normalize_name, AccountKind, BudgetError, BudgetExercise, BudgetStage, BudgetYear, Euros,
    Expectations, LawReference, Mission, NatureBreakdown, NewBudgetExercise, Programme,
    ProgrammeAction, ProgrammeNumber, SourceReference,
};

/// Une ligne du fichier officiel, déjà ramenée à un programme : le lecteur du
/// fichier agrège ses lignes d'action avant d'appeler le use case.
#[derive(Debug, Clone)]
pub struct BudgetLine {
    pub account_kind: AccountKind,
    pub mission: String,
    pub programme_number: ProgrammeNumber,
    pub programme_name: String,
    pub cp: Euros,
    /// Vide pour un programme que la nomenclature seule connaît (0 €).
    pub actions: Vec<ProgrammeAction>,
}

/// Un exercice à importer : ses références, les lignes du fichier et les
/// attendus de la loi contre lesquels elles sont vérifiées.
#[derive(Debug, Clone)]
pub struct ImportBudgetExerciseCommand {
    pub year: BudgetYear,
    pub stage: BudgetStage,
    pub law: LawReference,
    pub source: SourceReference,
    pub lines: Vec<BudgetLine>,
    pub expectations: Expectations,
}

#[derive(Debug, thiserror::Error)]
pub enum ImportBudgetExerciseError {
    #[error(transparent)]
    Budget(#[from] BudgetError),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

/// Ce que l'import a écrit, et ce qu'il a écarté.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportReport {
    pub year: BudgetYear,
    pub general_budget_missions: usize,
    pub general_budget_programmes: usize,
    pub general_budget_total: Euros,
    /// Actions des programmes du budget général.
    pub general_budget_actions: usize,
    /// Nature de tout le budget général, remboursements compris.
    pub general_budget_nature: NatureBreakdown,
    pub off_budget: Vec<ImportedAccount>,
    /// Lignes hors budget général que les attendus ne listent pas : écartées de
    /// l'exercice, donc invisibles ailleurs, d'où leur compte ici.
    pub discarded_lines: usize,
    /// Missions de ces lignes, une fois chacune, dans l'ordre du fichier.
    pub discarded_missions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedAccount {
    pub mission: String,
    pub programmes: usize,
    pub total: Euros,
}

/// Importe l'exercice d'un fichier officiel : construit l'agrégat, le compare
/// aux attendus de la loi, puis le remplace en base. Rien n'est écrit tant que
/// la vérification n'a pas réussi ; rejouer le même import laisse le même
/// exercice.
pub struct ImportBudgetExercise<'a> {
    repository: &'a dyn BudgetRepository,
}

struct MissionDraft {
    name: String,
    kind: AccountKind,
    programmes: Vec<Programme>,
}

impl<'a> ImportBudgetExercise<'a> {
    pub fn new(repository: &'a dyn BudgetRepository) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        command: ImportBudgetExerciseCommand,
    ) -> Result<ImportReport, ImportBudgetExerciseError> {
        let ImportBudgetExerciseCommand {
            year,
            stage,
            law,
            source,
            lines,
            expectations,
        } = command;

        let mut drafts: Vec<MissionDraft> = Vec::new();
        let mut position_of: HashMap<(String, AccountKind), usize> = HashMap::new();
        let mut discarded_lines = 0;
        let mut discarded_missions: Vec<String> = Vec::new();

        for line in lines {
            // Seuls le budget général et les comptes que la loi attend sont
            // retenus. Le reste est écarté ici, et signalé dans le compte rendu.
            let kept = line.account_kind == AccountKind::GeneralBudget
                || expectations
                    .off_budget_account(&line.mission, line.account_kind)
                    .is_some();
            if !kept {
                discarded_lines += 1;
                if !discarded_missions.contains(&line.mission) {
                    discarded_missions.push(line.mission);
                }
                continue;
            }

            let key = (normalize_name(&line.mission), line.account_kind);
            let position = *position_of.entry(key).or_insert_with(|| {
                drafts.push(MissionDraft {
                    name: line.mission.clone(),
                    kind: line.account_kind,
                    programmes: Vec::new(),
                });
                drafts.len() - 1
            });
            drafts[position].programmes.push(Programme::with_actions(
                line.programme_number,
                line.programme_name,
                line.cp,
                line.actions,
            )?);
        }

        let missions = drafts
            .into_iter()
            .map(|draft| Mission::new(draft.name, draft.kind, draft.programmes))
            .collect::<Result<Vec<_>, _>>()?;
        let exercise = BudgetExercise::new(NewBudgetExercise {
            year,
            stage,
            law,
            source,
            missions,
        })?;

        exercise.verify(&expectations)?;
        self.repository.replace_exercise(&exercise).await?;

        tracing::info!(
            year = year.get(),
            missions = exercise.general_budget_missions().count(),
            total = exercise.general_budget_total().get(),
            discarded_lines,
            "budget exercise imported"
        );

        Ok(ImportReport {
            year,
            general_budget_missions: exercise.general_budget_missions().count(),
            general_budget_programmes: exercise
                .general_budget_missions()
                .map(|mission| mission.programmes().len())
                .sum(),
            general_budget_total: exercise.general_budget_total(),
            general_budget_actions: exercise
                .general_budget_missions()
                .flat_map(Mission::programmes)
                .map(|programme| programme.actions().len())
                .sum(),
            general_budget_nature: exercise.general_budget_nature(),
            off_budget: exercise
                .off_budget_missions()
                .map(|mission| ImportedAccount {
                    mission: mission.name().to_string(),
                    programmes: mission.programmes().len(),
                    total: mission.total(),
                })
                .collect(),
            discarded_lines,
            discarded_missions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::use_cases::budget_fakes::{
        euros, law, source, InMemoryBudgetRepository,
    };
    use crate::domain::budget::{ActionCode, CategoryCredit, OffBudgetAccount, SpendingCategory};

    /// Ligne d'un programme doté d'une action unique en fonctionnement.
    fn line(kind: AccountKind, mission: &str, number: u16, name: &str, cp: i64) -> BudgetLine {
        let action = ProgrammeAction::new(
            ActionCode::new("01").unwrap(),
            "Action unique",
            vec![CategoryCredit {
                category: SpendingCategory::new(31).unwrap(),
                cp: euros(cp),
            }],
        )
        .unwrap();
        BudgetLine {
            account_kind: kind,
            mission: mission.to_string(),
            programme_number: ProgrammeNumber::new(number).unwrap(),
            programme_name: name.to_string(),
            cp: euros(cp),
            actions: vec![action],
        }
    }

    fn general(mission: &str, number: u16, name: &str, cp: i64) -> BudgetLine {
        line(AccountKind::GeneralBudget, mission, number, name, cp)
    }

    fn audiovisual(number: u16, name: &str, cp: i64) -> BudgetLine {
        line(
            AccountKind::FinancialAssistance,
            "Avances à l'audiovisuel public",
            number,
            name,
            cp,
        )
    }

    /// Deux missions du budget général (3 programmes, 1 250 CP) et le compte de
    /// l'audiovisuel public (2 programmes, 70 CP).
    fn sample_lines() -> Vec<BudgetLine> {
        vec![
            general("Défense", 144, "Environnement et prospective", 600),
            general("Culture", 131, "Création", 250),
            audiovisual(841, "France Télévisions", 50),
            general("Défense", 178, "Préparation", 400),
            audiovisual(842, "ARTE France", 20),
        ]
    }

    fn expectations() -> Expectations {
        Expectations {
            general_budget_cp: euros(1_250),
            missions: 2,
            programmes: 3,
            off_budget: vec![OffBudgetAccount {
                mission: "Avances à l'audiovisuel public".to_string(),
                kind: AccountKind::FinancialAssistance,
                programmes: 2,
                cp: euros(70),
            }],
        }
    }

    fn command(lines: Vec<BudgetLine>, expectations: Expectations) -> ImportBudgetExerciseCommand {
        ImportBudgetExerciseCommand {
            year: BudgetYear::new(2026).unwrap(),
            stage: BudgetStage::Lfi,
            law: law(),
            source: source(),
            lines,
            expectations,
        }
    }

    #[tokio::test]
    async fn imports_the_lines_grouped_by_mission() {
        let repository = InMemoryBudgetRepository::default();

        let report = ImportBudgetExercise::new(&repository)
            .execute(command(sample_lines(), expectations()))
            .await
            .unwrap();

        let stored = repository.stored(2026).unwrap();
        let defense = stored
            .missions()
            .iter()
            .find(|mission| mission.name() == "Défense")
            .unwrap();
        assert_eq!(defense.programmes().len(), 2);
        assert_eq!(defense.total(), euros(1_000));
        assert_eq!(stored.missions().len(), 3);
        assert_eq!(report.general_budget_missions, 2);
        assert_eq!(report.general_budget_programmes, 3);
        assert_eq!(report.general_budget_total, euros(1_250));
        assert_eq!(
            report.off_budget,
            vec![ImportedAccount {
                mission: "Avances à l'audiovisuel public".to_string(),
                programmes: 2,
                total: euros(70),
            }]
        );
    }

    #[tokio::test]
    async fn a_wrong_total_is_refused_without_writing() {
        let repository = InMemoryBudgetRepository::default();
        let mut expectations = expectations();
        expectations.general_budget_cp = euros(1_251);

        let error = ImportBudgetExercise::new(&repository)
            .execute(command(sample_lines(), expectations))
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            ImportBudgetExerciseError::Budget(BudgetError::GeneralBudgetTotalMismatch {
                expected,
                actual,
            }) if expected == euros(1_251) && actual == euros(1_250)
        ));
        assert_eq!(repository.replace_calls(), 0);
        assert!(repository.stored(2026).is_none());
    }

    #[tokio::test]
    async fn a_mission_without_a_theme_is_refused_without_writing() {
        let repository = InMemoryBudgetRepository::default();
        let mut lines = sample_lines();
        lines.push(general("Mission nouvelle", 999, "Programme nouveau", 10));

        let error = ImportBudgetExercise::new(&repository)
            .execute(command(lines, expectations()))
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            ImportBudgetExerciseError::Budget(BudgetError::MissionWithoutTheme { mission })
                if mission == "Mission nouvelle"
        ));
        assert_eq!(repository.replace_calls(), 0);
        assert!(repository.stored(2026).is_none());
    }

    #[tokio::test]
    async fn a_blank_programme_name_is_refused_without_writing() {
        let repository = InMemoryBudgetRepository::default();
        let mut lines = sample_lines();
        lines.push(general("Défense", 146, "  ", 5));

        let error = ImportBudgetExercise::new(&repository)
            .execute(command(lines, expectations()))
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            ImportBudgetExerciseError::Budget(BudgetError::EmptyProgrammeName)
        ));
        assert_eq!(repository.replace_calls(), 0);
    }

    #[tokio::test]
    async fn an_expected_account_the_file_lacks_is_refused_without_writing() {
        let repository = InMemoryBudgetRepository::default();
        let lines: Vec<BudgetLine> = sample_lines()
            .into_iter()
            .filter(|line| line.account_kind == AccountKind::GeneralBudget)
            .collect();

        let error = ImportBudgetExercise::new(&repository)
            .execute(command(lines, expectations()))
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            ImportBudgetExerciseError::Budget(BudgetError::OffBudgetAccountMissing { .. })
        ));
        assert_eq!(repository.replace_calls(), 0);
    }

    #[tokio::test]
    async fn off_budget_lines_the_expectations_do_not_list_are_discarded_and_reported() {
        let repository = InMemoryBudgetRepository::default();
        let mut lines = sample_lines();
        lines.push(line(
            AccountKind::FinancialAssistance,
            "Compte de test",
            700,
            "Programme de test",
            5,
        ));
        lines.push(line(
            AccountKind::FinancialAssistance,
            "Compte de test",
            701,
            "Autre programme de test",
            5,
        ));

        let report = ImportBudgetExercise::new(&repository)
            .execute(command(lines, expectations()))
            .await
            .unwrap();

        assert_eq!(report.discarded_lines, 2);
        assert_eq!(
            report.discarded_missions,
            vec!["Compte de test".to_string()]
        );
        let stored = repository.stored(2026).unwrap();
        assert!(stored
            .missions()
            .iter()
            .all(|mission| mission.name() != "Compte de test"));
    }

    #[tokio::test]
    async fn the_apostrophe_shape_of_the_file_does_not_hide_an_expected_account() {
        let repository = InMemoryBudgetRepository::default();
        let lines = vec![
            general("Défense", 144, "Environnement et prospective", 600),
            general("Culture", 131, "Création", 250),
            general("Défense", 178, "Préparation", 400),
            line(
                AccountKind::FinancialAssistance,
                "Avances à l\u{2019}audiovisuel public",
                841,
                "France Télévisions",
                50,
            ),
            line(
                AccountKind::FinancialAssistance,
                "Avances à l\u{2019}audiovisuel public",
                842,
                "ARTE France",
                20,
            ),
        ];

        let report = ImportBudgetExercise::new(&repository)
            .execute(command(lines, expectations()))
            .await
            .unwrap();

        assert_eq!(report.discarded_lines, 0);
        assert_eq!(report.off_budget.len(), 1);
    }

    #[tokio::test]
    async fn importing_the_same_exercise_twice_leaves_the_same_state() {
        let repository = InMemoryBudgetRepository::default();

        let first = ImportBudgetExercise::new(&repository)
            .execute(command(sample_lines(), expectations()))
            .await
            .unwrap();
        let after_first = repository.stored(2026).unwrap();
        let second = ImportBudgetExercise::new(&repository)
            .execute(command(sample_lines(), expectations()))
            .await
            .unwrap();

        assert_eq!(first, second);
        assert_eq!(repository.stored(2026).unwrap(), after_first);
        assert_eq!(repository.exercises.lock().unwrap().len(), 1);
    }
}
