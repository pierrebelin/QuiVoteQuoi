//! Le dépôt Postgres des explications de vote remplace une séance entière en
//! une transaction (RM-08) et relit les versions stockées (RM-11).
//!
//! Les migrations sont jouées sur la base visée, qui doit donc être jetable :
//!
//! ```text
//! DATABASE_URL=postgresql://... \
//!   cargo test --test vote_explanations_read_back -- --ignored
//! ```

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use quivotequoi::application::ports::vote_explanation_repository::{
    StoredSitting, VoteExplanationRepository,
};
use quivotequoi::domain::actor::ActorUid;
use quivotequoi::domain::scrutin::ScrutinUid;
use quivotequoi::domain::sitting_record::{
    AnnouncedResult, AttachmentCounters, ExplanationCounters, ScrutinSittingKey,
    SittingKeysFingerprint, SittingRecordUid, SittingRef, SpeechAnchor,
};
use quivotequoi::domain::vote_explanation::{NewVoteExplanation, SpeakerGroup, VoteExplanation};
use quivotequoi::infrastructure::persistence::pg_vote_explanation_repository::PgVoteExplanationRepository;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

const RECORD_A: &str = "CRREADBACKA";
const RECORD_B: &str = "CRREADBACKB";
const RECORD_C: &str = "CRREADBACKC";
const SITTING_REF: &str = "RUREADBACK1";

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 2, hour, 30, 0).unwrap()
}

fn key(scrutin: &str, voters: u32) -> ScrutinSittingKey {
    ScrutinSittingKey::new(
        ScrutinUid::new(scrutin.into()).unwrap(),
        SittingRef::new(SITTING_REF.into()).unwrap(),
        AnnouncedResult {
            voters,
            expressed: voters,
            votes_for: voters,
            votes_against: 0,
        },
        "l'ensemble du projet de loi",
    )
}

fn explanation(record: &str, scrutin: &str, rank: u16, anchor: &str, text: &str) -> VoteExplanation {
    VoteExplanation::new(NewVoteExplanation {
        scrutin_uid: ScrutinUid::new(scrutin.into()).unwrap(),
        sitting_record_uid: SittingRecordUid::new(record.into()).unwrap(),
        rank,
        speaker_uid: ActorUid::new("PAREADBACK".into()).unwrap(),
        speaker_name: Some("Jeanne Durand".into()),
        group: SpeakerGroup::Undetermined,
        anchor: SpeechAnchor::new(anchor.into()).unwrap(),
        heading: "Explications de vote".into(),
        text: text.into(),
    })
}

fn sitting(
    record: &str,
    generated_at: DateTime<Utc>,
    keys: SittingKeysFingerprint,
    attached: Vec<(&str, bool)>,
    explanations: Vec<VoteExplanation>,
) -> StoredSitting {
    StoredSitting {
        record_uid: SittingRecordUid::new(record.into()).unwrap(),
        sitting_ref: SittingRef::new(SITTING_REF.into()).unwrap(),
        date: NaiveDate::from_ymd_opt(2026, 11, 2).unwrap(),
        version: "v-test".into(),
        generated_at,
        keys,
        counters: AttachmentCounters::default(),
        block_counters: ExplanationCounters::default(),
        attached: attached
            .into_iter()
            .map(|(uid, has_block)| (ScrutinUid::new(uid.into()).unwrap(), has_block))
            .collect(),
        explanations,
    }
}

async fn connect(records: [&str; 2]) -> PgPool {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();
    remove_fixture(&pool, records).await;
    pool
}

/// Chaque test a ses propres séances: les tests tournent en parallèle.
async fn remove_fixture(pool: &PgPool, records: [&str; 2]) {
    sqlx::query("DELETE FROM sitting_records WHERE uid IN ($1, $2)")
        .bind(records[0])
        .bind(records[1])
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "base Postgres jetable à poser dans DATABASE_URL"]
async fn a_replaced_sitting_keeps_only_its_new_attachments_and_explanations() {
    let pool = connect([RECORD_A, RECORD_A]).await;
    let repository = PgVoteExplanationRepository::new(pool.clone());
    let keys = SittingKeysFingerprint::of(&[key("VTREADBACK1", 10)]);
    repository
        .replace_sitting(&sitting(
            RECORD_A,
            at(9),
            keys.clone(),
            vec![("VTREADBACK1", true), ("VTREADBACK2", false)],
            vec![
                explanation(RECORD_A, "VTREADBACK1", 0, "ANCHOR-OLD-1", "Premier texte."),
                explanation(RECORD_A, "VTREADBACK1", 1, "ANCHOR-OLD-2", "Second texte."),
            ],
        ))
        .await
        .unwrap();

    repository
        .replace_sitting(&sitting(
            RECORD_A,
            at(11),
            keys,
            vec![("VTREADBACK3", true)],
            vec![explanation(
                RECORD_A,
                "VTREADBACK3",
                0,
                "ANCHOR-NEW-1",
                "Texte remplaçant.",
            )],
        ))
        .await
        .unwrap();

    let attached: Vec<String> = sqlx::query_scalar(
        "SELECT scrutin_uid FROM sitting_record_scrutins
         WHERE sitting_record_uid = $1 ORDER BY scrutin_uid",
    )
    .bind(RECORD_A)
    .fetch_all(&pool)
    .await
    .unwrap();
    let explanations: Vec<(String, String)> = sqlx::query_as(
        "SELECT anchor, text FROM vote_explanations
         WHERE sitting_record_uid = $1 ORDER BY rank",
    )
    .bind(RECORD_A)
    .fetch_all(&pool)
    .await
    .unwrap();
    remove_fixture(&pool, [RECORD_A, RECORD_A]).await;

    assert_eq!(attached, vec!["VTREADBACK3".to_string()]);
    assert_eq!(
        explanations,
        vec![("ANCHOR-NEW-1".to_string(), "Texte remplaçant.".to_string())]
    );
}

#[tokio::test]
#[ignore = "base Postgres jetable à poser dans DATABASE_URL"]
async fn stored_versions_return_the_generation_time_and_keys_fingerprint_of_each_sitting() {
    let pool = connect([RECORD_B, RECORD_C]).await;
    let repository = PgVoteExplanationRepository::new(pool.clone());
    let keys_a = SittingKeysFingerprint::of(&[key("VTREADBACK1", 10)]);
    let keys_b = SittingKeysFingerprint::of(&[key("VTREADBACK2", 20)]);
    repository
        .replace_sitting(&sitting(RECORD_B, at(9), keys_a.clone(), vec![], vec![]))
        .await
        .unwrap();
    repository
        .replace_sitting(&sitting(RECORD_C, at(14), keys_b.clone(), vec![], vec![]))
        .await
        .unwrap();

    let versions = repository.stored_versions().await.unwrap();
    remove_fixture(&pool, [RECORD_B, RECORD_C]).await;

    let a = &versions[&SittingRecordUid::new(RECORD_B.into()).unwrap()];
    let b = &versions[&SittingRecordUid::new(RECORD_C.into()).unwrap()];
    assert_eq!((a.generated_at, &a.keys), (at(9), &keys_a));
    assert_eq!((b.generated_at, &b.keys), (at(14), &keys_b));
}
