use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::api::dto::{OutcomeDto, StageDto};
use crate::api::theme_dto::AssignedFamilyDto;
use crate::application::ports::scrutin_repository::ScrutinFilter;
use crate::application::use_cases::browse_dossier_amendments::CitedAmendmentView;
use crate::application::use_cases::get_scrutin_detail::{
    CitedNumberView, ExplanationView, ExplanationsView, ScrutinDetail,
};
use crate::domain::vote_explanation::ExplanationsState;
use crate::application::use_cases::list_scrutins::{ScrutinListItem, ScrutinListPage};
use crate::application::use_cases::refresh_scrutins::ScrutinsSummary;
use crate::domain::amendment::SummaryPreview;
use crate::domain::dossier::LegislativeDossier;
use crate::domain::final_vote::{is_final_vote_subject, reading_of};
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
    /// RM-08: l'objet officiel, conserve tel que publie.
    pub subject: String,
    /// RM-09: libelle court calcule par le cas d'usage.
    pub short_label: String,
    pub ballot_type: String,
    pub outcome_code: String,
    pub outcome_label: String,
    pub tally: TallyDto,
    pub dossier_uid: Option<String>,
    pub dossier_label: Option<String>,
    /// Ou en est le dossier rattache, avec les memes sort et etape que la liste
    /// des dossiers. Absent sans dossier, ou quand le dossier n'est pas en base.
    pub dossier_status: Option<DossierStatusDto>,
    pub has_reconstructed_tallies: bool,
    /// RM-07: le lien source accompagne chaque scrutin, jusque dans la liste.
    pub official_url: String,
    /// RM-05, RM-08: amendements rapproches, dans l'ordre de citation; vide
    /// quand le scrutin n'est pas rapproche (la liste garde alors le libelle court).
    pub cited: Vec<CitedAmendmentDto>,
    /// Familles rattachees au scrutin, posees par le handler; vide sans rattachement.
    pub families: Vec<AssignedFamilyDto>,
}

impl From<ScrutinListItem> for ScrutinSummaryDto {
    fn from(item: ScrutinListItem) -> Self {
        let s = item.summary;
        let official_url = scrutin_official_url(s.legislature, &s.number);
        Self {
            uid: s.uid,
            number: s.number,
            date: s.date,
            subject: s.subject,
            short_label: item.short_label.as_str().to_string(),
            ballot_type: s.ballot_type_label,
            outcome_code: s.outcome_code,
            outcome_label: s.outcome_label,
            tally: s.tally.into(),
            dossier_uid: s.dossier_uid,
            dossier_label: s.dossier_label,
            dossier_status: item.dossier.as_ref().map(DossierStatusDto::from),
            has_reconstructed_tallies: s.has_reconstructed_tallies,
            official_url,
            cited: item
                .cited
                .into_iter()
                .map(CitedAmendmentDto::from)
                .collect(),
            families: Vec::new(),
        }
    }
}

/// Etat du dossier d'un scrutin, tel que la liste des dossiers le montre.
///
/// Le sort et l'etape sont ceux de `/api/dossiers`, memes types: quand la
/// source ne conclut rien, le libelle le dit et le dernier acte reste la seule
/// information (README.md §6).
#[derive(Serialize)]
pub struct DossierStatusDto {
    pub outcome: OutcomeDto,
    pub current_stage: Option<StageDto>,
    pub last_activity_label: String,
    pub last_activity_date: NaiveDate,
}

impl From<&LegislativeDossier> for DossierStatusDto {
    fn from(d: &LegislativeDossier) -> Self {
        Self {
            outcome: OutcomeDto::from(d.outcome()),
            current_stage: d.current_stage().map(StageDto::from),
            last_activity_label: d.last_activity_label().to_string(),
            last_activity_date: d.last_activity_date(),
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

impl From<(ScrutinListPage, i64)> for ScrutinListResponse {
    fn from((page, offset): (ScrutinListPage, i64)) -> Self {
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

/// Amendement mis aux voix: l'apercu de son expose dans la liste (RM-05), son
/// expose entier sur le detail (RM-06, `summary`).
#[derive(Serialize)]
pub struct CitedAmendmentDto {
    pub dossier_uid: String,
    pub number: String,
    pub target_title: String,
    /// Expose entier, servi par le detail seulement.
    pub summary: Option<String>,
    /// Apercu (RM-05); `None` quand la source ne publie pas d'expose.
    pub summary_preview: Option<String>,
    pub summary_preview_truncated: bool,
    pub author_name: Option<String>,
    pub author_group_abbrev: Option<String>,
    /// Couleur publiee du groupe au depot (RM-10); `None` quand il n'en publie pas.
    pub author_group_color: Option<String>,
    /// `white` ou `black`: ton du texte lisible sur la couleur du groupe.
    pub author_text_tone: Option<String>,
    pub cosignatory_count: i64,
}

impl From<CitedAmendmentView> for CitedAmendmentDto {
    fn from(view: CitedAmendmentView) -> Self {
        let (summary_preview, summary_preview_truncated) = match view.preview {
            SummaryPreview::Unpublished => (None, false),
            SummaryPreview::Text { text, truncated } => (Some(text), truncated),
        };
        Self {
            dossier_uid: view.dossier_uid,
            number: view.number,
            target_title: view.target_title,
            summary: view.summary,
            summary_preview,
            summary_preview_truncated,
            author_name: view.author.name,
            author_group_abbrev: view.author.group_abbrev,
            author_group_color: view.author.group_color,
            author_text_tone: view.author.text_tone.map(|tone| tone.as_str().to_string()),
            cosignatory_count: view.cosignatory_count,
        }
    }
}

/// Un numero cite: l'amendement et la methode du lien, ou la cause de l'absence (RM-06, RM-08).
#[derive(Serialize)]
pub struct CitedNumberDto {
    pub rank: u16,
    pub cited_number: Option<String>,
    pub amendment: Option<CitedAmendmentDto>,
    pub method_labels: Vec<&'static str>,
    pub cause_code: Option<&'static str>,
    pub cause_label: Option<&'static str>,
}

impl From<CitedNumberView> for CitedNumberDto {
    fn from(view: CitedNumberView) -> Self {
        let method_labels = view
            .amendment
            .as_ref()
            .and_then(|a| a.method)
            .map(|m| m.labels())
            .unwrap_or_default();
        Self {
            rank: view.rank,
            cited_number: view.cited_number,
            amendment: view.amendment.map(CitedAmendmentDto::from),
            method_labels,
            cause_code: view.cause.map(|c| c.as_str()),
            cause_label: view.cause.map(|c| c.label()),
        }
    }
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
    /// RM-06, RM-08: ce qui a ete mis aux voix, numero par numero.
    pub cited: Vec<CitedNumberDto>,
    pub unnumbered_identical_amendments: bool,
    pub coverage_note: &'static str,
    /// Familles courantes heritees du texte mis aux voix (THEMES RM-06).
    /// Vide tant que le texte n'est rattache a aucune famille.
    pub families: Vec<AssignedFamilyDto>,
    /// Mention de lecture d'un vote sur l'ensemble, reprise telle quelle de
    /// l'objet (« lecture definitive »). Absente sur tout autre vote.
    pub reading: Option<String>,
    /// RM-09: `None` hors vote sur l'ensemble.
    pub explanations: Option<ExplanationsDto>,
}

#[derive(Serialize)]
pub struct GroupWithoutSpeakerDto {
    pub group_uid: String,
    pub abbrev: Option<String>,
    pub label: Option<String>,
}

#[derive(Serialize)]
pub struct ExplanationDto {
    pub rank: u16,
    pub speaker_uid: String,
    pub speaker_name: Option<String>,
    pub group_uid: Option<String>,
    pub group_basis: String,
    pub speaker_position: Option<String>,
    pub group_tally: Option<TallyDto>,
    pub preview: Option<String>,
    pub preview_truncated: bool,
    pub text: String,
    pub official_url: String,
}

#[derive(Serialize)]
pub struct ExplanationsDto {
    pub state: String,
    pub source_url: Option<String>,
    pub report_version: Option<String>,
    pub items: Vec<ExplanationDto>,
    pub groups_without_speaker: Vec<GroupWithoutSpeakerDto>,
}

/// Etat code d'une section d'explications (RM-09): le libelle est au front.
pub(crate) fn explanations_state_code(state: ExplanationsState) -> &'static str {
    match state {
        ExplanationsState::NotPublished => "report_not_published",
        ExplanationsState::NotAttached => "not_attached",
        ExplanationsState::NoExplanation => "no_explanation",
        ExplanationsState::Explained => "explained",
    }
}

impl From<&ExplanationView> for ExplanationDto {
    fn from(view: &ExplanationView) -> Self {
        let (preview, preview_truncated) = match &view.preview {
            SummaryPreview::Text { text, truncated } => (Some(text.clone()), *truncated),
            SummaryPreview::Unpublished => (None, false),
        };
        Self {
            rank: view.rank,
            speaker_uid: view.speaker_uid.clone(),
            speaker_name: view.speaker_name.clone(),
            group_uid: view.group_uid.clone(),
            group_basis: view.group_basis.to_string(),
            speaker_position: view.speaker_position.map(|p| p.as_str().to_string()),
            group_tally: view.group_tally.map(TallyDto::from),
            preview,
            preview_truncated,
            text: view.text.clone(),
            official_url: view.url.clone(),
        }
    }
}

impl From<&ExplanationsView> for ExplanationsDto {
    fn from(view: &ExplanationsView) -> Self {
        Self {
            state: explanations_state_code(view.state).to_string(),
            source_url: view.source_url.clone(),
            report_version: view.report_version.clone(),
            items: view.items.iter().map(ExplanationDto::from).collect(),
            groups_without_speaker: view
                .groups_without_speaker
                .iter()
                .map(|group| GroupWithoutSpeakerDto {
                    group_uid: group.group_uid.clone(),
                    abbrev: group.abbrev.clone(),
                    label: group.label.clone(),
                })
                .collect(),
        }
    }
}

/// Seul un vote sur l'ensemble porte une mention de lecture: sur un
/// amendement ou un article, la parenthese finale de l'objet dit autre chose.
fn final_vote_reading(subject: &str) -> Option<String> {
    is_final_vote_subject(subject)
        .then(|| reading_of(subject, ""))
        .flatten()
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
            reading: final_vote_reading(s.subject()),
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
            cited: detail.cited.into_iter().map(CitedNumberDto::from).collect(),
            unnumbered_identical_amendments: detail.unnumbered_identical_amendments,
            coverage_note: SHOW_OF_HANDS_NOTE,
            families: Vec::new(),
            explanations: detail.explanations.as_ref().map(ExplanationsDto::from),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::use_cases::browse_dossier_amendments::{
        AuthorView, CitedAmendmentView,
    };
    use crate::application::use_cases::list_scrutins::ScrutinListItem;
    use crate::application::use_cases::read_fakes::scrutin_row;
    use crate::domain::actor::TextTone;
    use crate::domain::amendment::SummaryPreview;
    use crate::domain::amendment_citation::LinkMethod;
    use crate::domain::scrutin::{ShortLabel, VotePosition};
    use crate::application::use_cases::get_scrutin_detail::GroupWithoutSpeakerView;

    fn item(uid: &str, subject: &str) -> ScrutinListItem {
        ScrutinListItem {
            summary: scrutin_row(uid, subject),
            short_label: ShortLabel::from_subject(subject),
            cited: Vec::new(),
            dossier: None,
        }
    }

    #[test]
    fn should_serve_dossier_outcome_stage_and_last_act_when_scrutin_has_a_dossier() {
        use crate::application::use_cases::read_fakes::listed_dossier;
        use crate::domain::dossier::{DossierOutcome, LegislativeStage};

        let mut listed = item("S1", "l'article 3 du projet de loi de finances");
        listed.dossier = Some(listed_dossier(
            "DLR5L17N54629",
            DossierOutcome::Rejected {
                date: NaiveDate::from_ymd_opt(2026, 11, 4).unwrap(),
                label: "rejet\u{e9}".into(),
            },
            Some(LegislativeStage::FirstReadingAN),
        ));

        let json = serde_json::to_value(ScrutinSummaryDto::from(listed)).unwrap();

        let status = &json["dossier_status"];
        assert_eq!(status["outcome"]["kind"], "rejected");
        assert_eq!(status["outcome"]["label"], "rejet\u{e9}");
        assert_eq!(status["outcome"]["date"], "2026-11-04");
        assert_eq!(status["outcome"]["is_final"], false);
        assert_eq!(
            status["current_stage"]["chamber"],
            "Assembl\u{e9}e nationale"
        );
        assert_eq!(status["last_activity_label"], "R\u{e9}union de commission");
        assert_eq!(status["last_activity_date"], "2026-11-04");
    }

    #[test]
    fn should_serve_null_dossier_status_when_scrutin_has_no_dossier() {
        let json = serde_json::to_value(ScrutinSummaryDto::from(item("S1", "l'ensemble"))).unwrap();

        assert!(json["dossier_status"].is_null());
    }

    #[test]
    fn should_serve_reading_when_subject_is_a_final_vote() {
        assert_eq!(
            final_vote_reading(
                "l'ensemble de la proposition de loi relative au droit à l'aide à mourir (lecture définitive)."
            ),
            Some("lecture définitive".to_string())
        );
    }

    #[test]
    fn should_serve_no_reading_when_subject_is_not_a_final_vote() {
        assert_eq!(
            final_vote_reading("l'amendement n° 12 de M. Dupont (deuxième délibération)."),
            None
        );
    }

    #[test]
    fn should_serve_short_label_and_official_subject_when_list_response_is_built() {
        let page = ScrutinListPage {
            items: vec![item("S1", "l'article 3 du projet de loi de finances")],
            total: 1,
        };

        let json = serde_json::to_value(ScrutinListResponse::from((page, 0))).unwrap();

        assert_eq!(json["scrutins"][0]["short_label"], "Vote sur l'article 3");
        assert_eq!(
            json["scrutins"][0]["subject"],
            "l'article 3 du projet de loi de finances"
        );
    }

    fn cited_view(rank: u16, number: &str) -> CitedAmendmentView {
        CitedAmendmentView {
            rank,
            cited_number: Some(number.to_string()),
            dossier_uid: "DLR5L17N1".to_string(),
            number: number.to_string(),
            target_title: "Article 3".to_string(),
            author: AuthorView {
                name: Some("Marie Dupont".to_string()),
                official_url: None,
                group_label: Some("Gauche democrate".to_string()),
                group_abbrev: Some("GDR".to_string()),
                group_color: Some("#8c1c13".to_string()),
                text_tone: Some(TextTone::Light),
            },
            cosignatory_count: 3,
            preview: SummaryPreview::Text {
                text: "Supprime l'article.".to_string(),
                truncated: false,
            },
            summary: None,
            method: Some(LinkMethod {
                by_cited_article: false,
            }),
            cause: None,
        }
    }

    #[test]
    fn should_serve_cited_amendments_with_preview_in_citation_order_when_scrutin_is_linked() {
        let mut linked = item("S1", "l'amendement n° 12 de Mme Dupont a l'article 3");
        linked.cited = vec![cited_view(0, "12"), cited_view(1, "14")];
        let page = ScrutinListPage {
            items: vec![linked, item("S2", "objet inconnu de la source")],
            total: 2,
        };

        let json = serde_json::to_value(ScrutinListResponse::from((page, 0))).unwrap();

        let cited = &json["scrutins"][0]["cited"];
        assert_eq!(cited[0]["number"], "12");
        assert_eq!(cited[1]["number"], "14");
        assert_eq!(cited[0]["dossier_uid"], "DLR5L17N1");
        assert_eq!(cited[0]["target_title"], "Article 3");
        assert_eq!(cited[0]["author_name"], "Marie Dupont");
        assert_eq!(cited[0]["author_group_abbrev"], "GDR");
        assert_eq!(cited[0]["author_text_tone"], "white");
        assert_eq!(cited[0]["cosignatory_count"], 3);
        assert_eq!(cited[0]["summary_preview"], "Supprime l'article.");
        assert_eq!(cited[0]["summary_preview_truncated"], false);
        assert!(cited[0]["summary"].is_null());
        assert_eq!(json["scrutins"][1]["cited"], serde_json::json!([]));
    }

    #[test]
    fn should_serve_no_preview_when_cited_amendment_has_no_published_summary() {
        let mut view = cited_view(0, "12");
        view.preview = SummaryPreview::Unpublished;
        let mut linked = item("S1", "l'amendement n° 12");
        linked.cited = vec![view];
        let page = ScrutinListPage {
            items: vec![linked],
            total: 1,
        };

        let json = serde_json::to_value(ScrutinListResponse::from((page, 0))).unwrap();

        let cited = &json["scrutins"][0]["cited"][0];
        assert!(cited["summary_preview"].is_null());
        assert_eq!(cited["summary_preview_truncated"], false);
    }

    #[test]
    fn should_serve_short_labels_in_source_order_when_list_response_is_built() {
        let page = ScrutinListPage {
            items: vec![
                item("S2", "l'ensemble du projet de loi"),
                item("S1", "objet inconnu de la source"),
            ],
            total: 2,
        };

        let json = serde_json::to_value(ScrutinListResponse::from((page, 0))).unwrap();

        let labels: Vec<&str> = json["scrutins"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["short_label"].as_str().unwrap())
            .collect();
        assert_eq!(
            labels,
            vec!["Vote sur l'ensemble du texte", "objet inconnu de la source"]
        );
    }

    fn final_vote_detail(explanations: ExplanationsView) -> ScrutinDetail {
        use crate::domain::scrutin::{
            BallotType, NewScrutin, Outcome, Scrutin, ScrutinUid, VoteSynthesis,
        };

        let scrutin = Scrutin::new(NewScrutin {
            uid: ScrutinUid::new("V1".into()).unwrap(),
            number: "1".into(),
            legislature: 17,
            date: NaiveDate::from_ymd_opt(2025, 3, 27).unwrap(),
            session_ref: None,
            sitting_ref: Some("RUANR5L17S2025IDS1".into()),
            place: None,
            ballot_type: BallotType::new("SPO".into(), "ordinaire".into(), None).unwrap(),
            outcome: Outcome::new("adopt\u{e9}".into(), "adopt\u{e9}".into()).unwrap(),
            requester: None,
            subject: "l'ensemble du projet de loi".into(),
            synthesis: VoteSynthesis {
                voters: 3,
                expressed: 3,
                required: 2,
                announcement: "annonce".into(),
                tally: VoteTally::default(),
            },
            group_tallies: Vec::new(),
            nominal_votes: Vec::new(),
            corrections: Vec::new(),
            dossier: None,
        })
        .unwrap();
        ScrutinDetail {
            scrutin,
            groups: Vec::new(),
            corrections: Vec::new(),
            unknown_actors: 0,
            cited: Vec::new(),
            unnumbered_identical_amendments: false,
            explanations: Some(explanations),
        }
    }

    fn explanation_view(preview: SummaryPreview) -> ExplanationView {
        ExplanationView {
            rank: 1,
            speaker_uid: "PA1".into(),
            speaker_name: Some("Marie Dupont".into()),
            group_uid: Some("PO_A".into()),
            group_basis: "membership",
            speaker_position: Some(VotePosition::Against),
            group_tally: Some(VoteTally {
                votes_for: 2,
                votes_against: 5,
                abstentions: 1,
                ..VoteTally::default()
            }),
            preview,
            text: "Texte complet de l'explication.".into(),
            url: "https://www.assemblee-nationale.fr/dyn/17/comptes-rendus/seance/CRSANR5L17S2025IDS1#point_3".into(),
        }
    }

    #[test]
    fn maps_explanations_with_state_preview_and_official_link() {
        let view = ExplanationsView {
            state: ExplanationsState::Explained,
            items: vec![explanation_view(SummaryPreview::Text {
                text: "Texte complet".into(),
                truncated: true,
            })],
            groups_without_speaker: vec![GroupWithoutSpeakerView {
                group_uid: "PO_B".into(),
                abbrev: Some("LFI".into()),
                label: Some("La France insoumise".into()),
            }],
            source_url: None,
            report_version: Some("v2".into()),
        };

        let json = serde_json::to_value(ScrutinDetailDto::from(final_vote_detail(view))).unwrap();

        let explanations = &json["explanations"];
        assert_eq!(explanations["state"], "explained");
        assert_eq!(explanations["report_version"], "v2");
        let item = &explanations["items"][0];
        assert_eq!(item["rank"], 1);
        assert_eq!(item["speaker_name"], "Marie Dupont");
        assert_eq!(item["group_uid"], "PO_A");
        assert_eq!(item["group_basis"], "membership");
        assert_eq!(item["speaker_position"], "against");
        assert_eq!(item["group_tally"]["votes_against"], 5);
        assert_eq!(item["preview"], "Texte complet");
        assert_eq!(item["preview_truncated"], true);
        assert_eq!(item["text"], "Texte complet de l'explication.");
        assert_eq!(
            item["official_url"],
            "https://www.assemblee-nationale.fr/dyn/17/comptes-rendus/seance/CRSANR5L17S2025IDS1#point_3"
        );
        assert_eq!(explanations["groups_without_speaker"][0]["group_uid"], "PO_B");
        assert_eq!(explanations["groups_without_speaker"][0]["abbrev"], "LFI");
    }

    #[test]
    fn maps_an_undetermined_speaker_group_without_group_uid() {
        let mut item = explanation_view(SummaryPreview::Unpublished);
        item.group_uid = None;
        item.group_basis = "undetermined";
        item.group_tally = None;
        item.speaker_position = None;
        let view = ExplanationsView {
            state: ExplanationsState::Explained,
            items: vec![item],
            groups_without_speaker: Vec::new(),
            source_url: None,
            report_version: None,
        };

        let json = serde_json::to_value(ScrutinDetailDto::from(final_vote_detail(view))).unwrap();

        let item = &json["explanations"]["items"][0];
        assert_eq!(item["group_basis"], "undetermined");
        assert!(item["group_uid"].is_null());
        assert!(item["group_tally"].is_null());
        assert!(item["speaker_position"].is_null());
        assert!(item["preview"].is_null());
        assert_eq!(item["preview_truncated"], false);
        assert_eq!(json["explanations"]["groups_without_speaker"], serde_json::json!([]));
        assert!(json["explanations"]["report_version"].is_null());
    }
}
