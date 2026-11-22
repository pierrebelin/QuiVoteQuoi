//! Où en est la loi (todo/navette/SPEC-navette.md).
//!
//! Un seul calcul, pur, part des décisions des chambres (actes `*-DEC` et leur
//! `statutConclusion`) et du sort du dossier. Il donne l'état (RM-01 à RM-04)
//! et les quatre jalons (RM-05). Le résumé persisté et le détail relu en base
//! passent tous deux par `LawProgress::from_acts` : aucun second calcul.

use chrono::NaiveDate;
use serde::Serialize;

use super::dossier::{Chamber, DossierOutcome, LawPublication, LegislativeAct, LegislativeStage};
use super::scrutin::ScrutinUid;

/// Sens d'une décision, lu dans le `statutConclusion` de l'acte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionSense {
    /// « adoptée », « considéré comme adopté » (49.3), « adoptée, dans les
    /// conditions prévues à l'article 45, alinéa 3 ».
    Adopted,
    /// « adoptée sans modification » : la chambre vote le texte de l'autre
    /// tel quel (vote conforme).
    AdoptedUnchanged,
    /// « modifiée », « adoptée avec modifications ».
    Modified,
    /// « rejetée », « considéré comme rejeté » (49.3).
    Rejected,
    /// Conclusion « Accord » de la commission mixte paritaire.
    CmpAgreement,
    /// Conclusion « Désaccord » de la commission mixte paritaire.
    CmpDisagreement,
}

impl DecisionSense {
    /// Sens d'après le libellé de la source ; à défaut de libellé, d'après la
    /// famille de sort (`fam_code`). Un libellé inconnu n'a pas de sens : la
    /// décision est ignorée plutôt que devinée.
    pub fn from_source(label: Option<&str>, fam_code: Option<&str>) -> Option<Self> {
        match label.map(str::trim).filter(|l| !l.is_empty()) {
            Some(label) => Self::from_label(label),
            None => Self::from_fam_code(fam_code?.trim()),
        }
    }

    fn from_label(label: &str) -> Option<Self> {
        let label = label.to_lowercase();
        match label.as_str() {
            "adoptée" | "adopté" => return Some(Self::Adopted),
            "adoptée sans modification" | "adopté sans modification" | "conforme" => {
                return Some(Self::AdoptedUnchanged)
            }
            "modifiée" | "modifié" | "adoptée avec modifications" | "adopté avec modifications" => {
                return Some(Self::Modified)
            }
            "rejetée" | "rejeté" => return Some(Self::Rejected),
            "accord" => return Some(Self::CmpAgreement),
            "désaccord" => return Some(Self::CmpDisagreement),
            _ => {}
        }
        let article_49_3 = label.contains("article 49, alinéa 3");
        let article_45_3 = label.contains("article 45, alinéa 3");
        if article_49_3
            && (label.starts_with("considéré comme adopté")
                || label.starts_with("considérée comme adoptée"))
        {
            return Some(Self::Adopted);
        }
        if article_49_3
            && (label.starts_with("considéré comme rejeté")
                || label.starts_with("considérée comme rejetée"))
        {
            return Some(Self::Rejected);
        }
        if article_45_3 && (label.starts_with("adoptée,") || label.starts_with("adopté,")) {
            return Some(Self::Adopted);
        }
        None
    }

    fn from_fam_code(fam_code: &str) -> Option<Self> {
        match fam_code {
            "TSORTF01" | "TSORTF06" | "TSORTF18" => Some(Self::Adopted),
            "TSORTF03" => Some(Self::AdoptedUnchanged),
            "TSORTF02" | "TSORTF05" => Some(Self::Modified),
            "TSORTF07" | "TSORTF24" => Some(Self::Rejected),
            "TCCMP01" => Some(Self::CmpAgreement),
            "TCCMP02" => Some(Self::CmpDisagreement),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Adopted => "adopted",
            Self::AdoptedUnchanged => "adopted_unchanged",
            Self::Modified => "modified",
            Self::Rejected => "rejected",
            Self::CmpAgreement => "cmp_agreement",
            Self::CmpDisagreement => "cmp_disagreement",
        }
    }

    /// Relit le code que `as_str` ecrit (resume d'etat stocke).
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "adopted" => Some(Self::Adopted),
            "adopted_unchanged" => Some(Self::AdoptedUnchanged),
            "modified" => Some(Self::Modified),
            "rejected" => Some(Self::Rejected),
            "cmp_agreement" => Some(Self::CmpAgreement),
            "cmp_disagreement" => Some(Self::CmpDisagreement),
            _ => None,
        }
    }

    /// La chambre a adopté un texte, modifié ou non (RM-03).
    pub fn adopts(&self) -> bool {
        matches!(
            self,
            Self::Adopted | Self::AdoptedUnchanged | Self::Modified
        )
    }
}

/// Moment de la procédure où tombe une décision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStage {
    /// Lecture par une chambre : 1re, 2e ou nouvelle lecture.
    ChamberReading,
    /// Conclusion de la commission mixte paritaire (`CMP-DEC`).
    JointCommittee,
    /// Lecture du texte de la CMP par une chambre (`CMP-DEBATS-AN-DEC`,
    /// `CMP-DEBATS-SN-DEC`).
    JointCommitteeReading,
    /// Lecture définitive par l'Assemblée (`ANLDEF-*-DEC`).
    FinalReading,
}

/// Décision d'une chambre ou de la CMP : acte `*-DEC` au sens connu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Decision {
    pub code: String,
    pub date: NaiveDate,
    /// Chambre qui décide ; `Joint` pour la CMP ; `None` quand la source ne la
    /// nomme pas (`CMP-DEBATS-DEC`).
    pub chamber: Chamber,
    pub stage: DecisionStage,
    pub sense: DecisionSense,
}

impl Decision {
    /// Décision portée par l'acte, s'il en est une : code en `-DEC`, étape
    /// connue, sens connu. Les lectures uniques (`ANLUNI`, résolutions) n'en
    /// sont pas : une résolution ne devient pas une loi.
    pub fn from_act(act: &LegislativeAct) -> Option<Self> {
        let code = act.code.as_deref()?;
        if !code.ends_with("-DEC") {
            return None;
        }
        let stage = LegislativeStage::from_code(code.split('-').next()?)?;
        let (stage, chamber) = match stage {
            LegislativeStage::FirstReadingAN
            | LegislativeStage::SecondReadingAN
            | LegislativeStage::NewReadingAN
            | LegislativeStage::FirstReadingSenate
            | LegislativeStage::SecondReadingSenate
            | LegislativeStage::NewReadingSenate => {
                (DecisionStage::ChamberReading, stage.chamber())
            }
            LegislativeStage::FinalReading => (DecisionStage::FinalReading, stage.chamber()),
            LegislativeStage::JointCommittee => joint_committee_decision(code),
            LegislativeStage::ConstitutionalReview | LegislativeStage::Promulgation => return None,
        };
        let sense = DecisionSense::from_source(
            act.conclusion_label.as_deref(),
            act.conclusion_code.as_deref(),
        )?;
        Some(Self {
            code: code.to_string(),
            date: act.date,
            chamber,
            stage,
            sense,
        })
    }

    /// Code de lecture : premier segment du code de l'acte (`AN1`, `SN2`,
    /// `CMP`, `ANLDEF`).
    pub fn reading(&self) -> &str {
        self.code.split('-').next().unwrap_or(&self.code)
    }

    /// Jalon (RM-05) auquel la décision appartient.
    pub fn step(&self) -> MilestoneStep {
        step_of(self)
    }
}

fn joint_committee_decision(code: &str) -> (DecisionStage, Chamber) {
    if code == "CMP-DEC" {
        (DecisionStage::JointCommittee, Chamber::Joint)
    } else if code.starts_with("CMP-DEBATS-AN-") {
        (
            DecisionStage::JointCommitteeReading,
            Chamber::AssembleeNationale,
        )
    } else if code.starts_with("CMP-DEBATS-SN-") {
        (DecisionStage::JointCommitteeReading, Chamber::Senat)
    } else {
        (DecisionStage::JointCommitteeReading, Chamber::None)
    }
}

/// Façon dont les deux chambres sont arrivées au même texte (§2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Agreement {
    /// Vote conforme d'une chambre sur le texte de l'autre.
    SameText,
    /// Texte de la CMP adopté par les deux chambres.
    JointCommittee,
    /// Lecture définitive adoptée par l'Assemblée.
    FinalReading,
}

impl Agreement {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SameText => "same_text",
            Self::JointCommittee => "joint_committee",
            Self::FinalReading => "final_reading",
        }
    }
}

/// État d'un dossier qui a reçu au moins une adoption, ou qui est devenu loi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LawState {
    /// RM-03 : au moins une chambre a adopté, le même texte n'est pas atteint.
    InShuttle { last_decision: Decision },
    /// RM-02 : la dernière décision produit le même texte, la promulgation
    /// n'a pas eu lieu.
    DefinitivelyAdopted {
        date: NaiveDate,
        agreement: Agreement,
    },
    /// RM-01 : acte `PROM-PUB`, repris de `DossierOutcome::Promulgated`.
    BecameLaw {
        date: NaiveDate,
        publication: LawPublication,
    },
}

/// Les trois états, sans leurs compléments : ce que la base stocke, filtre et
/// compte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LawStateKind {
    InShuttle,
    DefinitivelyAdopted,
    BecameLaw,
}

impl LawStateKind {
    pub const ALL: [Self; 3] = [Self::InShuttle, Self::DefinitivelyAdopted, Self::BecameLaw];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InShuttle => "in_shuttle",
            Self::DefinitivelyAdopted => "definitively_adopted",
            Self::BecameLaw => "became_law",
        }
    }

    /// Relit le code stocké ; `NO_LAW_STATE` et un code inconnu ne sont pas
    /// des états.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "in_shuttle" => Some(Self::InShuttle),
            "definitively_adopted" => Some(Self::DefinitivelyAdopted),
            "became_law" => Some(Self::BecameLaw),
            _ => None,
        }
    }
}

impl LawState {
    pub fn kind(&self) -> LawStateKind {
        match self {
            Self::InShuttle { .. } => LawStateKind::InShuttle,
            Self::DefinitivelyAdopted { .. } => LawStateKind::DefinitivelyAdopted,
            Self::BecameLaw { .. } => LawStateKind::BecameLaw,
        }
    }

    pub fn as_str(&self) -> &'static str {
        self.kind().as_str()
    }

    /// Date de l'état : dernière décision, décision qui donne le même texte,
    /// ou promulgation.
    pub fn date(&self) -> NaiveDate {
        match self {
            Self::InShuttle { last_decision } => last_decision.date,
            Self::DefinitivelyAdopted { date, .. } | Self::BecameLaw { date, .. } => *date,
        }
    }
}

/// Étape des jalons (RM-05) : chambre de dépôt, autre chambre, Accord, Loi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "chamber", rename_all = "snake_case")]
pub enum MilestoneStep {
    Chamber(Chamber),
    Agreement,
    Law,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MilestoneStatus {
    /// Étape franchie : point noir.
    Done,
    /// Étape en cours, porte l'icône d'état.
    Current,
    /// Étape à venir : cercle en pointillé.
    Upcoming,
}

/// Un jalon de RM-05. Données brutes : les sous-titres (« modifié 20 nov. »,
/// « même texte · 19 nov. ») sont composés par l'interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Milestone {
    pub step: MilestoneStep,
    pub status: MilestoneStatus,
    /// Sens de la dernière décision de l'étape. Absent : l'étape n'a pas de
    /// décision (jalon Loi, accord par vote conforme, étape à venir).
    pub sense: Option<DecisionSense>,
    /// Date de la dernière décision de l'étape, de l'accord ou de la
    /// promulgation.
    pub date: Option<NaiveDate>,
    /// Jalon Accord seulement : la voie du même texte, atteinte ou en cours.
    pub agreement: Option<Agreement>,
    /// Jalon Loi d'un dossier devenu loi.
    pub publication: Option<LawPublication>,
}

/// Où en est la loi : état, dernière décision et jalons d'un dossier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LawProgress {
    /// `None` (RM-04) : aucune adoption nulle part, retiré ou fusionné.
    state: Option<LawState>,
    /// Dernière décision au sens connu, quel que soit l'état.
    last_decision: Option<Decision>,
    /// Décisions au sens connu, par date ; à date égale, ordre de la source.
    decisions: Vec<Decision>,
    /// Vide quand il n'y a pas d'état, ou quand la chambre de dépôt est
    /// introuvable.
    milestones: Vec<Milestone>,
}

/// Code d'état stocké pour un dossier sans état (RM-04).
pub const NO_LAW_STATE: &str = "none";

impl LawProgress {
    /// Calcule l'état et les jalons à partir des actes du dossier et de son
    /// sort. Les actes peuvent venir de la source comme de la base.
    pub fn from_acts(acts: &[LegislativeAct], outcome: &DossierOutcome) -> Self {
        let mut decisions: Vec<Decision> = acts.iter().filter_map(Decision::from_act).collect();
        // Tri stable : à date égale, l'ordre de la source est conservé.
        decisions.sort_by_key(|d| d.date);
        let last_decision = decisions.last().cloned();
        let deposit_chamber = deposit_chamber(acts, &decisions);
        let agreement = agreement_reached(&decisions);

        let state = match outcome {
            DossierOutcome::Withdrawn { .. } | DossierOutcome::MergedInto { .. } => None,
            DossierOutcome::Promulgated { date, publication } => Some(LawState::BecameLaw {
                date: *date,
                publication: publication.clone(),
            }),
            DossierOutcome::Rejected { .. } | DossierOutcome::NoRecordedConclusion => {
                if !decisions.iter().any(|d| d.sense.adopts()) {
                    None
                } else if let Some((agreement, date)) = agreement {
                    Some(LawState::DefinitivelyAdopted { date, agreement })
                } else {
                    last_decision
                        .clone()
                        .map(|last_decision| LawState::InShuttle { last_decision })
                }
            }
        };

        let milestones = match (&state, deposit_chamber) {
            (Some(state), Some(deposit)) => milestones(state, deposit, &decisions, agreement),
            _ => Vec::new(),
        };

        Self {
            state,
            last_decision,
            decisions,
            milestones,
        }
    }

    /// État tel qu'il était le soir de `day` : seuls comptent les actes datés
    /// au plus tard ce jour-là, et un sort daté après n'est pas encore acquis
    /// (une promulgation postérieure ne compte pas, RM-08).
    pub fn as_of(acts: &[LegislativeAct], outcome: &DossierOutcome, day: NaiveDate) -> Self {
        let acts: Vec<LegislativeAct> = acts.iter().filter(|a| a.date <= day).cloned().collect();
        let outcome = match outcome.date() {
            Some(date) if date > day => DossierOutcome::NoRecordedConclusion,
            _ => outcome.clone(),
        };
        Self::from_acts(&acts, &outcome)
    }

    pub fn state(&self) -> Option<&LawState> {
        self.state.as_ref()
    }

    /// Code d'état tel que la base le stocke : `LawState::as_str`, ou
    /// `NO_LAW_STATE`.
    pub fn state_kind(&self) -> &'static str {
        self.state.as_ref().map_or(NO_LAW_STATE, LawState::as_str)
    }

    pub fn last_decision(&self) -> Option<&Decision> {
        self.last_decision.as_ref()
    }

    pub fn decisions(&self) -> &[Decision] {
        &self.decisions
    }

    pub fn milestones(&self) -> &[Milestone] {
        &self.milestones
    }
}

/// Dernière décision telle que le résumé la stocke.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DecisionSummary {
    pub chamber: Chamber,
    pub sense: DecisionSense,
    pub date: NaiveDate,
}

/// Résumé de l'état d'un dossier, persisté pour le filtre, les compteurs et
/// les cartes de liste, lues sans les actes. Il vient toujours de
/// `LawProgress` : aucun second calcul.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LawStateSummary {
    kind: LawStateKind,
    date: NaiveDate,
    last_decision: Option<DecisionSummary>,
}

impl LawStateSummary {
    /// `None` : le dossier n'a pas d'état (RM-04).
    pub fn from_progress(progress: &LawProgress) -> Option<Self> {
        let state = progress.state()?;
        Some(Self {
            kind: state.kind(),
            date: state.date(),
            last_decision: progress.last_decision().map(|d| DecisionSummary {
                chamber: d.chamber,
                sense: d.sense,
                date: d.date,
            }),
        })
    }

    /// Relit les colonnes écrites depuis `kind`, `date` et `last_decision` :
    /// `NO_LAW_STATE`, un code inconnu ou une date absente ne donnent pas
    /// d'état ; une dernière décision incomplète est absente.
    pub fn from_stored(
        kind: Option<&str>,
        date: Option<NaiveDate>,
        decision_chamber: Option<&str>,
        decision_sense: Option<&str>,
        decision_date: Option<NaiveDate>,
    ) -> Option<Self> {
        let kind = LawStateKind::parse(kind?)?;
        let last_decision = match (decision_chamber, decision_sense, decision_date) {
            (Some(chamber), Some(sense), Some(date)) => Some(DecisionSummary {
                chamber: Chamber::parse(chamber)?,
                sense: DecisionSense::parse(sense)?,
                date,
            }),
            _ => None,
        };
        Some(Self {
            kind,
            date: date?,
            last_decision,
        })
    }

    pub fn kind(&self) -> LawStateKind {
        self.kind
    }

    pub fn date(&self) -> NaiveDate {
        self.date
    }

    pub fn last_decision(&self) -> Option<&DecisionSummary> {
        self.last_decision.as_ref()
    }
}

/// Jalon de la décision qu'un scrutin a tranchée, d'après l'acte qui le cite
/// (`voteRefs`). Un acte au sens inconnu se range d'après son code de lecture.
pub fn step_of_vote(acts: &[LegislativeAct], vote: &ScrutinUid) -> Option<MilestoneStep> {
    let act = acts
        .iter()
        .find(|act| act.decision_vote.as_ref() == Some(vote))?;
    if let Some(decision) = Decision::from_act(act) {
        return Some(decision.step());
    }
    let stage = LegislativeStage::from_code(act.code.as_deref()?.split('-').next()?)?;
    match stage {
        LegislativeStage::JointCommittee | LegislativeStage::FinalReading => {
            Some(MilestoneStep::Agreement)
        }
        LegislativeStage::ConstitutionalReview | LegislativeStage::Promulgation => None,
        reading => Some(MilestoneStep::Chamber(reading.chamber())),
    }
}

/// Code de lecture d'une promulgation dans les avancées (RM-08).
pub const PROMULGATION_READING: &str = "PROM";

/// Ce qui s'est passé pour un dossier pendant une période (RM-08) : la
/// promulgation si elle y tombe, sinon la dernière décision des chambres.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdvanceEvent {
    pub date: NaiveDate,
    /// Code de lecture (`AN1`, `SN2`, `CMP`…), `PROM` pour la promulgation.
    pub reading: String,
    /// Absente pour la promulgation et pour une lecture du texte de CMP dont
    /// la source ne nomme pas la chambre.
    pub chamber: Option<Chamber>,
    pub sense: Option<DecisionSense>,
    /// La source cite le scrutin public qui a tranché (`voteRefs`).
    pub has_public_vote: bool,
    /// Événement de CMP (conclusion ou lecture de son texte) : date de
    /// l'accord en CMP, s'il y en a eu un.
    pub joint_committee_agreement: Option<NaiveDate>,
}

/// Avancée d'un dossier pendant une période (RM-08).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LawAdvance {
    /// État le soir du dernier jour de la période.
    pub state: LawStateSummary,
    pub event: AdvanceEvent,
}

impl LawAdvance {
    /// `None` : aucune décision des chambres ni promulgation entre `from` et
    /// `to`, ou aucun état le soir de `to` (un rejet seul, par exemple).
    pub fn between(
        acts: &[LegislativeAct],
        outcome: &DossierOutcome,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Option<Self> {
        let within = |act: &&LegislativeAct| act.date >= from && act.date <= to;
        let promulgation = acts
            .iter()
            .filter(within)
            .filter(|act| {
                act.code.as_deref().and_then(|c| c.split('-').next()) == Some(PROMULGATION_READING)
            })
            .max_by_key(|act| act.date);
        let decision = acts
            .iter()
            .filter(within)
            .filter_map(|act| Decision::from_act(act).map(|decision| (act, decision)))
            .max_by_key(|(_, decision)| decision.date);

        let event = match (promulgation, decision) {
            (Some(act), _) => AdvanceEvent {
                date: act.date,
                reading: PROMULGATION_READING.to_string(),
                chamber: None,
                sense: None,
                has_public_vote: false,
                joint_committee_agreement: None,
            },
            (None, Some((act, decision))) => AdvanceEvent {
                date: decision.date,
                reading: decision.reading().to_string(),
                chamber: match decision.chamber {
                    Chamber::None => None,
                    chamber => Some(chamber),
                },
                sense: Some(decision.sense),
                has_public_vote: act.decision_vote.is_some(),
                joint_committee_agreement: joint_committee_agreement(acts, &decision),
            },
            (None, None) => return None,
        };
        let state = LawStateSummary::from_progress(&LawProgress::as_of(acts, outcome, to))?;
        Some(Self { state, event })
    }
}

/// Date du dernier accord en CMP au plus tard le jour d'une décision de CMP.
fn joint_committee_agreement(acts: &[LegislativeAct], decision: &Decision) -> Option<NaiveDate> {
    if !matches!(
        decision.stage,
        DecisionStage::JointCommittee | DecisionStage::JointCommitteeReading
    ) {
        return None;
    }
    acts.iter()
        .filter_map(Decision::from_act)
        .filter(|d| {
            d.stage == DecisionStage::JointCommittee
                && d.sense == DecisionSense::CmpAgreement
                && d.date <= decision.date
        })
        .map(|d| d.date)
        .max()
}

/// Chambre de la première lecture (§2) : le plus ancien acte daté sous `AN1`
/// ou `SN1`, à défaut la chambre de la première lecture décidée.
fn deposit_chamber(acts: &[LegislativeAct], decisions: &[Decision]) -> Option<Chamber> {
    acts.iter()
        .filter_map(|act| {
            let prefix = act.code.as_deref()?.split('-').next()?;
            match LegislativeStage::from_code(prefix)? {
                LegislativeStage::FirstReadingAN => Some((act.date, Chamber::AssembleeNationale)),
                LegislativeStage::FirstReadingSenate => Some((act.date, Chamber::Senat)),
                _ => None,
            }
        })
        .min_by_key(|(date, _)| *date)
        .map(|(_, chamber)| chamber)
        .or_else(|| {
            decisions
                .iter()
                .find(|d| d.stage == DecisionStage::ChamberReading)
                .map(|d| d.chamber)
        })
}

fn other_chamber(chamber: Chamber) -> Option<Chamber> {
    match chamber {
        Chamber::AssembleeNationale => Some(Chamber::Senat),
        Chamber::Senat => Some(Chamber::AssembleeNationale),
        Chamber::Joint | Chamber::None => None,
    }
}

/// Même texte atteint par la dernière décision (RM-02, §2). Un rejet
/// postérieur, étant la dernière décision, relance la navette.
fn agreement_reached(decisions: &[Decision]) -> Option<(Agreement, NaiveDate)> {
    let (last, earlier) = decisions.split_last()?;
    match (last.stage, last.sense) {
        (DecisionStage::ChamberReading, DecisionSense::AdoptedUnchanged) => {
            let other = other_chamber(last.chamber)?;
            earlier
                .iter()
                .any(|d| d.chamber == other && d.sense.adopts())
                .then_some((Agreement::SameText, last.date))
        }
        (DecisionStage::JointCommitteeReading, sense) if sense.adopts() => {
            let other = other_chamber(last.chamber)?;
            earlier
                .iter()
                .rev()
                .find(|d| d.stage == DecisionStage::JointCommitteeReading && d.chamber == other)
                .filter(|d| d.sense.adopts())
                .map(|_| (Agreement::JointCommittee, last.date))
        }
        (DecisionStage::FinalReading, sense) if sense.adopts() => {
            Some((Agreement::FinalReading, last.date))
        }
        _ => None,
    }
}

fn step_of(decision: &Decision) -> MilestoneStep {
    match decision.stage {
        DecisionStage::ChamberReading => MilestoneStep::Chamber(decision.chamber),
        DecisionStage::JointCommittee
        | DecisionStage::JointCommitteeReading
        | DecisionStage::FinalReading => MilestoneStep::Agreement,
    }
}

fn agreement_of_stage(stage: DecisionStage) -> Option<Agreement> {
    match stage {
        DecisionStage::JointCommittee | DecisionStage::JointCommitteeReading => {
            Some(Agreement::JointCommittee)
        }
        DecisionStage::FinalReading => Some(Agreement::FinalReading),
        DecisionStage::ChamberReading => None,
    }
}

/// Les quatre jalons de RM-05.
///
/// En navette, l'étape de la dernière décision est en cours ; une chambre qui
/// a décidé est franchie ; le reste est à venir. L'Accord n'est jamais franchi
/// en navette : un accord abouti donnerait l'état adopté définitivement.
fn milestones(
    state: &LawState,
    deposit: Chamber,
    decisions: &[Decision],
    agreement: Option<(Agreement, NaiveDate)>,
) -> Vec<Milestone> {
    let Some(other) = other_chamber(deposit) else {
        return Vec::new();
    };
    let last_of = |step: MilestoneStep| decisions.iter().rev().find(|d| step_of(d) == step);

    let chamber_milestone = |chamber: Chamber, status: MilestoneStatus| {
        let last = last_of(MilestoneStep::Chamber(chamber));
        Milestone {
            step: MilestoneStep::Chamber(chamber),
            status,
            sense: last.map(|d| d.sense),
            date: last.map(|d| d.date),
            agreement: None,
            publication: None,
        }
    };
    let upcoming = |step: MilestoneStep| Milestone {
        step,
        status: MilestoneStatus::Upcoming,
        sense: None,
        date: None,
        agreement: None,
        publication: None,
    };
    let reached_agreement = || {
        let last = last_of(MilestoneStep::Agreement);
        Milestone {
            step: MilestoneStep::Agreement,
            status: MilestoneStatus::Done,
            sense: last.map(|d| d.sense),
            date: agreement.map(|(_, date)| date),
            agreement: agreement.map(|(agreement, _)| agreement),
            publication: None,
        }
    };

    match state {
        LawState::BecameLaw { date, publication } => vec![
            chamber_milestone(deposit, MilestoneStatus::Done),
            chamber_milestone(other, MilestoneStatus::Done),
            reached_agreement(),
            Milestone {
                step: MilestoneStep::Law,
                status: MilestoneStatus::Done,
                sense: None,
                date: Some(*date),
                agreement: None,
                publication: Some(publication.clone()),
            },
        ],
        LawState::DefinitivelyAdopted { .. } => vec![
            chamber_milestone(deposit, MilestoneStatus::Done),
            chamber_milestone(other, MilestoneStatus::Done),
            reached_agreement(),
            Milestone {
                status: MilestoneStatus::Current,
                ..upcoming(MilestoneStep::Law)
            },
        ],
        LawState::InShuttle { last_decision } => {
            let current = step_of(last_decision);
            let chamber_status = |chamber: Chamber| {
                if current == MilestoneStep::Chamber(chamber) {
                    MilestoneStatus::Current
                } else if last_of(MilestoneStep::Chamber(chamber)).is_some() {
                    MilestoneStatus::Done
                } else {
                    MilestoneStatus::Upcoming
                }
            };
            let chamber = |chamber: Chamber| match chamber_status(chamber) {
                MilestoneStatus::Upcoming => upcoming(MilestoneStep::Chamber(chamber)),
                status => chamber_milestone(chamber, status),
            };
            let agreement_milestone = if current == MilestoneStep::Agreement {
                Milestone {
                    step: MilestoneStep::Agreement,
                    status: MilestoneStatus::Current,
                    sense: Some(last_decision.sense),
                    date: Some(last_decision.date),
                    agreement: agreement_of_stage(last_decision.stage),
                    publication: None,
                }
            } else {
                upcoming(MilestoneStep::Agreement)
            };
            vec![
                chamber(deposit),
                chamber(other),
                agreement_milestone,
                upcoming(MilestoneStep::Law),
            ]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn act(code: &str, on: NaiveDate) -> LegislativeAct {
        LegislativeAct {
            date: on,
            label: code.into(),
            code: Some(code.into()),
            decision_vote: None,
            conclusion_label: None,
            conclusion_code: None,
        }
    }

    fn decision(code: &str, on: NaiveDate, label: &str) -> LegislativeAct {
        LegislativeAct {
            conclusion_label: Some(label.into()),
            ..act(code, on)
        }
    }

    fn promulgated(on: NaiveDate, law_code: &str, jo: NaiveDate) -> DossierOutcome {
        DossierOutcome::Promulgated {
            date: on,
            publication: LawPublication {
                law_code: Some(law_code.into()),
                jo_date: Some(jo),
                legifrance_url: None,
            },
        }
    }

    fn statuses(progress: &LawProgress) -> Vec<MilestoneStatus> {
        progress.milestones().iter().map(|m| m.status).collect()
    }

    fn steps(progress: &LawProgress) -> Vec<MilestoneStep> {
        progress.milestones().iter().map(|m| m.step).collect()
    }

    use MilestoneStatus::{Current, Done, Upcoming};

    const AN: Chamber = Chamber::AssembleeNationale;
    const SN: Chamber = Chamber::Senat;

    // Témoin DLR5L17N51037 (légitime défense).
    #[test]
    fn a_text_modified_by_the_second_chamber_is_in_shuttle() {
        let acts = [
            act("AN1-DEPOT", date(2024, 12, 3)),
            decision("AN1-DEBATS-DEC", date(2026, 7, 7), "adoptée"),
            act("SN1-DEPOT", date(2026, 7, 7)),
            decision("SN1-DEBATS-DEC", date(2026, 11, 20), "modifiée"),
        ];

        let progress = LawProgress::from_acts(&acts, &DossierOutcome::NoRecordedConclusion);

        let Some(LawState::InShuttle { last_decision }) = progress.state() else {
            panic!("expected in shuttle, got {:?}", progress.state());
        };
        assert_eq!(last_decision.chamber, SN);
        assert_eq!(last_decision.sense, DecisionSense::Modified);
        assert_eq!(last_decision.date, date(2026, 11, 20));
        assert_eq!(progress.state_kind(), "in_shuttle");
        assert_eq!(
            steps(&progress),
            vec![
                MilestoneStep::Chamber(AN),
                MilestoneStep::Chamber(SN),
                MilestoneStep::Agreement,
                MilestoneStep::Law
            ]
        );
        assert_eq!(statuses(&progress), vec![Done, Current, Upcoming, Upcoming]);
        let an = &progress.milestones()[0];
        assert_eq!(an.sense, Some(DecisionSense::Adopted));
        assert_eq!(an.date, Some(date(2026, 7, 7)));
    }

    // Témoin DLR5L17N51962 (Territoires zéro chômeur).
    #[test]
    fn a_conforming_second_reading_by_the_deposit_chamber_is_definitive() {
        let acts = [
            act("AN1-DEPOT", date(2025, 4, 17)),
            decision("AN1-DEBATS-DEC", date(2026, 1, 27), "adoptée"),
            decision("SN1-DEBATS-DEC", date(2026, 6, 9), "modifiée"),
            decision(
                "AN2-DEBATS-DEC",
                date(2026, 11, 19),
                "adoptée sans modification",
            ),
        ];

        let progress = LawProgress::from_acts(&acts, &DossierOutcome::NoRecordedConclusion);

        assert_eq!(
            progress.state(),
            Some(&LawState::DefinitivelyAdopted {
                date: date(2026, 11, 19),
                agreement: Agreement::SameText
            })
        );
        assert_eq!(statuses(&progress), vec![Done, Done, Done, Current]);
        let an = &progress.milestones()[0];
        assert_eq!(an.sense, Some(DecisionSense::AdoptedUnchanged));
        assert_eq!(an.date, Some(date(2026, 11, 19)));
        let accord = &progress.milestones()[2];
        assert_eq!(accord.agreement, Some(Agreement::SameText));
        assert_eq!(accord.date, Some(date(2026, 11, 19)));
    }

    // Témoin DLR5L17N51095 (vie chère outre-mer).
    #[test]
    fn a_conforming_first_reading_by_the_second_chamber_is_definitive() {
        let acts = [
            act("SN1-DEPOT", date(2024, 12, 10)),
            decision("SN1-DEBATS-DEC", date(2025, 3, 5), "adoptée"),
            act("AN1-DEPOT", date(2025, 3, 6)),
            decision(
                "AN1-DEBATS-DEC",
                date(2026, 11, 19),
                "adoptée sans modification",
            ),
        ];

        let progress = LawProgress::from_acts(&acts, &DossierOutcome::NoRecordedConclusion);

        assert_eq!(
            progress.state(),
            Some(&LawState::DefinitivelyAdopted {
                date: date(2026, 11, 19),
                agreement: Agreement::SameText
            })
        );
        assert_eq!(
            steps(&progress)[..2],
            [MilestoneStep::Chamber(SN), MilestoneStep::Chamber(AN)]
        );
        assert_eq!(statuses(&progress), vec![Done, Done, Done, Current]);
    }

    // Témoins DLR5L17N54096 (CREPS de Vichy) et DLR5L17N53886 (médico-social).
    #[test]
    fn a_promulgated_text_became_law_with_its_publication() {
        let acts = [
            act("SN1-DEPOT", date(2026, 4, 9)),
            decision("SN1-DEBATS-DEC", date(2026, 6, 15), "adoptée"),
            decision(
                "AN1-DEBATS-DEC",
                date(2026, 10, 12),
                "adoptée sans modification",
            ),
            act("PROM-PUB", date(2026, 10, 26)),
        ];
        let outcome = promulgated(date(2026, 10, 26), "2026-992", date(2026, 10, 27));

        let progress = LawProgress::from_acts(&acts, &outcome);

        let Some(LawState::BecameLaw {
            date: on,
            publication,
        }) = progress.state()
        else {
            panic!("expected became law, got {:?}", progress.state());
        };
        assert_eq!(*on, date(2026, 10, 26));
        assert_eq!(publication.law_code.as_deref(), Some("2026-992"));
        assert_eq!(statuses(&progress), vec![Done, Done, Done, Done]);
        let accord = &progress.milestones()[2];
        assert_eq!(accord.agreement, Some(Agreement::SameText));
        assert_eq!(accord.date, Some(date(2026, 10, 12)));
        let law = &progress.milestones()[3];
        assert_eq!(law.date, Some(date(2026, 10, 26)));
        assert_eq!(law.publication.as_ref(), Some(publication));
    }

    // DLR5L17N51330 (droits voisins).
    #[test]
    fn a_joint_committee_text_adopted_by_both_chambers_is_definitive() {
        let acts = [
            act("AN1-DEPOT", date(2025, 1, 21)),
            decision("AN1-DEBATS-DEC", date(2026, 3, 26), "adoptée"),
            decision("SN1-DEBATS-DEC", date(2026, 6, 16), "modifiée"),
            decision("CMP-DEC", date(2026, 11, 4), "Accord"),
            decision(
                "CMP-DEBATS-AN-DEC",
                date(2026, 11, 12),
                "adoptée, dans les conditions prévues à l'article 45, alinéa 3, de la Constitution",
            ),
            decision(
                "CMP-DEBATS-SN-DEC",
                date(2026, 11, 19),
                "adoptée, dans les conditions prévues à l'article 45, alinéa 3, de la Constitution",
            ),
        ];

        let progress = LawProgress::from_acts(&acts, &DossierOutcome::NoRecordedConclusion);

        assert_eq!(
            progress.state(),
            Some(&LawState::DefinitivelyAdopted {
                date: date(2026, 11, 19),
                agreement: Agreement::JointCommittee
            })
        );
        assert_eq!(statuses(&progress), vec![Done, Done, Done, Current]);
        // Les chambres gardent leur dernière lecture ; la lecture du texte de
        // la CMP appartient à l'étape Accord.
        assert_eq!(progress.milestones()[0].date, Some(date(2026, 3, 26)));
        assert_eq!(
            progress.milestones()[2].agreement,
            Some(Agreement::JointCommittee)
        );
    }

    // DLR5L17N53159 (moyens d'action) : seul le Sénat a lu le texte de la CMP.
    #[test]
    fn a_joint_committee_agreement_read_by_one_chamber_only_is_still_in_shuttle() {
        let acts = [
            act("SN1-DEPOT", date(2025, 11, 14)),
            decision("SN1-DEBATS-DEC", date(2026, 1, 14), "adoptée"),
            decision("AN1-DEBATS-DEC", date(2026, 6, 3), "modifiée"),
            decision(
                "CMP-DEBATS-SN-DEC",
                date(2026, 11, 19),
                "adoptée, dans les conditions prévues à l'article 45, alinéa 3, de la Constitution",
            ),
            decision("CMP-DEC", date(2026, 11, 18), "Accord"),
        ];

        let progress = LawProgress::from_acts(&acts, &DossierOutcome::NoRecordedConclusion);

        assert_eq!(progress.state_kind(), "in_shuttle");
        assert_eq!(statuses(&progress), vec![Done, Done, Current, Upcoming]);
        let accord = &progress.milestones()[2];
        assert_eq!(accord.agreement, Some(Agreement::JointCommittee));
        assert_eq!(accord.date, Some(date(2026, 11, 19)));
    }

    #[test]
    fn a_joint_committee_agreement_alone_puts_the_agreement_step_in_progress() {
        let acts = [
            act("AN1-DEPOT", date(2025, 1, 21)),
            decision("AN1-DEBATS-DEC", date(2026, 3, 26), "adoptée"),
            decision("SN1-DEBATS-DEC", date(2026, 6, 16), "modifiée"),
            decision("CMP-DEC", date(2026, 11, 4), "Accord"),
        ];

        let progress = LawProgress::from_acts(&acts, &DossierOutcome::NoRecordedConclusion);

        assert_eq!(statuses(&progress), vec![Done, Done, Current, Upcoming]);
        assert_eq!(
            progress.milestones()[2].sense,
            Some(DecisionSense::CmpAgreement)
        );
    }

    // DLR5L17N51670 (fin de vie), avant sa promulgation.
    #[test]
    fn an_adopted_final_reading_is_definitive() {
        let acts = [
            act("AN1-DEPOT", date(2025, 3, 11)),
            decision("AN1-DEBATS-DEC", date(2025, 5, 27), "adoptée"),
            decision("SN1-DEBATS-DEC", date(2026, 1, 28), "rejetée"),
            decision("AN2-DEBATS-DEC", date(2026, 2, 25), "adoptée"),
            decision("SN2-DEBATS-DEC", date(2026, 5, 12), "rejetée"),
            decision("CMP-DEC", date(2026, 6, 3), "Désaccord"),
            decision("ANNLEC-DEBATS-DEC", date(2026, 6, 30), "adoptée"),
            decision("SNNLEC-DEBATS-DEC", date(2026, 7, 7), "rejetée"),
            decision("ANLDEF-DEBATS-DEC", date(2026, 7, 15), "adoptée"),
        ];

        let progress = LawProgress::from_acts(&acts, &DossierOutcome::NoRecordedConclusion);

        assert_eq!(
            progress.state(),
            Some(&LawState::DefinitivelyAdopted {
                date: date(2026, 7, 15),
                agreement: Agreement::FinalReading
            })
        );
        assert_eq!(statuses(&progress), vec![Done, Done, Done, Current]);
        let senate = &progress.milestones()[1];
        assert_eq!(senate.sense, Some(DecisionSense::Rejected));
        assert_eq!(senate.date, Some(date(2026, 7, 7)));
    }

    // DLR5L17N52428 (PLF 2026), lecture définitive au 49.3.
    #[test]
    fn a_text_considered_adopted_under_article_49_3_counts_as_adopted() {
        let acts = [
            act("AN1-DEPOT", date(2025, 10, 14)),
            decision("AN1-DEBATS-DEC", date(2025, 11, 21), "rejeté"),
            decision("SN1-DEBATS-DEC", date(2025, 12, 15), "modifié"),
            decision(
                "ANLDEF-DEBATS-DEC",
                date(2026, 2, 2),
                "considéré comme adopté par l'Assemblée nationale en application de l'article 49, alinéa 3 de la Constitution",
            ),
        ];

        let progress = LawProgress::from_acts(&acts, &DossierOutcome::NoRecordedConclusion);

        assert_eq!(
            progress.state(),
            Some(&LawState::DefinitivelyAdopted {
                date: date(2026, 2, 2),
                agreement: Agreement::FinalReading
            })
        );
    }

    #[test]
    fn a_rejection_by_one_chamber_then_a_modification_by_the_other_is_in_shuttle() {
        let acts = [
            act("AN1-DEPOT", date(2024, 10, 10)),
            decision("AN1-DEBATS-DEC", date(2024, 11, 12), "rejeté"),
            decision("SN1-DEBATS-DEC", date(2025, 1, 23), "modifié"),
        ];

        let progress = LawProgress::from_acts(&acts, &DossierOutcome::NoRecordedConclusion);

        assert_eq!(progress.state_kind(), "in_shuttle");
        assert_eq!(statuses(&progress), vec![Done, Current, Upcoming, Upcoming]);
        assert_eq!(
            progress.milestones()[0].sense,
            Some(DecisionSense::Rejected)
        );
    }

    #[test]
    fn an_adoption_then_a_rejection_by_the_other_chamber_is_in_shuttle() {
        let acts = [
            act("AN1-DEPOT", date(2025, 3, 11)),
            decision("AN1-DEBATS-DEC", date(2025, 5, 27), "adoptée"),
            decision("SN1-DEBATS-DEC", date(2026, 1, 28), "rejetée"),
        ];
        let outcome = DossierOutcome::Rejected {
            date: date(2026, 1, 28),
            label: "rejetée".into(),
        };

        let progress = LawProgress::from_acts(&acts, &outcome);

        let Some(LawState::InShuttle { last_decision }) = progress.state() else {
            panic!("expected in shuttle, got {:?}", progress.state());
        };
        assert_eq!(last_decision.sense, DecisionSense::Rejected);
        assert_eq!(statuses(&progress), vec![Done, Current, Upcoming, Upcoming]);
    }

    #[test]
    fn a_rejection_after_a_conforming_vote_restarts_the_shuttle() {
        let acts = [
            act("SN1-DEPOT", date(2024, 12, 10)),
            decision("SN1-DEBATS-DEC", date(2025, 3, 5), "adoptée"),
            decision(
                "AN1-DEBATS-DEC",
                date(2025, 6, 1),
                "adoptée sans modification",
            ),
            decision("SN2-DEBATS-DEC", date(2025, 7, 1), "rejetée"),
        ];

        let progress = LawProgress::from_acts(&acts, &DossierOutcome::NoRecordedConclusion);

        assert_eq!(progress.state_kind(), "in_shuttle");
    }

    #[test]
    fn a_rejection_without_any_adoption_has_no_state() {
        let acts = [
            act("AN1-DEPOT", date(2025, 1, 1)),
            decision("AN1-DEBATS-DEC", date(2025, 2, 1), "rejetée"),
        ];
        let outcome = DossierOutcome::Rejected {
            date: date(2025, 2, 1),
            label: "rejetée".into(),
        };

        let progress = LawProgress::from_acts(&acts, &outcome);

        assert_eq!(progress.state(), None);
        assert_eq!(progress.state_kind(), NO_LAW_STATE);
        assert!(progress.milestones().is_empty());
        assert_eq!(
            progress.last_decision().map(|d| d.sense),
            Some(DecisionSense::Rejected)
        );
    }

    #[test]
    fn a_withdrawn_text_has_no_state_even_after_an_adoption() {
        let acts = [
            act("AN1-DEPOT", date(2025, 1, 1)),
            decision("AN1-DEBATS-DEC", date(2025, 2, 1), "adoptée"),
            act("SN1-RTRINI", date(2025, 3, 1)),
        ];
        let outcome = DossierOutcome::Withdrawn {
            date: date(2025, 3, 1),
        };

        let progress = LawProgress::from_acts(&acts, &outcome);

        assert_eq!(progress.state(), None);
        assert!(progress.milestones().is_empty());
    }

    #[test]
    fn a_dossier_without_decision_has_no_state() {
        let acts = [act("AN1-DEPOT", date(2025, 1, 1))];

        let progress = LawProgress::from_acts(&acts, &DossierOutcome::NoRecordedConclusion);

        assert_eq!(progress.state(), None);
        assert_eq!(progress.last_decision(), None);
    }

    #[test]
    fn a_decision_with_an_unknown_wording_is_ignored() {
        let acts = [
            act("AN1-DEPOT", date(2025, 1, 1)),
            decision("AN1-DEBATS-DEC", date(2025, 2, 1), "adoptée"),
            LegislativeAct {
                conclusion_code: Some("TSORTF99".into()),
                ..decision("SN1-DEBATS-DEC", date(2025, 3, 1), "renvoyée en commission")
            },
        ];

        let progress = LawProgress::from_acts(&acts, &DossierOutcome::NoRecordedConclusion);

        let Some(LawState::InShuttle { last_decision }) = progress.state() else {
            panic!("expected in shuttle, got {:?}", progress.state());
        };
        assert_eq!(last_decision.code, "AN1-DEBATS-DEC");
        assert_eq!(
            statuses(&progress),
            vec![Current, Upcoming, Upcoming, Upcoming]
        );
    }

    #[test]
    fn a_resolution_adopted_in_single_reading_has_no_state() {
        let acts = [
            act("ANLUNI-DEPOT", date(2025, 1, 1)),
            decision("ANLUNI-DEBATS-DEC", date(2025, 2, 1), "adoptée"),
        ];

        let progress = LawProgress::from_acts(&acts, &DossierOutcome::NoRecordedConclusion);

        assert_eq!(progress.state(), None);
    }

    #[test]
    fn the_sense_falls_back_on_the_family_code_when_the_wording_is_missing() {
        assert_eq!(
            DecisionSense::from_source(None, Some("TSORTF03")),
            Some(DecisionSense::AdoptedUnchanged)
        );
        assert_eq!(DecisionSense::from_source(None, Some("TSORTFnull")), None);
        assert_eq!(DecisionSense::from_source(None, None), None);
    }

    #[test]
    fn every_wording_of_the_source_maps_to_its_sense() {
        let cases = [
            ("adoptée", DecisionSense::Adopted),
            ("adopté", DecisionSense::Adopted),
            ("adoptée sans modification", DecisionSense::AdoptedUnchanged),
            ("adopté sans modification", DecisionSense::AdoptedUnchanged),
            ("conforme", DecisionSense::AdoptedUnchanged),
            ("modifiée", DecisionSense::Modified),
            ("modifié", DecisionSense::Modified),
            ("adoptée avec modifications", DecisionSense::Modified),
            ("adopté avec modifications", DecisionSense::Modified),
            ("rejetée", DecisionSense::Rejected),
            ("rejeté", DecisionSense::Rejected),
            ("Accord", DecisionSense::CmpAgreement),
            ("Désaccord", DecisionSense::CmpDisagreement),
            (
                "considéré comme adopté par l'Assemblée nationale en application de l'article 49, alinéa 3 de la Constitution",
                DecisionSense::Adopted,
            ),
            (
                "considéré comme rejeté par l'Assemblée nationale en application de l'article 49, alinéa 3 de la Constitution",
                DecisionSense::Rejected,
            ),
            (
                "adopté, dans les conditions prévues à l'article 45, alinéa 3, de la Constitution",
                DecisionSense::Adopted,
            ),
        ];
        for (label, sense) in cases {
            assert_eq!(
                DecisionSense::from_source(Some(label), None),
                Some(sense),
                "{label}"
            );
        }
        assert_eq!(
            DecisionSense::from_source(
                Some("considérée comme définitive en application de l'article 151-7 du Règlement"),
                Some("TSORTF19")
            ),
            None
        );
    }

    #[test]
    fn a_joint_committee_reading_without_named_chamber_never_completes_the_agreement() {
        let acts = [
            act("AN1-DEPOT", date(2025, 1, 1)),
            decision("AN1-DEBATS-DEC", date(2025, 2, 1), "adoptée"),
            decision("SN1-DEBATS-DEC", date(2025, 3, 1), "modifiée"),
            decision("CMP-DEBATS-AN-DEC", date(2025, 4, 1), "adoptée"),
            decision("CMP-DEBATS-DEC", date(2025, 4, 2), "adoptée"),
        ];

        let progress = LawProgress::from_acts(&acts, &DossierOutcome::NoRecordedConclusion);

        assert_eq!(progress.state_kind(), "in_shuttle");
        assert_eq!(statuses(&progress), vec![Done, Done, Current, Upcoming]);
    }

    #[test]
    fn law_state_kinds_round_trip_through_their_stored_code() {
        for kind in LawStateKind::ALL {
            assert_eq!(LawStateKind::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(LawStateKind::parse(NO_LAW_STATE), None);
    }

    #[test]
    fn decision_senses_and_chambers_round_trip_through_their_stored_code() {
        for sense in [
            DecisionSense::Adopted,
            DecisionSense::AdoptedUnchanged,
            DecisionSense::Modified,
            DecisionSense::Rejected,
            DecisionSense::CmpAgreement,
            DecisionSense::CmpDisagreement,
        ] {
            assert_eq!(DecisionSense::parse(sense.as_str()), Some(sense));
        }
        for chamber in [AN, SN, Chamber::Joint, Chamber::None] {
            assert_eq!(Chamber::parse(chamber.as_str()), Some(chamber));
        }
    }

    fn shuttle_acts() -> Vec<LegislativeAct> {
        vec![
            act("AN1-DEPOT", date(2024, 12, 3)),
            decision("AN1-DEBATS-DEC", date(2026, 7, 7), "adoptée"),
            decision("SN1-DEBATS-DEC", date(2026, 11, 20), "modifiée"),
        ]
    }

    fn stored(summary: &LawStateSummary) -> Option<LawStateSummary> {
        let last = summary.last_decision();
        LawStateSummary::from_stored(
            Some(summary.kind().as_str()),
            Some(summary.date()),
            last.map(|d| d.chamber.as_str()),
            last.map(|d| d.sense.as_str()),
            last.map(|d| d.date),
        )
    }

    #[test]
    fn the_state_summary_reads_back_as_written() {
        let progress =
            LawProgress::from_acts(&shuttle_acts(), &DossierOutcome::NoRecordedConclusion);
        let summary = LawStateSummary::from_progress(&progress).unwrap();

        assert_eq!(summary.kind(), LawStateKind::InShuttle);
        assert_eq!(summary.date(), date(2026, 11, 20));
        assert_eq!(
            summary.last_decision(),
            Some(&DecisionSummary {
                chamber: SN,
                sense: DecisionSense::Modified,
                date: date(2026, 11, 20),
            })
        );
        assert_eq!(stored(&summary), Some(summary));
    }

    #[test]
    fn a_dossier_without_state_stores_none_and_reads_back_none() {
        let acts = [decision("AN1-DEBATS-DEC", date(2026, 3, 1), "rejetée")];
        let progress = LawProgress::from_acts(&acts, &DossierOutcome::NoRecordedConclusion);

        assert_eq!(LawStateSummary::from_progress(&progress), None);
        assert_eq!(
            LawStateSummary::from_stored(Some(progress.state_kind()), None, None, None, None),
            None
        );
        assert_eq!(
            LawStateSummary::from_stored(None, Some(date(2026, 3, 1)), None, None, None),
            None
        );
    }

    #[test]
    fn the_state_as_of_a_day_ignores_later_acts_and_a_later_promulgation() {
        let acts = [
            act("SN1-DEPOT", date(2026, 3, 1)),
            decision("SN1-DEBATS-DEC", date(2026, 6, 15), "adoptée"),
            decision(
                "AN1-DEBATS-DEC",
                date(2026, 10, 12),
                "adoptée sans modification",
            ),
            act("PROM-PUB", date(2026, 10, 26)),
        ];
        let outcome = promulgated(date(2026, 10, 26), "2026-992", date(2026, 10, 27));

        let before = LawProgress::as_of(&acts, &outcome, date(2026, 10, 18));
        let after = LawProgress::as_of(&acts, &outcome, date(2026, 11, 1));
        let earlier = LawProgress::as_of(&acts, &outcome, date(2026, 7, 1));

        assert_eq!(before.state_kind(), "definitively_adopted");
        assert_eq!(after.state_kind(), "became_law");
        assert_eq!(earlier.state_kind(), "in_shuttle");
    }

    #[test]
    fn the_milestone_of_a_vote_is_the_one_of_the_decision_citing_it() {
        let vote = ScrutinUid::new("VTANR5L17V7987".into()).unwrap();
        let mut acts = shuttle_acts();
        acts[1].decision_vote = Some(vote.clone());

        assert_eq!(step_of_vote(&acts, &vote), Some(MilestoneStep::Chamber(AN)));
        let other = ScrutinUid::new("VTANR5L17V1".into()).unwrap();
        assert_eq!(step_of_vote(&acts, &other), None);
    }

    #[test]
    fn a_vote_cited_by_a_decision_of_unknown_sense_keeps_its_reading_milestone() {
        let vote = ScrutinUid::new("VTANR5L17V10080".into()).unwrap();
        let mut cmp = act("CMP-DEBATS-AN-DEC", date(2026, 11, 19));
        cmp.decision_vote = Some(vote.clone());

        assert_eq!(step_of_vote(&[cmp], &vote), Some(MilestoneStep::Agreement));
    }

    fn week() -> (NaiveDate, NaiveDate) {
        (date(2026, 11, 16), date(2026, 11, 22))
    }

    #[test]
    fn a_decision_of_the_week_advances_the_text_with_its_state_at_the_end_of_the_week() {
        let mut acts = shuttle_acts();
        acts[2].decision_vote = None;
        let (monday, sunday) = week();

        let advance =
            LawAdvance::between(&acts, &DossierOutcome::NoRecordedConclusion, monday, sunday)
                .unwrap();

        assert_eq!(advance.state.kind(), LawStateKind::InShuttle);
        assert_eq!(advance.event.date, date(2026, 11, 20));
        assert_eq!(advance.event.reading, "SN1");
        assert_eq!(advance.event.chamber, Some(SN));
        assert_eq!(advance.event.sense, Some(DecisionSense::Modified));
        assert!(!advance.event.has_public_vote);
        assert_eq!(advance.event.joint_committee_agreement, None);
    }

    #[test]
    fn a_text_without_decision_in_the_week_has_not_advanced() {
        let (monday, sunday) = week();
        let acts = &shuttle_acts()[..2];

        assert_eq!(
            LawAdvance::between(acts, &DossierOutcome::NoRecordedConclusion, monday, sunday),
            None
        );
    }

    #[test]
    fn a_text_only_rejected_in_the_week_has_no_state_and_is_not_an_advance() {
        let (monday, sunday) = week();
        let acts = [decision("AN1-DEBATS-DEC", date(2026, 11, 18), "rejetée")];

        assert_eq!(
            LawAdvance::between(&acts, &DossierOutcome::NoRecordedConclusion, monday, sunday),
            None
        );
    }

    #[test]
    fn a_promulgation_of_the_week_is_the_event_and_a_later_one_does_not_count() {
        let acts = [
            act("SN1-DEPOT", date(2026, 3, 1)),
            decision("SN1-DEBATS-DEC", date(2026, 6, 15), "adoptée"),
            decision(
                "AN1-DEBATS-DEC",
                date(2026, 11, 17),
                "adoptée sans modification",
            ),
            act("PROM-PUB", date(2026, 11, 20)),
        ];
        let outcome = promulgated(date(2026, 11, 20), "2026-1100", date(2026, 11, 21));
        let (monday, sunday) = week();

        let advance = LawAdvance::between(&acts, &outcome, monday, sunday).unwrap();
        assert_eq!(advance.state.kind(), LawStateKind::BecameLaw);
        assert_eq!(advance.event.reading, PROMULGATION_READING);
        assert_eq!(advance.event.chamber, None);

        let advance = LawAdvance::between(&acts, &outcome, monday, date(2026, 11, 19)).unwrap();
        assert_eq!(advance.state.kind(), LawStateKind::DefinitivelyAdopted);
        assert_eq!(advance.event.reading, "AN1");
        assert_eq!(advance.event.sense, Some(DecisionSense::AdoptedUnchanged));
    }

    #[test]
    fn a_joint_committee_reading_of_the_week_carries_the_agreement_date() {
        let vote = ScrutinUid::new("VTANR5L17V9000".into()).unwrap();
        let mut reading = decision("CMP-DEBATS-AN-DEC", date(2026, 11, 18), "adoptée");
        reading.decision_vote = Some(vote);
        let acts = [
            act("AN1-DEPOT", date(2026, 1, 5)),
            decision("AN1-DEBATS-DEC", date(2026, 3, 1), "adoptée"),
            decision("SN1-DEBATS-DEC", date(2026, 6, 1), "modifiée"),
            decision("CMP-DEC", date(2026, 11, 4), "Accord"),
            reading,
        ];
        let (monday, sunday) = week();

        let advance =
            LawAdvance::between(&acts, &DossierOutcome::NoRecordedConclusion, monday, sunday)
                .unwrap();

        assert_eq!(advance.state.kind(), LawStateKind::InShuttle);
        assert_eq!(advance.event.reading, "CMP");
        assert_eq!(advance.event.chamber, Some(AN));
        assert!(advance.event.has_public_vote);
        assert_eq!(
            advance.event.joint_committee_agreement,
            Some(date(2026, 11, 4))
        );
    }
}
