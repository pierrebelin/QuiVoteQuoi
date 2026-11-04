use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::actor::{
    ActorDirectory, ActorRole, ActorUid, GroupUid, MembershipQuality, ParliamentaryGroup,
};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DossierError {
    #[error("dossier uid must not be empty")]
    EmptyDossierUid,
    #[error("an initiator group must always carry its reference date")]
    GroupWithoutReferenceDate,
    #[error("score progress must be 0-10, got {0}")]
    InvalidProgress(u8),
    #[error("score magnitude must be 0-10, got {0}")]
    InvalidMagnitude(u8),
    #[error("score momentum must be 0-10, got {0}")]
    InvalidMomentum(u8),
    #[error("score total must be 0-100, got {0}")]
    InvalidTotal(u8),
    #[error("initiator name must not be empty")]
    EmptyInitiatorName,
    #[error("committee name must not be empty")]
    EmptyCommittee,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct DossierUid(String);

impl DossierUid {
    pub fn new(raw: String) -> Result<Self, DossierError> {
        if raw.trim().is_empty() {
            return Err(DossierError::EmptyDossierUid);
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DossierUid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Committee(String);

impl Committee {
    pub fn new(raw: String) -> Result<Self, DossierError> {
        if raw.trim().is_empty() {
            return Err(DossierError::EmptyCommittee);
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LegislativeAct {
    pub date: NaiveDate,
    pub label: String,
    pub code: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LegislativeDocument {
    pub document_uid: String,
    pub title: String,
    pub short_title: Option<String>,
    pub doc_type: String,
    pub date: Option<NaiveDate>,
    /// Page officielle qui porte le document précis, et non le seul dossier.
    pub official_url: Option<String>,
    /// Archive open data d'où viennent l'identifiant et les métadonnées.
    pub source_archive_url: Option<String>,
    pub source_license: Option<String>,
    /// Empreinte SHA-256 de la notice officielle archivée, pas du texte brut.
    /// Elle rend détectable une mise à jour des métadonnées lors d'une ingestion.
    pub source_metadata_fingerprint: Option<String>,
    pub source_retrieved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Score {
    progress: u8,
    magnitude: u8,
    momentum: u8,
    total: u8,
}

impl Score {
    pub fn new(progress: u8, magnitude: u8, momentum: u8, total: u8) -> Result<Self, DossierError> {
        if progress > 10 {
            return Err(DossierError::InvalidProgress(progress));
        }
        if magnitude > 10 {
            return Err(DossierError::InvalidMagnitude(magnitude));
        }
        if momentum > 10 {
            return Err(DossierError::InvalidMomentum(momentum));
        }
        if total > 100 {
            return Err(DossierError::InvalidTotal(total));
        }
        Ok(Self {
            progress,
            magnitude,
            momentum,
            total,
        })
    }

    pub fn progress(&self) -> u8 {
        self.progress
    }

    pub fn magnitude(&self) -> u8 {
        self.magnitude
    }

    pub fn momentum(&self) -> u8 {
        self.momentum
    }

    pub fn total(&self) -> u8 {
        self.total
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Chamber {
    AssembleeNationale,
    Senat,
    Joint,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum LegislativeStage {
    FirstReadingAN,
    FirstReadingSenate,
    SecondReadingAN,
    SecondReadingSenate,
    JointCommittee,
    NewReadingAN,
    NewReadingSenate,
    FinalReading,
    ConstitutionalReview,
    Promulgation,
}

impl LegislativeStage {
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "AN1" => Some(Self::FirstReadingAN),
            "SN1" => Some(Self::FirstReadingSenate),
            "AN2" => Some(Self::SecondReadingAN),
            "SN2" => Some(Self::SecondReadingSenate),
            "CMP" => Some(Self::JointCommittee),
            "ANNLEC" => Some(Self::NewReadingAN),
            "SNNLEC" => Some(Self::NewReadingSenate),
            "ANLDEF" => Some(Self::FinalReading),
            "CC" => Some(Self::ConstitutionalReview),
            "PROM" => Some(Self::Promulgation),
            _ => None,
        }
    }

    pub fn to_code(&self) -> &'static str {
        match self {
            Self::FirstReadingAN => "AN1",
            Self::FirstReadingSenate => "SN1",
            Self::SecondReadingAN => "AN2",
            Self::SecondReadingSenate => "SN2",
            Self::JointCommittee => "CMP",
            Self::NewReadingAN => "ANNLEC",
            Self::NewReadingSenate => "SNNLEC",
            Self::FinalReading => "ANLDEF",
            Self::ConstitutionalReview => "CC",
            Self::Promulgation => "PROM",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::FirstReadingAN => "1\u{00e8}re lecture",
            Self::FirstReadingSenate => "1\u{00e8}re lecture",
            Self::SecondReadingAN => "2\u{00e8}me lecture",
            Self::SecondReadingSenate => "2\u{00e8}me lecture",
            Self::JointCommittee => "Commission mixte paritaire",
            Self::NewReadingAN => "Nouvelle lecture",
            Self::NewReadingSenate => "Nouvelle lecture",
            Self::FinalReading => "Lecture d\u{00e9}finitive",
            Self::ConstitutionalReview => "Conseil constitutionnel",
            Self::Promulgation => "Promulgation",
        }
    }

    pub fn chamber(&self) -> Chamber {
        match self {
            Self::FirstReadingAN
            | Self::SecondReadingAN
            | Self::NewReadingAN
            | Self::FinalReading => Chamber::AssembleeNationale,
            Self::FirstReadingSenate | Self::SecondReadingSenate | Self::NewReadingSenate => {
                Chamber::Senat
            }
            Self::JointCommittee => Chamber::Joint,
            Self::ConstitutionalReview | Self::Promulgation => Chamber::None,
        }
    }
}

/// Groupe d'un initiateur, tel qu'il etait a la date de depot du dossier.
#[derive(Debug, Clone, Serialize)]
pub struct InitiatorGroup {
    pub uid: GroupUid,
    pub abbrev: String,
    pub label: String,
    pub quality: Option<MembershipQuality>,
}

impl InitiatorGroup {
    pub fn from_group(group: &ParliamentaryGroup, quality: Option<MembershipQuality>) -> Self {
        Self {
            uid: group.uid().clone(),
            abbrev: group.abbrev().to_string(),
            label: group.label().to_string(),
            quality,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Initiator {
    full_name: String,
    actor_uid: Option<ActorUid>,
    role: Option<ActorRole>,
    group: Option<InitiatorGroup>,
    reference_date: Option<NaiveDate>,
    official_url: Option<String>,
}

impl Initiator {
    /// Initiateur non resolu: le nom brut est conserve, aucun groupe n'est
    /// devine (RM-04).
    pub fn unresolved(full_name: String) -> Result<Self, DossierError> {
        if full_name.trim().is_empty() {
            return Err(DossierError::EmptyInitiatorName);
        }
        Ok(Self {
            full_name,
            actor_uid: None,
            role: None,
            group: None,
            reference_date: None,
            official_url: None,
        })
    }

    /// Initiateur rattache au referentiel.
    ///
    /// RM-01: un groupe ne peut etre porte sans la date a laquelle il a ete lu.
    pub fn resolved(
        full_name: String,
        actor_uid: ActorUid,
        role: ActorRole,
        group: Option<InitiatorGroup>,
        reference_date: Option<NaiveDate>,
        official_url: Option<String>,
    ) -> Result<Self, DossierError> {
        if full_name.trim().is_empty() {
            return Err(DossierError::EmptyInitiatorName);
        }
        if group.is_some() && reference_date.is_none() {
            return Err(DossierError::GroupWithoutReferenceDate);
        }
        Ok(Self {
            full_name,
            actor_uid: Some(actor_uid),
            role: Some(role),
            group,
            reference_date,
            official_url,
        })
    }

    pub fn full_name(&self) -> &str {
        &self.full_name
    }

    pub fn actor_uid(&self) -> Option<&ActorUid> {
        self.actor_uid.as_ref()
    }

    pub fn role(&self) -> Option<ActorRole> {
        self.role
    }

    pub fn group(&self) -> Option<&InitiatorGroup> {
        self.group.as_ref()
    }

    /// Date de l'acte a laquelle le groupe a ete lu. Toujours presente quand un
    /// groupe est affiche (RM-01).
    pub fn reference_date(&self) -> Option<NaiveDate> {
        self.reference_date
    }

    pub fn official_url(&self) -> Option<&str> {
        self.official_url.as_deref()
    }

    /// Initiateur d'un dossier dont la source ne nomme aucun acteur.
    ///
    /// Un projet de loi vient du gouvernement: la source ne liste alors aucun
    /// depute, et l'initiateur est le gouvernement lui-meme, sans acteur ni
    /// groupe. Toute autre initiative, ou une initiative inconnue, ne donne
    /// rien: l'initiateur absent n'est jamais devine.
    pub fn default_for(initiative: Option<Initiative>, has_initiator_refs: bool) -> Vec<Self> {
        if initiative == Some(Initiative::Government) && !has_initiator_refs {
            Self::unresolved(GOVERNMENT_INITIATOR_NAME.to_string())
                .into_iter()
                .collect()
        } else {
            vec![]
        }
    }
}

/// Nom porte par l'initiateur d'un projet de loi que la source ne rattache a
/// aucun acteur.
pub const GOVERNMENT_INITIATOR_NAME: &str = "Gouvernement";

/// Qui a depose le texte, lu dans le libelle de procedure de la source.
///
/// « Projet de loi ... » vient du gouvernement, « Proposition de loi ... » ou
/// « Proposition de resolution ... » d'un parlementaire. C'est la seule lecture
/// faite du libelle: tout autre intitule reste sans initiative connue plutot
/// que range d'office (README.md §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Initiative {
    Government,
    Parliamentary,
}

impl Initiative {
    /// Prefixe du libelle de procedure qui porte cette initiative. Une seule
    /// regle, partagee par la lecture de la source et par le filtre de liste.
    pub fn procedure_prefix(&self) -> &'static str {
        match self {
            Self::Government => "Projet de loi",
            Self::Parliamentary => "Proposition",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Government => "government",
            Self::Parliamentary => "parliamentary",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "government" => Some(Self::Government),
            "parliamentary" => Some(Self::Parliamentary),
            _ => None,
        }
    }

    /// Initiative portee par un libelle de procedure, si elle s'y lit.
    pub fn from_procedure(procedure: &str) -> Option<Self> {
        [Self::Government, Self::Parliamentary]
            .into_iter()
            .find(|initiative| procedure.starts_with(initiative.procedure_prefix()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurationStatus {
    New,
    Selected,
    Dismissed,
    Published,
}

impl CurationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Selected => "selected",
            Self::Dismissed => "dismissed",
            Self::Published => "published",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "new" => Some(Self::New),
            "selected" => Some(Self::Selected),
            "dismissed" => Some(Self::Dismissed),
            "published" => Some(Self::Published),
            _ => None,
        }
    }
}

/// Publication au Journal officiel d'une loi promulguee.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LawPublication {
    pub law_code: Option<String>,
    pub jo_date: Option<NaiveDate>,
    pub legifrance_url: Option<String>,
}

/// Sort du dossier, tel que la source l'atteste — jamais deduit d'un silence.
///
/// L'open data de l'Assemblee ne porte aucun champ de statut: le sort se lit
/// dans les actes. 92% des dossiers de la legislature 17 n'en portent aucun
/// (2 788 sur 3 035): pour ceux-la la seule chose vraie est « aucune
/// conclusion enregistree ». Les declarer abandonnes serait une inference,
/// les declarer en cours aussi (README.md §6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DossierOutcome {
    /// Acte `PROM-PUB`. Definitif.
    Promulgated {
        date: NaiveDate,
        #[serde(flatten)]
        publication: LawPublication,
    },
    /// Acte `*-RTRINI`: l'initiative a ete retiree. Definitif.
    Withdrawn { date: NaiveDate },
    /// `fusionDossier`: le dossier a ete absorbe par un autre. Definitif.
    MergedInto {
        dossier_uid: DossierUid,
        cause: Option<String>,
    },
    /// Dernier sort de seance connu, quand il vaut rejet.
    ///
    /// **Pas definitif**: un rejet en premiere lecture ne clot pas la navette,
    /// le texte peut revenir. C'est le dernier fait connu, pas une fin.
    Rejected { date: NaiveDate, label: String },
    /// Aucun acte de conclusion. Le dernier acte du dossier fait foi.
    NoRecordedConclusion,
}

impl DossierOutcome {
    /// Un sort definitif ne peut plus changer: le dossier n'a plus a etre
    /// reecrit lors des rafraichissements suivants.
    ///
    /// `Rejected` en est exclu volontairement: la navette peut reprendre.
    pub fn is_final(&self) -> bool {
        matches!(
            self,
            Self::Promulgated { .. } | Self::Withdrawn { .. } | Self::MergedInto { .. }
        )
    }

    /// Codes de sort tels que `kind()` les rend et que la base les stocke.
    /// Partages entre `kind()` et `kind_is_final()`: une seule ecriture de
    /// chaque code.
    const KIND_PROMULGATED: &'static str = "promulgated";
    const KIND_WITHDRAWN: &'static str = "withdrawn";
    const KIND_MERGED_INTO: &'static str = "merged_into";
    const KIND_REJECTED: &'static str = "rejected";
    const KIND_NO_RECORDED_CONCLUSION: &'static str = "no_recorded_conclusion";

    pub fn kind(&self) -> &'static str {
        match self {
            Self::Promulgated { .. } => Self::KIND_PROMULGATED,
            Self::Withdrawn { .. } => Self::KIND_WITHDRAWN,
            Self::MergedInto { .. } => Self::KIND_MERGED_INTO,
            Self::Rejected { .. } => Self::KIND_REJECTED,
            Self::NoRecordedConclusion => Self::KIND_NO_RECORDED_CONCLUSION,
        }
    }

    /// Vrai quand le code de sort stocke, celui que rend `kind()`, designe un
    /// sort definitif (voir `is_final`). Le contenu de la variante n'est pas
    /// necessaire pour le savoir: un sort promulgue reste definitif meme si sa
    /// date manque en base. Un code inconnu n'est pas definitif.
    pub fn kind_is_final(kind: &str) -> bool {
        Self::FINAL_KINDS.contains(&kind)
    }

    /// Les codes de sort definitifs (voir `kind_is_final`).
    const FINAL_KINDS: [&'static str; 3] = [
        Self::KIND_PROMULGATED,
        Self::KIND_WITHDRAWN,
        Self::KIND_MERGED_INTO,
    ];


    pub fn date(&self) -> Option<NaiveDate> {
        match self {
            Self::Promulgated { date, .. }
            | Self::Withdrawn { date }
            | Self::Rejected { date, .. } => Some(*date),
            Self::MergedInto { .. } | Self::NoRecordedConclusion => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LegislativeDossier {
    uid: DossierUid,
    title: String,
    procedure: String,
    legislature: u16,
    url: Option<String>,
    summary: Option<String>,
    /// Date de depot du texte. Date de reference du rattachement des
    /// initiateurs a leur groupe (RM-01).
    deposit_date: Option<NaiveDate>,
    last_activity_date: NaiveDate,
    last_activity_label: String,
    acts: Vec<LegislativeAct>,
    documents: Vec<LegislativeDocument>,
    score: Score,
    current_stage: Option<LegislativeStage>,
    initiators: Vec<Initiator>,
    committee: Option<Committee>,
    curation_status: CurationStatus,
    outcome: DossierOutcome,
}

/// Elements d'un dossier legislatif, tels que lus dans la source ou en base.
///
/// Une struct plutot que dix-sept parametres positionnels: le champ est nomme
/// au site d'appel, et un oubli se voit a la compilation.
#[derive(Debug, Clone)]
pub struct NewLegislativeDossier {
    pub uid: DossierUid,
    pub title: String,
    pub procedure: String,
    pub legislature: u16,
    pub url: Option<String>,
    pub summary: Option<String>,
    /// Date de depot du texte. Date de reference du rattachement des
    /// initiateurs a leur groupe (RM-01).
    pub deposit_date: Option<NaiveDate>,
    pub last_activity_date: NaiveDate,
    pub last_activity_label: String,
    pub acts: Vec<LegislativeAct>,
    pub documents: Vec<LegislativeDocument>,
    pub score: Score,
    pub current_stage: Option<LegislativeStage>,
    pub initiators: Vec<Initiator>,
    pub committee: Option<Committee>,
    pub curation_status: CurationStatus,
    pub outcome: DossierOutcome,
}

/// Ce qu'un rattachement d'initiateurs a produit, y compris ce qu'il n'a pas pu
/// produire. Toute lacune est comptee pour etre signalee plutot que silencieuse.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InitiatorAttachment {
    pub resolved: usize,
    /// Acteurs absents du referentiel: nom brut conserve, aucun groupe (RM-04).
    pub unknown_actors: usize,
    /// Acteurs connus sans appartenance valide a la date de depot.
    pub without_group: usize,
    /// Plusieurs groupes concurrents a la date de depot: rien n'est affiche.
    pub ambiguous: usize,
    /// Dossier sans date de depot: le groupe ne peut pas etre date (RM-01).
    pub missing_deposit_date: bool,
}

/// Ce que la base sait deja d'un dossier, lu avant toute reecriture.
///
/// Le rafraichissement compare cet etat a celui de la source pour n'ecrire
/// que ce qui a bouge : reecrire les 3 035 dossiers coute une quinzaine
/// d'allers-retours chacun sur la base PostgreSQL locale du VPS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredDossierState {
    pub last_activity_date: NaiveDate,
    pub act_count: usize,
    /// Sort qui ne peut plus changer (promulgation, retrait, fusion).
    pub outcome_is_final: bool,
}

/// Ce qu'un rafraichissement fait d'un dossier lu dans la source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteDecision {
    /// Dossier absent de la base, ou dont la source a bouge: a reecrire.
    Write,
    /// Sort definitif deja stocke: plus rien ne peut changer.
    SkipFinal,
    /// Dernier acte et nombre d'actes inchanges depuis la derniere ingestion.
    SkipUnchanged,
}

/// Periode qu'un dossier couvre, du premier fait date a la derniere activite.
///
/// Elle borne les groupes parlementaires dont l'existence recoupe le dossier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DossierPeriod {
    start: NaiveDate,
    end: NaiveDate,
}

impl DossierPeriod {
    /// Le debut est la plus ancienne des dates connues parmi le depot, le
    /// premier document et le premier acte; sans aucune de ces dates, la
    /// periode se resume a la derniere activite. La fin est toujours la
    /// derniere activite.
    ///
    /// Une date absente n'est jamais devinee: elle est ignoree, et seule
    /// l'absence de toutes ramene le debut a la derniere activite.
    pub fn from_dates(
        deposit_date: Option<NaiveDate>,
        first_document_date: Option<NaiveDate>,
        first_act_date: Option<NaiveDate>,
        last_activity_date: NaiveDate,
    ) -> Self {
        let start = [deposit_date, first_document_date, first_act_date]
            .into_iter()
            .flatten()
            .min()
            .unwrap_or(last_activity_date);

        Self {
            start,
            end: last_activity_date,
        }
    }

    pub fn start(&self) -> NaiveDate {
        self.start
    }

    pub fn end(&self) -> NaiveDate {
        self.end
    }
}

impl LegislativeDossier {
    /// Assemble le dossier a partir de ses elements. Infaillible: chaque champ
    /// est deja type, aucun invariant ne se cumule entre eux.
    pub fn new(parts: NewLegislativeDossier) -> Self {
        Self {
            uid: parts.uid,
            title: parts.title,
            procedure: parts.procedure,
            legislature: parts.legislature,
            url: parts.url,
            summary: parts.summary,
            deposit_date: parts.deposit_date,
            last_activity_date: parts.last_activity_date,
            last_activity_label: parts.last_activity_label,
            acts: parts.acts,
            documents: parts.documents,
            score: parts.score,
            current_stage: parts.current_stage,
            initiators: parts.initiators,
            committee: parts.committee,
            curation_status: parts.curation_status,
            outcome: parts.outcome,
        }
    }

    pub fn uid(&self) -> &DossierUid {
        &self.uid
    }
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn procedure(&self) -> &str {
        &self.procedure
    }
    pub fn legislature(&self) -> u16 {
        self.legislature
    }
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }
    pub fn summary(&self) -> Option<&str> {
        self.summary.as_deref()
    }
    pub fn deposit_date(&self) -> Option<NaiveDate> {
        self.deposit_date
    }
    pub fn last_activity_date(&self) -> NaiveDate {
        self.last_activity_date
    }
    pub fn last_activity_label(&self) -> &str {
        &self.last_activity_label
    }
    pub fn acts(&self) -> &[LegislativeAct] {
        &self.acts
    }
    pub fn documents(&self) -> &[LegislativeDocument] {
        &self.documents
    }
    pub fn score(&self) -> &Score {
        &self.score
    }
    pub fn current_stage(&self) -> Option<LegislativeStage> {
        self.current_stage
    }
    pub fn initiators(&self) -> &[Initiator] {
        &self.initiators
    }
    pub fn committee(&self) -> Option<&Committee> {
        self.committee.as_ref()
    }
    pub fn curation_status(&self) -> CurationStatus {
        self.curation_status
    }
    pub fn outcome(&self) -> &DossierOutcome {
        &self.outcome
    }

    /// Remplace l'etat de curation du dossier (nouveau, retenu, ecarte, publie).
    pub fn change_curation_status(&mut self, status: CurationStatus) {
        self.curation_status = status;
    }

    /// Un dossier n'est reecrit que si la source a bouge.
    ///
    /// Deux raisons de sauter, dans cet ordre:
    ///
    /// 1. le sort stocke est definitif (promulgation, retrait, fusion) — il ne
    ///    peut plus changer, quoi que la source republie;
    /// 2. le dernier acte et le nombre d'actes sont identiques a ce qui est
    ///    stocke — rien n'a bouge.
    ///
    /// L'anciennete n'est deliberement pas un critere: une proposition de loi
    /// dormante depuis deux ans peut etre inscrite a l'ordre du jour du jour au
    /// lendemain. La sauter sur son age figerait sa page au moment ou elle
    /// redevient interessante, sans rien economiser de plus que la regle 2 —
    /// qui la saute deja tant qu'elle dort.
    ///
    /// Limite connue: une correction de titre ou de resume qui ne toucherait
    /// aucun acte n'est pas detectee. Une reecriture complete la rattrape.
    pub fn write_decision(&self, stored: Option<&StoredDossierState>) -> WriteDecision {
        let Some(stored) = stored else {
            return WriteDecision::Write;
        };

        if stored.outcome_is_final {
            return WriteDecision::SkipFinal;
        }

        if stored.last_activity_date == self.last_activity_date
            && stored.act_count == self.acts.len()
        {
            return WriteDecision::SkipUnchanged;
        }

        WriteDecision::Write
    }

    /// Rattache chaque initiateur au groupe qu'il avait a la date de depot.
    ///
    /// Sans date de depot, l'identite de l'acteur est conservee mais aucun
    /// groupe n'est porte: un groupe non date reecrirait l'histoire (RM-01).
    pub fn attach_initiators(
        &mut self,
        actor_refs: &[ActorUid],
        directory: &ActorDirectory,
    ) -> InitiatorAttachment {
        let mut report = InitiatorAttachment {
            missing_deposit_date: self.deposit_date.is_none(),
            ..Default::default()
        };

        let mut initiators = Vec::with_capacity(actor_refs.len());

        for actor_ref in actor_refs {
            let resolved = self
                .deposit_date
                .and_then(|date| directory.resolve_at(actor_ref, date))
                .or_else(|| {
                    // Sans date de depot on lit quand meme l'identite: la
                    // resolution a une date arbitraire ne renverrait pas de
                    // groupe utilisable, seul l'acteur nous interesse ici.
                    directory
                        .actor(actor_ref)
                        .map(|actor| super::actor::ActorAtDate {
                            actor,
                            group: None,
                            quality: None,
                            ambiguous: false,
                        })
                });

            let Some(resolved) = resolved else {
                report.unknown_actors += 1;
                if let Ok(initiator) = Initiator::unresolved(actor_ref.as_str().to_string()) {
                    initiators.push(initiator);
                }
                continue;
            };

            if resolved.ambiguous {
                report.ambiguous += 1;
            } else if resolved.group.is_none() {
                report.without_group += 1;
            }

            let group = resolved
                .group
                .map(|g| InitiatorGroup::from_group(g, resolved.quality.cloned()));
            let reference_date = self.deposit_date;

            let initiator = Initiator::resolved(
                resolved.actor.full_name(),
                resolved.actor.uid().clone(),
                resolved.actor.role(),
                group.filter(|_| reference_date.is_some()),
                reference_date,
                resolved.actor.official_url(),
            );

            match initiator {
                Ok(initiator) => {
                    report.resolved += 1;
                    initiators.push(initiator);
                }
                Err(_) => {
                    report.unknown_actors += 1;
                    if let Ok(initiator) = Initiator::unresolved(actor_ref.as_str().to_string()) {
                        initiators.push(initiator);
                    }
                }
            }
        }

        self.initiators = initiators;
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::actor::{Actor, GroupMembership, MembershipPeriod};

    fn dossier_deposited_on(deposit_date: Option<NaiveDate>) -> LegislativeDossier {
        LegislativeDossier::new(NewLegislativeDossier {
            uid: DossierUid::new("D1".into()).unwrap(),
            title: "Loi test".into(),
            procedure: "Proposition de loi".into(),
            legislature: 17,
            url: None,
            summary: None,
            deposit_date,
            last_activity_date: NaiveDate::from_ymd_opt(2025, 5, 13).unwrap(),
            last_activity_label: "D\u{00e9}p\u{00f4}t".into(),
            acts: vec![],
            documents: vec![],
            score: Score::new(2, 4, 2, 23).unwrap(),
            current_stage: None,
            initiators: vec![],
            committee: None,
            curation_status: CurationStatus::New,
            outcome: DossierOutcome::NoRecordedConclusion,
        })
    }

    fn directory_with_group_change() -> ActorDirectory {
        let actor = Actor::new(
            ActorUid::new("PA1".into()).unwrap(),
            Some("M.".into()),
            "Jean".into(),
            "Dupont".into(),
            ActorRole::Deputy,
        )
        .unwrap();

        let group_a = ParliamentaryGroup::new(
            GroupUid::new("PO_A".into()).unwrap(),
            17,
            "Groupe A".into(),
            "A".into(),
            None,
            None,
            None,
        )
        .unwrap();
        let group_b = ParliamentaryGroup::new(
            GroupUid::new("PO_B".into()).unwrap(),
            17,
            "Groupe B".into(),
            "B".into(),
            None,
            None,
            None,
        )
        .unwrap();

        let membership = |source: &str, group: &str, start, end| {
            GroupMembership::new(
                source.into(),
                ActorUid::new("PA1".into()).unwrap(),
                GroupUid::new(group.into()).unwrap(),
                17,
                MembershipPeriod::new(start, end).unwrap(),
                MembershipQuality::new("Membre".into()).unwrap(),
            )
        };

        ActorDirectory::new(
            vec![actor],
            vec![group_a, group_b],
            vec![
                membership(
                    "PM1",
                    "PO_A",
                    NaiveDate::from_ymd_opt(2024, 7, 19).unwrap(),
                    Some(NaiveDate::from_ymd_opt(2025, 6, 30).unwrap()),
                ),
                membership(
                    "PM2",
                    "PO_B",
                    NaiveDate::from_ymd_opt(2025, 7, 1).unwrap(),
                    None,
                ),
            ],
        )
    }

    #[test]
    fn attaches_the_group_held_on_the_deposit_date_not_the_current_one() {
        let mut dossier = dossier_deposited_on(Some(NaiveDate::from_ymd_opt(2025, 5, 13).unwrap()));
        let directory = directory_with_group_change();
        let refs = vec![ActorUid::new("PA1".into()).unwrap()];

        let report = dossier.attach_initiators(&refs, &directory);

        assert_eq!(report.resolved, 1);
        let initiator = &dossier.initiators[0];
        assert_eq!(initiator.full_name(), "Jean Dupont");
        assert_eq!(initiator.group().unwrap().abbrev, "A");
        assert_eq!(
            initiator.reference_date(),
            Some(NaiveDate::from_ymd_opt(2025, 5, 13).unwrap())
        );
        assert_eq!(
            initiator.official_url(),
            Some("https://www.assemblee-nationale.fr/dyn/deputes/PA1")
        );
    }

    #[test]
    fn a_later_deposit_gets_the_later_group() {
        let mut dossier = dossier_deposited_on(Some(NaiveDate::from_ymd_opt(2025, 9, 1).unwrap()));
        let directory = directory_with_group_change();
        let refs = vec![ActorUid::new("PA1".into()).unwrap()];

        dossier.attach_initiators(&refs, &directory);

        assert_eq!(dossier.initiators[0].group().unwrap().abbrev, "B");
    }

    #[test]
    fn unknown_actor_keeps_its_raw_reference_and_no_group() {
        let mut dossier = dossier_deposited_on(Some(NaiveDate::from_ymd_opt(2025, 5, 13).unwrap()));
        let directory = directory_with_group_change();
        let refs = vec![ActorUid::new("PA_UNKNOWN".into()).unwrap()];

        let report = dossier.attach_initiators(&refs, &directory);

        assert_eq!(report.unknown_actors, 1);
        assert_eq!(dossier.initiators[0].full_name(), "PA_UNKNOWN");
        assert!(dossier.initiators[0].group().is_none());
    }

    #[test]
    fn without_deposit_date_the_actor_is_named_but_carries_no_group() {
        let mut dossier = dossier_deposited_on(None);
        let directory = directory_with_group_change();
        let refs = vec![ActorUid::new("PA1".into()).unwrap()];

        let report = dossier.attach_initiators(&refs, &directory);

        assert!(report.missing_deposit_date);
        let initiator = &dossier.initiators[0];
        assert_eq!(initiator.full_name(), "Jean Dupont");
        assert!(initiator.group().is_none());
        assert!(initiator.reference_date().is_none());
    }

    #[test]
    fn actor_outside_every_membership_period_is_reported_without_group() {
        let mut dossier = dossier_deposited_on(Some(NaiveDate::from_ymd_opt(2024, 7, 1).unwrap()));
        let directory = directory_with_group_change();
        let refs = vec![ActorUid::new("PA1".into()).unwrap()];

        let report = dossier.attach_initiators(&refs, &directory);

        assert_eq!(report.without_group, 1);
        assert!(dossier.initiators[0].group().is_none());
        assert_eq!(dossier.initiators[0].full_name(), "Jean Dupont");
    }

    #[test]
    fn attaching_twice_replaces_instead_of_accumulating() {
        let mut dossier = dossier_deposited_on(Some(NaiveDate::from_ymd_opt(2025, 5, 13).unwrap()));
        let directory = directory_with_group_change();
        let refs = vec![ActorUid::new("PA1".into()).unwrap()];

        dossier.attach_initiators(&refs, &directory);
        dossier.attach_initiators(&refs, &directory);

        assert_eq!(dossier.initiators.len(), 1);
    }

    #[test]
    fn dossier_uid_rejects_empty() {
        assert_eq!(
            DossierUid::new("".into()).unwrap_err(),
            DossierError::EmptyDossierUid
        );
    }

    #[test]
    fn dossier_uid_rejects_blank() {
        assert_eq!(
            DossierUid::new(" \t".into()).unwrap_err(),
            DossierError::EmptyDossierUid
        );
    }

    #[test]
    fn dossier_uid_accepts_valid() {
        let uid = DossierUid::new("DLR5L17N12345".into()).unwrap();
        assert_eq!(uid.as_str(), "DLR5L17N12345");
    }

    #[test]
    fn score_rejects_out_of_range() {
        assert_eq!(
            Score::new(11, 5, 5, 50).unwrap_err(),
            DossierError::InvalidProgress(11)
        );
        assert_eq!(
            Score::new(5, 11, 5, 50).unwrap_err(),
            DossierError::InvalidMagnitude(11)
        );
        assert_eq!(
            Score::new(5, 5, 11, 50).unwrap_err(),
            DossierError::InvalidMomentum(11)
        );
        assert_eq!(
            Score::new(5, 5, 5, 101).unwrap_err(),
            DossierError::InvalidTotal(101)
        );
    }

    #[test]
    fn score_accepts_valid() {
        let s = Score::new(10, 10, 10, 100).unwrap();
        assert_eq!(s.progress(), 10);
        assert_eq!(s.magnitude(), 10);
        assert_eq!(s.momentum(), 10);
        assert_eq!(s.total(), 100);
    }

    #[test]
    fn score_accepts_zero() {
        let s = Score::new(0, 0, 0, 0).unwrap();
        assert_eq!(s.total(), 0);
    }

    #[test]
    fn initiator_rejects_empty_name() {
        assert_eq!(
            Initiator::unresolved("".into()).unwrap_err(),
            DossierError::EmptyInitiatorName
        );
    }

    #[test]
    fn initiator_rejects_blank_name() {
        assert_eq!(
            Initiator::unresolved(" \t".into()).unwrap_err(),
            DossierError::EmptyInitiatorName
        );
    }

    #[test]
    fn resolved_initiator_rejects_blank_name() {
        assert_eq!(
            Initiator::resolved(
                " \t".into(),
                ActorUid::new("PA1".into()).unwrap(),
                ActorRole::Deputy,
                None,
                None,
                None,
            )
            .unwrap_err(),
            DossierError::EmptyInitiatorName
        );
    }

    #[test]
    fn unresolved_initiator_keeps_the_raw_name_without_group() {
        let i = Initiator::unresolved("PA999999".into()).unwrap();
        assert_eq!(i.full_name(), "PA999999");
        assert!(i.group().is_none());
        assert!(i.reference_date().is_none());
        assert!(i.official_url().is_none());
    }

    #[test]
    fn resolved_initiator_carries_group_and_reference_date() {
        let group = ParliamentaryGroup::new(
            GroupUid::new("PO845414".into()).unwrap(),
            17,
            "Ensemble pour la R\u{00e9}publique".into(),
            "EPR".into(),
            None,
            None,
            None,
        )
        .unwrap();
        let quality = MembershipQuality::new("Membre".into()).unwrap();
        let reference = NaiveDate::from_ymd_opt(2025, 5, 13).unwrap();

        let i = Initiator::resolved(
            "Jean Dupont".into(),
            ActorUid::new("PA1".into()).unwrap(),
            ActorRole::Deputy,
            Some(InitiatorGroup::from_group(&group, Some(quality))),
            Some(reference),
            Some("https://www.assemblee-nationale.fr/dyn/deputes/PA1".into()),
        )
        .unwrap();

        assert_eq!(i.group().unwrap().abbrev, "EPR");
        assert_eq!(i.reference_date(), Some(reference));
        assert_eq!(i.role(), Some(ActorRole::Deputy));
    }

    #[test]
    fn resolved_initiator_rejects_group_without_reference_date() {
        let group = ParliamentaryGroup::new(
            GroupUid::new("PO845414".into()).unwrap(),
            17,
            "Ensemble pour la R\u{00e9}publique".into(),
            "EPR".into(),
            None,
            None,
            None,
        )
        .unwrap();

        let result = Initiator::resolved(
            "Jean Dupont".into(),
            ActorUid::new("PA1".into()).unwrap(),
            ActorRole::Deputy,
            Some(InitiatorGroup::from_group(&group, None)),
            None,
            None,
        );

        assert!(matches!(
            result,
            Err(DossierError::GroupWithoutReferenceDate)
        ));
    }

    #[test]
    fn minister_initiator_is_resolved_without_group() {
        let i = Initiator::resolved(
            "Jean Dupont".into(),
            ActorUid::new("PA2".into()).unwrap(),
            ActorRole::Minister,
            None,
            Some(NaiveDate::from_ymd_opt(2025, 5, 13).unwrap()),
            None,
        )
        .unwrap();

        assert!(i.group().is_none());
        assert_eq!(i.role(), Some(ActorRole::Minister));
    }

    #[test]
    fn committee_rejects_empty() {
        assert_eq!(
            Committee::new("".into()).unwrap_err(),
            DossierError::EmptyCommittee
        );
    }

    #[test]
    fn committee_rejects_blank() {
        assert_eq!(
            Committee::new(" \t".into()).unwrap_err(),
            DossierError::EmptyCommittee
        );
    }

    #[test]
    fn committee_accepts_valid() {
        let c = Committee::new("Finances".into()).unwrap();
        assert_eq!(c.as_str(), "Finances");
    }

    fn outcome_date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 4, 21).unwrap()
    }

    #[test]
    fn promulgation_withdrawal_and_merge_are_final() {
        let promulgated = DossierOutcome::Promulgated {
            date: outcome_date(),
            publication: LawPublication {
                law_code: Some("2026-300".into()),
                jo_date: None,
                legifrance_url: None,
            },
        };
        let withdrawn = DossierOutcome::Withdrawn {
            date: outcome_date(),
        };
        let merged = DossierOutcome::MergedInto {
            dossier_uid: DossierUid::new("DLR5L17N54344".into()).unwrap(),
            cause: Some("Dossier absorb\u{00e9}".into()),
        };

        assert!(promulgated.is_final());
        assert!(withdrawn.is_final());
        assert!(merged.is_final());
    }

    /// Un rejet en lecture ne clot pas la navette: le dossier doit continuer a
    /// etre relu a chaque rafraichissement.
    #[test]
    fn a_rejection_is_not_final() {
        let rejected = DossierOutcome::Rejected {
            date: outcome_date(),
            label: "rejet\u{00e9}".into(),
        };

        assert!(!rejected.is_final());
    }

    #[test]
    fn an_absent_conclusion_is_not_final() {
        assert!(!DossierOutcome::NoRecordedConclusion.is_final());
    }

    #[test]
    fn a_stored_kind_is_final_exactly_when_its_outcome_is() {
        let outcomes = [
            DossierOutcome::Promulgated {
                date: outcome_date(),
                publication: LawPublication {
                    law_code: None,
                    jo_date: None,
                    legifrance_url: None,
                },
            },
            DossierOutcome::Withdrawn {
                date: outcome_date(),
            },
            DossierOutcome::MergedInto {
                dossier_uid: DossierUid::new("DLR5L17N54344".into()).unwrap(),
                cause: None,
            },
            DossierOutcome::Rejected {
                date: outcome_date(),
                label: "rejet\u{00e9}".into(),
            },
            DossierOutcome::NoRecordedConclusion,
        ];

        for outcome in outcomes {
            assert_eq!(
                DossierOutcome::kind_is_final(outcome.kind()),
                outcome.is_final(),
                "{}",
                outcome.kind()
            );
        }
    }

    #[test]
    fn an_unknown_stored_kind_is_not_final() {
        assert!(!DossierOutcome::kind_is_final("unknown"));
    }

    #[test]
    fn only_dated_outcomes_carry_a_date() {
        assert_eq!(
            DossierOutcome::Withdrawn {
                date: outcome_date()
            }
            .date(),
            Some(outcome_date())
        );
        assert_eq!(DossierOutcome::NoRecordedConclusion.date(), None);
    }

    #[test]
    fn curation_status_roundtrip() {
        for status in [
            CurationStatus::New,
            CurationStatus::Selected,
            CurationStatus::Dismissed,
            CurationStatus::Published,
        ] {
            assert_eq!(CurationStatus::parse(status.as_str()), Some(status));
        }
    }

    #[test]
    fn curation_status_parse_invalid() {
        assert_eq!(CurationStatus::parse("invalid"), None);
    }

    #[test]
    fn initiative_roundtrip() {
        for initiative in [Initiative::Government, Initiative::Parliamentary] {
            assert_eq!(Initiative::parse(initiative.as_str()), Some(initiative));
        }
        assert_eq!(Initiative::parse("invalid"), None);
    }

    #[test]
    fn initiative_read_from_the_procedure_label() {
        assert_eq!(
            Initiative::from_procedure("Projet de loi ordinaire"),
            Some(Initiative::Government)
        );
        assert_eq!(
            Initiative::from_procedure("Proposition de loi organique"),
            Some(Initiative::Parliamentary)
        );
        assert_eq!(
            Initiative::from_procedure("Proposition de r\u{00e9}solution"),
            Some(Initiative::Parliamentary)
        );
    }

    /// Un libelle que la source n'a pas prefixe ainsi n'est pas range d'office.
    #[test]
    fn unreadable_procedure_label_carries_no_initiative() {
        assert_eq!(Initiative::from_procedure("Rapport d'information"), None);
        assert_eq!(Initiative::from_procedure(""), None);
    }

    #[test]
    fn stage_from_known_codes() {
        assert_eq!(
            LegislativeStage::from_code("AN1"),
            Some(LegislativeStage::FirstReadingAN)
        );
        assert_eq!(
            LegislativeStage::from_code("CMP"),
            Some(LegislativeStage::JointCommittee)
        );
        assert_eq!(
            LegislativeStage::from_code("PROM"),
            Some(LegislativeStage::Promulgation)
        );
    }

    #[test]
    fn stage_round_trips_through_its_code() {
        for stage in [
            LegislativeStage::FirstReadingAN,
            LegislativeStage::FirstReadingSenate,
            LegislativeStage::SecondReadingAN,
            LegislativeStage::SecondReadingSenate,
            LegislativeStage::JointCommittee,
            LegislativeStage::NewReadingAN,
            LegislativeStage::NewReadingSenate,
            LegislativeStage::FinalReading,
            LegislativeStage::ConstitutionalReview,
            LegislativeStage::Promulgation,
        ] {
            assert_eq!(LegislativeStage::from_code(stage.to_code()), Some(stage));
        }
    }

    #[test]
    fn stage_from_unknown_code_returns_none() {
        assert_eq!(LegislativeStage::from_code("UNKNOWN"), None);
        assert_eq!(LegislativeStage::from_code("AN1-DEPOT"), None);
    }

    #[test]
    fn stages_are_ordered_by_progression() {
        assert!(LegislativeStage::FirstReadingAN < LegislativeStage::FirstReadingSenate);
        assert!(LegislativeStage::JointCommittee < LegislativeStage::Promulgation);
    }

    #[test]
    fn each_stage_maps_to_the_chamber_that_holds_it() {
        assert_eq!(
            LegislativeStage::FirstReadingAN.chamber(),
            Chamber::AssembleeNationale
        );
        assert_eq!(
            LegislativeStage::FirstReadingSenate.chamber(),
            Chamber::Senat
        );
        assert_eq!(LegislativeStage::JointCommittee.chamber(), Chamber::Joint);
        assert_eq!(LegislativeStage::Promulgation.chamber(), Chamber::None);
    }

    #[test]
    fn government_bill_without_initiator_reference_defaults_to_the_government() {
        let initiators = Initiator::default_for(Some(Initiative::Government), false);

        assert_eq!(initiators.len(), 1);
        assert_eq!(initiators[0].full_name(), GOVERNMENT_INITIATOR_NAME);
        assert_eq!(initiators[0].full_name(), "Gouvernement");
        assert!(initiators[0].actor_uid().is_none());
        assert!(initiators[0].group().is_none());
    }

    #[test]
    fn government_bill_with_initiator_references_gets_no_default_initiator() {
        let initiators = Initiator::default_for(Some(Initiative::Government), true);

        assert!(initiators.is_empty());
    }

    #[test]
    fn parliamentary_bill_without_initiator_reference_gets_no_default_initiator() {
        let initiators = Initiator::default_for(Some(Initiative::Parliamentary), false);

        assert!(initiators.is_empty());
    }

    #[test]
    fn procedure_without_known_initiative_gets_no_default_initiator() {
        let initiators = Initiator::default_for(None, false);

        assert!(initiators.is_empty());
    }

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn act_on(date: NaiveDate) -> LegislativeAct {
        LegislativeAct {
            date,
            label: "Acte".into(),
            code: None,
        }
    }

    /// Dossier de la source: derniere activite le 13 mai 2025, deux actes.
    fn source_dossier_with_two_acts() -> LegislativeDossier {
        let mut dossier = dossier_deposited_on(None);
        dossier.acts = vec![act_on(date(2025, 4, 1)), act_on(date(2025, 5, 13))];
        dossier
    }

    fn stored_state(
        last_activity_date: NaiveDate,
        act_count: usize,
        outcome_is_final: bool,
    ) -> StoredDossierState {
        StoredDossierState {
            last_activity_date,
            act_count,
            outcome_is_final,
        }
    }

    #[test]
    fn dossier_absent_from_the_database_is_written() {
        let dossier = source_dossier_with_two_acts();

        assert_eq!(dossier.write_decision(None), WriteDecision::Write);
    }

    #[test]
    fn dossier_with_a_final_outcome_is_skipped_even_when_the_source_moved() {
        let dossier = source_dossier_with_two_acts();
        let stored = stored_state(date(2024, 1, 1), 1, true);

        assert_eq!(
            dossier.write_decision(Some(&stored)),
            WriteDecision::SkipFinal
        );
    }

    #[test]
    fn dossier_with_a_final_outcome_and_unchanged_acts_is_counted_as_final_not_unchanged() {
        let dossier = source_dossier_with_two_acts();
        let stored = stored_state(date(2025, 5, 13), 2, true);

        assert_eq!(
            dossier.write_decision(Some(&stored)),
            WriteDecision::SkipFinal
        );
    }

    #[test]
    fn dossier_with_same_last_activity_and_act_count_is_skipped_as_unchanged() {
        let dossier = source_dossier_with_two_acts();
        let stored = stored_state(date(2025, 5, 13), 2, false);

        assert_eq!(
            dossier.write_decision(Some(&stored)),
            WriteDecision::SkipUnchanged
        );
    }

    #[test]
    fn dossier_with_an_inserted_act_on_the_same_date_is_written() {
        let dossier = source_dossier_with_two_acts();
        let stored = stored_state(date(2025, 5, 13), 1, false);

        assert_eq!(dossier.write_decision(Some(&stored)), WriteDecision::Write);
    }

    #[test]
    fn dossier_with_a_different_last_activity_date_is_written() {
        let dossier = source_dossier_with_two_acts();
        let stored = stored_state(date(2025, 5, 12), 2, false);

        assert_eq!(dossier.write_decision(Some(&stored)), WriteDecision::Write);
    }

    #[test]
    fn dormant_dossier_is_judged_on_its_acts_not_on_its_age() {
        let mut dossier = source_dossier_with_two_acts();
        dossier.last_activity_date = date(2022, 1, 1);
        let unchanged = stored_state(date(2022, 1, 1), 2, false);
        let revived = stored_state(date(2022, 1, 1), 1, false);

        assert_eq!(
            dossier.write_decision(Some(&unchanged)),
            WriteDecision::SkipUnchanged
        );
        assert_eq!(dossier.write_decision(Some(&revived)), WriteDecision::Write);
    }

    #[test]
    fn period_starts_on_the_deposit_date_when_it_is_the_only_known_date() {
        let period =
            DossierPeriod::from_dates(Some(date(2024, 3, 1)), None, None, date(2025, 5, 13));

        assert_eq!(period.start(), date(2024, 3, 1));
        assert_eq!(period.end(), date(2025, 5, 13));
    }

    #[test]
    fn period_starts_on_the_first_document_when_it_precedes_the_deposit() {
        let period = DossierPeriod::from_dates(
            Some(date(2024, 3, 1)),
            Some(date(2024, 2, 10)),
            Some(date(2024, 4, 1)),
            date(2025, 5, 13),
        );

        assert_eq!(period.start(), date(2024, 2, 10));
    }

    #[test]
    fn period_starts_on_the_oldest_act_when_it_precedes_every_other_date() {
        let period = DossierPeriod::from_dates(
            Some(date(2024, 3, 1)),
            Some(date(2024, 2, 10)),
            Some(date(2024, 1, 5)),
            date(2025, 5, 13),
        );

        assert_eq!(period.start(), date(2024, 1, 5));
    }

    #[test]
    fn period_without_any_known_date_collapses_onto_the_last_activity() {
        let period = DossierPeriod::from_dates(None, None, None, date(2025, 5, 13));

        assert_eq!(period.start(), date(2025, 5, 13));
        assert_eq!(period.end(), date(2025, 5, 13));
    }

    #[test]
    fn period_ignores_a_missing_deposit_date_instead_of_guessing_one() {
        let period = DossierPeriod::from_dates(
            None,
            Some(date(2024, 6, 1)),
            Some(date(2024, 7, 1)),
            date(2025, 5, 13),
        );

        assert_eq!(period.start(), date(2024, 6, 1));
    }

    #[test]
    fn new_dossier_exposes_every_given_part_through_its_getters() {
        let document = LegislativeDocument {
            document_uid: "PIONANR5L17B0001".into(),
            title: "Proposition de loi".into(),
            short_title: None,
            doc_type: "Proposition de loi".into(),
            date: Some(date(2025, 5, 13)),
            official_url: None,
            source_archive_url: None,
            source_license: None,
            source_metadata_fingerprint: None,
            source_retrieved_at: None,
        };
        let initiator = Initiator::unresolved("Jean Dupont".into()).unwrap();

        let dossier = LegislativeDossier::new(NewLegislativeDossier {
            uid: DossierUid::new("DLR5L17N1".into()).unwrap(),
            title: "Loi test".into(),
            procedure: "Proposition de loi".into(),
            legislature: 17,
            url: Some("https://example.test/dossier".into()),
            summary: Some("Resume".into()),
            deposit_date: Some(date(2025, 5, 13)),
            last_activity_date: date(2025, 6, 2),
            last_activity_label: "Reunion".into(),
            acts: vec![act_on(date(2025, 5, 13)), act_on(date(2025, 6, 2))],
            documents: vec![document],
            score: Score::new(2, 4, 2, 23).unwrap(),
            current_stage: Some(LegislativeStage::FirstReadingAN),
            initiators: vec![initiator],
            committee: Some(Committee::new("Finances".into()).unwrap()),
            curation_status: CurationStatus::Selected,
            outcome: DossierOutcome::Withdrawn {
                date: date(2025, 7, 1),
            },
        });

        assert_eq!(dossier.uid().as_str(), "DLR5L17N1");
        assert_eq!(dossier.title(), "Loi test");
        assert_eq!(dossier.procedure(), "Proposition de loi");
        assert_eq!(dossier.legislature(), 17);
        assert_eq!(dossier.url(), Some("https://example.test/dossier"));
        assert_eq!(dossier.summary(), Some("Resume"));
        assert_eq!(dossier.deposit_date(), Some(date(2025, 5, 13)));
        assert_eq!(dossier.last_activity_date(), date(2025, 6, 2));
        assert_eq!(dossier.last_activity_label(), "Reunion");
        assert_eq!(dossier.acts().len(), 2);
        assert_eq!(dossier.documents()[0].document_uid, "PIONANR5L17B0001");
        assert_eq!(dossier.score().total(), 23);
        assert_eq!(
            dossier.current_stage(),
            Some(LegislativeStage::FirstReadingAN)
        );
        assert_eq!(dossier.initiators()[0].full_name(), "Jean Dupont");
        assert_eq!(dossier.committee().map(Committee::as_str), Some("Finances"));
        assert_eq!(dossier.curation_status(), CurationStatus::Selected);
        assert_eq!(
            *dossier.outcome(),
            DossierOutcome::Withdrawn {
                date: date(2025, 7, 1)
            }
        );
    }

    #[test]
    fn changing_the_curation_status_replaces_it_and_leaves_the_rest_untouched() {
        let mut dossier = dossier_deposited_on(Some(date(2025, 5, 13)));

        dossier.change_curation_status(CurationStatus::Dismissed);

        assert_eq!(dossier.curation_status(), CurationStatus::Dismissed);
        assert_eq!(dossier.uid().as_str(), "D1");
        assert_eq!(dossier.deposit_date(), Some(date(2025, 5, 13)));
    }
}
