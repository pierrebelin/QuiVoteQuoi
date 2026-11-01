use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::application::ports::theme_repository::{
    AssignedFamily, FamilyCoverage, MethodReport, TextPage, TextScrutin, TextSummary,
};
use crate::application::use_cases::browse_themes::{FamilyDescription, TextDetail};
use crate::application::use_cases::extract_debated_texts::ExtractionReport;
use crate::application::use_cases::propose_theme_families::ProposalRun;
use crate::domain::theme::{ThemeAssignment, ThemeProposal, MAX_FAMILIES};
use crate::domain::theme_rules::{ThemeRule, RULES};

/// Mention portee par les pages de theme (README.md §2).
pub const METHOD_NOTE: &str = "Le rattachement d'un texte à une famille est le seul jugement du \
     site. La méthode est publiée, chaque rattachement est révisable et son historique conservé. \
     Les textes non rattachés restent consultables.";

#[derive(Debug, Serialize)]
pub struct FamilyDto {
    pub code: String,
    pub label: String,
    pub scope: String,
}

impl From<FamilyDescription> for FamilyDto {
    fn from(family: FamilyDescription) -> Self {
        Self {
            code: family.code.as_str().to_string(),
            label: family.label.to_string(),
            scope: family.scope.to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FamiliesResponse {
    pub families: Vec<FamilyDto>,
    pub max_families_per_text: usize,
    pub method_note: &'static str,
}

impl FamiliesResponse {
    pub fn new(families: Vec<FamilyDescription>) -> Self {
        Self {
            families: families.into_iter().map(FamilyDto::from).collect(),
            max_families_per_text: MAX_FAMILIES,
            method_note: METHOD_NOTE,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AssignedFamilyDto {
    pub code: String,
    pub label: String,
    pub opened_on: NaiveDate,
    pub motive: Option<String>,
}

impl From<AssignedFamily> for AssignedFamilyDto {
    fn from(assigned: AssignedFamily) -> Self {
        Self {
            code: assigned.family.as_str().to_string(),
            label: assigned.family.label().to_string(),
            opened_on: assigned.opened_on,
            motive: assigned.motive,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TextSummaryDto {
    pub key: String,
    pub label: String,
    pub scrutin_count: i64,
    pub first_vote: Option<NaiveDate>,
    pub last_vote: Option<NaiveDate>,
    pub dossier_uid: Option<String>,
    pub dossier_label: Option<String>,
    pub families: Vec<AssignedFamilyDto>,
    /// `no_family` : le modèle n'a retenu aucune famille. `failed` : il n'a pas
    /// répondu. Absent : jamais soumis.
    pub last_attempt_outcome: Option<String>,
}

impl From<TextSummary> for TextSummaryDto {
    fn from(text: TextSummary) -> Self {
        Self {
            key: text.key,
            label: text.label,
            scrutin_count: text.scrutin_count,
            first_vote: text.first_vote,
            last_vote: text.last_vote,
            dossier_uid: text.dossier_uid,
            dossier_label: text.dossier_label,
            families: text
                .families
                .into_iter()
                .map(AssignedFamilyDto::from)
                .collect(),
            last_attempt_outcome: text.last_attempt_outcome,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TextListResponse {
    pub items: Vec<TextSummaryDto>,
    pub total: i64,
    pub offset: i64,
    pub method_note: &'static str,
}

impl TextListResponse {
    pub fn new(page: TextPage, offset: i64) -> Self {
        Self {
            items: page.items.into_iter().map(TextSummaryDto::from).collect(),
            total: page.total,
            offset,
            method_note: METHOD_NOTE,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TextScrutinDto {
    pub uid: String,
    pub number: String,
    pub date: NaiveDate,
    pub subject: String,
    pub outcome_label: String,
    pub votes_for: i16,
    pub votes_against: i16,
    pub abstentions: i16,
}

impl From<TextScrutin> for TextScrutinDto {
    fn from(scrutin: TextScrutin) -> Self {
        Self {
            uid: scrutin.uid,
            number: scrutin.number,
            date: scrutin.date,
            subject: scrutin.subject,
            outcome_label: scrutin.outcome_label,
            votes_for: scrutin.votes_for,
            votes_against: scrutin.votes_against,
            abstentions: scrutin.abstentions,
        }
    }
}

/// Rattachement historique: il a valu jusqu'a `closed_on` (RM-07).
#[derive(Debug, Serialize)]
pub struct AssignmentHistoryDto {
    pub code: String,
    pub label: String,
    pub opened_on: NaiveDate,
    pub closed_on: Option<NaiveDate>,
    pub author: String,
    pub motive: Option<String>,
}

impl From<&ThemeAssignment> for AssignmentHistoryDto {
    fn from(assignment: &ThemeAssignment) -> Self {
        Self {
            code: assignment.family().as_str().to_string(),
            label: assignment.family().label().to_string(),
            opened_on: assignment.opened_on(),
            closed_on: assignment.closed_on(),
            author: assignment.author().to_string(),
            motive: assignment.motive().map(str::to_string),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ProposalDto {
    pub model: String,
    pub prompt_version: String,
    pub produced_on: NaiveDate,
    pub families: Vec<ProposedFamilyDto>,
}

#[derive(Debug, Serialize)]
pub struct ProposedFamilyDto {
    pub code: String,
    pub label: String,
    pub justification: String,
}

impl From<&ThemeProposal> for ProposalDto {
    fn from(proposal: &ThemeProposal) -> Self {
        Self {
            model: proposal.model().to_string(),
            prompt_version: proposal.prompt_version().to_string(),
            produced_on: proposal.produced_on(),
            families: proposal
                .families()
                .iter()
                .map(|f| ProposedFamilyDto {
                    code: f.family().as_str().to_string(),
                    label: f.family().label().to_string(),
                    justification: f.justification().to_string(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TextDetailResponse {
    #[serde(flatten)]
    pub text: TextSummaryDto,
    pub scrutins: Vec<TextScrutinDto>,
    pub history: Vec<AssignmentHistoryDto>,
    pub proposal: Option<ProposalDto>,
    pub method_note: &'static str,
}

impl TextDetailResponse {
    pub fn new(detail: TextDetail, scrutins: Vec<TextScrutin>) -> Self {
        Self {
            text: TextSummaryDto::from(detail.summary),
            scrutins: scrutins.into_iter().map(TextScrutinDto::from).collect(),
            history: detail
                .history
                .iter()
                .map(AssignmentHistoryDto::from)
                .collect(),
            proposal: detail.proposal.as_ref().map(ProposalDto::from),
            method_note: METHOD_NOTE,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FamilyCoverageDto {
    pub code: String,
    pub label: String,
    pub text_count: i64,
    pub scrutin_count: i64,
}

impl From<FamilyCoverage> for FamilyCoverageDto {
    fn from(coverage: FamilyCoverage) -> Self {
        Self {
            code: coverage.family.as_str().to_string(),
            label: coverage.family.label().to_string(),
            text_count: coverage.text_count,
            scrutin_count: coverage.scrutin_count,
        }
    }
}

/// Ce que publie la page methode (CU-06). Tous ces nombres viennent de la base.
#[derive(Debug, Serialize)]
pub struct MethodResponse {
    pub families: Vec<FamilyCoverageDto>,
    pub max_families_per_text: usize,
    /// Table des règles publiée telle qu'elle s'applique (RM-13).
    pub rules: Vec<ThemeRuleDto>,
    pub texts_total: i64,
    pub texts_assigned: i64,
    pub texts_without_family: i64,
    pub texts_attempt_failed: i64,
    pub texts_never_attempted: i64,
    pub scrutins_total: i64,
    pub scrutins_with_text: i64,
    pub scrutins_assigned: i64,
    /// Objets ne nommant aucun texte: consultables, jamais rattachés (RM-01).
    pub scrutins_without_text: i64,
    pub dossiers_total: i64,
    pub dossiers_linked_to_text: i64,
    pub dossiers_assigned: i64,
    /// Dossiers qu'aucun scrutin ne relie à un texte : ils sont classés sur
    /// leur propre titre, et restent consultables en attendant (RM-01).
    pub dossiers_without_text: i64,
    pub extraction_rule: &'static str,
    pub model_scope: &'static str,
    pub rule_scope: &'static str,
    pub method_note: &'static str,
}

/// Une règle du référentiel publié, servie telle que le domaine la porte.
#[derive(Debug, Serialize)]
pub struct ThemeRuleDto {
    pub id: &'static str,
    /// Fragment cherché dans le libellé normalisé du texte.
    pub marker: &'static str,
    /// Le fragment doit former un mot entier dans le libellé.
    pub whole_word: bool,
    pub families: Vec<FamilyDto>,
    /// Énoncé publié, conservé comme justification du rattachement.
    pub statement: &'static str,
}

impl From<&'static ThemeRule> for ThemeRuleDto {
    fn from(rule: &'static ThemeRule) -> Self {
        Self {
            id: rule.id(),
            marker: rule.marker(),
            whole_word: rule.whole_word(),
            families: rule
                .families()
                .iter()
                .map(|family| FamilyDto {
                    code: family.as_str().to_string(),
                    label: family.label().to_string(),
                    scope: family.scope().to_string(),
                })
                .collect(),
            statement: rule.statement(),
        }
    }
}

const EXTRACTION_RULE: &str = "Le texte débattu est extrait de l'objet du scrutin par une règle \
     fixe : on retient la dernière formule « projet de loi », « proposition de loi », \
     « proposition de résolution », « motion de censure » ou « déclaration de politique \
     générale » de l'objet, jusqu'à sa fin, mention de lecture retirée. Casse, espacement et \
     forme de l'apostrophe sont normalisés. Aucun modèle n'intervient dans cette étape.";

const MODEL_SCOPE: &str = "Le modèle ne reçoit que le libellé du texte : ni décompte, ni \
     position de vote, ni groupe parlementaire. Il rend une à trois familles du référentiel et \
     une justification. Il ne produit aucun chiffre : tous les nombres de cette page sont lus \
     en base.";

const RULE_SCOPE: &str = "Certains textes portent leur famille dans leur nature juridique : un \
     projet de loi de finances est un texte budgétaire, un projet de loi autorisant la \
     ratification d'un accord est un texte international. Ceux-là sont rattachés par les règles \
     ci-dessous, sans passer par le modèle. Chaque règle porte sur l'objet du texte, jamais sur \
     son orientation, et reste révisable comme n'importe quel autre rattachement.";

impl From<MethodReport> for MethodResponse {
    fn from(report: MethodReport) -> Self {
        Self {
            families: report
                .families
                .into_iter()
                .map(FamilyCoverageDto::from)
                .collect(),
            max_families_per_text: MAX_FAMILIES,
            rules: RULES.iter().map(ThemeRuleDto::from).collect(),
            texts_total: report.texts_total,
            texts_assigned: report.texts_assigned,
            texts_without_family: report.texts_without_family,
            texts_attempt_failed: report.texts_attempt_failed,
            texts_never_attempted: report.texts_never_attempted,
            scrutins_total: report.scrutins_total,
            scrutins_with_text: report.scrutins_with_text,
            scrutins_assigned: report.scrutins_assigned,
            scrutins_without_text: report.scrutins_total - report.scrutins_with_text,
            dossiers_total: report.dossiers_total,
            dossiers_linked_to_text: report.dossiers_linked_to_text,
            dossiers_assigned: report.dossiers_assigned,
            dossiers_without_text: report.dossiers_without_text,
            extraction_rule: EXTRACTION_RULE,
            model_scope: MODEL_SCOPE,
            rule_scope: RULE_SCOPE,
            method_note: METHOD_NOTE,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TextListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl TextListQuery {
    pub fn bounds(&self) -> (i64, i64) {
        let limit = self.limit.unwrap_or(50).clamp(1, 200);
        let offset = self.offset.unwrap_or(0).max(0);
        (limit, offset)
    }
}

#[derive(Debug, Serialize)]
pub struct ExtractionResponse {
    pub scrutins_read: usize,
    pub texts_found: usize,
    pub scrutins_linked: usize,
    pub scrutins_without_text: usize,
    pub dossiers_linked: usize,
}

impl From<ExtractionReport> for ExtractionResponse {
    fn from(report: ExtractionReport) -> Self {
        Self {
            scrutins_read: report.scrutins_read,
            texts_found: report.texts_found,
            scrutins_linked: report.scrutins_linked,
            scrutins_without_text: report.scrutins_without_text,
            dossiers_linked: report.dossiers_linked,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ProposalRequest {
    pub batch: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ProposalRunResponse {
    pub attempted: usize,
    /// Rattachés par règle publiée, sans appel au modèle (RM-13).
    pub ruled: usize,
    pub proposed: usize,
    pub without_family: usize,
    pub failed: usize,
    /// Appels au modèle réellement passés sur la passe. Une passe qui en compte
    /// peu pour beaucoup d'objets est une passe qui a bien travaillé (RM-14).
    pub model_calls: usize,
}

impl From<ProposalRun> for ProposalRunResponse {
    fn from(run: ProposalRun) -> Self {
        Self {
            attempted: run.attempted,
            ruled: run.ruled,
            proposed: run.proposed,
            without_family: run.without_family,
            failed: run.failed,
            model_calls: run.model_calls,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ArbitrationRequest {
    /// « text » ou « dossier ».
    pub subject_kind: String,
    pub subject_id: String,
    pub families: Vec<String>,
    pub author: String,
    pub motive: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ArbitrationResponse {
    pub subject_kind: String,
    pub subject_id: String,
    pub families: Vec<AssignedFamilyDto>,
}
