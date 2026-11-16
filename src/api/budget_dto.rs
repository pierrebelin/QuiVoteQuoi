use chrono::NaiveDate;
use serde::Serialize;

use crate::application::use_cases::get_budget_exercise::{
    ActionView, BudgetView, MissionView, ProgrammeView,
};
use crate::domain::budget::{BudgetYear, CategoryCredit, NatureBreakdown, NatureShare};
use crate::domain::theme::FamilyCode;

/// Année lue dans l'adresse. `None` pour tout ce qui n'est pas une année de
/// quatre chiffres dans les bornes du domaine : l'adresse est invalide.
pub fn requested_year(raw: &str) -> Option<BudgetYear> {
    if raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    raw.parse::<u16>()
        .ok()
        .and_then(|year| BudgetYear::new(year).ok())
}

/// Sujet du site auquel une mission est rattachée (lien vers `/themes/<code>`).
#[derive(Debug, Serialize)]
pub struct BudgetThemeDto {
    pub code: &'static str,
    pub label: &'static str,
}

impl From<FamilyCode> for BudgetThemeDto {
    fn from(family: FamilyCode) -> Self {
        Self {
            code: family.as_str(),
            label: family.label(),
        }
    }
}

/// Catégorie de dépense (deux chiffres) et son libellé officiel.
#[derive(Debug, Serialize)]
pub struct BudgetCategoryDto {
    pub code: u8,
    pub label: &'static str,
    pub cp_euros: i64,
}

impl From<CategoryCredit> for BudgetCategoryDto {
    fn from(credit: CategoryCredit) -> Self {
        Self {
            code: credit.category.code(),
            label: credit.category.official_label(),
            cp_euros: credit.cp.get(),
        }
    }
}

/// Nature d'une dépense : titre de la LOLF (art. 5), libellé officiel, et ses
/// catégories. Les libellés simples sont ceux de la page.
#[derive(Debug, Serialize)]
pub struct BudgetNatureDto {
    pub title: u8,
    pub label: &'static str,
    pub cp_euros: i64,
    pub categories: Vec<BudgetCategoryDto>,
}

impl From<NatureShare> for BudgetNatureDto {
    fn from(share: NatureShare) -> Self {
        Self {
            title: share.nature.title(),
            label: share.nature.official_label(),
            cp_euros: share.cp.get(),
            categories: share
                .categories
                .into_iter()
                .map(BudgetCategoryDto::from)
                .collect(),
        }
    }
}

/// Titres présents, du 1 au 7.
fn natures_of(breakdown: NatureBreakdown) -> Vec<BudgetNatureDto> {
    breakdown
        .natures()
        .iter()
        .cloned()
        .map(BudgetNatureDto::from)
        .collect()
}

#[derive(Debug, Serialize)]
pub struct BudgetActionDto {
    pub code: String,
    pub name: String,
    pub cp_euros: i64,
}

impl From<ActionView> for BudgetActionDto {
    fn from(action: ActionView) -> Self {
        Self {
            code: action.code.as_str().to_string(),
            name: action.name,
            cp_euros: action.total.get(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct BudgetProgrammeDto {
    pub number: u16,
    pub name: String,
    pub cp_euros: i64,
    pub nature: Vec<BudgetNatureDto>,
    /// Triées par montant décroissant ; vide pour un programme à 0 €.
    pub actions: Vec<BudgetActionDto>,
}

impl From<ProgrammeView> for BudgetProgrammeDto {
    fn from(programme: ProgrammeView) -> Self {
        Self {
            number: programme.number.get(),
            name: programme.name,
            cp_euros: programme.total.get(),
            nature: natures_of(programme.nature),
            actions: programme
                .actions
                .into_iter()
                .map(BudgetActionDto::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct BudgetMissionDto {
    pub name: String,
    pub account_kind: &'static str,
    /// `null` seulement pour une mission que le rattachement ne connaît pas ;
    /// l'import la refuse, donc un exercice publié n'en porte pas.
    pub theme: Option<BudgetThemeDto>,
    pub cp_euros: i64,
    /// Ce que paient ses crédits, titre par titre.
    pub nature: Vec<BudgetNatureDto>,
    pub programmes: Vec<BudgetProgrammeDto>,
}

impl From<MissionView> for BudgetMissionDto {
    fn from(mission: MissionView) -> Self {
        Self {
            name: mission.name,
            account_kind: mission.account_kind.as_str(),
            theme: mission.theme.map(BudgetThemeDto::from),
            cp_euros: mission.total.get(),
            nature: natures_of(mission.nature),
            programmes: mission
                .programmes
                .into_iter()
                .map(BudgetProgrammeDto::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct BudgetLawDto {
    pub number: String,
    pub date: NaiveDate,
    /// Article de la loi qui ouvre les crédits.
    pub credits_article: u16,
    pub adoption_procedure: &'static str,
    pub legifrance_url: String,
}

#[derive(Debug, Serialize)]
pub struct BudgetSourceDto {
    pub title: String,
    pub url: String,
    pub published_on: NaiveDate,
}

#[derive(Debug, Serialize)]
pub struct GeneralBudgetDto {
    /// Somme des CP des missions du budget général, en euros.
    pub cp_euros: i64,
    /// Comprise dans `cp_euros`. `null` si l'exercice n'a pas cette mission.
    pub refunds_and_rebates_cp_euros: Option<i64>,
    /// Nature des dépenses hors mission des remboursements et dégrèvements :
    /// sa somme et `refunds_and_rebates_cp_euros` redonnent `cp_euros`.
    pub nature_without_refunds: Vec<BudgetNatureDto>,
    pub missions: Vec<BudgetMissionDto>,
}

/// Exercice budgétaire : crédits de paiement votés en loi de finances initiale,
/// en euros entiers. Parts et arrondis se calculent à l'affichage.
#[derive(Debug, Serialize)]
pub struct BudgetExerciseResponse {
    pub year: u16,
    pub stage: &'static str,
    pub law: BudgetLawDto,
    pub source: BudgetSourceDto,
    pub general_budget: GeneralBudgetDto,
    /// Comptes hors budget général, jamais ajoutés à son total.
    pub off_budget: Vec<BudgetMissionDto>,
    /// De la plus récente à la plus ancienne.
    pub available_years: Vec<u16>,
}

impl From<BudgetView> for BudgetExerciseResponse {
    fn from(view: BudgetView) -> Self {
        Self {
            year: view.year.get(),
            stage: view.stage.as_str(),
            law: BudgetLawDto {
                number: view.law.number,
                date: view.law.date,
                credits_article: view.law.credits_article,
                adoption_procedure: view.law.adoption_procedure.as_str(),
                legifrance_url: view.law.legifrance_url,
            },
            source: BudgetSourceDto {
                title: view.source.title,
                url: view.source.url,
                published_on: view.source.published_on,
            },
            general_budget: GeneralBudgetDto {
                cp_euros: view.general_budget_total.get(),
                refunds_and_rebates_cp_euros: view.refunds_and_rebates_total.map(|e| e.get()),
                nature_without_refunds: natures_of(view.general_budget_nature),
                missions: view
                    .general_budget_missions
                    .into_iter()
                    .map(BudgetMissionDto::from)
                    .collect(),
            },
            off_budget: view
                .off_budget_missions
                .into_iter()
                .map(BudgetMissionDto::from)
                .collect(),
            available_years: view.available_years.iter().map(|year| year.get()).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::use_cases::budget_fakes::{
        exercise, mission, programme, InMemoryBudgetRepository,
    };
    use crate::application::use_cases::get_budget_exercise::GetBudgetExercise;
    use crate::domain::budget::AccountKind;

    async fn response_of_sample() -> BudgetExerciseResponse {
        let repository = InMemoryBudgetRepository::with_exercises(vec![exercise(
            2026,
            vec![
                mission(
                    "Culture",
                    AccountKind::GeneralBudget,
                    vec![programme(131, "Création", 250)],
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
        )]);
        let view = GetBudgetExercise::new(&repository)
            .execute(None)
            .await
            .unwrap();
        BudgetExerciseResponse::from(view)
    }

    #[tokio::test]
    async fn serializes_the_shape_the_front_reads() {
        let json = serde_json::to_value(response_of_sample().await).unwrap();

        assert_eq!(
            json,
            serde_json::json!({
                "year": 2026,
                "stage": "lfi",
                "law": {
                    "number": "2026-103",
                    "date": "2026-02-19",
                    "credits_article": 148,
                    "adoption_procedure": "article_49_3",
                    "legifrance_url": "https://www.legifrance.example/loi"
                },
                "source": {
                    "title": "Crédits AE et CP votés",
                    "url": "https://www.budget.example/fichier",
                    "published_on": "2026-03-03"
                },
                "general_budget": {
                    "cp_euros": 550,
                    "refunds_and_rebates_cp_euros": 300,
                    "nature_without_refunds": [{ "title": 3, "label": "Dépenses de fonctionnement", "cp_euros": 250, "categories": [{ "code": 31, "label": "Dépenses de fonctionnement autres que celles de personnel", "cp_euros": 250 }] }],
                    "missions": [
                        {
                            "name": "Remboursements et dégrèvements",
                            "account_kind": "budget_general",
                            "theme": {
                                "code": "economie-finances-publiques",
                                "label": "Économie / finances publiques"
                            },
                            "cp_euros": 300,
                            "nature": [{ "title": 3, "label": "Dépenses de fonctionnement", "cp_euros": 300, "categories": [{ "code": 31, "label": "Dépenses de fonctionnement autres que celles de personnel", "cp_euros": 300 }] }],
                            "programmes": [
                                {
                                    "number": 200,
                                    "name": "Remboursements d'impôts d'État",
                                    "cp_euros": 300,
                                    "nature": [{ "title": 3, "label": "Dépenses de fonctionnement", "cp_euros": 300, "categories": [{ "code": 31, "label": "Dépenses de fonctionnement autres que celles de personnel", "cp_euros": 300 }] }],
                                    "actions": [{ "code": "01", "name": "Action unique", "cp_euros": 300 }]
                                }
                            ]
                        },
                        {
                            "name": "Culture",
                            "account_kind": "budget_general",
                            "theme": { "code": "education-culture", "label": "Éducation / culture" },
                            "cp_euros": 250,
                            "nature": [{ "title": 3, "label": "Dépenses de fonctionnement", "cp_euros": 250, "categories": [{ "code": 31, "label": "Dépenses de fonctionnement autres que celles de personnel", "cp_euros": 250 }] }],
                            "programmes": [
                                {
                                    "number": 131,
                                    "name": "Création",
                                    "cp_euros": 250,
                                    "nature": [{ "title": 3, "label": "Dépenses de fonctionnement", "cp_euros": 250, "categories": [{ "code": 31, "label": "Dépenses de fonctionnement autres que celles de personnel", "cp_euros": 250 }] }],
                                    "actions": [{ "code": "01", "name": "Action unique", "cp_euros": 250 }]
                                }
                            ]
                        }
                    ]
                },
                "off_budget": [
                    {
                        "name": "Avances à l'audiovisuel public",
                        "account_kind": "concours_financiers",
                        "theme": { "code": "education-culture", "label": "Éducation / culture" },
                        "cp_euros": 50,
                        "nature": [{ "title": 3, "label": "Dépenses de fonctionnement", "cp_euros": 50, "categories": [{ "code": 31, "label": "Dépenses de fonctionnement autres que celles de personnel", "cp_euros": 50 }] }],
                        "programmes": [
                            {
                                    "number": 841,
                                    "name": "France Télévisions",
                                    "cp_euros": 50,
                                    "nature": [{ "title": 3, "label": "Dépenses de fonctionnement", "cp_euros": 50, "categories": [{ "code": 31, "label": "Dépenses de fonctionnement autres que celles de personnel", "cp_euros": 50 }] }],
                                    "actions": [{ "code": "01", "name": "Action unique", "cp_euros": 50 }]
                                }
                        ]
                    }
                ],
                "available_years": [2026]
            })
        );
    }

    #[tokio::test]
    async fn emits_the_keys_in_the_documented_order() {
        let text = serde_json::to_string(&response_of_sample().await).unwrap();

        let keys = [
            "\"year\"",
            "\"stage\"",
            "\"law\"",
            "\"source\"",
            "\"general_budget\"",
            "\"off_budget\"",
            "\"available_years\"",
        ];
        let positions: Vec<usize> = keys.iter().map(|key| text.find(key).unwrap()).collect();
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn reads_a_four_digit_year_from_the_address() {
        assert_eq!(requested_year("2026").unwrap().get(), 2026);
    }

    #[test]
    fn refuses_what_is_not_a_year_in_bounds() {
        for raw in [
            "", "abc", "+2026", "-2026", "20 26", "1999", "2101", "99999", "2026.0",
        ] {
            assert!(requested_year(raw).is_none(), "adresse {raw:?}");
        }
    }
}
