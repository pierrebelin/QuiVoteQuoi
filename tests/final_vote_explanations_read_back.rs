//! La liste des votes sur l'ensemble lit en une requête les explications de
//! toute une page, avec le vote nominatif de chaque orateur (RM-09).
//!
//! Les migrations sont jouées sur la base visée, qui doit donc être jetable :
//!
//! ```text
//! DATABASE_URL=postgresql://... \
//!   cargo test --test final_vote_explanations_read_back -- --ignored
//! ```

use chrono::{NaiveDate, TimeZone, Utc};
use quivotequoi::application::ports::vote_explanation_repository::{
    FinalVoteExplanationsRead, StoredSitting, VoteExplanationRepository,
};
use quivotequoi::domain::actor::{ActorUid, GroupUid};
use quivotequoi::domain::scrutin::{ScrutinUid, VotePosition};
use quivotequoi::domain::sitting_record::{
    AttachmentCounters, ExplanationCounters, SittingKeysFingerprint, SittingRecordUid, SittingRef,
    SpeechAnchor,
};
use quivotequoi::domain::vote_explanation::{NewVoteExplanation, SpeakerGroup, VoteExplanation};
use quivotequoi::infrastructure::persistence::pg_vote_explanation_repository::PgVoteExplanationRepository;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

const SITTING_REF: &str = "RUFINALREADBACK";
const OLD_RECORD: &str = "CRFINALREADBACKOLD";
const RECORD: &str = "CRFINALREADBACK";
/// Rattaché, deux explications.
const EXPLAINED: &str = "VTFINALREADBACK1";
/// Rattaché, bloc sans orateur.
const SILENT: &str = "VTFINALREADBACK2";
/// Même séance, absent du compte rendu.
const UNATTACHED: &str = "VTFINALREADBACK3";
/// Sans référence de séance.
const NO_SITTING: &str = "VTFINALREADBACK4";
const SCRUTINS: [&str; 4] = [EXPLAINED, SILENT, UNATTACHED, NO_SITTING];

fn uid(raw: &str) -> ScrutinUid {
    ScrutinUid::new(raw.into()).unwrap()
}

fn explanation(
    record: &str,
    rank: u16,
    speaker: &str,
    group: SpeakerGroup,
    text: &str,
) -> VoteExplanation {
    VoteExplanation::new(NewVoteExplanation {
        scrutin_uid: uid(EXPLAINED),
        sitting_record_uid: SittingRecordUid::new(record.into()).unwrap(),
        rank,
        speaker_uid: ActorUid::new(speaker.into()).unwrap(),
        speaker_name: Some(format!("Orateur {speaker}")),
        group,
        anchor: SpeechAnchor::new(format!("ancre-{rank}")).unwrap(),
        heading: "Explications de vote".into(),
        text: text.into(),
    })
}

fn sitting(
    record: &str,
    hour: u32,
    attached: Vec<(&str, bool)>,
    explanations: Vec<VoteExplanation>,
) -> StoredSitting {
    StoredSitting {
        record_uid: SittingRecordUid::new(record.into()).unwrap(),
        sitting_ref: SittingRef::new(SITTING_REF.into()).unwrap(),
        date: NaiveDate::from_ymd_opt(2026, 10, 29).unwrap(),
        version: "v-test".into(),
        generated_at: Utc.with_ymd_and_hms(2026, 10, 30, hour, 0, 0).unwrap(),
        keys: SittingKeysFingerprint::of(&[]),
        counters: AttachmentCounters::default(),
        block_counters: ExplanationCounters::default(),
        attached: attached
            .into_iter()
            .map(|(scrutin, has_block)| (uid(scrutin), has_block))
            .collect(),
        explanations,
    }
}

async fn insert_scrutin(pool: &PgPool, scrutin: &str, sitting_ref: Option<&str>) {
    sqlx::query(
        "INSERT INTO scrutins (uid, number, legislature, scrutin_date, sitting_ref,
             ballot_type_code, ballot_type_label, outcome_code, outcome_label, subject,
             voters, expressed, required, announcement,
             votes_for, votes_against, abstentions, not_voting, voluntary_not_voting)
         VALUES ($1, '1', 17, '2026-10-29', $2, 'SPO', 'scrutin public ordinaire',
             'adopté', 'adopté', 'l''ensemble de la proposition de loi',
             1, 1, 1, 'adopté', 1, 0, 0, 0, 0)",
    )
    .bind(scrutin)
    .bind(sitting_ref)
    .execute(pool)
    .await
    .unwrap();
}

async fn connect() -> PgPool {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();
    remove_fixture(&pool).await;
    pool
}

async fn remove_fixture(pool: &PgPool) {
    sqlx::query("DELETE FROM sitting_records WHERE uid IN ($1, $2)")
        .bind(OLD_RECORD)
        .bind(RECORD)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM scrutins WHERE uid = ANY($1)")
        .bind(SCRUTINS.to_vec())
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "base Postgres jetable à poser dans DATABASE_URL"]
async fn reads_the_explanations_of_a_page_from_the_latest_report_with_each_speaker_vote() {
    let pool = connect().await;
    let repository = PgVoteExplanationRepository::new(pool.clone());
    insert_scrutin(&pool, EXPLAINED, Some(SITTING_REF)).await;
    insert_scrutin(&pool, SILENT, Some(SITTING_REF)).await;
    insert_scrutin(&pool, UNATTACHED, Some(SITTING_REF)).await;
    insert_scrutin(&pool, NO_SITTING, None).await;
    sqlx::query(
        "INSERT INTO scrutin_votes (scrutin_uid, actor_uid, group_uid, position, by_delegation)
         VALUES ($1, 'PAFINALREADBACK1', 'POFINALREADBACK', 'abstention', FALSE)",
    )
    .bind(EXPLAINED)
    .execute(&pool)
    .await
    .unwrap();
    let group = GroupUid::new("POFINALREADBACK".into()).unwrap();
    repository
        .replace_sitting(&sitting(
            OLD_RECORD,
            8,
            vec![(EXPLAINED, true)],
            vec![explanation(
                OLD_RECORD,
                1,
                "PAFINALREADBACK9",
                SpeakerGroup::Undetermined,
                "Version périmée.",
            )],
        ))
        .await
        .unwrap();
    repository
        .replace_sitting(&sitting(
            RECORD,
            20,
            vec![(EXPLAINED, true), (SILENT, false)],
            vec![
                explanation(
                    RECORD,
                    2,
                    "PAFINALREADBACK2",
                    SpeakerGroup::Undetermined,
                    "Sans vote publié.",
                ),
                explanation(
                    RECORD,
                    1,
                    "PAFINALREADBACK1",
                    SpeakerGroup::FromScrutin(group.clone()),
                    "Nous nous abstiendrons.",
                ),
            ],
        ))
        .await
        .unwrap();

    let mut reads = repository
        .final_vote_explanations(&SCRUTINS.map(uid))
        .await
        .unwrap();
    remove_fixture(&pool).await;
    reads.sort_by(|a, b| a.scrutin_uid.as_str().cmp(b.scrutin_uid.as_str()));

    assert_eq!(reads.len(), 4);
    let explained = &reads[0];
    assert_eq!(explained.scrutin_uid, uid(EXPLAINED));
    assert!(explained.sitting_known && explained.report_stored);
    assert_eq!(explained.attached, Some(true));
    let ranks: Vec<u16> = explained.explanations.iter().map(|e| e.rank).collect();
    assert_eq!(ranks, vec![1, 2]);
    assert_eq!(explained.explanations[0].group_uid, Some(group));
    assert_eq!(
        explained.explanations[0].speaker_position,
        Some(VotePosition::Abstention)
    );
    assert_eq!(explained.explanations[0].text, "Nous nous abstiendrons.");
    assert_eq!(explained.explanations[1].group_uid, None);
    assert_eq!(explained.explanations[1].speaker_position, None);

    assert_eq!(
        reads[1],
        FinalVoteExplanationsRead {
            scrutin_uid: uid(SILENT),
            sitting_known: true,
            report_stored: true,
            attached: Some(false),
            explanations: vec![],
        }
    );
    assert_eq!(reads[2].attached, None);
    assert!(reads[2].report_stored);
    assert!(!reads[3].sitting_known);
    assert!(!reads[3].report_stored);
}
