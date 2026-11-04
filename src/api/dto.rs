use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::api::theme_dto::AssignedFamilyDto;
use crate::application::ports::dossier_repository::DossierCriteria;
use crate::application::use_cases::browse_dossiers::{
    DossierCard, InitiatorView, DEFAULT_PER_PAGE,
};
use crate::application::use_cases::check_health::HealthReport;
use crate::domain::dossier::{CurationStatus, DossierOutcome, Initiative, LegislativeDossier};
use crate::domain::theme::FamilyCode;

#[derive(Deserialize)]
pub struct RecentActivityQuery {
    #[serde(default = "default_days")]
    pub days: u32,
}

fn default_days() -> u32 {
    7
}

#[derive(Serialize)]
pub struct DossierDto {
    pub uid: String,
    pub title: String,
    pub procedure: String,
    pub legislature: u16,
    pub url: Option<String>,
    pub deposit_date: Option<NaiveDate>,
    pub last_activity_date: NaiveDate,
    pub last_activity_label: String,
    pub score_total: u8,
    pub current_stage: Option<StageDto>,
    pub committee: Option<String>,
    pub curation_status: String,
    pub outcome: OutcomeDto,
}

/// Sort du dossier tel qu'il s'affiche.
///
/// `label` est libelle pour etre lu tel quel: quand la source ne conclut rien,
/// il le dit, il ne comble pas (README.md §6). Le dernier acte du dossier
/// reste la seule information disponible dans ce cas.
#[derive(Serialize)]
pub struct OutcomeDto {
    pub kind: String,
    pub label: String,
    pub date: Option<NaiveDate>,
    /// Vrai quand le sort ne peut plus changer. Un rejet n'en est pas un: la
    /// navette peut reprendre.
    pub is_final: bool,
    pub law_code: Option<String>,
    pub law_jo_date: Option<NaiveDate>,
    pub legifrance_url: Option<String>,
    pub merged_into_uid: Option<String>,
}

impl From<&DossierOutcome> for OutcomeDto {
    fn from(o: &DossierOutcome) -> Self {
        let mut dto = Self {
            kind: o.kind().to_string(),
            label: String::new(),
            date: o.date(),
            is_final: o.is_final(),
            law_code: None,
            law_jo_date: None,
            legifrance_url: None,
            merged_into_uid: None,
        };

        match o {
            DossierOutcome::Promulgated { publication, .. } => {
                dto.label = "Promulgu\u{00e9}e".into();
                dto.law_code = publication.law_code.clone();
                dto.law_jo_date = publication.jo_date;
                dto.legifrance_url = publication.legifrance_url.clone();
            }
            DossierOutcome::Withdrawn { .. } => {
                dto.label = "Initiative retir\u{00e9}e".into();
            }
            DossierOutcome::MergedInto { dossier_uid, .. } => {
                dto.label = "Absorb\u{00e9} par un autre dossier".into();
                dto.merged_into_uid = Some(dossier_uid.as_str().to_string());
            }
            // Le libelle vient de la source, mot pour mot: « rejete »,
            // « considere comme rejete en application de l'article 49-3 »...
            DossierOutcome::Rejected { label, .. } => {
                dto.label = label.clone();
            }
            DossierOutcome::NoRecordedConclusion => {
                dto.label = "Sans conclusion enregistr\u{00e9}e".into();
            }
        }

        dto
    }
}

impl From<LegislativeDossier> for DossierDto {
    fn from(d: LegislativeDossier) -> Self {
        Self {
            outcome: OutcomeDto::from(d.outcome()),
            uid: d.uid().as_str().to_string(),
            title: d.title().to_string(),
            procedure: d.procedure().to_string(),
            legislature: d.legislature(),
            url: d.url().map(str::to_string),
            deposit_date: d.deposit_date(),
            last_activity_date: d.last_activity_date(),
            last_activity_label: d.last_activity_label().to_string(),
            score_total: d.score().total(),
            current_stage: d.current_stage().map(StageDto::from),
            committee: d.committee().map(|c| c.as_str().to_string()),
            curation_status: d.curation_status().as_str().to_string(),
        }
    }
}

#[derive(Serialize)]
pub struct RecentDossiersResponse {
    pub count: usize,
    pub dossiers: Vec<DossierDto>,
}

#[derive(Deserialize)]
pub struct DossierPageQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
    /// Fragment cherché dans le titre.
    pub search: Option<String>,
    /// Sort du dossier: `promulgated`, `rejected`, `withdrawn`, `merged_into`,
    /// `no_recorded_conclusion`.
    pub outcome: Option<String>,
    /// `government` ou `parliamentary`.
    pub initiative: Option<String>,
    /// Code d'une famille de thèmes (`logement`, `sante`…).
    pub family: Option<String>,
}

fn default_page() -> i64 {
    1
}

fn default_per_page() -> i64 {
    DEFAULT_PER_PAGE
}

/// `?search=` sans valeur est une absence de critere, pas un titre vide.
fn non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Un parametre vide ou illisible ne restreint rien: comme pour la pagination,
/// la liste ramene la demande a ce qu'elle sait faire plutot que de la refuser.
impl From<&DossierPageQuery> for DossierCriteria {
    fn from(q: &DossierPageQuery) -> Self {
        Self {
            search: non_empty(q.search.as_deref()),
            outcome_kinds: q
                .outcome
                .as_deref()
                .unwrap_or_default()
                .split(',')
                .map(str::trim)
                .filter(|kind| !kind.is_empty())
                .map(str::to_string)
                .collect(),
            initiative: q
                .initiative
                .as_deref()
                .map(str::trim)
                .and_then(Initiative::parse),
            family: q
                .family
                .as_deref()
                .map(str::trim)
                .and_then(|code| FamilyCode::parse(code).ok()),
        }
    }
}

#[derive(Serialize)]
pub struct DossierPageResponse {
    /// Page réellement servie, après remise dans les bornes.
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub total_pages: i64,
    pub dossiers: Vec<DossierListItemDto>,
}

/// Carte de la liste : le dossier, qui l'a déposé, ses thèmes et ses votes.
#[derive(Serialize)]
pub struct DossierListItemDto {
    #[serde(flatten)]
    pub dossier: DossierDto,
    pub initiators: Vec<DossierInitiatorDto>,
    pub families: Vec<AssignedFamilyDto>,
    pub scrutin_count: i64,
}

impl From<DossierCard> for DossierListItemDto {
    fn from(card: DossierCard) -> Self {
        Self {
            dossier: DossierDto::from(card.listing.dossier),
            initiators: card
                .initiators
                .into_iter()
                .map(DossierInitiatorDto::from)
                .collect(),
            families: card
                .listing
                .families
                .into_iter()
                .map(AssignedFamilyDto::from)
                .collect(),
            scrutin_count: card.listing.scrutin_count,
        }
    }
}

/// Initiateur d'une carte. `group_color` absent : le groupe ne publie pas de
/// couleur, il s'affiche en texte.
#[derive(Serialize)]
pub struct DossierInitiatorDto {
    pub full_name: String,
    /// `deputy`, `minister`, `senator`, `other` ; absent si non résolu.
    pub role: Option<String>,
    pub group_abbrev: Option<String>,
    pub group_label: Option<String>,
    pub group_color: Option<String>,
    pub group_text_tone: Option<String>,
}

impl From<InitiatorView> for DossierInitiatorDto {
    fn from(view: InitiatorView) -> Self {
        Self {
            full_name: view.full_name,
            role: view.role.map(|role| role.as_str().to_string()),
            group_abbrev: view.group_abbrev,
            group_label: view.group_label,
            group_color: view.group_color,
            group_text_tone: view.text_tone.map(|tone| tone.as_str().to_string()),
        }
    }
}

#[derive(Serialize)]
pub struct ActDto {
    pub date: NaiveDate,
    pub label: String,
    pub code: Option<String>,
}

#[derive(Serialize)]
pub struct DocumentDto {
    pub document_uid: String,
    pub title: String,
    pub short_title: Option<String>,
    pub doc_type: String,
    pub date: Option<NaiveDate>,
    pub official_url: Option<String>,
    pub source_archive_url: Option<String>,
    pub source_license: Option<String>,
    pub source_metadata_fingerprint: Option<String>,
    pub source_retrieved_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Serialize)]
pub struct ScoreDto {
    pub progress: u8,
    pub magnitude: u8,
    pub momentum: u8,
    pub total: u8,
}

#[derive(Serialize)]
pub struct StageDto {
    pub label: String,
    pub chamber: String,
}

impl From<crate::domain::dossier::LegislativeStage> for StageDto {
    fn from(s: crate::domain::dossier::LegislativeStage) -> Self {
        let chamber = match s.chamber() {
            crate::domain::dossier::Chamber::AssembleeNationale => {
                "Assembl\u{00e9}e nationale".to_string()
            }
            crate::domain::dossier::Chamber::Senat => "S\u{00e9}nat".to_string(),
            crate::domain::dossier::Chamber::Joint => "Conjointe".to_string(),
            crate::domain::dossier::Chamber::None => String::new(),
        };
        Self {
            label: s.label().to_string(),
            chamber,
        }
    }
}

#[derive(Serialize)]
pub struct InitiatorGroupDto {
    pub uid: String,
    pub abbrev: String,
    /// Libelle officiel du groupe, jamais traduit en parti (RM-06).
    pub label: String,
    pub quality: Option<String>,
}

#[derive(Serialize)]
pub struct InitiatorDto {
    pub full_name: String,
    pub actor_uid: Option<String>,
    pub role: Option<String>,
    pub group: Option<InitiatorGroupDto>,
    /// Date a laquelle le groupe a ete lu. Toujours servie avec le groupe: un
    /// groupe sans sa date de reference n'est pas affichable (RM-01).
    pub reference_date: Option<NaiveDate>,
    pub official_url: Option<String>,
}

impl From<crate::domain::dossier::Initiator> for InitiatorDto {
    fn from(i: crate::domain::dossier::Initiator) -> Self {
        Self {
            full_name: i.full_name().to_string(),
            actor_uid: i.actor_uid().map(|u| u.as_str().to_string()),
            role: i.role().map(|r| r.label().to_string()),
            group: i.group().map(|g| InitiatorGroupDto {
                uid: g.uid.as_str().to_string(),
                abbrev: g.abbrev.clone(),
                label: g.label.clone(),
                quality: g.quality.as_ref().map(|q| q.as_str().to_string()),
            }),
            reference_date: i.reference_date(),
            official_url: i.official_url().map(String::from),
        }
    }
}

#[derive(Serialize)]
pub struct RegistryResponse {
    pub actors: usize,
    pub groups: usize,
    pub memberships: usize,
}

impl From<crate::application::ports::actor_repository::RegistrySummary> for RegistryResponse {
    fn from(s: crate::application::ports::actor_repository::RegistrySummary) -> Self {
        Self {
            actors: s.actors,
            groups: s.groups,
            memberships: s.memberships,
        }
    }
}

/// `?full=true` reecrit tous les dossiers au lieu des seuls qui ont bouge.
#[derive(Deserialize)]
pub struct RefreshQuery {
    #[serde(default)]
    pub full: bool,
}

#[derive(Serialize)]
pub struct DossiersRefreshResponse {
    /// Dossiers lus dans la source.
    pub seen: usize,
    /// Dossiers reecrits en base.
    pub written: usize,
    /// Sautes parce que leur sort est definitif.
    pub skipped_final: usize,
    /// Sautes parce que rien n'a bouge depuis la derniere ingestion.
    pub skipped_unchanged: usize,
}

impl From<crate::application::use_cases::refresh_dossiers::DossiersSummary>
    for DossiersRefreshResponse
{
    fn from(s: crate::application::use_cases::refresh_dossiers::DossiersSummary) -> Self {
        Self {
            seen: s.seen,
            written: s.written,
            skipped_final: s.skipped_final,
            skipped_unchanged: s.skipped_unchanged,
        }
    }
}

#[derive(Serialize)]
pub struct RefreshResponse {
    /// Dossiers presents dans la source. Le detail de ce qui a ete reecrit est
    /// dans `dossiers`.
    pub count: usize,
    pub dossiers: DossiersRefreshResponse,
    pub registry: Option<RegistryResponse>,
    /// Renseigne quand le referentiel n'a pas pu etre rafraichi: les
    /// rattachements reposent alors sur la version precedente.
    pub registry_anomaly: Option<String>,
    pub scrutins: Option<crate::api::scrutin_dto::ScrutinsRefreshResponse>,
    /// Renseigne quand la source des scrutins n'a pas repondu: les scrutins
    /// deja stockes restent en place.
    pub scrutins_anomaly: Option<String>,
    /// Extraction des textes debattus, deterministe (RM-02).
    pub extraction: Option<crate::api::theme_dto::ExtractionResponse>,
    pub extraction_anomaly: Option<String>,
    /// Passe de rattachement thematique, plafonnee (RM-14).
    pub themes: Option<crate::api::theme_dto::ProposalRunResponse>,
    /// Renseigne quand la passe n'a pas pu avoir lieu: les objets concernes
    /// restent non rattaches et consultables (RM-01).
    pub themes_anomaly: Option<String>,
    /// Passe d'ingestion des amendements, plafonnee elle aussi.
    pub amendments: Option<crate::api::amendment_dto::AmendmentsRefreshResponse>,
    /// Renseigne quand la source des amendements n'a pas repondu: ceux deja
    /// stockes restent en place.
    pub amendments_anomaly: Option<String>,
    pub dossier_summaries: Option<DossierSummaryRefreshResponse>,
    /// Passe « En bref » : versions enregistrées, textes lus et lignes écrites
    /// (SPEC-en-bref CU-01, CU-02). Sa défaillance est dans
    /// `text_briefs.anomaly`, jamais un échec de la route (RM-18).
    pub text_briefs: Option<TextBriefRefreshResponse>,
}

#[derive(Serialize)]
pub struct TextBriefRefreshResponse {
    pub versions_seen: usize,
    pub dossiers_read: usize,
    pub texts_read: usize,
    pub texts_missing: usize,
    /// Articles dont l'empreinte avait déjà une ligne publiable.
    pub lines_reused: usize,
    /// Lignes publiables écrites, lignes fixes des articles supprimés comprises.
    pub lines_written: usize,
    /// Refus écrits, avec leur motif.
    pub lines_rejected: usize,
    /// Articles sans ligne à la fin de la passe : source manquante ou modèle en
    /// échec.
    pub articles_pending: usize,
    pub anomaly: Option<String>,
}

impl From<crate::application::use_cases::refresh_text_briefs::TextBriefRefreshReport>
    for TextBriefRefreshResponse
{
    fn from(
        report: crate::application::use_cases::refresh_text_briefs::TextBriefRefreshReport,
    ) -> Self {
        Self {
            versions_seen: report.versions_seen,
            dossiers_read: report.dossiers_read,
            texts_read: report.texts_read,
            texts_missing: report.texts_missing,
            lines_reused: report.lines_reused,
            lines_written: report.lines_written,
            lines_rejected: report.lines_rejected,
            articles_pending: report.articles_pending,
            anomaly: report.anomaly,
        }
    }
}

#[derive(Serialize)]
pub struct DossierSummaryRefreshResponse {
    pub dossiers_seen: usize,
    pub dossiers_refreshed: usize,
    pub summaries_ready: usize,
    pub summaries_pending: usize,
    pub anomaly: Option<String>,
}

impl From<crate::application::use_cases::refresh_all::RefreshOutcome> for RefreshResponse {
    fn from(o: crate::application::use_cases::refresh_all::RefreshOutcome) -> Self {
        Self {
            count: o.dossiers.seen,
            dossiers: DossiersRefreshResponse::from(o.dossiers),
            registry: o.registry.map(RegistryResponse::from),
            registry_anomaly: o.registry_anomaly,
            scrutins: o
                .scrutins
                .map(crate::api::scrutin_dto::ScrutinsRefreshResponse::from),
            scrutins_anomaly: o.scrutins_anomaly,
            extraction: o
                .extraction
                .map(crate::api::theme_dto::ExtractionResponse::from),
            extraction_anomaly: o.extraction_anomaly,
            themes: o
                .themes
                .map(crate::api::theme_dto::ProposalRunResponse::from),
            themes_anomaly: o.themes_anomaly,
            amendments: o
                .amendments
                .map(crate::api::amendment_dto::AmendmentsRefreshResponse::from),
            amendments_anomaly: o.amendments_anomaly,
            dossier_summaries: None,
            text_briefs: None,
        }
    }
}

impl
    From<(
        crate::application::use_cases::refresh_all::RefreshOutcome,
        crate::application::use_cases::refresh_dossier_group_summaries::DossierSummaryRefreshReport,
        crate::application::use_cases::refresh_text_briefs::TextBriefRefreshReport,
    )> for RefreshResponse
{
    fn from(
        (o, summaries, text_briefs): (
            crate::application::use_cases::refresh_all::RefreshOutcome,
            crate::application::use_cases::refresh_dossier_group_summaries::DossierSummaryRefreshReport,
            crate::application::use_cases::refresh_text_briefs::TextBriefRefreshReport,
        ),
    ) -> Self {
        let mut response = RefreshResponse::from(o);
        response.dossier_summaries = Some(DossierSummaryRefreshResponse {
            dossiers_seen: summaries.dossiers_seen,
            dossiers_refreshed: summaries.dossiers_refreshed,
            summaries_ready: summaries.summaries_ready,
            summaries_pending: summaries.summaries_pending,
            anomaly: summaries.anomaly,
        });
        response.text_briefs = Some(TextBriefRefreshResponse::from(text_briefs));
        response
    }
}

#[derive(Serialize)]
pub struct DossierDetailDto {
    pub uid: String,
    pub title: String,
    pub procedure: String,
    pub legislature: u16,
    pub url: Option<String>,
    pub summary: Option<String>,
    pub deposit_date: Option<NaiveDate>,
    pub last_activity_date: NaiveDate,
    pub last_activity_label: String,
    pub acts: Vec<ActDto>,
    pub documents: Vec<DocumentDto>,
    pub score: ScoreDto,
    pub persisted: bool,
    pub current_stage: Option<StageDto>,
    pub initiators: Vec<InitiatorDto>,
    pub committee: Option<String>,
    pub curation_status: String,
    pub outcome: OutcomeDto,
    /// Familles rattachees au dossier, posees par le handler.
    pub families: Vec<AssignedFamilyDto>,
}

impl DossierDetailDto {
    pub fn from_result(
        result: crate::application::use_cases::get_dossier_detail::DossierDetailResult,
    ) -> Self {
        let d = result.dossier;
        Self {
            families: Vec::new(),
            outcome: OutcomeDto::from(d.outcome()),
            uid: d.uid().as_str().to_string(),
            title: d.title().to_string(),
            procedure: d.procedure().to_string(),
            legislature: d.legislature(),
            url: d.url().map(str::to_string),
            summary: d.summary().map(str::to_string),
            deposit_date: d.deposit_date(),
            last_activity_date: d.last_activity_date(),
            last_activity_label: d.last_activity_label().to_string(),
            acts: d
                .acts()
                .iter()
                .map(|a| ActDto {
                    date: a.date,
                    label: a.label.clone(),
                    code: a.code.clone(),
                })
                .collect(),
            documents: d
                .documents()
                .iter()
                .map(|doc| DocumentDto {
                    document_uid: doc.document_uid.clone(),
                    title: doc.title.clone(),
                    short_title: doc.short_title.clone(),
                    doc_type: doc.doc_type.clone(),
                    date: doc.date,
                    official_url: doc.official_url.clone(),
                    source_archive_url: doc.source_archive_url.clone(),
                    source_license: doc.source_license.clone(),
                    source_metadata_fingerprint: doc.source_metadata_fingerprint.clone(),
                    source_retrieved_at: doc.source_retrieved_at,
                })
                .collect(),
            score: ScoreDto {
                progress: d.score().progress(),
                magnitude: d.score().magnitude(),
                momentum: d.score().momentum(),
                total: d.score().total(),
            },
            persisted: result.persisted,
            current_stage: d.current_stage().map(StageDto::from),
            initiators: d
                .initiators()
                .iter()
                .cloned()
                .map(InitiatorDto::from)
                .collect(),
            committee: d.committee().map(|c| c.as_str().to_string()),
            curation_status: d.curation_status().as_str().to_string(),
        }
    }
}

#[derive(Deserialize)]
pub struct SuggestionsQuery {
    #[serde(default = "default_count")]
    pub count: usize,
}

fn default_count() -> usize {
    3
}

#[derive(Serialize)]
pub struct SuggestionsResponse {
    pub count: usize,
    pub suggestions: Vec<DossierDto>,
}

#[derive(Deserialize)]
pub struct CurateBody {
    pub status: CurationStatus,
}

/// Reponse du health check. Le serveur repond, donc `status` vaut toujours
/// `ok`; l'etat de la base est rendu a part, sans faire echouer la requete.
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub database: &'static str,
}

impl From<HealthReport> for HealthResponse {
    fn from(report: HealthReport) -> Self {
        Self {
            status: "ok",
            database: if report.database_reachable {
                "connected"
            } else {
                "disconnected"
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unreachable_database_still_answers_ok() {
        let response = HealthResponse::from(HealthReport {
            database_reachable: false,
        });

        assert_eq!(response.status, "ok");
        assert_eq!(response.database, "disconnected");
    }

    #[test]
    fn a_reachable_database_reads_connected() {
        let response = HealthResponse::from(HealthReport {
            database_reachable: true,
        });

        assert_eq!(response.database, "connected");
    }
}
