use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::application::ports::scrutin_repository::{ScrutinFilter, ScrutinPage, ScrutinSummary};
use crate::application::use_cases::get_scrutin_detail::ScrutinDetail;
use crate::application::use_cases::refresh_scrutins::ScrutinsSummary;
use crate::domain::scrutin::{scrutin_official_url, VoteTally};

/// RM-06: la lacune est servie avec les listes, pas enfouie ailleurs.
pub const SHOW_OF_HANDS_NOTE: &str = "Les votes \u{00e0} main lev\u{00e9}e ne figurent pas dans les donn\u{00e9}es publi\u{00e9}es par l'Assembl\u{00e9}e nationale : le site n'en rend pas compte.";

/// RM-03: mention portee par toute repartition produite par le site.
pub const RECONSTRUCTED_NOTE: &str = "R\u{00e9}partition reconstitu\u{00e9}e \u{00e0} partir du d\u{00e9}compte nominatif \u{2014} la source ne publie pas les groupes sur ce scrutin.";

#[derive(Debug, Deserialize)]
pub struct ScrutinListQuery {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub outcome: Option<String>,
    pub ballot_type: Option<String>,
    pub with_dossier: Option<bool>,
    pub dossier: Option<String>,
    pub search: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

impl From<ScrutinListQuery> for ScrutinFilter {
    fn from(q: ScrutinListQuery) -> Self {
        Self {
            from: q.from,
            to: q.to,
            outcome_code: q.outcome,
            ballot_type_code: q.ballot_type,
            with_dossier: q.with_dossier,
            dossier_uid: q.dossier,
            search: q.search,
            limit: q.limit,
            offset: q.offset,
        }
    }
}

#[derive(Serialize)]
pub struct TallyDto {
    pub votes_for: u16,
    pub votes_against: u16,
    pub abstentions: u16,
    pub not_voting: u16,
    /// Publie par groupe, absent de la synthese officielle qui affiche 0.
    pub voluntary_not_voting: u16,
}

impl From<VoteTally> for TallyDto {
    fn from(t: VoteTally) -> Self {
        Self {
            votes_for: t.votes_for,
            votes_against: t.votes_against,
            abstentions: t.abstentions,
            not_voting: t.not_voting,
            voluntary_not_voting: t.voluntary_not_voting,
        }
    }
}

#[derive(Serialize)]
pub struct ScrutinSummaryDto {
    pub uid: String,
    pub number: String,
    pub date: NaiveDate,
    pub subject: String,
    pub ballot_type: String,
    pub outcome_code: String,
    pub outcome_label: String,
    pub tally: TallyDto,
    pub dossier_uid: Option<String>,
    pub dossier_label: Option<String>,
    pub has_reconstructed_tallies: bool,
    /// RM-07: le lien source accompagne chaque scrutin, jusque dans la liste.
    pub official_url: String,
}

impl From<ScrutinSummary> for ScrutinSummaryDto {
    fn from(s: ScrutinSummary) -> Self {
        let official_url = scrutin_official_url(s.legislature, &s.number);
        Self {
            uid: s.uid,
            number: s.number,
            date: s.date,
            subject: s.subject,
            ballot_type: s.ballot_type_label,
            outcome_code: s.outcome_code,
            outcome_label: s.outcome_label,
            tally: s.tally.into(),
            dossier_uid: s.dossier_uid,
            dossier_label: s.dossier_label,
            has_reconstructed_tallies: s.has_reconstructed_tallies,
            official_url,
        }
    }
}

#[derive(Serialize)]
pub struct ScrutinListResponse {
    pub total: i64,
    pub count: usize,
    pub offset: i64,
    pub scrutins: Vec<ScrutinSummaryDto>,
    pub coverage_note: &'static str,
}

impl From<(ScrutinPage, i64)> for ScrutinListResponse {
    fn from((page, offset): (ScrutinPage, i64)) -> Self {
        let scrutins: Vec<ScrutinSummaryDto> = page
            .items
            .into_iter()
            .map(ScrutinSummaryDto::from)
            .collect();
        Self {
            total: page.total,
            count: scrutins.len(),
            offset,
            scrutins,
            coverage_note: SHOW_OF_HANDS_NOTE,
        }
    }
}

#[derive(Serialize)]
pub struct DossierScrutinsResponse {
    pub total: i64,
    pub count: usize,
    pub offset: i64,
    pub scrutins: Vec<ScrutinSummaryDto>,
    pub coverage_note: &'static str,
}

#[derive(Serialize)]
pub struct VoteDto {
    pub actor_uid: String,
    /// Absent quand l'acteur ne figure pas au referentiel (ACTEURS RM-04).
    pub full_name: Option<String>,
    pub official_url: Option<String>,
    pub position: String,
    /// Code publie par la source, affiche tel quel faute de libelle officiel.
    pub cause_code: Option<String>,
    pub by_delegation: bool,
    pub seat: Option<u16>,
}

#[derive(Serialize)]
pub struct GroupBreakdownDto {
    pub group_uid: Option<String>,
    pub abbrev: Option<String>,
    /// Libelle officiel, jamais traduit en parti (ACTEURS RM-06).
    pub label: Option<String>,
    pub color: Option<String>,
    pub member_count: Option<u16>,
    pub majority_position: Option<String>,
    pub tally: TallyDto,
    pub origin: String,
    /// Servie avec la ligne quand elle est reconstruite (RM-03).
    pub method_note: Option<&'static str>,
    pub votes: Vec<VoteDto>,
}

#[derive(Serialize)]
pub struct CorrectionDto {
    pub actor_uid: String,
    pub full_name: Option<String>,
    pub claimed_position: String,
    /// Vrai quand la source la classe en dysfonctionnement du materiel de vote.
    pub malfunction: bool,
}

#[derive(Serialize)]
pub struct SynthesisDto {
    pub voters: u16,
    pub expressed: u16,
    pub required: u16,
    pub announcement: String,
    pub tally: TallyDto,
}

#[derive(Serialize)]
pub struct ScrutinDetailDto {
    pub uid: String,
    pub number: String,
    pub legislature: u16,
    pub date: NaiveDate,
    pub session_ref: Option<String>,
    pub sitting_ref: Option<String>,
    pub place: Option<String>,
    pub ballot_type_code: String,
    pub ballot_type_label: String,
    pub majority_label: Option<String>,
    pub outcome_code: String,
    pub outcome_label: String,
    pub requester: Option<String>,
    pub subject: String,
    pub synthesis: SynthesisDto,
    pub groups: Vec<GroupBreakdownDto>,
    /// RM-05: affichees a part, sans effet sur les decomptes.
    pub corrections: Vec<CorrectionDto>,
    pub dossier_uid: Option<String>,
    pub dossier_label: Option<String>,
    pub official_url: String,
    pub unknown_actors: usize,
    pub coverage_note: &'static str,
}

impl From<ScrutinDetail> for ScrutinDetailDto {
    fn from(detail: ScrutinDetail) -> Self {
        let s = detail.scrutin;
        Self {
            uid: s.uid().as_str().to_string(),
            number: s.number().to_string(),
            legislature: s.legislature(),
            date: s.date(),
            session_ref: s.session_ref().map(str::to_string),
            sitting_ref: s.sitting_ref().map(str::to_string),
            place: s.place().map(str::to_string),
            ballot_type_code: s.ballot_type().code().to_string(),
            ballot_type_label: s.ballot_type().label().to_string(),
            majority_label: s.ballot_type().majority().map(str::to_string),
            outcome_code: s.outcome().code().to_string(),
            outcome_label: s.outcome().label().to_string(),
            requester: s.requester().map(str::to_string),
            subject: s.subject().to_string(),
            synthesis: SynthesisDto {
                voters: s.synthesis().voters,
                expressed: s.synthesis().expressed,
                required: s.synthesis().required,
                announcement: s.synthesis().announcement.clone(),
                tally: s.synthesis().tally.into(),
            },
            groups: detail
                .groups
                .into_iter()
                .map(|g| {
                    let reconstructed =
                        matches!(g.origin, crate::domain::scrutin::TallyOrigin::Reconstructed);
                    GroupBreakdownDto {
                        group_uid: g.group_uid,
                        abbrev: g.abbrev,
                        label: g.label,
                        color: g.color,
                        member_count: g.member_count,
                        majority_position: g.majority_position.map(|p| p.as_str().to_string()),
                        tally: g.tally.into(),
                        origin: g.origin.as_str().to_string(),
                        method_note: reconstructed.then_some(RECONSTRUCTED_NOTE),
                        votes: g
                            .votes
                            .into_iter()
                            .map(|v| VoteDto {
                                actor_uid: v.actor_uid,
                                full_name: v.full_name,
                                official_url: v.official_url,
                                position: v.position.as_str().to_string(),
                                cause_code: v.cause.map(|c| c.as_str().to_string()),
                                by_delegation: v.by_delegation,
                                seat: v.seat,
                            })
                            .collect(),
                    }
                })
                .collect(),
            corrections: detail
                .corrections
                .into_iter()
                .map(|c| CorrectionDto {
                    actor_uid: c.actor_uid,
                    full_name: c.full_name,
                    claimed_position: c.claimed_position.as_str().to_string(),
                    malfunction: c.malfunction,
                })
                .collect(),
            dossier_uid: s.dossier().map(|d| d.uid.to_string()),
            dossier_label: s.dossier().map(|d| d.label.clone()),
            official_url: s.official_url(),
            unknown_actors: detail.unknown_actors,
            coverage_note: SHOW_OF_HANDS_NOTE,
        }
    }
}

#[derive(Serialize)]
pub struct ScrutinsRefreshResponse {
    pub scrutins: usize,
    pub without_dossier: usize,
    pub reconstructed_scrutins: usize,
    pub unresolved_votes: usize,
    pub registry_anomaly: Option<String>,
}

impl From<ScrutinsSummary> for ScrutinsRefreshResponse {
    fn from(s: ScrutinsSummary) -> Self {
        Self {
            scrutins: s.scrutins,
            without_dossier: s.without_dossier,
            reconstructed_scrutins: s.reconstructed_scrutins,
            unresolved_votes: s.unresolved_votes,
            registry_anomaly: s.registry_anomaly,
        }
    }
}
