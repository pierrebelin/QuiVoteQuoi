//! Comptes rendus de seance.

use chrono::{DateTime, NaiveDate, Utc};

use super::actor::ActorUid;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SittingRecordError {
    #[error("sitting record uid is empty")]
    EmptySittingRecordUid,
    #[error("sitting reference is empty")]
    EmptySittingRef,
    #[error("speech anchor is empty")]
    EmptySpeechAnchor,
}

macro_rules! non_blank_newtype {
    ($name:ident, $error:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            pub fn new(raw: String) -> Result<Self, SittingRecordError> {
                if raw.trim().is_empty() {
                    return Err(SittingRecordError::$error);
                }
                Ok(Self(raw))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

non_blank_newtype!(SittingRecordUid, EmptySittingRecordUid);
non_blank_newtype!(SittingRef, EmptySittingRef);
non_blank_newtype!(SpeechAnchor, EmptySpeechAnchor);

/// Racine des comptes rendus de séance sur le site de l'Assemblée (ADR-005).
const OFFICIAL_REPORT_ROOT: &str = "https://www.assemblee-nationale.fr/dyn/17/comptes-rendus/seance";

impl SittingRecordUid {
    /// Lien vers le compte rendu officiel, ancré sur un paragraphe s'il est donné (RM-12).
    pub fn official_url(&self, anchor: Option<&SpeechAnchor>) -> String {
        match anchor {
            Some(anchor) => format!("{OFFICIAL_REPORT_ROOT}/{}#{}", self.0, anchor.as_str()),
            None => format!("{OFFICIAL_REPORT_ROOT}/{}", self.0),
        }
    }
}

/// Entree d'un compte rendu, dans l'ordre du document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SittingEntry {
    Heading {
        level: u8,
        text: String,
    },
    Speech {
        actor_uid: ActorUid,
        anchor: SpeechAnchor,
        presiding: bool,
        interjection: bool,
        text: String,
    },
    Result(AnnouncedResult),
}

pub struct NewSittingRecord {
    pub uid: SittingRecordUid,
    pub sitting_ref: SittingRef,
    pub date: NaiveDate,
    pub version: String,
    pub generated_at: DateTime<Utc>,
    pub entries: Vec<SittingEntry>,
}

/// Compte rendu d'une seance: possede ses entrees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SittingRecord {
    uid: SittingRecordUid,
    sitting_ref: SittingRef,
    date: NaiveDate,
    version: String,
    generated_at: DateTime<Utc>,
    entries: Vec<SittingEntry>,
}

impl SittingRecord {
    pub fn new(new: NewSittingRecord) -> Self {
        Self {
            uid: new.uid,
            sitting_ref: new.sitting_ref,
            date: new.date,
            version: new.version,
            generated_at: new.generated_at,
            entries: new.entries,
        }
    }

    pub fn uid(&self) -> &SittingRecordUid {
        &self.uid
    }

    pub fn sitting_ref(&self) -> &SittingRef {
        &self.sitting_ref
    }

    pub fn date(&self) -> NaiveDate {
        self.date
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn generated_at(&self) -> DateTime<Utc> {
        self.generated_at
    }

    pub fn entries(&self) -> &[SittingEntry] {
        &self.entries
    }
}

use std::collections::{HashMap, HashSet};

use super::final_vote::is_final_vote_subject;

use super::scrutin::{Scrutin, ScrutinUid};

/// Resultat annonce par la presidence, lu en quatre nombres (RM-04).
///
/// La majorite absolue est annoncee mais jamais lue: elle ne sert pas a
/// rattacher le resultat a un scrutin. Un nombre manquant = aucun resultat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnouncedResult {
    pub voters: u32,
    pub expressed: u32,
    pub votes_for: u32,
    pub votes_against: u32,
}

impl AnnouncedResult {
    pub fn from_text(text: &str) -> Option<Self> {
        let normalized = text.replace('\u{a0}', " ").replace('’', "'");
        let tokens: Vec<&str> = normalized.split_whitespace().collect();
        Some(Self {
            voters: number_after(&tokens, &["Nombre", "de", "votants"])?,
            expressed: number_after(&tokens, &["Nombre", "de", "suffrages", "exprimés"])?,
            votes_for: number_after(&tokens, &["Pour", "l'adoption"])?,
            votes_against: number_after(&tokens, &["Contre"])?,
        })
    }
}

/// Nombre qui suit la premiere occurrence de l'etiquette.
fn number_after(tokens: &[&str], label: &[&str]) -> Option<u32> {
    let at = tokens.windows(label.len()).position(|window| window == label)?;
    tokens.get(at + label.len())?.parse().ok()
}

/// Compteurs du rattachement des resultats annonces (RM-04).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AttachmentCounters {
    pub results_attached: usize,
    pub results_unmatched: usize,
    pub results_ambiguous: usize,
}

/// Issue du rattachement d'un resultat annonce a un scrutin de sa seance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SittingAttachment {
    pub attached: Option<ScrutinUid>,
}

/// Empreintes (quatre nombres) des scrutins d'une seance (RM-04).
#[derive(Debug, Clone, Default)]
pub struct ScrutinFingerprints {
    by_numbers: HashMap<(u32, u32, u32, u32), Vec<ScrutinUid>>,
    final_votes: HashSet<ScrutinUid>,
}

impl ScrutinFingerprints {
    /// Ne retient que les scrutins de la seance, cles par leurs quatre nombres.
    pub fn for_sitting(sitting_ref: &str, scrutins: &[Scrutin]) -> Self {
        let mut by_numbers: HashMap<(u32, u32, u32, u32), Vec<ScrutinUid>> = HashMap::new();
        let mut final_votes = HashSet::new();
        for scrutin in scrutins
            .iter()
            .filter(|scrutin| scrutin.sitting_ref() == Some(sitting_ref))
        {
            let synthesis = scrutin.synthesis();
            let numbers = (
                u32::from(synthesis.voters),
                u32::from(synthesis.expressed),
                u32::from(synthesis.tally.votes_for),
                u32::from(synthesis.tally.votes_against),
            );
            if is_final_vote_subject(scrutin.subject()) {
                final_votes.insert(scrutin.uid().clone());
            }
            by_numbers
                .entry(numbers)
                .or_default()
                .push(scrutin.uid().clone());
        }
        Self {
            by_numbers,
            final_votes,
        }
    }

    pub fn len(&self) -> usize {
        self.by_numbers.values().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.by_numbers.is_empty()
    }

    /// Rattache au seul scrutin portant les quatre nombres (RM-04); aucun
    /// scrutin ou plusieurs: rien n'est rattache, le cas est compte.
    pub fn attach(
        &self,
        result: &AnnouncedResult,
        counters: &mut AttachmentCounters,
    ) -> SittingAttachment {
        let key = (
            result.voters,
            result.expressed,
            result.votes_for,
            result.votes_against,
        );
        match self.by_numbers.get(&key).map(Vec::as_slice) {
            Some([only]) => {
                counters.results_attached += 1;
                SittingAttachment {
                    attached: Some(only.clone()),
                }
            }
            Some(_) => {
                counters.results_ambiguous += 1;
                SittingAttachment { attached: None }
            }
            None => {
                counters.results_unmatched += 1;
                SittingAttachment { attached: None }
            }
        }
    }
}

/// Explication d'un orateur dans un bloc d'explications de vote (RM-03).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeakerSpeech {
    pub actor_uid: ActorUid,
    pub anchor: SpeechAnchor,
    pub text: String,
}

/// Bloc d'explications: intitule et une explication par orateur (RM-03).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplanationBlock {
    pub title: String,
    pub speeches: Vec<SpeakerSpeech>,
}

/// Vote sur l'ensemble de la seance et le bloc qui lui est rattache (RM-01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachedScrutin {
    pub scrutin: ScrutinUid,
    pub block: Option<ExplanationBlock>,
}

/// Compteurs du rattachement des blocs d'explications (RM-05).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExplanationCounters {
    pub blocks_unattached: usize,
    pub headings_unrecognized: usize,
}

/// Intitules qui ouvrent un bloc d'explications (RM-05).
pub const EXPLANATION_HEADINGS: [&str; 3] = [
    "Explications de vote",
    "Explication de vote",
    "Explications de vote communes",
];

/// Bloc en cours de lecture; `attached` une fois lie a un vote sur l'ensemble.
struct OpenBlock {
    block: ExplanationBlock,
    attached: bool,
}

impl OpenBlock {
    fn open(title: &str) -> Self {
        Self {
            block: ExplanationBlock {
                title: title.to_string(),
                speeches: Vec::new(),
            },
            attached: false,
        }
    }

    /// Une explication par orateur: ses reprises sont jointes a la premiere (RM-03).
    fn push_speech(&mut self, actor_uid: &ActorUid, anchor: &SpeechAnchor, text: &str) {
        match self
            .block
            .speeches
            .iter_mut()
            .find(|speech| &speech.actor_uid == actor_uid)
        {
            Some(existing) => {
                existing.text.push_str("\n\n");
                existing.text.push_str(text);
            }
            None => self.block.speeches.push(SpeakerSpeech {
                actor_uid: actor_uid.clone(),
                anchor: anchor.clone(),
                text: text.to_string(),
            }),
        }
    }
}

fn is_explanation_heading(text: &str) -> bool {
    let text = text.trim();
    EXPLANATION_HEADINGS
        .iter()
        .any(|known| known.eq_ignore_ascii_case(text))
}

fn close_block(open: &mut Option<OpenBlock>, counters: &mut ExplanationCounters) {
    if let Some(block) = open.take() {
        if !block.attached {
            counters.blocks_unattached += 1;
        }
    }
}

impl ScrutinFingerprints {
    /// Rattache chaque bloc aux votes sur l'ensemble annonces apres lui (RM-01,
    /// RM-05): un autre bloc, un scrutin autre qu'un vote sur l'ensemble ou un
    /// titre de niveau 1 le ferme.
    pub fn attach_explanations(
        &self,
        entries: &[SittingEntry],
        counters: &mut ExplanationCounters,
    ) -> Vec<AttachedScrutin> {
        let mut attached = Vec::new();
        let mut open: Option<OpenBlock> = None;
        for entry in entries {
            match entry {
                SittingEntry::Heading { level, text } => {
                    if *level == 1 {
                        close_block(&mut open, counters);
                    } else if is_explanation_heading(text) {
                        close_block(&mut open, counters);
                        open = Some(OpenBlock::open(text.trim()));
                    } else if text.to_lowercase().contains("explication") {
                        counters.headings_unrecognized += 1;
                    }
                }
                SittingEntry::Speech {
                    actor_uid,
                    anchor,
                    presiding,
                    interjection,
                    text,
                } => {
                    if let Some(block) = open.as_mut().filter(|block| !block.attached) {
                        if !presiding && !interjection {
                            block.push_speech(actor_uid, anchor, text);
                        }
                    }
                }
                SittingEntry::Result(result) => {
                    let mut ignored = AttachmentCounters::default();
                    let Some(uid) = self.attach(result, &mut ignored).attached else {
                        continue;
                    };
                    if self.final_votes.contains(&uid) {
                        let block = open.as_mut().map(|open| {
                            open.attached = true;
                            open.block.clone()
                        });
                        attached.push(AttachedScrutin { scrutin: uid, block });
                    } else {
                        close_block(&mut open, counters);
                    }
                }
            }
        }
        close_block(&mut open, counters);
        attached
    }
}

/// Clé de rattachement d'un scrutin à une séance (RM-04, RM-11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrutinSittingKey {
    pub uid: ScrutinUid,
    pub sitting_ref: SittingRef,
    pub result: AnnouncedResult,
    pub final_vote: bool,
}

impl ScrutinSittingKey {
    /// `final_vote` suit le même critère que la fiche du scrutin.
    pub fn new(
        uid: ScrutinUid,
        sitting_ref: SittingRef,
        result: AnnouncedResult,
        subject: &str,
    ) -> Self {
        Self {
            uid,
            sitting_ref,
            result,
            final_vote: is_final_vote_subject(subject),
        }
    }
}

/// Empreinte des clés des scrutins d'une séance (RM-11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SittingKeysFingerprint(String);

impl SittingKeysFingerprint {
    /// Texte canonique des clés triées par scrutin: deux lectures des mêmes
    /// clés donnent la même empreinte, quel que soit l'ordre de lecture.
    pub fn of(keys: &[ScrutinSittingKey]) -> Self {
        let mut sorted: Vec<&ScrutinSittingKey> = keys.iter().collect();
        sorted.sort_by(|a, b| a.uid.as_str().cmp(b.uid.as_str()));
        let text = sorted
            .iter()
            .map(|key| {
                format!(
                    "{}:{}:{}:{}:{}:{}:{}",
                    key.uid,
                    key.sitting_ref,
                    key.result.voters,
                    key.result.expressed,
                    key.result.votes_for,
                    key.result.votes_against,
                    key.final_vote
                )
            })
            .collect::<Vec<_>>()
            .join(";");
        Self(text)
    }

    /// Relit l'empreinte telle que stockée, sans la recalculer.
    pub fn from_stored(text: String) -> Self {
        Self(text)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Issue du rattachement d'une séance lue (RM-03, RM-04, RM-05).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecordAttachment {
    pub attached: Vec<AttachedScrutin>,
    pub results: AttachmentCounters,
    pub explanations: ExplanationCounters,
}

impl SittingRecord {
    /// Empreinte des seules clés de cette séance (RM-11).
    pub fn keys_fingerprint(&self, scrutins: &[ScrutinSittingKey]) -> SittingKeysFingerprint {
        SittingKeysFingerprint::of(&self.own_keys(scrutins))
    }

    /// Rattache les résultats annoncés puis les blocs d'explications aux
    /// scrutins de la séance (RM-03, RM-04, RM-05).
    pub fn attach(&self, scrutins: &[ScrutinSittingKey]) -> RecordAttachment {
        let fingerprints = ScrutinFingerprints::from_keys(&self.own_keys(scrutins));
        let mut results = AttachmentCounters::default();
        for entry in &self.entries {
            if let SittingEntry::Result(result) = entry {
                fingerprints.attach(result, &mut results);
            }
        }
        let mut explanations = ExplanationCounters::default();
        let attached = fingerprints.attach_explanations(&self.entries, &mut explanations);
        RecordAttachment {
            attached,
            results,
            explanations,
        }
    }

    fn own_keys(&self, scrutins: &[ScrutinSittingKey]) -> Vec<ScrutinSittingKey> {
        scrutins
            .iter()
            .filter(|key| key.sitting_ref == self.sitting_ref)
            .cloned()
            .collect()
    }
}

impl ScrutinFingerprints {
    /// Empreintes des clés de scrutin déjà filtrées sur la séance (RM-04).
    pub fn from_keys(keys: &[ScrutinSittingKey]) -> Self {
        let mut fingerprints = Self::default();
        for key in keys {
            let numbers = (
                key.result.voters,
                key.result.expressed,
                key.result.votes_for,
                key.result.votes_against,
            );
            if key.final_vote {
                fingerprints.final_votes.insert(key.uid.clone());
            }
            fingerprints
                .by_numbers
                .entry(numbers)
                .or_default()
                .push(key.uid.clone());
        }
        fingerprints
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::domain::scrutin::{BallotType, NewScrutin, Outcome, VoteSynthesis, VoteTally};

    const SITTING: &str = "RUANR5L17S2025IDS29344";
    const OTHER_SITTING: &str = "RUANR5L17S2025IDS99999";

    fn scrutin(uid: &str, sitting: &str, numbers: (u16, u16, u16, u16)) -> Scrutin {
        scrutin_about(uid, sitting, "l'ensemble du projet de loi", numbers)
    }

    fn scrutin_about(
        uid: &str,
        sitting: &str,
        subject: &str,
        numbers: (u16, u16, u16, u16),
    ) -> Scrutin {
        Scrutin::new(NewScrutin {
            uid: ScrutinUid::new(uid.into()).unwrap(),
            number: "1".into(),
            legislature: 17,
            date: NaiveDate::from_ymd_opt(2025, 3, 10).unwrap(),
            session_ref: None,
            sitting_ref: Some(sitting.into()),
            place: None,
            ballot_type: BallotType::new("SPO".into(), "scrutin public ordinaire".into(), None)
                .unwrap(),
            outcome: Outcome::new("adopté".into(), "adopté".into()).unwrap(),
            requester: None,
            subject: subject.into(),
            synthesis: VoteSynthesis {
                voters: numbers.0,
                expressed: numbers.1,
                required: 1,
                announcement: "adopté".into(),
                tally: VoteTally {
                    votes_for: numbers.2,
                    votes_against: numbers.3,
                    ..VoteTally::default()
                },
            },
            group_tallies: vec![],
            nominal_votes: vec![],
            corrections: vec![],
            dossier: None,
        })
        .unwrap()
    }

    fn result(voters: u32, expressed: u32, votes_for: u32, votes_against: u32) -> AnnouncedResult {
        AnnouncedResult {
            voters,
            expressed,
            votes_for,
            votes_against,
        }
    }

    fn uid(raw: &str) -> ScrutinUid {
        ScrutinUid::new(raw.into()).unwrap()
    }

    #[test]
    fn attaches_a_result_to_the_only_scrutin_of_the_sitting_with_the_same_four_numbers() {
        let scrutins = [
            scrutin("VTANR5L17V1", SITTING, (553, 540, 300, 240)),
            scrutin("VTANR5L17V2", SITTING, (500, 490, 250, 240)),
        ];
        let fingerprints = ScrutinFingerprints::for_sitting(SITTING, &scrutins);
        let mut counters = AttachmentCounters::default();

        let attachment = fingerprints.attach(&result(553, 540, 300, 240), &mut counters);

        assert_eq!(attachment.attached, Some(uid("VTANR5L17V1")));
        assert_eq!(
            counters,
            AttachmentCounters {
                results_attached: 1,
                results_unmatched: 0,
                results_ambiguous: 0
            }
        );
    }

    #[test]
    fn ignores_a_scrutin_of_another_sitting_with_the_same_numbers() {
        let scrutins = [scrutin("VTANR5L17V1", OTHER_SITTING, (553, 540, 300, 240))];
        let fingerprints = ScrutinFingerprints::for_sitting(SITTING, &scrutins);
        let mut counters = AttachmentCounters::default();

        let attachment = fingerprints.attach(&result(553, 540, 300, 240), &mut counters);

        assert_eq!(attachment.attached, None);
        assert_eq!(
            counters,
            AttachmentCounters {
                results_attached: 0,
                results_unmatched: 1,
                results_ambiguous: 0
            }
        );
    }

    #[test]
    fn counts_a_result_matching_no_scrutin_as_unmatched() {
        let scrutins = [scrutin("VTANR5L17V1", SITTING, (553, 540, 300, 240))];
        let fingerprints = ScrutinFingerprints::for_sitting(SITTING, &scrutins);
        let mut counters = AttachmentCounters::default();

        let attachment = fingerprints.attach(&result(553, 540, 301, 239), &mut counters);

        assert_eq!(attachment.attached, None);
        assert_eq!(
            counters,
            AttachmentCounters {
                results_attached: 0,
                results_unmatched: 1,
                results_ambiguous: 0
            }
        );
    }

    #[test]
    fn counts_a_result_matching_two_scrutins_as_ambiguous_and_attaches_neither() {
        let scrutins = [
            scrutin("VTANR5L17V1", SITTING, (553, 540, 300, 240)),
            scrutin("VTANR5L17V2", SITTING, (553, 540, 300, 240)),
        ];
        let fingerprints = ScrutinFingerprints::for_sitting(SITTING, &scrutins);
        let mut counters = AttachmentCounters::default();

        let attachment = fingerprints.attach(&result(553, 540, 300, 240), &mut counters);

        assert_eq!(attachment.attached, None);
        assert_eq!(
            counters,
            AttachmentCounters {
                results_attached: 0,
                results_unmatched: 0,
                results_ambiguous: 1
            }
        );
    }

    #[test]
    fn fingerprints_only_the_scrutin_keys_of_its_own_sitting() {
        let scrutins = [
            scrutin("VTANR5L17V1", SITTING, (553, 540, 300, 240)),
            scrutin("VTANR5L17V2", OTHER_SITTING, (500, 490, 250, 240)),
        ];

        let fingerprints = ScrutinFingerprints::for_sitting(SITTING, &scrutins);

        assert_eq!(fingerprints.len(), 1);
    }


    #[test]
    fn reads_the_four_numbers_of_an_announced_result() {
        let text = "Voici le résultat du scrutin : Nombre de votants 553 Nombre de suffrages exprimés 540 Majorité absolue 271 Pour l’adoption 300 Contre 240";
        assert_eq!(
            AnnouncedResult::from_text(text),
            Some(AnnouncedResult {
                voters: 553,
                expressed: 540,
                votes_for: 300,
                votes_against: 240
            })
        );
    }

    #[test]
    fn reads_numbers_written_with_non_breaking_spaces() {
        let text = "Voici le résultat du scrutin :\u{a0}Nombre\u{a0}de\u{a0}votants\u{a0}553 Nombre\u{a0}de\u{a0}suffrages\u{a0}exprimés\u{a0}540 Majorité\u{a0}absolue\u{a0}271 Pour\u{a0}l’adoption\u{a0}300 Contre\u{a0}240";
        assert_eq!(
            AnnouncedResult::from_text(text),
            Some(AnnouncedResult {
                voters: 553,
                expressed: 540,
                votes_for: 300,
                votes_against: 240
            })
        );
    }

    #[test]
    fn reads_nothing_when_one_of_the_four_numbers_is_missing() {
        let text = "Voici le résultat du scrutin : Nombre de votants 553 Nombre de suffrages exprimés 540 Majorité absolue 271 Pour l’adoption 300";
        assert_eq!(AnnouncedResult::from_text(text), None);
    }

    fn actor(raw: &str) -> ActorUid {
        ActorUid::new(raw.into()).unwrap()
    }

    fn heading(level: u8, text: &str) -> SittingEntry {
        SittingEntry::Heading {
            level,
            text: text.into(),
        }
    }

    fn speech_with(
        speaker: &str,
        anchor: &str,
        text: &str,
        presiding: bool,
        interjection: bool,
    ) -> SittingEntry {
        SittingEntry::Speech {
            actor_uid: actor(speaker),
            anchor: SpeechAnchor::new(anchor.into()).unwrap(),
            presiding,
            interjection,
            text: text.into(),
        }
    }

    fn speech(speaker: &str, anchor: &str, text: &str) -> SittingEntry {
        speech_with(speaker, anchor, text, false, false)
    }

    fn presiding_speech(speaker: &str, anchor: &str, text: &str) -> SittingEntry {
        speech_with(speaker, anchor, text, true, false)
    }

    fn interjection(speaker: &str, anchor: &str, text: &str) -> SittingEntry {
        speech_with(speaker, anchor, text, false, true)
    }

    fn announced(numbers: (u16, u16, u16, u16)) -> SittingEntry {
        SittingEntry::Result(result(
            numbers.0.into(),
            numbers.1.into(),
            numbers.2.into(),
            numbers.3.into(),
        ))
    }

    fn spoken(speaker: &str, anchor: &str, text: &str) -> SpeakerSpeech {
        SpeakerSpeech {
            actor_uid: actor(speaker),
            anchor: SpeechAnchor::new(anchor.into()).unwrap(),
            text: text.into(),
        }
    }

    const FINAL_A: (u16, u16, u16, u16) = (553, 540, 300, 240);
    const FINAL_B: (u16, u16, u16, u16) = (500, 490, 250, 240);
    const AMENDMENT: (u16, u16, u16, u16) = (450, 440, 200, 240);
    const EXPLANATIONS: &str = "Explications de vote";

    fn fingerprints(scrutins: &[Scrutin]) -> ScrutinFingerprints {
        ScrutinFingerprints::for_sitting(SITTING, scrutins)
    }

    #[test]
    fn attaches_the_block_to_the_final_vote_announced_after_it() {
        let fingerprints = fingerprints(&[scrutin("V1", SITTING, FINAL_A)]);
        let entries = [
            heading(2, EXPLANATIONS),
            speech("PA1", "a1", "Nous voterons pour."),
            speech("PA2", "a2", "Nous voterons contre."),
            announced(FINAL_A),
        ];
        let mut counters = ExplanationCounters::default();

        let attached = fingerprints.attach_explanations(&entries, &mut counters);

        assert_eq!(
            attached,
            vec![AttachedScrutin {
                scrutin: uid("V1"),
                block: Some(ExplanationBlock {
                    title: EXPLANATIONS.into(),
                    speeches: vec![
                        spoken("PA1", "a1", "Nous voterons pour."),
                        spoken("PA2", "a2", "Nous voterons contre."),
                    ],
                }),
            }]
        );
        assert_eq!(counters, ExplanationCounters::default());
    }

    #[test]
    fn attaches_a_joint_block_to_each_final_vote_that_follows_it() {
        let fingerprints = fingerprints(&[
            scrutin("V1", SITTING, FINAL_A),
            scrutin("V2", SITTING, FINAL_B),
        ]);
        let entries = [
            heading(2, EXPLANATIONS),
            speech("PA1", "a1", "Un mot."),
            announced(FINAL_A),
            announced(FINAL_B),
        ];
        let mut counters = ExplanationCounters::default();

        let attached = fingerprints.attach_explanations(&entries, &mut counters);

        let block = Some(ExplanationBlock {
            title: EXPLANATIONS.into(),
            speeches: vec![spoken("PA1", "a1", "Un mot.")],
        });
        assert_eq!(
            attached,
            vec![
                AttachedScrutin {
                    scrutin: uid("V1"),
                    block: block.clone()
                },
                AttachedScrutin {
                    scrutin: uid("V2"),
                    block
                },
            ]
        );
        assert_eq!(counters, ExplanationCounters::default());
    }

    #[test]
    fn closes_the_block_at_a_scrutin_that_is_not_a_final_vote() {
        let fingerprints = fingerprints(&[
            scrutin_about("V0", SITTING, "l'amendement n° 12 de M. Dupont", AMENDMENT),
            scrutin("V1", SITTING, FINAL_A),
        ]);
        let entries = [
            heading(2, EXPLANATIONS),
            speech("PA1", "a1", "Un mot."),
            announced(AMENDMENT),
            announced(FINAL_A),
        ];
        let mut counters = ExplanationCounters::default();

        let attached = fingerprints.attach_explanations(&entries, &mut counters);

        assert_eq!(
            attached,
            vec![AttachedScrutin {
                scrutin: uid("V1"),
                block: None
            }]
        );
        assert_eq!(
            counters,
            ExplanationCounters {
                blocks_unattached: 1,
                headings_unrecognized: 0
            }
        );
    }

    #[test]
    fn closes_the_block_at_a_first_level_heading() {
        let fingerprints = fingerprints(&[scrutin("V1", SITTING, FINAL_A)]);
        let entries = [
            heading(2, EXPLANATIONS),
            speech("PA1", "a1", "Un mot."),
            heading(1, "Suite de la discussion"),
            announced(FINAL_A),
        ];
        let mut counters = ExplanationCounters::default();

        let attached = fingerprints.attach_explanations(&entries, &mut counters);

        assert_eq!(
            attached,
            vec![AttachedScrutin {
                scrutin: uid("V1"),
                block: None
            }]
        );
        assert_eq!(
            counters,
            ExplanationCounters {
                blocks_unattached: 1,
                headings_unrecognized: 0
            }
        );
    }

    #[test]
    fn counts_a_block_followed_by_no_attached_final_vote() {
        let fingerprints = fingerprints(&[scrutin("V1", SITTING, FINAL_A)]);
        let entries = [
            heading(2, EXPLANATIONS),
            speech("PA1", "a1", "Un mot."),
            announced((1, 1, 1, 0)),
        ];
        let mut counters = ExplanationCounters::default();

        let attached = fingerprints.attach_explanations(&entries, &mut counters);

        assert_eq!(attached, vec![]);
        assert_eq!(
            counters,
            ExplanationCounters {
                blocks_unattached: 1,
                headings_unrecognized: 0
            }
        );
    }

    #[test]
    fn counts_an_unrecognised_explanation_heading_without_opening_a_block() {
        let fingerprints = fingerprints(&[scrutin("V1", SITTING, FINAL_A)]);
        let entries = [
            heading(2, "Explication des groupes"),
            speech("PA1", "a1", "Un mot."),
            announced(FINAL_A),
        ];
        let mut counters = ExplanationCounters::default();

        let attached = fingerprints.attach_explanations(&entries, &mut counters);

        assert_eq!(
            attached,
            vec![AttachedScrutin {
                scrutin: uid("V1"),
                block: None
            }]
        );
        assert_eq!(
            counters,
            ExplanationCounters {
                blocks_unattached: 0,
                headings_unrecognized: 1
            }
        );
    }

    #[test]
    fn keeps_one_explanation_per_speaker_without_interjections_or_presiding_words() {
        let fingerprints = fingerprints(&[scrutin("V1", SITTING, FINAL_A)]);
        let entries = [
            heading(2, EXPLANATIONS),
            presiding_speech("PRES", "p1", "La parole est à M. Durand."),
            speech("PA1", "a1", "Premier orateur."),
            interjection("PA9", "i1", "Très bien !"),
            presiding_speech("PRES", "p2", "Merci."),
            speech("PA2", "a2", "Second orateur."),
            announced(FINAL_A),
        ];
        let mut counters = ExplanationCounters::default();

        let attached = fingerprints.attach_explanations(&entries, &mut counters);

        assert_eq!(
            attached[0].block.as_ref().unwrap().speeches,
            vec![
                spoken("PA1", "a1", "Premier orateur."),
                spoken("PA2", "a2", "Second orateur."),
            ]
        );
    }

    #[test]
    fn merges_a_speaker_resuming_after_an_interruption_and_keeps_bracketed_mentions() {
        let fingerprints = fingerprints(&[scrutin("V1", SITTING, FINAL_A)]);
        let entries = [
            heading(2, EXPLANATIONS),
            speech("PA1", "a1", "Début. (Exclamations)"),
            interjection("PA9", "i1", "Non !"),
            presiding_speech("PRES", "p1", "Silence."),
            speech("PA1", "a2", "Suite. (Applaudissements)"),
            announced(FINAL_A),
        ];
        let mut counters = ExplanationCounters::default();

        let attached = fingerprints.attach_explanations(&entries, &mut counters);

        assert_eq!(
            attached[0].block.as_ref().unwrap().speeches,
            vec![spoken(
                "PA1",
                "a1",
                "Début. (Exclamations)\n\nSuite. (Applaudissements)"
            )]
        );
    }
}
