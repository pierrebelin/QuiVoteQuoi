//! « En bref » d'un dossier tel que la fiche le lit (SPEC-en-bref CU-03,
//! README.md §8.1).

use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

use crate::application::use_cases::get_dossier_brief::{ArticleBriefView, DossierBrief};
use crate::domain::text_brief::ArticleBriefState;
use crate::domain::text_version::TextVersion;

/// Label de la section : les lignes sont produites par un modèle (README.md
/// §8.1).
const BRIEF_LABEL: &str = "Synthèse automatique";

/// `?version=<référence>` : la version à montrer ; sans elle, la dernière
/// disponible.
#[derive(Deserialize)]
pub struct DossierBriefQuery {
    pub version: Option<String>,
}

#[derive(Serialize)]
pub struct DossierBriefResponse {
    pub label: &'static str,
    /// Toutes les versions du dossier, de la plus récente à la plus ancienne.
    pub versions: Vec<TextVersionDto>,
    /// `null` : le dossier n'a aucune version dont l'Assemblée publie le texte.
    pub shown: Option<TextVersionDto>,
    /// `unread`, `read`, `missing` ou `senate_only` ; `null` sans version
    /// montrée. Seule une version `read` porte des articles.
    pub text_status: Option<&'static str>,
    /// La version montrée est la dernière disponible.
    pub is_latest: bool,
    /// Version plus récente que celle qui est montrée, dont le texte n'est pas
    /// publié par l'Assemblée (RM-14) : jamais résumée.
    pub newer_unavailable_version: Option<TextVersionDto>,
    pub articles: Vec<ArticleBriefDto>,
    /// Modèles distincts des lignes montrées.
    pub models: Vec<String>,
    pub generated_at: Option<NaiveDateTime>,
}

#[derive(Serialize)]
pub struct TextVersionDto {
    pub document_uid: String,
    pub label: &'static str,
    /// `an` ou `sn` : la chambre qui publie la version.
    pub chamber: &'static str,
    pub deposited_on: NaiveDate,
    /// `null` pour une version que l'Assemblée ne publie pas.
    pub official_url: Option<String>,
    /// Le texte de la version est publié par l'Assemblée et lisible (RM-14).
    pub available: bool,
}

#[derive(Serialize)]
pub struct ArticleBriefDto {
    /// Numéro tel que l'en-tête l'écrit : `1er`, `19 bis`.
    pub label: String,
    /// Chemin de la division, du plus haut niveau au plus bas, séparés par
    /// ` > `.
    pub division: Option<String>,
    /// `ready`, `pending` ou `unavailable`.
    pub state: &'static str,
    /// La ligne, quand l'état est `ready` ; jamais inventée sinon.
    pub line: Option<String>,
}

impl From<&TextVersion> for TextVersionDto {
    fn from(version: &TextVersion) -> Self {
        Self {
            document_uid: version.document_uid().to_string(),
            label: version.label(),
            chamber: version.chamber().as_str(),
            deposited_on: version.deposited_on(),
            official_url: version.official_url(),
            available: version.is_available(),
        }
    }
}

impl From<ArticleBriefView> for ArticleBriefDto {
    fn from(article: ArticleBriefView) -> Self {
        let state = article.state.as_str();
        let line = match article.state {
            ArticleBriefState::Ready(text) => Some(text),
            ArticleBriefState::Pending | ArticleBriefState::Unavailable => None,
        };
        Self {
            label: article.label,
            division: article.division,
            state,
            line,
        }
    }
}

impl From<DossierBrief> for DossierBriefResponse {
    fn from(brief: DossierBrief) -> Self {
        Self {
            label: BRIEF_LABEL,
            versions: brief.versions.iter().map(TextVersionDto::from).collect(),
            shown: brief.shown.as_ref().map(TextVersionDto::from),
            text_status: brief.text_status.map(|status| status.as_str()),
            is_latest: brief.is_latest,
            newer_unavailable_version: brief
                .newer_unavailable_version
                .as_ref()
                .map(TextVersionDto::from),
            articles: brief.articles.into_iter().map(Into::into).collect(),
            models: brief.models,
            generated_at: brief.generated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dossier::DossierUid;
    use crate::domain::text_version::{
        Chamber, DocumentUid, NewTextVersion, ReadingStatus, TextStatus, VersionKind,
    };

    fn day(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn version(
        uid: &str,
        kind: VersionKind,
        chamber: Chamber,
        reading: ReadingStatus,
    ) -> TextVersion {
        TextVersion::new(NewTextVersion {
            document_uid: DocumentUid::new(uid.to_string()).unwrap(),
            dossier_uid: DossierUid::new("DLR5L17N51670".to_string()).unwrap(),
            kind,
            chamber,
            deposited_on: day(2026, 3, 1),
            title: "titre".to_string(),
            reading,
        })
    }

    fn brief() -> DossierBrief {
        let adopted = version(
            "PIONANR5L17BTA0200",
            VersionKind::Adopted,
            Chamber::Assembly,
            ReadingStatus::Read,
        );
        let senate = version(
            "PIONSNR5S479BTA0043",
            VersionKind::Adopted,
            Chamber::Senate,
            ReadingStatus::Unread,
        );
        DossierBrief {
            versions: vec![senate.clone(), adopted.clone()],
            shown: Some(adopted),
            text_status: Some(TextStatus::Read),
            is_latest: true,
            newer_unavailable_version: Some(senate),
            articles: vec![
                ArticleBriefView {
                    label: "1er".to_string(),
                    division: Some("TITRE Ier".to_string()),
                    state: ArticleBriefState::Ready("Fixe un délai de trois mois.".to_string()),
                },
                ArticleBriefView {
                    label: "2".to_string(),
                    division: None,
                    state: ArticleBriefState::Pending,
                },
                ArticleBriefView {
                    label: "3".to_string(),
                    division: None,
                    state: ArticleBriefState::Unavailable,
                },
            ],
            models: vec!["modele-a".to_string()],
            generated_at: Some(day(2026, 11, 3).and_hms_opt(8, 0, 0).unwrap()),
        }
    }

    #[test]
    fn should_label_section_as_automatic_summary_when_mapped() {
        let response = DossierBriefResponse::from(brief());

        assert_eq!(response.label, "Synthèse automatique");
    }

    #[test]
    fn should_map_versions_with_label_official_url_and_availability() {
        let response = DossierBriefResponse::from(brief());

        let shown = response.shown.as_ref().unwrap();
        assert_eq!(shown.document_uid, "PIONANR5L17BTA0200");
        assert_eq!(shown.label, "Texte adopté par l'Assemblée nationale");
        assert_eq!(shown.deposited_on, day(2026, 3, 1));
        assert_eq!(
            shown.official_url.as_deref(),
            Some("https://www.assemblee-nationale.fr/dyn/docs/PIONANR5L17BTA0200.raw")
        );
        assert!(shown.available);
        assert_eq!(shown.chamber, "an");

        let senate = response.newer_unavailable_version.as_ref().unwrap();
        assert_eq!(senate.label, "Texte adopté par le Sénat");
        assert_eq!(senate.official_url, None);
        assert_eq!(senate.chamber, "sn");
        assert!(!senate.available);
        assert_eq!(response.versions.len(), 2);
        assert_eq!(response.versions[0].document_uid, "PIONSNR5S479BTA0043");
    }

    #[test]
    fn should_map_article_states_and_lines_when_mapped() {
        let response = DossierBriefResponse::from(brief());

        let rows: Vec<(&str, &str, Option<&str>, Option<&str>)> = response
            .articles
            .iter()
            .map(|a| {
                (
                    a.label.as_str(),
                    a.state,
                    a.division.as_deref(),
                    a.line.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            rows,
            [
                (
                    "1er",
                    "ready",
                    Some("TITRE Ier"),
                    Some("Fixe un délai de trois mois.")
                ),
                ("2", "pending", None, None),
                ("3", "unavailable", None, None),
            ]
        );
    }

    #[test]
    fn should_map_text_status_models_and_date_when_mapped() {
        let response = DossierBriefResponse::from(brief());

        assert_eq!(response.text_status, Some("read"));
        assert!(response.is_latest);
        assert_eq!(response.models, ["modele-a"]);
        assert_eq!(
            response.generated_at,
            Some(day(2026, 11, 3).and_hms_opt(8, 0, 0).unwrap())
        );
    }

    #[test]
    fn should_serialize_null_shown_version_when_dossier_has_none_available() {
        let mut empty = brief();
        empty.shown = None;
        empty.text_status = None;
        empty.is_latest = false;
        empty.newer_unavailable_version = None;
        empty.articles.clear();
        empty.models.clear();
        empty.generated_at = None;

        let json = serde_json::to_value(DossierBriefResponse::from(empty)).unwrap();

        assert_eq!(json["shown"], serde_json::Value::Null);
        assert_eq!(json["text_status"], serde_json::Value::Null);
        assert_eq!(json["articles"], serde_json::json!([]));
        assert_eq!(json["label"], "Synthèse automatique");
        assert_eq!(json["versions"].as_array().unwrap().len(), 2);
    }
}
