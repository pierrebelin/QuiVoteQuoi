use std::collections::HashMap;

use chrono::{Days, NaiveDate};

use crate::application::ports::actor_repository::ActorRepository;
use crate::application::ports::scrutin_repository::ScrutinRepository;
use crate::application::ports::sitting_record_source::SittingRecordSource;
use crate::application::ports::vote_explanation_repository::{
    StoredSitting, StoredVersion, VoteExplanationRepository,
};
use crate::domain::actor::ActorUid;
use crate::domain::sitting_record::{
    AttachedScrutin, RecordAttachment, ScrutinSittingKey, SittingRecord, SittingRecordUid,
};
use crate::domain::vote_explanation::{NewVoteExplanation, SpeakerGroup, VoteExplanation};

/// Âge maximal, en jours, d'une séance relue à part: au-delà, l'archive
/// quotidienne a repris depuis longtemps le compte rendu complet.
const REFETCH_WINDOW_DAYS: u64 = 7;

/// Plafond des comptes rendus relus à part par passe: une requête chacun.
const MAX_REFETCHES_PER_PASS: usize = 10;

#[derive(Debug, thiserror::Error)]
pub enum RefreshVoteExplanationsError {
    #[error("{0}")]
    Source(#[from] crate::application::ports::SourceError),
    #[error("{0}")]
    Repository(#[from] crate::application::ports::RepositoryError),
}

/// Ce qu'un rafraîchissement des explications a lu, remplacé et laissé (RM-08, RM-11).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExplanationsSummary {
    pub sittings_read: usize,
    pub sittings_replaced: usize,
    pub unchanged: usize,
    pub unreadable: usize,
    pub explanations: usize,
    /// Comptes rendus redemandés à part après l'archive, lus ou non.
    pub sittings_refetched: usize,
    /// Relectures à part échouées ou illisibles: la séance stockée reste.
    pub refetch_failed: usize,
}

/// Ce qu'une lecture d'un compte rendu a fait de la séance stockée.
enum Reading {
    /// Séance remplacée par cette lecture.
    Replaced(RecordAttachment),
    /// Même version et mêmes clés que la séance stockée.
    Unchanged,
    /// La séance stockée vient d'une lecture plus récente, faite à part: la
    /// garder évite que l'archive, plus ancienne, ne la remplace à chaque passe.
    StoredIsNewer,
}

/// CU-01 — Rafraîchir les explications de vote.
pub struct RefreshVoteExplanations<'a> {
    source: &'a dyn SittingRecordSource,
    repository: &'a dyn VoteExplanationRepository,
    scrutins: &'a dyn ScrutinRepository,
    actors: &'a dyn ActorRepository,
}

impl<'a> RefreshVoteExplanations<'a> {
    pub fn new(
        source: &'a dyn SittingRecordSource,
        repository: &'a dyn VoteExplanationRepository,
        scrutins: &'a dyn ScrutinRepository,
        actors: &'a dyn ActorRepository,
    ) -> Self {
        Self {
            source,
            repository,
            scrutins,
            actors,
        }
    }

    /// Remplace séance par séance celles dont la version a changé; une séance
    /// illisible garde ses explications stockées (RM-08, RM-11). Puis relit à
    /// part les comptes rendus récents dont un vote sur l'ensemble reste sans
    /// résultat rattaché: l'archive n'est republiée qu'une fois par jour.
    pub async fn execute(
        &self,
        legislature: u16,
        today: NaiveDate,
    ) -> Result<ExplanationsSummary, RefreshVoteExplanationsError> {
        let mut records = self.source.fetch_sitting_records(legislature).await?;
        let mut stored = self.repository.stored_versions().await?;
        let keys = self.scrutins.sitting_keys().await?;
        let since = today
            .checked_sub_days(Days::new(REFETCH_WINDOW_DAYS))
            .unwrap_or(NaiveDate::MIN);

        let mut summary = ExplanationsSummary::default();
        let mut candidates: Vec<(NaiveDate, SittingRecordUid)> = Vec::new();
        for item in &mut records {
            let record = match item {
                Ok(record) => record,
                Err(error) => {
                    tracing::warn!("Sitting record unreadable, stored explanations kept: {error}");
                    summary.unreadable += 1;
                    continue;
                }
            };
            summary.sittings_read += 1;

            let reading = self
                .store_if_changed(&record, &keys, &mut stored, &mut summary)
                .await?;
            if record.date() >= since && Self::worth_refetching(&record, reading, &keys, &stored) {
                candidates.push((record.date(), record.uid().clone()));
            }
        }

        self.refetch(candidates, &keys, &mut stored, &mut summary)
            .await?;

        tracing::info!(
            "Vote explanations refreshed: {} sittings read, {} replaced, {} unchanged, {} unreadable, {} refetched ({} failed)",
            summary.sittings_read,
            summary.sittings_replaced,
            summary.unchanged,
            summary.unreadable,
            summary.sittings_refetched,
            summary.refetch_failed
        );
        Ok(summary)
    }

    /// Un vote sur l'ensemble de la séance reste sans résultat rattaché, ou
    /// la séance stockée, plus récente, a été rattachée avec d'autres clés.
    fn worth_refetching(
        record: &SittingRecord,
        reading: Reading,
        keys: &[ScrutinSittingKey],
        stored: &HashMap<SittingRecordUid, StoredVersion>,
    ) -> bool {
        let attachment = match reading {
            Reading::Replaced(attachment) => attachment,
            Reading::Unchanged => record.attach(keys),
            Reading::StoredIsNewer => {
                let keys_changed = stored
                    .get(record.uid())
                    .is_some_and(|version| version.keys != record.keys_fingerprint(keys));
                if keys_changed {
                    return true;
                }
                // La lecture stockée prolonge celle-ci: si celle-ci rattache
                // déjà tous les votes sur l'ensemble, la stockée aussi.
                record.attach(keys)
            }
        };
        record.leaves_a_final_vote_unattached(&attachment, keys)
    }

    /// Relit à part les plus récentes des séances retenues, dans la limite
    /// d'une passe; un échec garde la séance stockée et n'arrête pas la passe.
    async fn refetch(
        &self,
        mut candidates: Vec<(NaiveDate, SittingRecordUid)>,
        keys: &[ScrutinSittingKey],
        stored: &mut HashMap<SittingRecordUid, StoredVersion>,
        summary: &mut ExplanationsSummary,
    ) -> Result<(), RefreshVoteExplanationsError> {
        candidates.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.as_str().cmp(b.1.as_str())));
        if candidates.len() > MAX_REFETCHES_PER_PASS {
            tracing::info!(
                "{} sitting records left for a later refetch",
                candidates.len() - MAX_REFETCHES_PER_PASS
            );
        }
        for (_, uid) in candidates.into_iter().take(MAX_REFETCHES_PER_PASS) {
            summary.sittings_refetched += 1;
            let record = match self.source.fetch_sitting_record(&uid).await {
                Ok(record) if record.uid() == &uid => record,
                Ok(record) => {
                    tracing::warn!(
                        "Sitting record {uid} refetched as {}, stored explanations kept",
                        record.uid()
                    );
                    summary.refetch_failed += 1;
                    continue;
                }
                Err(error) => {
                    tracing::warn!(
                        "Sitting record {uid} refetch failed, stored explanations kept: {error}"
                    );
                    summary.refetch_failed += 1;
                    continue;
                }
            };
            self.store_if_changed(&record, keys, stored, summary)
                .await?;
        }
        Ok(())
    }

    /// Remplace la séance stockée si cette lecture est plus récente ou si les
    /// clés de ses scrutins ont changé (RM-08, RM-11).
    async fn store_if_changed(
        &self,
        record: &SittingRecord,
        keys: &[ScrutinSittingKey],
        stored: &mut HashMap<SittingRecordUid, StoredVersion>,
        summary: &mut ExplanationsSummary,
    ) -> Result<Reading, RefreshVoteExplanationsError> {
        let fingerprint = record.keys_fingerprint(keys);
        if let Some(version) = stored.get(record.uid()) {
            if version.generated_at > record.generated_at() {
                summary.unchanged += 1;
                return Ok(Reading::StoredIsNewer);
            }
            if version.generated_at == record.generated_at() && version.keys == fingerprint {
                summary.unchanged += 1;
                return Ok(Reading::Unchanged);
            }
        }

        let attachment = record.attach(keys);
        let explanations = self.explanations_of(record, &attachment.attached).await?;
        summary.explanations += explanations.len();
        self.repository
            .replace_sitting(&StoredSitting {
                record_uid: record.uid().clone(),
                sitting_ref: record.sitting_ref().clone(),
                date: record.date(),
                version: record.version().to_string(),
                generated_at: record.generated_at(),
                keys: fingerprint.clone(),
                counters: attachment.results,
                block_counters: attachment.explanations,
                attached: attachment
                    .attached
                    .iter()
                    .map(|scrutin| (scrutin.scrutin.clone(), scrutin.block.is_some()))
                    .collect(),
                explanations,
            })
            .await?;
        stored.insert(
            record.uid().clone(),
            StoredVersion {
                generated_at: record.generated_at(),
                keys: fingerprint,
            },
        );
        summary.sittings_replaced += 1;
        Ok(Reading::Replaced(attachment))
    }

    /// Une explication par orateur de chaque bloc rattaché, groupe établi à la
    /// date du scrutin (RM-06).
    async fn explanations_of(
        &self,
        record: &SittingRecord,
        attached: &[AttachedScrutin],
    ) -> Result<Vec<VoteExplanation>, RefreshVoteExplanationsError> {
        let speakers: Vec<ActorUid> = attached
            .iter()
            .filter_map(|scrutin| scrutin.block.as_ref())
            .flat_map(|block| block.speeches.iter().map(|speech| speech.actor_uid.clone()))
            .collect();
        if speakers.is_empty() {
            return Ok(Vec::new());
        }
        let directory = self.actors.load_directory_for(&speakers).await?;

        let mut explanations = Vec::new();
        for scrutin in attached {
            let Some(block) = &scrutin.block else {
                continue;
            };
            let stored = self.scrutins.by_uid(&scrutin.scrutin).await?;
            for (index, speech) in block.speeches.iter().enumerate() {
                let group = stored.as_ref().map_or(SpeakerGroup::Undetermined, |s| {
                    SpeakerGroup::establish(s, &directory, &speech.actor_uid)
                });
                explanations.push(VoteExplanation::new(NewVoteExplanation {
                    scrutin_uid: scrutin.scrutin.clone(),
                    sitting_record_uid: record.uid().clone(),
                    rank: u16::try_from(index + 1).unwrap_or(u16::MAX),
                    speaker_uid: speech.actor_uid.clone(),
                    speaker_name: directory
                        .actor(&speech.actor_uid)
                        .map(|actor| actor.full_name()),
                    group,
                    anchor: speech.anchor.clone(),
                    heading: block.title.clone(),
                    text: speech.text.clone(),
                }));
            }
        }
        Ok(explanations)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, NaiveDate, TimeZone, Utc};

    use super::*;
    use crate::application::ports::vote_explanation_repository::StoredVersion;
    use crate::application::use_cases::explanation_fakes::InMemoryVoteExplanations;
    use crate::application::use_cases::read_fakes::FakeActors;
    use crate::application::use_cases::sitting_record_fakes::{
        FakeSittingRecords, FakeSittingScrutins,
    };
    use crate::domain::actor::{ActorUid, GroupUid};
    use crate::domain::scrutin::{
        BallotType, NewScrutin, NominalVote, Outcome, Scrutin, ScrutinUid, VotePosition,
        VoteSynthesis, VoteTally,
    };
    use crate::domain::sitting_record::{
        AnnouncedResult, NewSittingRecord, ScrutinSittingKey, SittingEntry, SittingKeysFingerprint,
        SittingRecord, SittingRecordUid, SittingRef, SpeechAnchor,
    };
    use crate::domain::vote_explanation::SpeakerGroup;

    const SITTING: &str = "RUANR5L17S2025IDS29344";
    const OTHER_SITTING: &str = "RUANR5L17S2025IDS55555";
    const LEGISLATURE: u16 = 17;

    fn at(hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2025, 3, 10, hour, 0, 0).unwrap()
    }

    fn day(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2025, 3, day).unwrap()
    }

    /// Lendemain des séances des fixtures.
    fn today() -> NaiveDate {
        day(11)
    }

    fn record_uid(raw: &str) -> SittingRecordUid {
        SittingRecordUid::new(raw.into()).unwrap()
    }

    fn announced() -> AnnouncedResult {
        AnnouncedResult {
            voters: 10,
            expressed: 9,
            votes_for: 6,
            votes_against: 3,
        }
    }

    fn speech(actor: &str, anchor: &str, text: &str) -> SittingEntry {
        SittingEntry::Speech {
            actor_uid: ActorUid::new(actor.into()).unwrap(),
            anchor: SpeechAnchor::new(anchor.into()).unwrap(),
            presiding: false,
            interjection: false,
            text: text.into(),
        }
    }

    /// Séance `uid` de la référence donnée: un bloc de deux orateurs, puis le résultat.
    fn record(uid: &str, sitting: &str, hour: u32) -> SittingRecord {
        SittingRecord::new(NewSittingRecord {
            uid: record_uid(uid),
            sitting_ref: SittingRef::new(sitting.into()).unwrap(),
            date: NaiveDate::from_ymd_opt(2025, 3, 10).unwrap(),
            version: "1".into(),
            generated_at: at(hour),
            entries: vec![
                SittingEntry::Heading {
                    level: 2,
                    text: "Explications de vote".into(),
                },
                speech("PA1", "A1", "Premier texte."),
                speech("PA2", "A2", "Second texte."),
                SittingEntry::Result(announced()),
            ],
        })
    }

    /// Compte rendu provisoire `uid` du jour donné: le bloc d'explications est
    /// là, le résultat du vote sur l'ensemble pas encore.
    fn truncated_on(uid: &str, date: NaiveDate, hour: u32) -> SittingRecord {
        let full = record(uid, SITTING, hour);
        SittingRecord::new(NewSittingRecord {
            uid: full.uid().clone(),
            sitting_ref: full.sitting_ref().clone(),
            date,
            version: "avant_JO".into(),
            generated_at: full.generated_at(),
            entries: full.entries()[..3].to_vec(),
        })
    }

    fn truncated(uid: &str, hour: u32) -> SittingRecord {
        truncated_on(uid, day(10), hour)
    }

    fn version_of(record: &SittingRecord) -> StoredVersion {
        StoredVersion {
            generated_at: record.generated_at(),
            keys: record.keys_fingerprint(&scrutins_of(SITTING).keys),
        }
    }

    fn key(uid: &str, sitting: &str) -> ScrutinSittingKey {
        ScrutinSittingKey {
            uid: ScrutinUid::new(uid.into()).unwrap(),
            sitting_ref: SittingRef::new(sitting.into()).unwrap(),
            result: announced(),
            final_vote: true,
        }
    }

    /// Vote sur l'ensemble `uid` dont la source range `PA1` dans le groupe `PO_A`.
    fn scrutin(uid: &str, sitting: &str) -> Scrutin {
        Scrutin::new(NewScrutin {
            uid: ScrutinUid::new(uid.into()).unwrap(),
            number: "1".into(),
            legislature: LEGISLATURE,
            date: NaiveDate::from_ymd_opt(2025, 3, 10).unwrap(),
            session_ref: None,
            sitting_ref: Some(sitting.into()),
            place: None,
            ballot_type: BallotType::new("SPO".into(), "scrutin public ordinaire".into(), None)
                .unwrap(),
            outcome: Outcome::new("adopté".into(), "adopté".into()).unwrap(),
            requester: None,
            subject: "l'ensemble du projet de loi".into(),
            synthesis: VoteSynthesis {
                voters: 10,
                expressed: 9,
                required: 5,
                announcement: "adopté".into(),
                tally: VoteTally {
                    votes_for: 6,
                    votes_against: 3,
                    ..VoteTally::default()
                },
            },
            group_tallies: vec![],
            nominal_votes: vec![NominalVote {
                actor_uid: ActorUid::new("PA1".into()).unwrap(),
                group_uid: Some(GroupUid::new("PO_A".into()).unwrap()),
                position: VotePosition::For,
                cause: None,
                by_delegation: false,
                seat: None,
            }],
            corrections: vec![],
            dossier: None,
        })
        .unwrap()
    }

    fn scrutins_of(sitting: &str) -> FakeSittingScrutins {
        FakeSittingScrutins {
            keys: vec![key("VTANR5L17V1", sitting)],
            scrutins: vec![scrutin("VTANR5L17V1", sitting)],
        }
    }

    fn actors() -> FakeActors {
        FakeActors { group_color: None }
    }

    #[tokio::test]
    async fn stores_each_speaker_explanation_with_name_group_and_attachment() {
        let source = FakeSittingRecords {
            items: vec![Ok(record("CR1", SITTING, 9))],
            ..Default::default()
        };
        let repository = InMemoryVoteExplanations::default();
        let scrutins = scrutins_of(SITTING);
        let actors = actors();

        let summary = RefreshVoteExplanations::new(&source, &repository, &scrutins, &actors)
            .execute(LEGISLATURE, today())
            .await
            .unwrap();

        assert_eq!(
            summary,
            ExplanationsSummary {
                sittings_read: 1,
                sittings_replaced: 1,
                unchanged: 0,
                unreadable: 0,
                explanations: 2,
                sittings_refetched: 0,
                refetch_failed: 0,
            }
        );
        let written = repository.written();
        assert_eq!(written.len(), 1);
        assert_eq!(
            written[0].attached,
            vec![(ScrutinUid::new("VTANR5L17V1".into()).unwrap(), true)]
        );
        let explanations = &written[0].explanations;
        assert_eq!(explanations.len(), 2);
        assert_eq!(explanations[0].scrutin_uid().as_str(), "VTANR5L17V1");
        assert_eq!(explanations[0].rank(), 1);
        assert_eq!(explanations[0].speaker_name(), Some("Jean Dupont"));
        assert_eq!(
            explanations[0].group(),
            &SpeakerGroup::FromScrutin(GroupUid::new("PO_A".into()).unwrap())
        );
        assert_eq!(explanations[0].anchor().as_str(), "A1");
        assert_eq!(explanations[0].heading(), "Explications de vote");
        assert_eq!(explanations[0].text(), "Premier texte.");
        assert_eq!(explanations[1].rank(), 2);
        assert_eq!(explanations[1].speaker_name(), None);
        assert_eq!(explanations[1].group(), &SpeakerGroup::Undetermined);
    }

    #[tokio::test]
    async fn replaces_a_sitting_whose_generation_time_changed() {
        let keys = [key("VTANR5L17V1", SITTING)];
        let stored = StoredVersion {
            generated_at: at(8),
            keys: record("CR1", SITTING, 9).keys_fingerprint(&keys),
        };
        let source = FakeSittingRecords {
            items: vec![Ok(record("CR1", SITTING, 9))],
            ..Default::default()
        };
        let repository = InMemoryVoteExplanations::with_versions(vec![(record_uid("CR1"), stored)]);
        let scrutins = scrutins_of(SITTING);
        let actors = actors();

        let summary = RefreshVoteExplanations::new(&source, &repository, &scrutins, &actors)
            .execute(LEGISLATURE, today())
            .await
            .unwrap();

        assert_eq!(summary.sittings_replaced, 1);
        assert_eq!(summary.unchanged, 0);
        assert_eq!(repository.written().len(), 1);
    }

    #[tokio::test]
    async fn replaces_a_sitting_whose_scrutin_keys_changed_since_the_last_reading() {
        let stored = StoredVersion {
            generated_at: at(9),
            keys: SittingKeysFingerprint::of(&[]),
        };
        let source = FakeSittingRecords {
            items: vec![Ok(record("CR1", SITTING, 9))],
            ..Default::default()
        };
        let repository = InMemoryVoteExplanations::with_versions(vec![(record_uid("CR1"), stored)]);
        let scrutins = scrutins_of(SITTING);
        let actors = actors();

        let summary = RefreshVoteExplanations::new(&source, &repository, &scrutins, &actors)
            .execute(LEGISLATURE, today())
            .await
            .unwrap();

        assert_eq!(summary.sittings_replaced, 1);
        assert_eq!(summary.unchanged, 0);
        assert_eq!(repository.written().len(), 1);
    }

    #[tokio::test]
    async fn skips_a_sitting_whose_generation_time_and_scrutin_keys_are_unchanged() {
        let keys = [key("VTANR5L17V1", SITTING)];
        let stored = StoredVersion {
            generated_at: at(9),
            keys: record("CR1", SITTING, 9).keys_fingerprint(&keys),
        };
        let source = FakeSittingRecords {
            items: vec![Ok(record("CR1", SITTING, 9))],
            ..Default::default()
        };
        let repository = InMemoryVoteExplanations::with_versions(vec![(record_uid("CR1"), stored)]);
        let scrutins = scrutins_of(SITTING);
        let actors = actors();

        let summary = RefreshVoteExplanations::new(&source, &repository, &scrutins, &actors)
            .execute(LEGISLATURE, today())
            .await
            .unwrap();

        assert_eq!(
            summary,
            ExplanationsSummary {
                sittings_read: 1,
                sittings_replaced: 0,
                unchanged: 1,
                unreadable: 0,
                explanations: 0,
                sittings_refetched: 0,
                refetch_failed: 0,
            }
        );
        assert!(repository.written().is_empty());
    }

    #[tokio::test]
    async fn keeps_the_stored_explanations_of_an_unreadable_sitting_and_reads_the_others() {
        let old = StoredVersion {
            generated_at: at(1),
            keys: SittingKeysFingerprint::of(&[]),
        };
        let source = FakeSittingRecords {
            items: vec![
                Err("séance illisible".to_string()),
                Ok(record("CR2", OTHER_SITTING, 9)),
            ],
            ..Default::default()
        };
        let repository = InMemoryVoteExplanations::with_versions(vec![(record_uid("CRBAD"), old)]);
        let scrutins = scrutins_of(OTHER_SITTING);
        let actors = actors();

        let summary = RefreshVoteExplanations::new(&source, &repository, &scrutins, &actors)
            .execute(LEGISLATURE, today())
            .await
            .unwrap();

        assert_eq!(summary.unreadable, 1);
        assert_eq!(summary.sittings_read, 1);
        assert_eq!(summary.sittings_replaced, 1);
        let written: Vec<String> = repository
            .written()
            .iter()
            .map(|sitting| sitting.record_uid.to_string())
            .collect();
        assert_eq!(written, vec!["CR2".to_string()]);
    }

    #[tokio::test]
    async fn returns_the_source_error_when_the_archive_cannot_be_opened() {
        let source = FakeSittingRecords {
            open_error: Some("archive absente".to_string()),
            ..Default::default()
        };
        let repository = InMemoryVoteExplanations::default();
        let scrutins = scrutins_of(SITTING);
        let actors = actors();

        let outcome = RefreshVoteExplanations::new(&source, &repository, &scrutins, &actors)
            .execute(LEGISLATURE, today())
            .await;

        assert!(matches!(
            outcome,
            Err(RefreshVoteExplanationsError::Source(_))
        ));
    }

    #[tokio::test]
    async fn returns_the_repository_error_when_a_sitting_cannot_be_written() {
        let source = FakeSittingRecords {
            items: vec![Ok(record("CR1", SITTING, 9))],
            ..Default::default()
        };
        let repository = InMemoryVoteExplanations::failing_on(record_uid("CR1"));
        let scrutins = scrutins_of(SITTING);
        let actors = actors();

        let outcome = RefreshVoteExplanations::new(&source, &repository, &scrutins, &actors)
            .execute(LEGISLATURE, today())
            .await;

        assert!(matches!(
            outcome,
            Err(RefreshVoteExplanationsError::Repository(_))
        ));
        assert!(repository.written().is_empty());
    }

    #[tokio::test]
    async fn refetches_a_recent_sitting_whose_final_vote_is_unattached_and_stores_the_fuller_version(
    ) {
        let source = FakeSittingRecords {
            items: vec![Ok(truncated("CR1", 9))],
            singles: vec![Ok(record("CR1", SITTING, 11))],
            ..Default::default()
        };
        let repository = InMemoryVoteExplanations::default();
        let scrutins = scrutins_of(SITTING);
        let actors = actors();

        let summary = RefreshVoteExplanations::new(&source, &repository, &scrutins, &actors)
            .execute(LEGISLATURE, today())
            .await
            .unwrap();

        assert_eq!(source.requested(), vec![record_uid("CR1")]);
        assert_eq!(summary.sittings_refetched, 1);
        assert_eq!(summary.sittings_replaced, 2);
        assert_eq!(summary.explanations, 2);
        let written = repository.written();
        let last = written.last().unwrap();
        assert_eq!(last.generated_at, at(11));
        assert_eq!(
            last.attached,
            vec![(ScrutinUid::new("VTANR5L17V1".into()).unwrap(), true)]
        );
        assert_eq!(last.explanations.len(), 2);
    }

    #[tokio::test]
    async fn does_not_refetch_a_sitting_whose_final_votes_are_all_attached() {
        let source = FakeSittingRecords {
            items: vec![Ok(record("CR1", SITTING, 9))],
            ..Default::default()
        };
        let repository = InMemoryVoteExplanations::default();
        let scrutins = scrutins_of(SITTING);
        let actors = actors();

        let summary = RefreshVoteExplanations::new(&source, &repository, &scrutins, &actors)
            .execute(LEGISLATURE, today())
            .await
            .unwrap();

        assert!(source.requested().is_empty());
        assert_eq!(summary.sittings_refetched, 0);
    }

    #[tokio::test]
    async fn does_not_refetch_a_sitting_older_than_the_window() {
        let source = FakeSittingRecords {
            items: vec![Ok(truncated("CR1", 9))],
            singles: vec![Ok(record("CR1", SITTING, 11))],
            ..Default::default()
        };
        let repository = InMemoryVoteExplanations::default();
        let scrutins = scrutins_of(SITTING);
        let actors = actors();

        let summary = RefreshVoteExplanations::new(&source, &repository, &scrutins, &actors)
            .execute(
                LEGISLATURE,
                day(10) + chrono::Days::new(REFETCH_WINDOW_DAYS + 1),
            )
            .await
            .unwrap();

        assert!(source.requested().is_empty());
        assert_eq!(summary.sittings_refetched, 0);
    }

    #[tokio::test]
    async fn keeps_the_stored_explanations_and_finishes_the_pass_when_the_refetch_fails() {
        let archived = truncated("CR1", 9);
        let source = FakeSittingRecords {
            items: vec![Ok(archived.clone())],
            singles: vec![Err((
                record_uid("CR1"),
                "compte rendu illisible".to_string(),
            ))],
            ..Default::default()
        };
        let repository = InMemoryVoteExplanations::with_versions(vec![(
            record_uid("CR1"),
            version_of(&archived),
        )]);
        let scrutins = scrutins_of(SITTING);
        let actors = actors();

        let summary = RefreshVoteExplanations::new(&source, &repository, &scrutins, &actors)
            .execute(LEGISLATURE, today())
            .await
            .unwrap();

        assert_eq!(source.requested(), vec![record_uid("CR1")]);
        assert_eq!(summary.sittings_refetched, 1);
        assert_eq!(summary.refetch_failed, 1);
        assert_eq!(summary.unchanged, 1);
        assert!(repository.written().is_empty());
    }

    #[tokio::test]
    async fn keeps_a_newer_stored_reading_over_the_older_archive_copy() {
        let newer = record("CR1", SITTING, 11);
        let source = FakeSittingRecords {
            items: vec![Ok(truncated("CR1", 9))],
            singles: vec![Ok(newer.clone())],
            ..Default::default()
        };
        let repository =
            InMemoryVoteExplanations::with_versions(vec![(record_uid("CR1"), version_of(&newer))]);
        let scrutins = scrutins_of(SITTING);
        let actors = actors();

        let summary = RefreshVoteExplanations::new(&source, &repository, &scrutins, &actors)
            .execute(LEGISLATURE, today())
            .await
            .unwrap();

        assert!(repository.written().is_empty());
        assert_eq!(summary.sittings_replaced, 0);
        assert_eq!(summary.unchanged, 2);
        assert_eq!(summary.sittings_refetched, 1);
    }

    #[tokio::test]
    async fn refetches_at_most_the_bound_per_pass_starting_with_the_most_recent_sittings() {
        let mut items: Vec<Result<SittingRecord, String>> =
            vec![Ok(truncated_on("CROLD", day(9), 9))];
        items.extend((0..MAX_REFETCHES_PER_PASS).map(|n| Ok(truncated(&format!("CR{n:02}"), 9))));
        let source = FakeSittingRecords {
            items,
            ..Default::default()
        };
        let repository = InMemoryVoteExplanations::default();
        let scrutins = scrutins_of(SITTING);
        let actors = actors();

        let summary = RefreshVoteExplanations::new(&source, &repository, &scrutins, &actors)
            .execute(LEGISLATURE, today())
            .await
            .unwrap();

        let requested = source.requested();
        assert_eq!(requested.len(), MAX_REFETCHES_PER_PASS);
        assert!(!requested.contains(&record_uid("CROLD")));
        assert_eq!(summary.sittings_refetched, MAX_REFETCHES_PER_PASS);
        assert_eq!(summary.refetch_failed, MAX_REFETCHES_PER_PASS);
    }
}
