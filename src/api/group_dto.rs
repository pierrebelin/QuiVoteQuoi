use chrono::NaiveDate;
use serde::Serialize;

use crate::application::use_cases::browse_groups::{GroupListView, GroupSummary};
use crate::application::use_cases::get_group_detail::GroupProfileView;
use crate::domain::group_profile::{MemberCountRange, ParticipationCounts, QualityCount};

/// Ce que le groupe parlementaire est, et ce qu'il n'est pas (README.md §3.1).
pub const PARTY_NOTE: &str = "L'Assemblée publie des groupes parlementaires, pas des partis. \
     Certains groupes rassemblent plusieurs partis, des députés y sont rattachés sans en être \
     membres, et certains partis n'ont aucun groupe. Le libellé affiché est celui du groupe.";

/// Formule du taux, publiée avec le chiffre pour qu'il soit refaisable
/// (README.md §9).
pub const RATE_NOTE: &str = "Les taux rapportent les positions que l'Assemblée publie pour ce \
     groupe : pour + contre + abstention + non-votants. « Voix \
     exprimées » réunit les pour et les contre sans les distinguer — le sens du vote se lit \
     scrutin par scrutin, jamais cumulé.";

/// H3 et H4 en une phrase: sans elle, deux fiches ouvertes cote a cote se
/// lisent comme un classement, ce que README.md §6 interdit.
pub const COMPARISON_NOTE: &str = "Ces taux ne se comparent pas d'un groupe à l'autre. Les \
     groupes ne siègent pas sur les mêmes scrutins ni sur les mêmes périodes, et l'effectif d'un \
     groupe change en cours de législature : les dénominateurs sont différents. Le site ne classe \
     pas les groupes et n'en note aucun.";

/// Lacune de la source, affichee plutot que subie (README.md §7, RM-14).
pub const HAND_VOTE_NOTE: &str = "Les votes à main levée ne sont pas publiés par l'Assemblée : \
     ils sont absents de ces chiffres, comme du reste du site.";

/// Mention de methode des lignes reconstituees (SPEC-scrutins RM-03).
pub const RECONSTRUCTED_NOTE: &str = "Une répartition reconstituée est recalculée par le site \
     depuis les positions nominales, faute de répartition publiée par l'Assemblée pour ce \
     scrutin.";

/// Lignes de vote sans un seul membre, a ne pas confondre avec une abstention
/// (H5).
pub const SILENT_LINE_NOTE: &str = "Sur ces scrutins, l'Assemblée publie une ligne pour le groupe \
     sans qu'aucun de ses membres y figure. Ce n'est ni une abstention ni un refus de vote.";

#[derive(Debug, Serialize)]
pub struct GroupSummaryDto {
    pub uid: String,
    pub abbrev: String,
    /// Sigles antérieurs quand le groupe a été renommé en cours de législature.
    pub former_abbrevs: Vec<String>,
    pub label: String,
    pub color: Option<String>,
    pub legislature: i16,
    pub created_on: Option<NaiveDate>,
    pub dissolved_on: Option<NaiveDate>,
    pub dissolved: bool,
    /// Date à laquelle `member_count` est compté : aujourd'hui pour un groupe
    /// actif, son dernier jour pour un groupe dissous.
    pub reference_date: NaiveDate,
    pub member_count: i64,
    /// Scrutins où la source publie une ligne pour ce groupe.
    pub scrutin_count: i64,
    /// Premier et dernier scrutin où le groupe apparaît. `null` quand il
    /// n'apparaît sur aucun.
    pub first_scrutin_date: Option<NaiveDate>,
    pub last_scrutin_date: Option<NaiveDate>,
}

impl From<GroupSummary> for GroupSummaryDto {
    fn from(summary: GroupSummary) -> Self {
        Self {
            dissolved: summary.is_dissolved(),
            first_scrutin_date: summary.window.map(|window| window.first()),
            last_scrutin_date: summary.window.map(|window| window.last()),
            uid: summary.uid,
            abbrev: summary.abbrev,
            former_abbrevs: summary.former_abbrevs,
            label: summary.label,
            color: summary.color,
            legislature: summary.legislature,
            created_on: summary.created_on,
            dissolved_on: summary.dissolved_on,
            reference_date: summary.reference_date,
            member_count: summary.member_count,
            scrutin_count: summary.scrutin_count,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct GroupListResponse {
    pub groups: Vec<GroupSummaryDto>,
    pub total: usize,
    pub party_note: &'static str,
    pub hand_vote_note: &'static str,
}

impl From<GroupListView> for GroupListResponse {
    fn from(view: GroupListView) -> Self {
        let groups: Vec<GroupSummaryDto> =
            view.groups.into_iter().map(GroupSummaryDto::from).collect();
        Self {
            total: groups.len(),
            groups,
            party_note: PARTY_NOTE,
            hand_vote_note: HAND_VOTE_NOTE,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct QualityCountDto {
    /// Libellé de la source, conservé tel quel (SPEC-acteurs RM-02).
    pub quality: String,
    pub members: i64,
}

impl From<QualityCount> for QualityCountDto {
    fn from(count: QualityCount) -> Self {
        Self {
            quality: count.quality,
            members: count.members,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MemberCountRangeDto {
    pub min: u16,
    pub max: u16,
    pub stable: bool,
}

impl From<MemberCountRange> for MemberCountRangeDto {
    fn from(range: MemberCountRange) -> Self {
        Self {
            min: range.min(),
            max: range.max(),
            stable: range.is_stable(),
        }
    }
}

/// Comptes bruts servis à côté des taux : le pourcentage ne remplace jamais le
/// chiffre (README.md §6).
///
/// `expressed` réunit les pour et les contre. Les deux ne sont pas exposés
/// séparément : cumulés sur toute la législature, dont 86 % de scrutins
/// d'amendement, ils ne décriraient aucune position.
#[derive(Debug, Serialize)]
pub struct ParticipationCountsDto {
    pub expressed: u64,
    pub abstentions: u64,
    pub not_voting: u64,
    pub voluntary_not_voting: u64,
    /// Dénominateur des taux : toutes les positions publiées pour ce groupe.
    pub published_positions: u64,
}

impl From<&ParticipationCounts> for ParticipationCountsDto {
    fn from(counts: &ParticipationCounts) -> Self {
        Self {
            expressed: counts.expressed(),
            abstentions: counts.abstentions,
            not_voting: counts.not_voting,
            voluntary_not_voting: counts.voluntary_not_voting,
            published_positions: counts.published_positions(),
        }
    }
}

/// Les trois parts, en pour mille. Le front affiche `x,y %`.
///
/// Le pour mille plutôt que le pourcentage entier : une abstention à 0,4 %
/// s'afficherait « 0 % », et un chiffre arrondi à zéro se lit comme une donnée
/// manquante.
#[derive(Debug, Serialize)]
pub struct ParticipationRatesDto {
    pub base: u64,
    pub expressed_per_mille: u16,
    pub abstention_per_mille: u16,
    pub absence_per_mille: u16,
}

#[derive(Debug, Serialize)]
pub struct GroupDetailResponse {
    #[serde(flatten)]
    pub group: GroupSummaryDto,
    pub total_member_count: i64,
    pub qualities: Vec<QualityCountDto>,
    pub published_member_range: Option<MemberCountRangeDto>,
    pub line_count: i64,
    pub reconstructed_count: i64,
    pub silent_line_count: i64,
    pub counts: ParticipationCountsDto,
    /// `null` quand la source ne publie aucune position pour ce groupe.
    pub rates: Option<ParticipationRatesDto>,
    pub party_note: &'static str,
    pub rate_note: &'static str,
    pub comparison_note: &'static str,
    pub hand_vote_note: &'static str,
    pub reconstructed_note: &'static str,
    pub silent_line_note: &'static str,
}

impl From<GroupProfileView> for GroupDetailResponse {
    fn from(view: GroupProfileView) -> Self {
        Self {
            counts: ParticipationCountsDto::from(&view.counts),
            rates: view.rates.map(|rates| ParticipationRatesDto {
                base: rates.base,
                expressed_per_mille: rates.expressed_per_mille,
                abstention_per_mille: rates.abstention_per_mille,
                absence_per_mille: rates.absence_per_mille,
            }),
            group: view.summary.into(),
            total_member_count: view.total_member_count,
            qualities: view
                .qualities
                .into_iter()
                .map(QualityCountDto::from)
                .collect(),
            published_member_range: view.published_member_range.map(MemberCountRangeDto::from),
            line_count: view.line_count,
            reconstructed_count: view.reconstructed_count,
            silent_line_count: view.silent_line_count,
            party_note: PARTY_NOTE,
            rate_note: RATE_NOTE,
            comparison_note: COMPARISON_NOTE,
            hand_vote_note: HAND_VOTE_NOTE,
            reconstructed_note: RECONSTRUCTED_NOTE,
            silent_line_note: SILENT_LINE_NOTE,
        }
    }
}
