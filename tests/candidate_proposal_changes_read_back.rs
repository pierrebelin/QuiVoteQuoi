//! Le remplacement des extraits d'une candidature (le seed) enregistre ce qui a
//! été ajouté et retiré, daté du jour, en comparant la photographie d'avant au
//! contenu d'après sur `(candidat, thème, extrait)` : une source qui change pour
//! le même texte n'est pas un changement. Une candidature absente de la
//! photographie voit tous ses extraits ajoutés.
//!
//! Seul un vrai Postgres relit la fonction et la migration. La CI n'en a pas,
//! d'où `#[ignore]`. Les migrations sont jouées sur la base visée, qui doit donc
//! être jetable :
//!
//! ```text
//! DATABASE_URL=postgresql://... \
//!   cargo test --test candidate_proposal_changes_read_back -- --ignored --test-threads=1
//! ```

use chrono::{Duration, Utc};
use quivotequoi::application::ports::candidate_repository::{
    CandidateRepository, ProposalChange, ProposalChangeKind,
};
use quivotequoi::infrastructure::persistence::pg_candidate_repository::PgCandidateRepository;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

const CANDIDATE_ID: &str = "test-proposal-changes";
const HOST: &str = "https://proposal-changes.test";

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
    sqlx::query("DELETE FROM candidate_proposal_changes WHERE candidate_id = $1")
        .bind(CANDIDATE_ID)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM candidate_program_proposals WHERE candidate_id = $1")
        .bind(CANDIDATE_ID)
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
    .bind(format!("{HOST}/programme"))
    .execute(pool)
    .await
    .unwrap();
}

/// Ce que fait le seed dans sa transaction : photographie, remplacement des
/// extraits de la candidature, puis enregistrement de la différence.
async fn replace_proposals(pool: &PgPool, proposals: &[(&str, &str, &str)]) {
    let mut tx = pool.begin().await.unwrap();
    sqlx::query(
        "CREATE TEMP TABLE candidate_proposals_before ON COMMIT DROP AS
         SELECT candidate_id, family_code, excerpt, source_url
           FROM candidate_program_proposals",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("DELETE FROM candidate_program_proposals WHERE candidate_id = $1")
        .bind(CANDIDATE_ID)
        .execute(&mut *tx)
        .await
        .unwrap();
    for (family, excerpt, source_path) in proposals {
        sqlx::query(
            "INSERT INTO candidate_program_proposals
                 (candidate_id, family_code, excerpt, source_url, source_label)
             VALUES ($1, $2, $3, $4, 'Programme de test')",
        )
        .bind(CANDIDATE_ID)
        .bind(family)
        .bind(excerpt)
        .bind(format!("{HOST}{source_path}"))
        .execute(&mut *tx)
        .await
        .unwrap();
    }
    sqlx::query("SELECT record_candidate_proposal_changes()")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

async fn forget_recorded_changes(pool: &PgPool) {
    sqlx::query("DELETE FROM candidate_proposal_changes WHERE candidate_id = $1")
        .bind(CANDIDATE_ID)
        .execute(pool)
        .await
        .unwrap();
}

async fn changes_of_the_week(pool: &PgPool) -> Vec<ProposalChange> {
    let today = Utc::now().date_naive();
    PgCandidateRepository::new(pool.clone())
        .proposal_changes_between(today - Duration::days(1), today + Duration::days(1))
        .await
        .unwrap()
        .into_iter()
        .filter(|change| change.candidate_id.as_str() == CANDIDATE_ID)
        .collect()
}

#[tokio::test]
#[ignore = "requires DATABASE_URL pointing at a disposable Postgres"]
async fn records_added_and_removed_excerpts_and_reads_them_by_publication_date() {
    let pool = connect().await;
    remove_fixture(&pool).await;
    insert_candidate(&pool).await;
    replace_proposals(
        &pool,
        &[
            ("sante", "Extrait retiré", "/programme"),
            ("sante", "Extrait conservé", "/programme"),
        ],
    )
    .await;
    forget_recorded_changes(&pool).await;

    replace_proposals(
        &pool,
        &[
            ("sante", "Extrait conservé", "/programme"),
            ("sante", "Extrait ajouté", "/programme"),
        ],
    )
    .await;
    let changes = changes_of_the_week(&pool).await;
    let today = Utc::now().date_naive();
    let outside = PgCandidateRepository::new(pool.clone())
        .proposal_changes_between(today + Duration::days(2), today + Duration::days(9))
        .await
        .unwrap()
        .into_iter()
        .filter(|change| change.candidate_id.as_str() == CANDIDATE_ID)
        .count();
    remove_fixture(&pool).await;

    assert_eq!(changes.len(), 2, "{changes:?}");
    let added = changes
        .iter()
        .find(|c| c.change == ProposalChangeKind::Added)
        .unwrap();
    let removed = changes
        .iter()
        .find(|c| c.change == ProposalChangeKind::Removed)
        .unwrap();
    assert_eq!(added.candidate_name, "Candidature de test");
    assert_eq!(removed.candidate_name, "Candidature de test");
    assert_eq!(added.family_code.as_str(), "sante");
    assert_eq!(removed.family_code.as_str(), "sante");
    assert_eq!(outside, 0);
}

#[tokio::test]
#[ignore = "requires DATABASE_URL pointing at a disposable Postgres"]
async fn ignores_an_excerpt_rewritten_unchanged() {
    let pool = connect().await;
    remove_fixture(&pool).await;
    insert_candidate(&pool).await;
    replace_proposals(&pool, &[("sante", "Extrait stable", "/programme")]).await;
    let first_publication = changes_of_the_week(&pool).await;
    forget_recorded_changes(&pool).await;

    replace_proposals(&pool, &[("sante", "Extrait stable", "/programme-nouvelle-adresse")]).await;
    let changes = changes_of_the_week(&pool).await;
    remove_fixture(&pool).await;

    assert_eq!(first_publication.len(), 1, "{first_publication:?}");
    assert!(changes.is_empty(), "{changes:?}");
}

#[tokio::test]
#[ignore = "requires DATABASE_URL pointing at a disposable Postgres"]
async fn records_every_excerpt_of_a_new_candidacy_as_added() {
    let pool = connect().await;
    remove_fixture(&pool).await;
    insert_candidate(&pool).await;

    replace_proposals(
        &pool,
        &[
            ("sante", "Premier extrait", "/programme"),
            ("logement", "Second extrait", "/programme"),
        ],
    )
    .await;
    let changes = changes_of_the_week(&pool).await;
    remove_fixture(&pool).await;

    assert_eq!(changes.len(), 2, "{changes:?}");
    assert!(changes
        .iter()
        .all(|change| change.change == ProposalChangeKind::Added));
}
