use chrono::NaiveDate;
use serde::Serialize;

use crate::application::use_cases::describe_dataset::DatasetOverview;
use crate::domain::dataset::CodeCount;

/// Les deux grandeurs dérivées sont des divisions, pas des mesures. La formule
/// est publiée avec le chiffre pour que le lecteur puisse la refaire.
pub const DERIVATION_NOTE: &str =
    "Part sans dossier et nombre de scrutins par texte sont calculés \
     à partir des totaux ci-dessus, arrondis respectivement à l'entier et à la décimale.";

#[derive(Debug, Serialize)]
pub struct CodeCountDto {
    pub code: String,
    pub label: String,
    pub count: i64,
}

impl From<CodeCount> for CodeCountDto {
    fn from(value: CodeCount) -> Self {
        Self {
            code: value.code,
            label: value.label,
            count: value.count,
        }
    }
}

/// Chiffres de la page « Comprendre ». Servis depuis la base, jamais rédigés.
#[derive(Debug, Serialize)]
pub struct DatasetResponse {
    pub scrutins_total: i64,
    pub scrutins_with_dossier: i64,
    pub scrutins_without_dossier: i64,
    pub scrutins_without_dossier_share: Option<i64>,
    pub texts_total: i64,
    pub scrutins_per_text: Option<f64>,
    pub dossiers_total: i64,
    pub first_scrutin_date: Option<NaiveDate>,
    pub last_scrutin_date: Option<NaiveDate>,
    pub legislatures: Vec<i64>,
    pub outcomes: Vec<CodeCountDto>,
    pub ballot_types: Vec<CodeCountDto>,
    pub derivation_note: &'static str,
}

impl From<DatasetOverview> for DatasetResponse {
    fn from(overview: DatasetOverview) -> Self {
        let shape = overview.shape;
        Self {
            scrutins_total: shape.scrutins_total,
            scrutins_with_dossier: shape.scrutins_with_dossier,
            scrutins_without_dossier: overview.scrutins_without_dossier,
            scrutins_without_dossier_share: overview.scrutins_without_dossier_share,
            texts_total: overview.texts_total,
            scrutins_per_text: overview.scrutins_per_text,
            dossiers_total: shape.dossiers_total,
            first_scrutin_date: shape.first_scrutin_date,
            last_scrutin_date: shape.last_scrutin_date,
            legislatures: shape.legislatures,
            outcomes: shape.outcomes.into_iter().map(CodeCountDto::from).collect(),
            ballot_types: shape
                .ballot_types
                .into_iter()
                .map(CodeCountDto::from)
                .collect(),
            derivation_note: DERIVATION_NOTE,
        }
    }
}
