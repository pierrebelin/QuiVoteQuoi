//! Scrutins publics de l'Assemblee nationale.
//!
//! Voir `todo/SPEC-scrutins.md`. Deux invariants portent tout le reste:
//! - RM-02: les chiffres publies ne sont jamais recalcules;
//! - RM-04: le groupe d'une position nominale est celui sous lequel la source
//!   range le votant dans ce scrutin, pas son appartenance courante.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::Serialize;

use super::actor::{ActorUid, GroupUid};

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
    pub uid: String,
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

#[allow(clippy::too_many_arguments)]
impl Scrutin {
    pub fn new(
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
    ) -> Result<Self, ScrutinError> {
        if number.trim().is_empty() {
            return Err(ScrutinError::EmptyScrutinNumber);
        }
        if subject.trim().is_empty() {
            return Err(ScrutinError::EmptySubject);
        }
        Ok(Self {
            uid,
            number,
            legislature,
            date,
            session_ref,
            sitting_ref,
            place,
            ballot_type,
            outcome,
            requester,
            subject,
            synthesis,
            group_tallies,
            nominal_votes,
            corrections,
            dossier,
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
    pub fn nominal_votes_mut(&mut self) -> &mut [NominalVote] {
        &mut self.nominal_votes
    }
    pub fn corrections(&self) -> &[VoteCorrection] {
        &self.corrections
    }
    pub fn dossier(&self) -> Option<&DossierReference> {
        self.dossier.as_ref()
    }

    /// Page officielle du scrutin (RM-07).
    pub fn official_url(&self) -> String {
        format!(
            "https://www.assemblee-nationale.fr/dyn/{}/scrutins/{}",
            self.legislature, self.number
        )
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
    /// scrutin, puis appelle `reconstruct_sentinel_groups`.
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

    /// Remplace les lignes de groupe factices par une repartition reconstruite
    /// depuis les positions nominales (RM-03).
    ///
    /// Prerequis: les positions concernees portent deja leur vrai groupe, resolu
    /// par l'appelant sur l'appartenance a la date du scrutin. Une position
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
        Scrutin::new(
            ScrutinUid::new("VTANR5L17V489".into()).unwrap(),
            "489".into(),
            17,
            date(),
            Some("SCR5A2025O1".into()),
            None,
            Some("H\u{00e9}micycle".into()),
            BallotType::new(
                "SPO".into(),
                "scrutin public ordinaire".into(),
                Some("Majorit\u{00e9} absolue des suffrages exprim\u{00e9}s".into()),
            )
            .unwrap(),
            Outcome::new(
                "rejet\u{00e9}".into(),
                "L'Assembl\u{00e9}e nationale n'a pas adopt\u{00e9}".into(),
            )
            .unwrap(),
            None,
            "l'amendement n\u{00b0} 1".into(),
            VoteSynthesis {
                voters: 2,
                expressed: 2,
                required: 2,
                announcement: "L'Assembl\u{00e9}e nationale n'a pas adopt\u{00e9}".into(),
                tally: VoteTally::default(),
            },
            group_tallies,
            nominal_votes,
            vec![],
            None,
        )
        .unwrap()
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

        let err = Scrutin::new(
            ScrutinUid::new("V1".into()).unwrap(),
            "  ".into(),
            17,
            date(),
            None,
            None,
            None,
            BallotType::new("SPO".into(), "ordinaire".into(), None).unwrap(),
            Outcome::new("adopt\u{00e9}".into(), "adopt\u{00e9}".into()).unwrap(),
            None,
            "objet".into(),
            VoteSynthesis {
                voters: 0,
                expressed: 0,
                required: 0,
                announcement: String::new(),
                tally: VoteTally::default(),
            },
            vec![],
            vec![],
            vec![],
            None,
        );
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
}
