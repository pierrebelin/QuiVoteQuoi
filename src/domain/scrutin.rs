//! Scrutins publics de l'Assemblee nationale.
//!
//! Voir `todo/SPEC-scrutins.md`. Deux invariants portent tout le reste:
//! - RM-02: les chiffres publies ne sont jamais recalcules;
//! - RM-04: le groupe d'une position nominale est celui sous lequel la source
//!   range le votant dans ce scrutin, pas son appartenance courante.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::Serialize;

use super::actor::{ActorDirectory, ActorUid, GroupUid};
use super::amendment::amendment_numbers_in_subject;
use super::article::ArticleNumber;
use super::dossier::DossierUid;

/// Libelle court d'un scrutin, calcule depuis son objet par quatre formes
/// fixes (RM-09); l'objet tel quel quand aucune ne le reconnait.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShortLabel(String);

impl ShortLabel {
    pub fn from_subject(subject: &str) -> Self {
        let normalized = subject.trim().replace('\u{2019}', "'");
        let lower = normalized.to_lowercase();
        let label = if lower.starts_with("l'amendement") {
            Self::amendment(&normalized["l'amendement".len()..])
        } else if lower.starts_with("l'ensemble") {
            Some("Vote sur l'ensemble du texte".to_string())
        } else if lower.starts_with("l'article") {
            ArticleNumber::in_text(&normalized).map(|n| format!("Vote sur l'article {n}"))
        } else if lower.starts_with("la motion ") {
            Self::motion(&normalized["la motion ".len()..])
        } else {
            None
        };
        Self(label.unwrap_or_else(|| subject.to_string()))
    }

    /// « n° 1376 de Mme Thomin apres l'article 15 ... » -> forme courte.
    fn amendment(rest: &str) -> Option<String> {
        let rest = rest.trim().strip_prefix("n\u{00b0}")?.trim_start();
        let (number, rest) = rest.split_once(' ')?;
        let rest = rest.strip_prefix("de ")?;
        let end = [" apr\u{00e8}s ", " avant ", " \u{00e0} l'"]
            .iter()
            .filter_map(|m| rest.find(m))
            .min()
            .unwrap_or(rest.len());
        let author = rest[..end].trim();
        let mut label = format!("Amendement n\u{00b0} {number}");
        if !author.is_empty() {
            label.push_str(&format!(" de {author}"));
        }
        if let Some(article) = ArticleNumber::in_text(&rest[end..]) {
            label.push_str(&format!(", article {article}"));
        }
        Some(label)
    }

    /// « de rejet prealable, deposee par ... » -> « Motion de rejet prealable ».
    fn motion(rest: &str) -> Option<String> {
        let kind = rest.split(',').next()?.trim().trim_end_matches('.');
        let kind = kind.split(" d\u{00e9}pos\u{00e9}").next()?.trim();
        kind.strip_prefix("de ")
            .filter(|k| !k.is_empty())
            .map(|_| format!("Motion {kind}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Ce que l'objet d'un scrutin annonce sans le nommer (SPEC-sujet-amendements).
pub struct SubjectMentions;

impl SubjectMentions {
    /// Vrai quand l'objet annonce des amendements identiques sans en donner un
    /// seul numero.
    pub fn unnumbered_identical_amendments(subject: &str) -> bool {
        subject.to_lowercase().contains("amendements identiques")
            && amendment_numbers_in_subject(subject).is_empty()
    }
}

/// Reference de groupe factice publiee par la source quand la ventilation par
/// groupe est perdue. La repartition se reconstruit alors depuis les positions
/// nominales (RM-03).
pub const MISSING_GROUP_SENTINEL: &str = "PO0";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ScrutinError {
    #[error("scrutin uid must not be empty")]
    EmptyScrutinUid,
    #[error("scrutin number must not be empty")]
    EmptyScrutinNumber,
    #[error("ballot type code must not be empty")]
    EmptyBallotTypeCode,
    #[error("outcome code must not be empty")]
    EmptyOutcomeCode,
    #[error("scrutin subject must not be empty")]
    EmptySubject,
    #[error("unknown vote position: {0}")]
    UnknownVotePosition(String),
    #[error("non-voting cause must not be empty")]
    EmptyNonVotingCause,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct ScrutinUid(String);

impl ScrutinUid {
    pub fn new(raw: String) -> Result<Self, ScrutinError> {
        if raw.trim().is_empty() {
            return Err(ScrutinError::EmptyScrutinUid);
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ScrutinUid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Position de vote. Les quatre seules valeurs publiees par la source.
///
/// Les « non-votants volontaires » n'en font pas partie: ils sont comptes par
/// groupe sans etre nommes (H10), ils vivent donc dans `VoteTally` seulement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VotePosition {
    For,
    Against,
    Abstention,
    NotVoting,
}

impl VotePosition {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::For => "for",
            Self::Against => "against",
            Self::Abstention => "abstention",
            Self::NotVoting => "not_voting",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "for" => Some(Self::For),
            "against" => Some(Self::Against),
            "abstention" => Some(Self::Abstention),
            "not_voting" => Some(Self::NotVoting),
            _ => None,
        }
    }

    /// Position majoritaire telle que la source la nomme (« pour », « contre »,
    /// « abstention »). RM-09: aucun autre libelle n'est accepte en silence.
    pub fn from_source(raw: &str) -> Result<Self, ScrutinError> {
        match raw.trim() {
            "pour" => Ok(Self::For),
            "contre" => Ok(Self::Against),
            "abstention" => Ok(Self::Abstention),
            "nonVotant" | "non-votant" => Ok(Self::NotVoting),
            other => Err(ScrutinError::UnknownVotePosition(other.to_string())),
        }
    }
}

/// Cause publiee d'un non-vote, conservee sous forme de code.
///
/// La source ne publie pas de libelle: le traduire serait produire un fait
/// (README.md §6). Le code est affiche tel quel tant que le libelle officiel
/// n'est pas source (SPEC-scrutins Q2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NonVotingCause(String);

impl NonVotingCause {
    pub fn new(raw: String) -> Result<Self, ScrutinError> {
        if raw.trim().is_empty() {
            return Err(ScrutinError::EmptyNonVotingCause);
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Type de vote publie: scrutin public ordinaire, solennel, motion de censure.
///
/// RM-09: le libelle affiche est celui de la source, le code sert aux filtres.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BallotType {
    code: String,
    label: String,
    /// Majorite requise, telle qu'annoncee par la source.
    majority: Option<String>,
}

impl BallotType {
    pub fn new(
        code: String,
        label: String,
        majority: Option<String>,
    ) -> Result<Self, ScrutinError> {
        if code.trim().is_empty() {
            return Err(ScrutinError::EmptyBallotTypeCode);
        }
        Ok(Self {
            code,
            label,
            majority,
        })
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn majority(&self) -> Option<&str> {
        self.majority.as_deref()
    }
}

/// Sort du scrutin: adopte / rejete, avec l'annonce complete de la source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Outcome {
    code: String,
    label: String,
}

impl Outcome {
    pub const ADOPTED: &'static str = "adopt\u{00e9}";

    pub fn new(code: String, label: String) -> Result<Self, ScrutinError> {
        if code.trim().is_empty() {
            return Err(ScrutinError::EmptyOutcomeCode);
        }
        Ok(Self { code, label })
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn is_adopted(&self) -> bool {
        self.code == Self::ADOPTED
    }
}

/// Decompte d'un scrutin ou d'un groupe, tel que publie (RM-02).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct VoteTally {
    pub votes_for: u16,
    pub votes_against: u16,
    pub abstentions: u16,
    pub not_voting: u16,
    /// Comptes par groupe, jamais nommes, et absents de la synthese officielle
    /// qui affiche toujours zero (H10). Affiche avec le libelle de la source.
    pub voluntary_not_voting: u16,
}

impl VoteTally {
    /// Nombre de positions nominales attendues pour ce decompte. Les
    /// non-votants volontaires en sont exclus: la source ne les nomme pas.
    pub fn named_count(&self) -> u32 {
        self.votes_for as u32
            + self.votes_against as u32
            + self.abstentions as u32
            + self.not_voting as u32
    }

    fn add(&mut self, position: VotePosition) {
        match position {
            VotePosition::For => self.votes_for += 1,
            VotePosition::Against => self.votes_against += 1,
            VotePosition::Abstention => self.abstentions += 1,
            VotePosition::NotVoting => self.not_voting += 1,
        }
    }
}

/// Origine d'une repartition par groupe.
///
/// `Reconstructed` declenche la mention de methode exigee par RM-03: le chiffre
/// est produit par le site, pas par l'Assemblee.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TallyOrigin {
    Published,
    Reconstructed,
}

impl TallyOrigin {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Published => "published",
            Self::Reconstructed => "reconstructed",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "published" => Some(Self::Published),
            "reconstructed" => Some(Self::Reconstructed),
            _ => None,
        }
    }
}

/// Une ligne de la ventilation par groupe.
#[derive(Debug, Clone, Serialize)]
pub struct GroupTally {
    pub group_uid: GroupUid,
    /// Effectif publie du groupe au moment du scrutin. Donne la portee du
    /// decompte, ne sert a aucun taux (RM-08).
    pub member_count: Option<u16>,
    /// Position majoritaire publiee. Jamais recalculee (RM-02).
    pub majority_position: Option<VotePosition>,
    pub tally: VoteTally,
    pub origin: TallyOrigin,
}

impl GroupTally {
    pub fn is_sentinel(&self) -> bool {
        self.group_uid.as_str() == MISSING_GROUP_SENTINEL
    }
}

/// Position d'un votant identifie.
#[derive(Debug, Clone, Serialize)]
pub struct NominalVote {
    pub actor_uid: ActorUid,
    /// Groupe sous lequel la source range le votant dans ce scrutin (RM-04).
    /// Absent tant que la sentinelle n'a pas ete resolue, ou si elle ne l'est pas.
    pub group_uid: Option<GroupUid>,
    pub position: VotePosition,
    pub cause: Option<NonVotingCause>,
    pub by_delegation: bool,
    /// Numero de place en seance, publie par la source.
    pub seat: Option<u16>,
}

/// Mise au point: un depute declare apres coup que son vote enregistre ne
/// correspond pas a son intention. RM-05: sans effet sur les decomptes.
#[derive(Debug, Clone, Serialize)]
pub struct VoteCorrection {
    pub actor_uid: ActorUid,
    /// Position revendiquee par le depute.
    pub claimed_position: VotePosition,
    /// Vrai quand la source la classe comme dysfonctionnement du materiel de
    /// vote plutot que comme mise au point ordinaire.
    pub malfunction: bool,
}

/// Synthese officielle du scrutin, publiee telle quelle (RM-02).
#[derive(Debug, Clone, Serialize)]
pub struct VoteSynthesis {
    pub voters: u16,
    pub expressed: u16,
    pub required: u16,
    /// Annonce du president de seance.
    pub announcement: String,
    pub tally: VoteTally,
}

/// Dossier legislatif designe par la source. Absent sur 69 % des scrutins,
/// qui restent exposes (RM-10).
#[derive(Debug, Clone, Serialize)]
pub struct DossierReference {
    pub uid: DossierUid,
    /// Libelle publie dans le scrutin. Conserve pour rester lisible meme quand
    /// le dossier n'est pas ingere.
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Scrutin {
    uid: ScrutinUid,
    number: String,
    legislature: u16,
    date: NaiveDate,
    session_ref: Option<String>,
    sitting_ref: Option<String>,
    place: Option<String>,
    ballot_type: BallotType,
    outcome: Outcome,
    requester: Option<String>,
    subject: String,
    synthesis: VoteSynthesis,
    group_tallies: Vec<GroupTally>,
    nominal_votes: Vec<NominalVote>,
    corrections: Vec<VoteCorrection>,
    dossier: Option<DossierReference>,
}

/// Ce que la reconstruction a change, pour le journal d'ingestion (RM-03).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReconstructionReport {
    /// Lignes factices remplacees.
    pub sentinel_lines: usize,
    /// Lignes de groupe produites par la reconstruction.
    pub rebuilt_lines: usize,
    /// Positions nominales qu'aucun groupe n'a pu accueillir. Toute valeur non
    /// nulle est une anomalie a signaler: le decompte reconstruit est alors
    /// incomplet.
    pub unresolved_votes: usize,
}

impl ReconstructionReport {
    pub fn happened(&self) -> bool {
        self.sentinel_lines > 0
    }
}

/// Page officielle d'un scrutin sur le site de l'Assemblee (RM-07). Une seule
/// forme d'adresse, partagee par le domaine et par les reponses de l'API.
pub fn scrutin_official_url(legislature: u16, number: &str) -> String {
    format!("https://www.assemblee-nationale.fr/dyn/{legislature}/scrutins/{number}")
}

/// Elements d'un scrutin, tels que lus dans la source.
///
/// Une struct plutot que seize parametres positionnels: le champ est nomme au
/// site d'appel, et un oubli se voit a la compilation.
#[derive(Debug, Clone)]
pub struct NewScrutin {
    pub uid: ScrutinUid,
    pub number: String,
    pub legislature: u16,
    pub date: NaiveDate,
    pub session_ref: Option<String>,
    pub sitting_ref: Option<String>,
    pub place: Option<String>,
    pub ballot_type: BallotType,
    pub outcome: Outcome,
    pub requester: Option<String>,
    pub subject: String,
    pub synthesis: VoteSynthesis,
    pub group_tallies: Vec<GroupTally>,
    pub nominal_votes: Vec<NominalVote>,
    pub corrections: Vec<VoteCorrection>,
    pub dossier: Option<DossierReference>,
}

impl Scrutin {
    /// Construit l'agregat et applique les invariants: le numero et l'objet du
    /// scrutin ne sont jamais blancs.
    pub fn new(parts: NewScrutin) -> Result<Self, ScrutinError> {
        if parts.number.trim().is_empty() {
            return Err(ScrutinError::EmptyScrutinNumber);
        }
        if parts.subject.trim().is_empty() {
            return Err(ScrutinError::EmptySubject);
        }
        Ok(Self {
            uid: parts.uid,
            number: parts.number,
            legislature: parts.legislature,
            date: parts.date,
            session_ref: parts.session_ref,
            sitting_ref: parts.sitting_ref,
            place: parts.place,
            ballot_type: parts.ballot_type,
            outcome: parts.outcome,
            requester: parts.requester,
            subject: parts.subject,
            synthesis: parts.synthesis,
            group_tallies: parts.group_tallies,
            nominal_votes: parts.nominal_votes,
            corrections: parts.corrections,
            dossier: parts.dossier,
        })
    }

    pub fn uid(&self) -> &ScrutinUid {
        &self.uid
    }
    pub fn number(&self) -> &str {
        &self.number
    }
    pub fn legislature(&self) -> u16 {
        self.legislature
    }
    pub fn date(&self) -> NaiveDate {
        self.date
    }
    pub fn session_ref(&self) -> Option<&str> {
        self.session_ref.as_deref()
    }
    pub fn sitting_ref(&self) -> Option<&str> {
        self.sitting_ref.as_deref()
    }
    pub fn place(&self) -> Option<&str> {
        self.place.as_deref()
    }
    pub fn ballot_type(&self) -> &BallotType {
        &self.ballot_type
    }
    pub fn outcome(&self) -> &Outcome {
        &self.outcome
    }
    pub fn requester(&self) -> Option<&str> {
        self.requester.as_deref()
    }
    pub fn subject(&self) -> &str {
        &self.subject
    }
    pub fn synthesis(&self) -> &VoteSynthesis {
        &self.synthesis
    }
    pub fn group_tallies(&self) -> &[GroupTally] {
        &self.group_tallies
    }
    pub fn nominal_votes(&self) -> &[NominalVote] {
        &self.nominal_votes
    }
    pub fn corrections(&self) -> &[VoteCorrection] {
        &self.corrections
    }
    pub fn dossier(&self) -> Option<&DossierReference> {
        self.dossier.as_ref()
    }

    /// Page officielle du scrutin (RM-07).
    pub fn official_url(&self) -> String {
        scrutin_official_url(self.legislature, &self.number)
    }

    /// Vrai quand au moins une ligne de groupe a ete reconstruite: declenche la
    /// mention de methode (RM-03).
    pub fn has_reconstructed_tallies(&self) -> bool {
        self.group_tallies
            .iter()
            .any(|t| t.origin == TallyOrigin::Reconstructed)
    }

    /// Vrai tant qu'une ligne factice n'a pas ete resolue.
    pub fn has_sentinel_tallies(&self) -> bool {
        self.group_tallies.iter().any(GroupTally::is_sentinel)
    }

    /// Acteurs dont le groupe reste a resoudre parce qu'ils votent sous la
    /// sentinelle. L'appelant leur cherche leur appartenance a la date du
    /// scrutin, puis appelle `attach_groups`.
    pub fn actors_under_sentinel(&self) -> Vec<ActorUid> {
        let mut uids: Vec<ActorUid> = self
            .nominal_votes
            .iter()
            .filter(|v| {
                v.group_uid
                    .as_ref()
                    .map_or(true, |g| g.as_str() == MISSING_GROUP_SENTINEL)
            })
            .map(|v| v.actor_uid.clone())
            .collect();
        uids.sort();
        uids.dedup();
        uids
    }

    /// Rattache chaque position nominale sans groupe au groupe que le repertoire
    /// donne a son votant a la date du scrutin, puis reconstruit la repartition
    /// (RM-03, RM-04 de la spec ACTEURS).
    ///
    /// Acteur inconnu du repertoire, sans appartenance ou en appartenance
    /// ambigue a cette date: rien n'est devine, la position reste sans groupe et
    /// le rapport la compte comme non resolue.
    ///
    /// Une position qui porte deja un groupe n'est jamais touchee, la sentinelle
    /// `PO0` comprise. Cas limite connu: une position rangee explicitement sous
    /// `PO0` par la source reste sous `PO0` et compte comme non resolue, meme si
    /// le repertoire connait son groupe.
    ///
    /// A n'appeler que sur un scrutin qui porte des lignes factices
    /// (`has_sentinel_tallies`): sans elles il n'y a rien a reconstruire, et les
    /// positions sans groupe n'ont pas a etre rattachees.
    pub fn attach_groups(&mut self, directory: &ActorDirectory) -> ReconstructionReport {
        let date = self.date;
        for vote in &mut self.nominal_votes {
            if vote.group_uid.is_some() {
                continue;
            }
            vote.group_uid = directory
                .resolve_at(&vote.actor_uid, date)
                .and_then(|resolved| resolved.group.map(|group| group.uid().clone()));
        }
        self.reconstruct_sentinel_groups()
    }

    /// Remplace les lignes de groupe factices par une repartition reconstruite
    /// depuis les positions nominales (RM-03).
    ///
    /// Prerequis: les positions concernees portent deja leur vrai groupe, resolu
    /// par l'appelant (`attach_groups`) sur l'appartenance a la date du scrutin. Une position
    /// laissee sans groupe n'est comptee nulle part et remonte dans le rapport:
    /// mieux vaut un decompte visiblement incomplet qu'un chiffre invente.
    pub fn reconstruct_sentinel_groups(&mut self) -> ReconstructionReport {
        let sentinel_lines = self
            .group_tallies
            .iter()
            .filter(|t| t.is_sentinel())
            .count();
        if sentinel_lines == 0 {
            return ReconstructionReport::default();
        }

        self.group_tallies.retain(|t| !t.is_sentinel());
        let published: Vec<GroupUid> = self
            .group_tallies
            .iter()
            .map(|t| t.group_uid.clone())
            .collect();

        let mut rebuilt: BTreeMap<GroupUid, VoteTally> = BTreeMap::new();
        let mut unresolved = 0usize;

        for vote in &self.nominal_votes {
            match vote.group_uid.as_ref() {
                // Un vote encore sous la sentinelle n'a pas de groupe: le
                // compter sous `PO0` fabriquerait un groupe qui n'existe pas.
                Some(group) if group.as_str() == MISSING_GROUP_SENTINEL => unresolved += 1,
                // Les lignes publiees restent telles quelles (RM-02): seules les
                // positions orphelines de la sentinelle alimentent la reconstruction.
                Some(group) if !published.contains(group) => {
                    rebuilt.entry(group.clone()).or_default().add(vote.position);
                }
                Some(_) => {}
                None => unresolved += 1,
            }
        }

        let rebuilt_lines = rebuilt.len();
        for (group_uid, tally) in rebuilt {
            self.group_tallies.push(GroupTally {
                group_uid,
                member_count: None,
                // Rien n'est deduit: la source ne publie pas de position
                // majoritaire pour ces lignes, le site n'en fabrique pas.
                majority_position: None,
                tally,
                origin: TallyOrigin::Reconstructed,
            });
        }

        ReconstructionReport {
            sentinel_lines,
            rebuilt_lines,
            unresolved_votes: unresolved,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::actor::{
        Actor, ActorRole, GroupMembership, MembershipPeriod, MembershipQuality, ParliamentaryGroup,
    };

    fn date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2024, 12, 2).unwrap()
    }

    fn group(uid: &str) -> GroupUid {
        GroupUid::new(uid.into()).unwrap()
    }

    fn actor(uid: &str) -> ActorUid {
        ActorUid::new(uid.into()).unwrap()
    }

    fn vote(actor_uid: &str, group_uid: Option<&str>, position: VotePosition) -> NominalVote {
        NominalVote {
            actor_uid: actor(actor_uid),
            group_uid: group_uid.map(group),
            position,
            cause: None,
            by_delegation: false,
            seat: None,
        }
    }

    fn tally(group_uid: &str, votes_for: u16, votes_against: u16) -> GroupTally {
        GroupTally {
            group_uid: group(group_uid),
            member_count: Some(10),
            majority_position: Some(VotePosition::For),
            tally: VoteTally {
                votes_for,
                votes_against,
                ..VoteTally::default()
            },
            origin: TallyOrigin::Published,
        }
    }

    fn scrutin(group_tallies: Vec<GroupTally>, nominal_votes: Vec<NominalVote>) -> Scrutin {
        Scrutin::new(NewScrutin {
            uid: ScrutinUid::new("VTANR5L17V489".into()).unwrap(),
            number: "489".into(),
            legislature: 17,
            date: date(),
            session_ref: Some("SCR5A2025O1".into()),
            sitting_ref: None,
            place: Some("H\u{00e9}micycle".into()),
            ballot_type: BallotType::new(
                "SPO".into(),
                "scrutin public ordinaire".into(),
                Some("Majorit\u{00e9} absolue des suffrages exprim\u{00e9}s".into()),
            )
            .unwrap(),
            outcome: Outcome::new(
                "rejet\u{00e9}".into(),
                "L'Assembl\u{00e9}e nationale n'a pas adopt\u{00e9}".into(),
            )
            .unwrap(),
            requester: None,
            subject: "l'amendement n\u{00b0} 1".into(),
            synthesis: VoteSynthesis {
                voters: 2,
                expressed: 2,
                required: 2,
                announcement: "L'Assembl\u{00e9}e nationale n'a pas adopt\u{00e9}".into(),
                tally: VoteTally::default(),
            },
            group_tallies,
            nominal_votes,
            corrections: vec![],
            dossier: None,
        })
        .unwrap()
    }

    #[test]
    fn should_detect_unnumbered_identical_amendments_when_subject_announces_them() {
        assert!(SubjectMentions::unnumbered_identical_amendments(
            "les amendements identiques de M. Martin, Mme Durand et plusieurs de leurs collègues après l'article 3 du projet de loi"
        ));
        assert!(!SubjectMentions::unnumbered_identical_amendments(
            "les amendements identiques n° 12 et 45 de M. Martin après l'article 3 du projet de loi"
        ));
        assert!(!SubjectMentions::unnumbered_identical_amendments(
            "l'amendement n° 12 de M. Martin après l'article 3 du projet de loi"
        ));
    }

    #[test]
    fn scrutin_uid_rejects_empty() {
        assert_eq!(
            ScrutinUid::new("  ".into()).unwrap_err(),
            ScrutinError::EmptyScrutinUid
        );
    }

    #[test]
    fn scrutin_rejects_empty_number_or_subject() {
        let base = scrutin(vec![], vec![]);
        assert!(base.number() == "489");

        let err = Scrutin::new(NewScrutin {
            uid: ScrutinUid::new("V1".into()).unwrap(),
            number: "  ".into(),
            legislature: 17,
            date: date(),
            session_ref: None,
            sitting_ref: None,
            place: None,
            ballot_type: BallotType::new("SPO".into(), "ordinaire".into(), None).unwrap(),
            outcome: Outcome::new("adopt\u{00e9}".into(), "adopt\u{00e9}".into()).unwrap(),
            requester: None,
            subject: "objet".into(),
            synthesis: VoteSynthesis {
                voters: 0,
                expressed: 0,
                required: 0,
                announcement: String::new(),
                tally: VoteTally::default(),
            },
            group_tallies: vec![],
            nominal_votes: vec![],
            corrections: vec![],
            dossier: None,
        });
        assert_eq!(err.unwrap_err(), ScrutinError::EmptyScrutinNumber);
    }

    #[test]
    fn vote_position_round_trips_through_its_string() {
        for position in [
            VotePosition::For,
            VotePosition::Against,
            VotePosition::Abstention,
            VotePosition::NotVoting,
        ] {
            assert_eq!(VotePosition::parse(position.as_str()), Some(position));
        }
    }

    #[test]
    fn tally_origin_round_trips_through_its_string() {
        for origin in [TallyOrigin::Published, TallyOrigin::Reconstructed] {
            assert_eq!(TallyOrigin::parse(origin.as_str()), Some(origin));
        }
    }

    #[test]
    fn position_reads_the_source_labels_and_refuses_the_rest() {
        assert_eq!(
            VotePosition::from_source("pour").unwrap(),
            VotePosition::For
        );
        assert_eq!(
            VotePosition::from_source("contre").unwrap(),
            VotePosition::Against
        );
        assert_eq!(
            VotePosition::from_source("abstention").unwrap(),
            VotePosition::Abstention
        );
        assert_eq!(
            VotePosition::from_source("favorable").unwrap_err(),
            ScrutinError::UnknownVotePosition("favorable".into())
        );
    }

    #[test]
    fn official_url_points_to_the_scrutin_page() {
        let s = scrutin(vec![], vec![]);
        assert_eq!(
            s.official_url(),
            "https://www.assemblee-nationale.fr/dyn/17/scrutins/489"
        );
    }

    #[test]
    fn scrutin_official_url_is_built_from_the_legislature_and_the_number() {
        assert_eq!(
            scrutin_official_url(17, "489"),
            "https://www.assemblee-nationale.fr/dyn/17/scrutins/489"
        );
        assert_eq!(
            scrutin_official_url(16, "1"),
            "https://www.assemblee-nationale.fr/dyn/16/scrutins/1"
        );
    }

    #[test]
    fn voluntary_non_voters_are_not_expected_among_named_positions() {
        let tally = VoteTally {
            votes_for: 10,
            votes_against: 5,
            abstentions: 2,
            not_voting: 1,
            voluntary_not_voting: 7,
        };
        assert_eq!(tally.named_count(), 18);
    }

    #[test]
    fn no_sentinel_leaves_published_tallies_untouched() {
        let mut s = scrutin(
            vec![tally("PO845401", 3, 0)],
            vec![vote("PA1", Some("PO845401"), VotePosition::For)],
        );

        let report = s.reconstruct_sentinel_groups();

        assert!(!report.happened());
        assert_eq!(s.group_tallies().len(), 1);
        assert_eq!(s.group_tallies()[0].origin, TallyOrigin::Published);
        assert_eq!(s.group_tallies()[0].tally.votes_for, 3);
    }

    #[test]
    fn sentinel_line_is_replaced_by_a_reconstructed_breakdown() {
        let mut s = scrutin(
            vec![GroupTally {
                group_uid: group(MISSING_GROUP_SENTINEL),
                member_count: Some(122),
                majority_position: Some(VotePosition::Against),
                tally: VoteTally::default(),
                origin: TallyOrigin::Published,
            }],
            vec![
                vote("PA1", Some("PO845401"), VotePosition::For),
                vote("PA2", Some("PO845401"), VotePosition::For),
                vote("PA3", Some("PO845407"), VotePosition::Against),
                vote("PA4", Some("PO845407"), VotePosition::NotVoting),
            ],
        );

        let report = s.reconstruct_sentinel_groups();

        assert_eq!(report.sentinel_lines, 1);
        assert_eq!(report.rebuilt_lines, 2);
        assert_eq!(report.unresolved_votes, 0);
        assert!(!s.has_sentinel_tallies());
        assert!(s.has_reconstructed_tallies());

        let first = &s.group_tallies()[0];
        assert_eq!(first.group_uid.as_str(), "PO845401");
        assert_eq!(first.tally.votes_for, 2);
        assert_eq!(first.origin, TallyOrigin::Reconstructed);

        let second = &s.group_tallies()[1];
        assert_eq!(second.group_uid.as_str(), "PO845407");
        assert_eq!(second.tally.votes_against, 1);
        assert_eq!(second.tally.not_voting, 1);
        // RM-02: rien n'est deduit pour une ligne reconstruite.
        assert!(second.majority_position.is_none());
        assert!(second.member_count.is_none());
    }

    #[test]
    fn published_lines_survive_a_partial_sentinel() {
        let mut s = scrutin(
            vec![
                tally("PO845407", 4, 0),
                GroupTally {
                    group_uid: group(MISSING_GROUP_SENTINEL),
                    member_count: None,
                    majority_position: None,
                    tally: VoteTally::default(),
                    origin: TallyOrigin::Published,
                },
            ],
            vec![
                vote("PA1", Some("PO845407"), VotePosition::For),
                vote("PA2", Some("PO845401"), VotePosition::Against),
            ],
        );

        let report = s.reconstruct_sentinel_groups();

        assert_eq!(report.rebuilt_lines, 1);
        // La ligne publiee garde son chiffre, pas celui des positions nominales.
        let published = s
            .group_tallies()
            .iter()
            .find(|t| t.group_uid.as_str() == "PO845407")
            .unwrap();
        assert_eq!(published.tally.votes_for, 4);
        assert_eq!(published.origin, TallyOrigin::Published);
    }

    #[test]
    fn unresolved_votes_are_counted_never_guessed() {
        let mut s = scrutin(
            vec![GroupTally {
                group_uid: group(MISSING_GROUP_SENTINEL),
                member_count: None,
                majority_position: None,
                tally: VoteTally::default(),
                origin: TallyOrigin::Published,
            }],
            vec![
                vote("PA1", Some("PO845401"), VotePosition::For),
                vote("PA2", None, VotePosition::For),
            ],
        );

        let report = s.reconstruct_sentinel_groups();

        assert_eq!(report.unresolved_votes, 1);
        assert_eq!(s.group_tallies().len(), 1);
        assert_eq!(s.group_tallies()[0].tally.votes_for, 1);
    }

    #[test]
    fn an_unreconstructed_sentinel_vote_never_becomes_a_group() {
        let mut s = scrutin(
            vec![GroupTally {
                group_uid: group(MISSING_GROUP_SENTINEL),
                member_count: None,
                majority_position: None,
                tally: VoteTally::default(),
                origin: TallyOrigin::Published,
            }],
            vec![vote("PA1", Some(MISSING_GROUP_SENTINEL), VotePosition::For)],
        );

        let report = s.reconstruct_sentinel_groups();

        assert_eq!(report.unresolved_votes, 1);
        assert_eq!(report.rebuilt_lines, 0);
        assert!(s.group_tallies().is_empty());
    }

    #[test]
    fn actors_under_sentinel_lists_who_needs_a_dated_membership() {
        let s = scrutin(
            vec![],
            vec![
                vote("PA2", Some(MISSING_GROUP_SENTINEL), VotePosition::For),
                vote("PA1", Some(MISSING_GROUP_SENTINEL), VotePosition::For),
                vote("PA1", Some(MISSING_GROUP_SENTINEL), VotePosition::Against),
                vote("PA3", Some("PO845401"), VotePosition::For),
            ],
        );

        let uids: Vec<String> = s
            .actors_under_sentinel()
            .iter()
            .map(|u| u.as_str().to_string())
            .collect();
        assert_eq!(uids, vec!["PA1".to_string(), "PA2".to_string()]);
    }

    fn sentinel_line() -> GroupTally {
        GroupTally {
            group_uid: group(MISSING_GROUP_SENTINEL),
            member_count: Some(122),
            majority_position: None,
            tally: VoteTally::default(),
            origin: TallyOrigin::Published,
        }
    }

    /// PA1 siege dans PO_A jusqu'au 30 juin 2025, puis dans PO_B. PA2 est
    /// absent du repertoire.
    fn directory_with_a_group_change() -> ActorDirectory {
        let deputy = Actor::new(
            actor("PA1"),
            Some("M.".into()),
            "Jean".into(),
            "Dupont".into(),
            ActorRole::Deputy,
        )
        .unwrap();
        let parliamentary_group = |uid: &str, abbrev: &str| {
            ParliamentaryGroup::new(
                group(uid),
                17,
                format!("Groupe {abbrev}"),
                abbrev.into(),
                None,
                None,
                None,
            )
            .unwrap()
        };
        let membership = |source: &str, group_uid: &str, start, end| {
            GroupMembership::new(
                source.into(),
                actor("PA1"),
                group(group_uid),
                17,
                MembershipPeriod::new(start, end).unwrap(),
                MembershipQuality::new("Membre".into()).unwrap(),
            )
        };

        ActorDirectory::new(
            vec![deputy],
            vec![
                parliamentary_group("PO_A", "A"),
                parliamentary_group("PO_B", "B"),
            ],
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
    fn attach_groups_gives_a_vote_without_group_the_directory_group_at_the_scrutin_date() {
        let mut s = scrutin(
            vec![sentinel_line()],
            vec![vote("PA1", None, VotePosition::For)],
        );

        let report = s.attach_groups(&directory_with_a_group_change());

        // Le scrutin date de decembre 2024: PA1 siegeait alors dans PO_A, pas
        // dans PO_B, son groupe actuel.
        assert_eq!(s.nominal_votes()[0].group_uid, Some(group("PO_A")));
        assert_eq!(report.sentinel_lines, 1);
        assert_eq!(report.rebuilt_lines, 1);
        assert_eq!(report.unresolved_votes, 0);
        assert!(!s.has_sentinel_tallies());
        assert_eq!(s.group_tallies()[0].group_uid, group("PO_A"));
        assert_eq!(s.group_tallies()[0].origin, TallyOrigin::Reconstructed);
        assert_eq!(s.group_tallies()[0].tally.votes_for, 1);
    }

    #[test]
    fn attach_groups_leaves_an_unknown_actor_without_group_and_counts_it_unresolved() {
        let mut s = scrutin(
            vec![sentinel_line()],
            vec![
                vote("PA1", None, VotePosition::For),
                vote("PA2", None, VotePosition::For),
            ],
        );

        let report = s.attach_groups(&directory_with_a_group_change());

        assert_eq!(s.nominal_votes()[1].group_uid, None);
        assert_eq!(report.unresolved_votes, 1);
        assert_eq!(s.group_tallies().len(), 1);
        assert_eq!(s.group_tallies()[0].tally.votes_for, 1);
    }

    #[test]
    fn attach_groups_does_not_touch_a_vote_that_already_has_a_group() {
        let mut s = scrutin(
            vec![sentinel_line()],
            vec![vote("PA1", Some("PO_B"), VotePosition::For)],
        );

        let report = s.attach_groups(&directory_with_a_group_change());

        // Le repertoire donnerait PO_A a cette date: le groupe porte par la
        // position l'emporte.
        assert_eq!(s.nominal_votes()[0].group_uid, Some(group("PO_B")));
        assert_eq!(report.unresolved_votes, 0);
        assert_eq!(s.group_tallies()[0].group_uid, group("PO_B"));
    }

    #[test]
    fn attach_groups_keeps_an_explicit_sentinel_vote_under_the_sentinel() {
        let mut s = scrutin(
            vec![sentinel_line()],
            vec![vote("PA1", Some(MISSING_GROUP_SENTINEL), VotePosition::For)],
        );

        let report = s.attach_groups(&directory_with_a_group_change());

        assert_eq!(
            s.nominal_votes()[0].group_uid,
            Some(group(MISSING_GROUP_SENTINEL))
        );
        assert_eq!(report.unresolved_votes, 1);
        assert!(s.group_tallies().is_empty());
    }

    #[test]
    fn outcome_reads_adoption_from_the_source_code() {
        let adopted = Outcome::new(
            "adopt\u{00e9}".into(),
            "l'Assembl\u{00e9}e nationale a adopt\u{00e9}".into(),
        )
        .unwrap();
        assert!(adopted.is_adopted());

        let rejected = Outcome::new(
            "rejet\u{00e9}".into(),
            "L'Assembl\u{00e9}e nationale n'a pas adopt\u{00e9}".into(),
        )
        .unwrap();
        assert!(!rejected.is_adopted());
    }

    fn short(subject: &str) -> String {
        ShortLabel::from_subject(subject).as_str().to_string()
    }

    #[test]
    fn should_label_article_vote_when_subject_puts_an_article_to_the_vote() {
        assert_eq!(
            short("l'article 15 du projet de loi de finances pour 2026"),
            "Vote sur l'article 15"
        );
    }

    #[test]
    fn should_label_amendment_with_cited_author_and_article_when_subject_names_them() {
        assert_eq!(
            short(
                "l'amendement n\u{00b0} 1376 de Mme Thomin apr\u{00e8}s l'article 15 du projet de loi de finances pour 2026"
            ),
            "Amendement n\u{00b0} 1376 de Mme Thomin, article 15"
        );
    }

    #[test]
    fn should_label_whole_text_vote_when_subject_puts_the_whole_text_to_the_vote() {
        assert_eq!(
            short("l'ensemble du projet de loi de finances pour 2026"),
            "Vote sur l'ensemble du texte"
        );
    }

    #[test]
    fn should_label_motion_with_its_type_when_subject_is_a_motion() {
        assert_eq!(
            short("la motion de rejet pr\u{00e9}alable, d\u{00e9}pos\u{00e9}e par Mme Panot, du projet de loi."),
            "Motion de rejet pr\u{00e9}alable"
        );
    }

    #[test]
    fn should_keep_subject_as_is_when_no_form_recognises_it() {
        let subject = "la d\u{00e9}claration du Gouvernement sur la politique \u{00e9}nerg\u{00e9}tique";
        assert_eq!(short(subject), subject);
    }
}
