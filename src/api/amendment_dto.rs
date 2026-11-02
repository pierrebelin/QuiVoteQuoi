use chrono::NaiveDate;
use serde::Serialize;

use crate::application::ports::amendment_repository::AmendmentGroupOption;
use crate::application::ports::amendment_repository::DossierAmendmentCoverage;
use crate::application::use_cases::browse_dossier_amendments::{AmendmentView, DossierAmendments};
use crate::application::use_cases::refresh_amendments::AmendmentsSummary;
use crate::domain::amendment::SummaryPreview;

/// RM-03. Le site reproduit, il ne redige pas.
pub const AMENDMENT_SOURCE_NOTE: &str = "L'exposé sommaire est reproduit mot pour mot, \
tel que déposé par son ou ses signataires. Le site ne le résume ni ne le commente.";

/// RM-07. Paginer n'est pas filtrer, mais la borne doit se dire.
pub const AMENDMENT_PAGINATION_NOTE: &str = "Les amendements sont affichés par page, \
dans l'ordre de dépôt publié par l'Assemblée. Le total est indiqué et la totalité \
est atteignable page à page : aucun amendement n'est écarté de la liste.";

/// RM-05. Un amendement dont le texte n'est relie a aucun dossier ingere n'est
/// pas perdu, mais il n'apparait pas ici: le dire plutot que de laisser croire
/// a une exhaustivite que la jointure ne donne pas.
pub const AMENDMENT_COVERAGE_NOTE: &str = "Cette liste contient les amendements que la \
source rattache à un texte de ce dossier. Un amendement dont le texte n'est pas encore \
ingéré n'y figure pas.";

#[derive(Serialize)]
pub struct AmendmentDto {
    pub uid: String,
    pub number: String,
    pub target_title: String,
    pub target_kind: Option<String>,
    pub author_kind: String,
    pub author_actor_uid: Option<String>,
    /// Nom du depute resolu depuis le referentiel, ou libelle publie d'un auteur
    /// institutionnel. `None` quand l'acteur est absent du referentiel: son
    /// identifiant reste affichable, aucun nom n'est devine.
    pub author_name: Option<String>,
    pub author_official_url: Option<String>,
    /// Groupe du signataire **a la date de depot** (RM-02).
    pub author_group_label: Option<String>,
    pub author_group_abbrev: Option<String>,
    /// Couleur publiee du groupe (RM-10); `None` quand il n'en publie pas.
    pub author_group_color: Option<String>,
    /// `white` ou `black`: ton du texte lisible sur la couleur du groupe.
    pub author_text_tone: Option<String>,
    /// Apercu de l'expose (RM-05); `None` quand la source n'en publie pas.
    pub summary_preview: Option<String>,
    pub summary_preview_truncated: bool,
    /// Sort tel qu'il s'affiche sur la carte (RM-08).
    pub fate_banner: String,
    /// `published`, `resolved_at_deposit` ou `unknown`. Sert a l'affichage: un
    /// groupe reconstitue et un groupe publie ne se presentent pas pareil.
    pub author_group_origin: String,
    /// Vrai quand deux groupes concurrents revendiquent le signataire a cette
    /// date: aucun groupe n'est affiche.
    pub author_group_ambiguous: bool,
    pub fate_code: String,
    pub fate_label: String,
    pub state_label: Option<String>,
    pub deposited_on: Option<NaiveDate>,
    /// Expose sommaire entier. Jamais tronque cote serveur (RM-03).
    pub summary: Option<String>,
    pub cosignatory_count: i64,
}

impl From<AmendmentView> for AmendmentDto {
    fn from(view: AmendmentView) -> Self {
        let summary = view.summary;
        let (summary_preview, summary_preview_truncated) = match view.preview {
            SummaryPreview::Unpublished => (None, false),
            SummaryPreview::Text { text, truncated } => (Some(text), truncated),
        };
        Self {
            author_group_color: view.author.group_color,
            author_text_tone: view.author.text_tone.map(|t| t.as_str().to_string()),
            summary_preview,
            summary_preview_truncated,
            fate_banner: view.fate_banner.label().to_string(),
            uid: summary.uid,
            number: summary.number,
            target_title: summary.target_title,
            target_kind: summary.target_kind,
            author_name: view.author.name.or(summary.author_label),
            author_official_url: view.author.official_url,
            author_group_label: view.author.group_label,
            author_group_abbrev: view.author.group_abbrev,
            author_kind: summary.author_kind,
            author_actor_uid: summary.author_actor_uid,
            author_group_origin: summary.author_group_origin,
            author_group_ambiguous: summary.author_group_ambiguous,
            fate_code: summary.fate_code,
            fate_label: summary.fate_label,
            state_label: summary.state_label,
            deposited_on: summary.deposited_on,
            summary: summary.summary,
            cosignatory_count: summary.cosignatory_count,
        }
    }
}

#[derive(Serialize)]
pub struct AmendmentCoverageDto {
    /// Amendements en base, tous dossiers confondus. Distingue « rien n'est
    /// ingere » de « rien ne se rattache a ce dossier » (RM-01, README §2).
    pub base_total: i64,
    pub total: i64,
    pub without_summary: i64,
    pub unknown_fates: i64,
}

impl From<DossierAmendmentCoverage> for AmendmentCoverageDto {
    fn from(coverage: DossierAmendmentCoverage) -> Self {
        Self {
            base_total: coverage.base_total,
            total: coverage.total,
            without_summary: coverage.without_summary,
            unknown_fates: coverage.unknown_fates,
        }
    }
}

#[derive(Serialize)]
pub struct DossierAmendmentsResponse {
    /// Total du dossier, pagination exclue.
    pub total: i64,
    pub count: usize,
    pub offset: i64,
    pub limit: i64,
    pub amendments: Vec<AmendmentDto>,
    pub groups: Vec<AmendmentGroupDto>,
    pub coverage: AmendmentCoverageDto,
    pub coverage_note: &'static str,
    pub pagination_note: &'static str,
    pub source_note: &'static str,
}

#[derive(Serialize)]
pub struct AmendmentGroupDto {
    pub uid: String,
    pub label: String,
    pub abbrev: String,
}

impl From<AmendmentGroupOption> for AmendmentGroupDto {
    fn from(group: AmendmentGroupOption) -> Self {
        Self {
            uid: group.uid,
            label: group.label,
            abbrev: group.abbrev,
        }
    }
}

impl DossierAmendmentsResponse {
    pub fn new(page: DossierAmendments, offset: i64, limit: i64) -> Self {
        let amendments: Vec<AmendmentDto> =
            page.items.into_iter().map(AmendmentDto::from).collect();
        Self {
            total: page.total,
            count: amendments.len(),
            offset,
            limit,
            amendments,
            groups: page
                .groups
                .into_iter()
                .map(AmendmentGroupDto::from)
                .collect(),
            coverage: AmendmentCoverageDto::from(page.coverage),
            coverage_note: AMENDMENT_COVERAGE_NOTE,
            pagination_note: AMENDMENT_PAGINATION_NOTE,
            source_note: AMENDMENT_SOURCE_NOTE,
        }
    }
}

#[derive(Serialize)]
pub struct AmendmentsRefreshResponse {
    pub skipped_unchanged: bool,
    pub written: usize,
    pub pending: usize,
    pub json_entries: usize,
    pub parsed: usize,
    pub undecodable: usize,
    pub malformed: usize,
    pub refused: usize,
    pub unreadable: usize,
    pub failures: std::collections::BTreeMap<String, usize>,
    pub top_level: std::collections::BTreeMap<String, usize>,
    pub without_text_ref: usize,
    pub other_legislature: usize,
    pub unknown_fates: std::collections::BTreeMap<String, usize>,
    pub groups_published: usize,
    pub groups_resolved: usize,
    pub groups_unresolved: usize,
    pub groups_ambiguous: usize,
    pub groups_undated: usize,
    pub registry_anomaly: Option<String>,
    pub dossier_summaries: Option<crate::api::dto::DossierSummaryRefreshResponse>,
}

impl From<AmendmentsSummary> for AmendmentsRefreshResponse {
    fn from(summary: AmendmentsSummary) -> Self {
        Self {
            skipped_unchanged: summary.skipped_unchanged,
            written: summary.written,
            pending: summary.pending,
            json_entries: summary.json_entries,
            parsed: summary.parsed,
            undecodable: summary.undecodable,
            malformed: summary.malformed,
            refused: summary.refused,
            unreadable: summary.unreadable,
            failures: summary.failures,
            top_level: summary.top_level,
            without_text_ref: summary.without_text_ref,
            other_legislature: summary.other_legislature,
            unknown_fates: summary.unknown_fates,
            groups_published: summary.groups.published,
            groups_resolved: summary.groups.resolved,
            groups_unresolved: summary.groups.unresolved,
            groups_ambiguous: summary.groups.ambiguous,
            groups_undated: summary.groups.undated,
            registry_anomaly: summary.registry_anomaly,
            dossier_summaries: None,
        }
    }
}
