use std::cmp::Reverse;

use crate::application::ports::budget_repository::{BudgetRepository, RepositoryError};
use crate::domain::budget::{
    AccountKind, ActionCode, BudgetExercise, BudgetStage, BudgetYear, Euros, LawReference, Mission,
    NatureBreakdown, Programme, ProgrammeNumber, SourceReference,
};
use crate::domain::theme::FamilyCode;

#[derive(Debug, thiserror::Error)]
pub enum GetBudgetExerciseError {
    #[error("no budget exercise for year {0}")]
    UnknownYear(BudgetYear),
    #[error("no budget exercise has been imported")]
    NoExercise,
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

/// Une action et ses CP, toutes catégories confondues.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionView {
    pub code: ActionCode,
    pub name: String,
    pub total: Euros,
}

/// Un programme, la nature de ses dépenses et ses actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgrammeView {
    pub number: ProgrammeNumber,
    pub name: String,
    pub total: Euros,
    pub nature: NatureBreakdown,
    /// De la plus dotée à la moins dotée ; à égalité, par code.
    pub actions: Vec<ActionView>,
}

impl From<&Programme> for ProgrammeView {
    fn from(programme: &Programme) -> Self {
        let mut actions: Vec<ActionView> = programme
            .actions()
            .iter()
            .map(|action| ActionView {
                code: action.code().clone(),
                name: action.name().to_string(),
                total: action.total(),
            })
            .collect();
        actions.sort_by(|a, b| b.total.cmp(&a.total).then_with(|| a.code.cmp(&b.code)));
        Self {
            number: programme.number(),
            name: programme.name().to_string(),
            total: programme.cp(),
            nature: programme.nature(),
            actions,
        }
    }
}

/// Une mission, ses programmes, son sujet et la nature de ses dépenses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionView {
    pub name: String,
    pub account_kind: AccountKind,
    /// `None` seulement pour une mission que `MISSION_FAMILIES` ne connaît pas :
    /// l'import la refuse, donc un exercice en base n'en porte pas.
    pub theme: Option<FamilyCode>,
    pub total: Euros,
    pub nature: NatureBreakdown,
    /// Du plus doté au moins doté ; à égalité, par numéro.
    pub programmes: Vec<ProgrammeView>,
}

impl From<&Mission> for MissionView {
    fn from(mission: &Mission) -> Self {
        let mut programmes: Vec<ProgrammeView> = mission
            .programmes()
            .iter()
            .map(ProgrammeView::from)
            .collect();
        programmes.sort_by_key(|programme| (Reverse(programme.total), programme.number));
        Self {
            name: mission.name().to_string(),
            account_kind: mission.account_kind(),
            theme: mission.theme(),
            total: mission.total(),
            nature: mission.nature(),
            programmes,
        }
    }
}

/// Un exercice prêt à être lu : le budget général d'un côté, les comptes hors
/// budget général de l'autre, jamais additionnés.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetView {
    pub year: BudgetYear,
    pub stage: BudgetStage,
    pub law: LawReference,
    pub source: SourceReference,
    pub general_budget_total: Euros,
    /// Comprise dans `general_budget_total`.
    pub refunds_and_rebates_total: Option<Euros>,
    /// Nature du budget général sans la mission des remboursements et
    /// dégrèvements : avec `refunds_and_rebates_total`, redonne le total.
    pub general_budget_nature: NatureBreakdown,
    /// De la mission la plus dotée à la moins dotée ; à égalité, par nom.
    pub general_budget_missions: Vec<MissionView>,
    pub off_budget_missions: Vec<MissionView>,
    /// De la plus récente à la plus ancienne.
    pub available_years: Vec<BudgetYear>,
}

/// Lit l'exercice d'une année, ou le plus récent quand aucune n'est demandée.
///
/// Le tri est une règle d'affichage mécanique : aucun classement éditorial, la
/// mission la plus dotée vient en premier parce que le montant l'ordonne.
pub struct GetBudgetExercise<'a> {
    repository: &'a dyn BudgetRepository,
}

impl<'a> GetBudgetExercise<'a> {
    pub fn new(repository: &'a dyn BudgetRepository) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        requested: Option<BudgetYear>,
    ) -> Result<BudgetView, GetBudgetExerciseError> {
        let mut available_years = self.repository.available_years().await?;
        available_years.sort_by_key(|year| Reverse(*year));

        let year = match requested {
            Some(year) => year,
            None => *available_years
                .first()
                .ok_or(GetBudgetExerciseError::NoExercise)?,
        };
        let exercise = self
            .repository
            .find_exercise(year)
            .await?
            .ok_or(GetBudgetExerciseError::UnknownYear(year))?;

        Ok(view_of(&exercise, available_years))
    }
}

fn view_of(exercise: &BudgetExercise, available_years: Vec<BudgetYear>) -> BudgetView {
    BudgetView {
        year: exercise.year(),
        stage: exercise.stage(),
        law: exercise.law().clone(),
        source: exercise.source().clone(),
        general_budget_total: exercise.general_budget_total(),
        refunds_and_rebates_total: exercise.refunds_and_rebates_total(),
        general_budget_nature: exercise.nature_without_refunds(),
        general_budget_missions: sorted_views(exercise.general_budget_missions()),
        off_budget_missions: sorted_views(exercise.off_budget_missions()),
        available_years,
    }
}

fn sorted_views<'a>(missions: impl Iterator<Item = &'a Mission>) -> Vec<MissionView> {
    let mut views: Vec<MissionView> = missions.map(MissionView::from).collect();
    views.sort_by(|a, b| b.total.cmp(&a.total).then_with(|| a.name.cmp(&b.name)));
    views
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::use_cases::budget_fakes::{
        action, euros, exercise, mission, programme, programme_with, InMemoryBudgetRepository,
    };
    use crate::domain::budget::AdoptionProcedure;

    fn year(value: u16) -> BudgetYear {
        BudgetYear::new(value).unwrap()
    }

    async fn view_of_year(
        repository: &InMemoryBudgetRepository,
        requested: Option<u16>,
    ) -> Result<BudgetView, GetBudgetExerciseError> {
        GetBudgetExercise::new(repository)
            .execute(requested.map(year))
            .await
    }

    fn sample_exercise(year: u16) -> BudgetExercise {
        exercise(
            year,
            vec![
                mission(
                    "Culture",
                    AccountKind::GeneralBudget,
                    vec![
                        programme(131, "Création", 250),
                        programme(224, "Soutien", 250),
                    ],
                ),
                mission(
                    "Défense",
                    AccountKind::GeneralBudget,
                    vec![
                        programme(178, "Préparation", 400),
                        programme(144, "Environnement", 600),
                    ],
                ),
                mission(
                    "Remboursements et dégrèvements",
                    AccountKind::GeneralBudget,
                    vec![programme(200, "Remboursements d'impôts d'État", 300)],
                ),
                mission(
                    "Avances à l'audiovisuel public",
                    AccountKind::FinancialAssistance,
                    vec![programme(841, "France Télévisions", 50)],
                ),
            ],
        )
    }

    #[tokio::test]
    async fn without_a_year_it_reads_the_most_recent_exercise() {
        let repository = InMemoryBudgetRepository::with_exercises(vec![
            sample_exercise(2026),
            sample_exercise(2027),
        ]);

        let view = view_of_year(&repository, None).await.unwrap();

        assert_eq!(view.year, year(2027));
    }

    #[tokio::test]
    async fn with_a_year_it_reads_that_exercise() {
        let repository = InMemoryBudgetRepository::with_exercises(vec![
            sample_exercise(2026),
            sample_exercise(2027),
        ]);

        let view = view_of_year(&repository, Some(2026)).await.unwrap();

        assert_eq!(view.year, year(2026));
        assert_eq!(view.available_years, vec![year(2027), year(2026)]);
    }

    #[tokio::test]
    async fn a_year_that_was_never_imported_is_unknown() {
        let repository = InMemoryBudgetRepository::with_exercises(vec![sample_exercise(2026)]);

        let error = view_of_year(&repository, Some(2030)).await.unwrap_err();

        assert!(matches!(error, GetBudgetExerciseError::UnknownYear(y) if y == year(2030)));
    }

    #[tokio::test]
    async fn without_any_imported_exercise_there_is_nothing_to_read() {
        let repository = InMemoryBudgetRepository::default();

        let error = view_of_year(&repository, None).await.unwrap_err();

        assert!(matches!(error, GetBudgetExerciseError::NoExercise));
    }

    #[tokio::test]
    async fn missions_come_by_decreasing_amount_then_by_name() {
        let repository = InMemoryBudgetRepository::with_exercises(vec![sample_exercise(2026)]);

        let view = view_of_year(&repository, None).await.unwrap();

        let names: Vec<&str> = view
            .general_budget_missions
            .iter()
            .map(|mission| mission.name.as_str())
            .collect();
        // Défense 1 000, Culture 500, Remboursements 300.
        assert_eq!(
            names,
            vec!["Défense", "Culture", "Remboursements et dégrèvements"]
        );
    }

    #[tokio::test]
    async fn two_missions_with_the_same_amount_are_ordered_by_name() {
        let repository = InMemoryBudgetRepository::with_exercises(vec![exercise(
            2026,
            vec![
                mission(
                    "Justice",
                    AccountKind::GeneralBudget,
                    vec![programme(166, "Justice judiciaire", 100)],
                ),
                mission(
                    "Culture",
                    AccountKind::GeneralBudget,
                    vec![programme(131, "Création", 100)],
                ),
            ],
        )]);

        let view = view_of_year(&repository, None).await.unwrap();

        let names: Vec<&str> = view
            .general_budget_missions
            .iter()
            .map(|mission| mission.name.as_str())
            .collect();
        assert_eq!(names, vec!["Culture", "Justice"]);
    }

    #[tokio::test]
    async fn programmes_come_by_decreasing_amount_then_by_number() {
        let repository = InMemoryBudgetRepository::with_exercises(vec![sample_exercise(2026)]);

        let view = view_of_year(&repository, None).await.unwrap();

        let numbers = |name: &str| -> Vec<u16> {
            view.general_budget_missions
                .iter()
                .find(|mission| mission.name == name)
                .unwrap()
                .programmes
                .iter()
                .map(|programme| programme.number.get())
                .collect()
        };
        assert_eq!(numbers("Défense"), vec![144, 178]);
        // Deux programmes à 250 : le plus petit numéro d'abord.
        assert_eq!(numbers("Culture"), vec![131, 224]);
    }

    #[tokio::test]
    async fn the_off_budget_accounts_stay_out_of_the_general_budget_and_its_total() {
        let repository = InMemoryBudgetRepository::with_exercises(vec![sample_exercise(2026)]);

        let view = view_of_year(&repository, None).await.unwrap();

        assert_eq!(view.general_budget_total, euros(1_800));
        assert_eq!(view.general_budget_missions.len(), 3);
        assert_eq!(view.off_budget_missions.len(), 1);
        assert_eq!(
            view.off_budget_missions[0].name,
            "Avances à l'audiovisuel public"
        );
        assert_eq!(view.off_budget_missions[0].total, euros(50));
    }

    #[tokio::test]
    async fn the_view_carries_the_refunds_total_and_the_theme_of_each_mission() {
        let repository = InMemoryBudgetRepository::with_exercises(vec![sample_exercise(2026)]);

        let view = view_of_year(&repository, None).await.unwrap();

        assert_eq!(view.refunds_and_rebates_total, Some(euros(300)));
        let theme_of = |name: &str| {
            view.general_budget_missions
                .iter()
                .chain(&view.off_budget_missions)
                .find(|mission| mission.name == name)
                .unwrap()
                .theme
        };
        assert_eq!(theme_of("Défense"), Some(FamilyCode::InternationalDefense));
        assert_eq!(theme_of("Culture"), Some(FamilyCode::EducationCulture));
        assert_eq!(
            theme_of("Avances à l'audiovisuel public"),
            Some(FamilyCode::EducationCulture)
        );
    }

    #[tokio::test]
    async fn the_view_keeps_the_law_and_the_source_of_the_exercise() {
        let repository = InMemoryBudgetRepository::with_exercises(vec![sample_exercise(2026)]);

        let view = view_of_year(&repository, None).await.unwrap();

        assert_eq!(view.stage, BudgetStage::Lfi);
        assert_eq!(view.law.number, "2026-103");
        assert_eq!(view.law.adoption_procedure, AdoptionProcedure::Article49_3);
        assert_eq!(view.source.title, "Crédits AE et CP votés");
    }

    #[tokio::test]
    async fn actions_come_by_decreasing_amount_then_by_code() {
        let repository = InMemoryBudgetRepository::with_exercises(vec![exercise(
            2026,
            vec![mission(
                "Justice",
                AccountKind::GeneralBudget,
                vec![programme_with(
                    101,
                    "Accès au droit",
                    100,
                    vec![
                        action("03", "Aide aux victimes", vec![(64, 20)]),
                        action("01", "Aide juridictionnelle", vec![(61, 50), (31, 10)]),
                        action("02", "Accès au droit", vec![(64, 20)]),
                    ],
                )],
            )],
        )]);

        let view = view_of_year(&repository, None).await.unwrap();

        let actions: Vec<(&str, i64)> = view.general_budget_missions[0].programmes[0]
            .actions
            .iter()
            .map(|action| (action.code.as_str(), action.total.get()))
            .collect();
        assert_eq!(actions, vec![("01", 60), ("02", 20), ("03", 20)]);
    }

    #[tokio::test]
    async fn the_exercise_nature_leaves_the_refunds_out_and_each_mission_has_its_own() {
        let repository = InMemoryBudgetRepository::with_exercises(vec![sample_exercise(2026)]);

        let view = view_of_year(&repository, None).await.unwrap();

        // Culture 500 + Défense 1 000, en fonctionnement ; remboursements 300 à part.
        assert_eq!(view.general_budget_nature.total(), euros(1_500));
        assert_eq!(
            view.general_budget_nature.total().get()
                + view.refunds_and_rebates_total.unwrap().get(),
            view.general_budget_total.get()
        );
        let refunds = view
            .general_budget_missions
            .iter()
            .find(|mission| mission.name == "Remboursements et dégrèvements")
            .unwrap();
        assert_eq!(refunds.nature.total(), euros(300));
    }
}
