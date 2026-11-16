//! La file de revue des sources de programme ne remonte que des programmes :
//! ni la déclaration de candidature (souvent un article de presse), ni une
//! adresse qu'aucune candidature n'utilise plus. Chaque changement porte les
//! fragments ajoutés et retirés quand la lecture précédente avait gardé son
//! texte, et rien sinon.
//!
//! Seul un vrai Postgres relit la requête et la migration. La CI n'en a pas,
//! d'où `#[ignore]`. Les migrations sont jouées sur la base visée, qui doit donc
//! être jetable :
//!
//! ```text
//! DATABASE_URL=postgresql://... \
//!   cargo test --test changed_sources_read_back -- --ignored
//! ```

use chrono::{Duration, Utc};
use quivotequoi::application::ports::candidate_repository::{
    CandidateRepository, ChangeExcerpt, ChangedSourceRecord,
};
use quivotequoi::infrastructure::persistence::pg_candidate_repository::PgCandidateRepository;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

const CANDIDATE_ID: &str = "test-changed-sources";
const HOST: &str = "https://changed-sources.test";

async fn connect() -> PgPool {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();
    pool
}

async fn remove_fixture(pool: &PgPool) {
    sqlx::query("DELETE FROM candidate_program_source_observations WHERE source_url LIKE $1")
        .bind(format!("{HOST}/%"))
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM presidential_candidates WHERE id = $1")
        .bind(CANDIDATE_ID)
        .execute(pool)
        .await
        .unwrap();
}

async fn insert_candidate(pool: &PgPool) {
    sqlx::query(
        "INSERT INTO presidential_candidates
             (id, display_name, declared_on, declaration_source_url, declaration_source_label,
              official_site_url, program_url)
         VALUES ($1, 'Candidature de test', '2026-09-01', $2, 'Article de presse', $3, $4)",
    )
    .bind(CANDIDATE_ID)
    .bind(format!("{HOST}/presse/declaration"))
    .bind(format!("{HOST}/"))
    .bind(format!("{HOST}/programme#page=3"))
    .execute(pool)
    .await
    .unwrap();
}

/// Observation modifiée hier, après une première lecture il y a un mois.
async fn insert_changed_observation(
    pool: &PgPool,
    url: &str,
    excerpt: Option<(&str, &str, i32, i32)>,
) {
    let (added, removed, added_count, removed_count) = match excerpt {
        Some((added, removed, added_count, removed_count)) => (
            Some(added),
            Some(removed),
            Some(added_count),
            Some(removed_count),
        ),
        None => (None, None, None, None),
    };
    sqlx::query(
        "INSERT INTO candidate_program_source_observations
             (source_url, first_observed_at, last_checked_at, last_changed_at, http_status,
              content_sha256, content_hash_method, content_text,
              last_added_text, last_removed_text, last_added_count, last_removed_count)
         VALUES ($1, NOW() - INTERVAL '30 days', NOW(), NOW() - INTERVAL '1 day', 200,
                 repeat('a', 64), 'visible-text-v1', 'texte courant',
                 $2, $3, $4, $5)",
    )
    .bind(url)
    .bind(added)
    .bind(removed)
    .bind(added_count)
    .bind(removed_count)
    .execute(pool)
    .await
    .unwrap();
}

async fn changed_sources(pool: &PgPool) -> Vec<ChangedSourceRecord> {
    PgCandidateRepository::new(pool.clone())
        .changed_sources(Utc::now() - Duration::days(7))
        .await
        .unwrap()
        .into_iter()
        .filter(|record| record.source_url.starts_with(HOST))
        .collect()
}

#[tokio::test]
#[ignore = "requires DATABASE_URL pointing at a disposable Postgres"]
async fn only_program_sources_reach_the_review_queue_with_their_excerpt() {
    let pool = connect().await;
    remove_fixture(&pool).await;
    insert_candidate(&pool).await;
    let program = format!("{HOST}/programme");
    insert_changed_observation(
        &pool,
        &program,
        Some(("Nouvelle mesure\nSecond fragment", "Ancienne mesure", 25, 1)),
    )
    .await;
    insert_changed_observation(&pool, &format!("{HOST}/"), None).await;
    insert_changed_observation(&pool, &format!("{HOST}/presse/declaration"), None).await;
    insert_changed_observation(&pool, &format!("{HOST}/plus-cite"), None).await;

    let records = changed_sources(&pool).await;
    remove_fixture(&pool).await;

    let urls: Vec<&str> = records.iter().map(|r| r.source_url.as_str()).collect();
    assert_eq!(urls.len(), 2, "{urls:?}");
    let program_record = records.iter().find(|r| r.source_url == program).unwrap();
    assert_eq!(program_record.candidate_names, vec!["Candidature de test"]);
    assert_eq!(
        program_record.excerpt,
        Some(ChangeExcerpt {
            added: vec!["Nouvelle mesure".into(), "Second fragment".into()],
            removed: vec!["Ancienne mesure".into()],
            added_total: 25,
            removed_total: 1,
        })
    );
    let site_record = records
        .iter()
        .find(|r| r.source_url == format!("{HOST}/"))
        .unwrap();
    assert_eq!(site_record.excerpt, None);
}
