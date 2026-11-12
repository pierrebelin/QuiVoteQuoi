//! Explication de vote: groupe porte par l'orateur (RM-06).

use super::actor::{ActorDirectory, ActorUid, GroupUid};
use super::scrutin::{Scrutin, ScrutinUid};
use super::sitting_record::{
    AttachmentCounters, ExplanationCounters, SittingRecordUid, SpeechAnchor,
};

/// Provenance du groupe d'un orateur: trois issues fermees, jamais devine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpeakerGroup {
    FromScrutin(GroupUid),
    FromMembership(GroupUid),
    Undetermined,
}

impl SpeakerGroup {
    pub fn as_str(&self) -> &'static str {
        match self {
            SpeakerGroup::FromScrutin(_) => "scrutin",
            SpeakerGroup::FromMembership(_) => "membership",
            SpeakerGroup::Undetermined => "undetermined",
        }
    }

    /// Relit la base stockee et son groupe; une base sans groupe attendu
    /// (ou un groupe sans base qui en porte) n'existe pas.
    pub fn parse(raw: &str, group: Option<GroupUid>) -> Option<Self> {
        match (raw, group) {
            ("scrutin", Some(group)) => Some(SpeakerGroup::FromScrutin(group)),
            ("membership", Some(group)) => Some(SpeakerGroup::FromMembership(group)),
            ("undetermined", None) => Some(SpeakerGroup::Undetermined),
            _ => None,
        }
    }

    pub fn group_uid(&self) -> Option<&GroupUid> {
        match self {
            SpeakerGroup::FromScrutin(group) | SpeakerGroup::FromMembership(group) => Some(group),
            SpeakerGroup::Undetermined => None,
        }
    }

    /// Groupe de l'orateur (RM-06): celui du scrutin, sinon celui de la date
    /// de la séance, sinon non établi. Jamais deviné.
    pub fn establish(scrutin: &Scrutin, directory: &ActorDirectory, speaker: &ActorUid) -> Self {
        let from_scrutin = scrutin
            .nominal_votes()
            .iter()
            .find(|vote| &vote.actor_uid == speaker)
            .and_then(|vote| vote.group_uid.clone());
        if let Some(group) = from_scrutin {
            return SpeakerGroup::FromScrutin(group);
        }
        directory
            .resolve_at(speaker, scrutin.date())
            .filter(|at_date| !at_date.ambiguous)
            .and_then(|at_date| at_date.group)
            .map_or(SpeakerGroup::Undetermined, |group| {
                SpeakerGroup::FromMembership(group.uid().clone())
            })
    }
}

/// Quatre états fermés de la section d'explications d'un vote sur l'ensemble (RM-09).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplanationsState {
    NotPublished,
    NotAttached,
    NoExplanation,
    Explained,
}

impl ExplanationsState {
    /// État décidé par la ligne lue (RM-09): jamais une absence rendue comme une liste vide.
    /// `attached`: `None` = scrutin non rattaché; `Some(bloc présent)`.
    pub fn decide(report_stored: bool, attached: Option<bool>, explained: bool) -> Self {
        match (report_stored, attached) {
            (false, _) => ExplanationsState::NotPublished,
            (true, None) => ExplanationsState::NotAttached,
            (true, Some(_)) if explained => ExplanationsState::Explained,
            (true, Some(_)) => ExplanationsState::NoExplanation,
        }
    }
}

pub struct NewVoteExplanation {
    pub scrutin_uid: ScrutinUid,
    pub sitting_record_uid: SittingRecordUid,
    pub rank: u16,
    pub speaker_uid: ActorUid,
    pub speaker_name: Option<String>,
    pub group: SpeakerGroup,
    pub anchor: SpeechAnchor,
    pub heading: String,
    pub text: String,
}

/// Explication de vote d'un orateur, reproduite mot pour mot (RM-03).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoteExplanation {
    scrutin_uid: ScrutinUid,
    sitting_record_uid: SittingRecordUid,
    rank: u16,
    speaker_uid: ActorUid,
    speaker_name: Option<String>,
    group: SpeakerGroup,
    anchor: SpeechAnchor,
    heading: String,
    text: String,
}

impl VoteExplanation {
    pub fn new(new: NewVoteExplanation) -> Self {
        Self {
            scrutin_uid: new.scrutin_uid,
            sitting_record_uid: new.sitting_record_uid,
            rank: new.rank,
            speaker_uid: new.speaker_uid,
            speaker_name: new.speaker_name,
            group: new.group,
            anchor: new.anchor,
            heading: new.heading,
            text: new.text,
        }
    }

    pub fn scrutin_uid(&self) -> &ScrutinUid {
        &self.scrutin_uid
    }

    pub fn sitting_record_uid(&self) -> &SittingRecordUid {
        &self.sitting_record_uid
    }

    pub fn rank(&self) -> u16 {
        self.rank
    }

    pub fn speaker_uid(&self) -> &ActorUid {
        &self.speaker_uid
    }

    pub fn speaker_name(&self) -> Option<&str> {
        self.speaker_name.as_deref()
    }

    pub fn group(&self) -> &SpeakerGroup {
        &self.group
    }

    pub fn anchor(&self) -> &SpeechAnchor {
        &self.anchor
    }

    pub fn heading(&self) -> &str {
        &self.heading
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

/// Votes sur l'ensemble et orateurs que le rattachement n'a pas placés (RM-13).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FinalVoteAttachmentCounts {
    pub unattached: u32,
    pub without_block: u32,
    pub speakers_without_group: u32,
}

/// Compteurs du rattachement, sur toutes les séances stockées (RM-10, RM-13).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExplanationCoverage {
    pub results_attached: u32,
    pub results_unmatched: u32,
    pub results_ambiguous: u32,
    pub final_votes_unattached: u32,
    pub final_votes_without_block: u32,
    pub blocks_unattached: u32,
    pub headings_unrecognized: u32,
    pub speakers_without_group: u32,
}

impl ExplanationCoverage {
    pub fn of(
        sittings: &[(AttachmentCounters, ExplanationCounters)],
        final_votes: FinalVoteAttachmentCounts,
    ) -> Self {
        let total = |pick: fn(&(AttachmentCounters, ExplanationCounters)) -> usize| {
            sittings
                .iter()
                .map(|s| u32::try_from(pick(s)).unwrap_or(u32::MAX))
                .fold(0u32, u32::saturating_add)
        };
        Self {
            results_attached: total(|s| s.0.results_attached),
            results_unmatched: total(|s| s.0.results_unmatched),
            results_ambiguous: total(|s| s.0.results_ambiguous),
            final_votes_unattached: final_votes.unattached,
            final_votes_without_block: final_votes.without_block,
            blocks_unattached: total(|s| s.1.blocks_unattached),
            headings_unrecognized: total(|s| s.1.headings_unrecognized),
            speakers_without_group: final_votes.speakers_without_group,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::domain::actor::{
        Actor, ActorRole, GroupMembership, MembershipPeriod, MembershipQuality, ParliamentaryGroup,
    };
    use crate::domain::scrutin::{
        BallotType, NewScrutin, NominalVote, Outcome, ScrutinUid, VotePosition, VoteSynthesis,
        VoteTally,
    };

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn group(uid: &str) -> GroupUid {
        GroupUid::new(uid.into()).unwrap()
    }

    fn actor(uid: &str) -> ActorUid {
        ActorUid::new(uid.into()).unwrap()
    }

    fn vote(actor_uid: &str, group_uid: Option<&str>) -> NominalVote {
        NominalVote {
            actor_uid: actor(actor_uid),
            group_uid: group_uid.map(group),
            position: VotePosition::For,
            cause: None,
            by_delegation: false,
            seat: None,
        }
    }

    fn scrutin(nominal_votes: Vec<NominalVote>) -> Scrutin {
        Scrutin::new(NewScrutin {
            uid: ScrutinUid::new("VTANR5L17V489".into()).unwrap(),
            number: "489".into(),
            legislature: 17,
            date: day(2025, 3, 10),
            session_ref: None,
            sitting_ref: Some("RUANR5L17S2025IDS29344".into()),
            place: None,
            ballot_type: BallotType::new("SPO".into(), "scrutin public ordinaire".into(), None)
                .unwrap(),
            outcome: Outcome::new("adopté".into(), "adopté".into()).unwrap(),
            requester: None,
            subject: "l'ensemble du projet de loi".into(),
            synthesis: VoteSynthesis {
                voters: 1,
                expressed: 1,
                required: 1,
                announcement: "adopté".into(),
                tally: VoteTally::default(),
            },
            group_tallies: vec![],
            nominal_votes,
            corrections: vec![],
            dossier: None,
        })
        .unwrap()
    }

    fn membership(
        source_uid: &str,
        group_uid: &str,
        start: NaiveDate,
        end: Option<NaiveDate>,
    ) -> GroupMembership {
        GroupMembership::new(
            source_uid.into(),
            actor("PA1"),
            group(group_uid),
            17,
            MembershipPeriod::new(start, end).unwrap(),
            MembershipQuality::new("Membre".into()).unwrap(),
        )
    }

    fn directory(memberships: Vec<GroupMembership>) -> ActorDirectory {
        let speaker = Actor::new(
            actor("PA1"),
            Some("Mme".into()),
            "Jeanne".into(),
            "Durand".into(),
            ActorRole::Deputy,
        )
        .unwrap();
        let groups = ["PO1", "PO2", "PO3"]
            .iter()
            .map(|uid| {
                ParliamentaryGroup::new(
                    group(uid),
                    17,
                    format!("Groupe {uid}"),
                    (*uid).into(),
                    None,
                    None,
                    None,
                )
                .unwrap()
            })
            .collect();
        ActorDirectory::new(vec![speaker], groups, memberships)
    }

    #[test]
    fn takes_the_group_the_scrutin_ranks_the_speaker_under() {
        let scrutin = scrutin(vec![vote("PA1", Some("PO1"))]);
        let directory = directory(vec![membership("M1", "PO2", day(2025, 1, 1), None)]);

        let established = SpeakerGroup::establish(&scrutin, &directory, &actor("PA1"));

        assert_eq!(established, SpeakerGroup::FromScrutin(group("PO1")));
    }

    #[test]
    fn takes_the_membership_held_on_the_sitting_date_when_the_scrutin_line_has_no_group() {
        let scrutin = scrutin(vec![vote("PA1", None)]);
        let directory = directory(vec![
            membership("M1", "PO2", day(2024, 7, 1), Some(day(2025, 1, 31))),
            membership("M2", "PO3", day(2025, 2, 1), None),
        ]);

        let established = SpeakerGroup::establish(&scrutin, &directory, &actor("PA1"));

        assert_eq!(established, SpeakerGroup::FromMembership(group("PO3")));
    }

    #[test]
    fn takes_the_membership_held_on_the_sitting_date_when_the_scrutin_has_no_line_for_the_speaker()
    {
        let scrutin = scrutin(vec![vote("PA9", Some("PO1"))]);
        let directory = directory(vec![
            membership("M1", "PO2", day(2024, 7, 1), Some(day(2025, 1, 31))),
            membership("M2", "PO3", day(2025, 2, 1), None),
        ]);

        let established = SpeakerGroup::establish(&scrutin, &directory, &actor("PA1"));

        assert_eq!(established, SpeakerGroup::FromMembership(group("PO3")));
    }

    #[test]
    fn leaves_the_group_undetermined_when_neither_scrutin_nor_membership_establishes_it() {
        let scrutin = scrutin(vec![vote("PA1", None)]);
        let overlapping = directory(vec![
            membership("M1", "PO2", day(2025, 1, 1), None),
            membership("M2", "PO3", day(2025, 2, 1), None),
        ]);
        let absent = directory(vec![membership("M1", "PO2", day(2025, 4, 1), None)]);
        let unknown_speaker = actor("PA404");

        assert_eq!(
            SpeakerGroup::establish(&scrutin, &overlapping, &actor("PA1")),
            SpeakerGroup::Undetermined
        );
        assert_eq!(
            SpeakerGroup::establish(&scrutin, &absent, &actor("PA1")),
            SpeakerGroup::Undetermined
        );
        assert_eq!(
            SpeakerGroup::establish(&scrutin, &absent, &unknown_speaker),
            SpeakerGroup::Undetermined
        );
    }

    #[test]
    fn speaker_group_basis_round_trips_through_as_str_and_parse() {
        let cases = [
            (SpeakerGroup::FromScrutin(group("PO1")), "scrutin"),
            (SpeakerGroup::FromMembership(group("PO2")), "membership"),
            (SpeakerGroup::Undetermined, "undetermined"),
        ];

        for (value, basis) in cases {
            assert_eq!(value.as_str(), basis);
            assert_eq!(
                SpeakerGroup::parse(basis, value.group_uid().cloned()),
                Some(value)
            );
        }
        assert_eq!(SpeakerGroup::parse("guessed", None), None);
    }
}
