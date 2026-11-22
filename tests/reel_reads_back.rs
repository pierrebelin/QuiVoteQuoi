//! La numérotation des scrutins d'une législature se lit en une requête : leur
//! nombre et le plus haut numéro (RM-09). Un numéro non numérique (« 12bis »)
//! n'est ni compté ni lu comme plus grand numéro.
//!
//! Seul un vrai Postgres met à l'épreuve la conversion du numéro, stocké en
//! texte. La CI n'en a pas, d'où `#[ignore]`. La fixture vit dans des
//! législatures fictives, sous un préfixe d'uid propre.
//!
//! Les migrations sont jouées sur la base visée, qui doit donc être jetable :
//!
//! ```text
//! DATABASE_URL=postgresql://... \
//!   cargo test --test reel_reads_back -- --ignored
//! ```

use chrono::NaiveDate;
use quivotequoi::application::ports::scrutin_repository::ScrutinRepository;
use quivotequoi::domain::reel::NumberingSpan;
use quivotequoi::infrastructure::persistence::pg_scrutin_repository::PgScrutinRepository;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

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

async fn remove_fixture(pool: &PgPool, prefix: &str) {
    sqlx::query("DELETE FROM scrutins WHERE uid LIKE $1")
        .bind(format!("{prefix}%"))
        .execute(pool)
        .await
        .unwrap();
}

async fn insert_scrutin(pool: &PgPool, uid: &str, number: &str, legislature: i16) {
    sqlx::query(
        "INSERT INTO scrutins
            (uid, number, legislature, scrutin_date, ballot_type_code, ballot_type_label,
             outcome_code, outcome_label, subject, voters, expressed, required,
             announcement, votes_for, votes_against, abstentions, not_voting,
             voluntary_not_voting)
         VALUES ($1, $2, $3, $4, 'SPO', 'Scrutin public ordinaire',
                 'adopté', 'adopté', 'Amendement', 10, 10, 6, 'Adopté', 6, 4, 0, 0, 0)",
    )
    .bind(uid)
    .bind(number)
    .bind(legislature)
    .bind(NaiveDate::from_ymd_opt(2095, 6, 1).unwrap())
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
#[ignore = "base Postgres jetable à poser dans DATABASE_URL"]
async fn numbering_span_counts_the_scrutins_and_reads_the_highest_number_of_the_legislature() {
    const PREFIX: &str = "RRBSPAN";
    const LEGISLATURE: i16 = 98;
    const OTHER_LEGISLATURE: i16 = 97;
    let pool = connect().await;
    remove_fixture(&pool, PREFIX).await;
    insert_scrutin(&pool, "RRBSPAN-1", "1", LEGISLATURE).await;
    insert_scrutin(&pool, "RRBSPAN-2", "2", LEGISLATURE).await;
    insert_scrutin(&pool, "RRBSPAN-4", "4", LEGISLATURE).await;
    insert_scrutin(&pool, "RRBSPAN-BIS", "12bis", LEGISLATURE).await;
    insert_scrutin(&pool, "RRBSPAN-OTHER", "900", OTHER_LEGISLATURE).await;

    let span = PgScrutinRepository::new(pool.clone())
        .numbering_span(LEGISLATURE as u16)
        .await;
    remove_fixture(&pool, PREFIX).await;

    assert_eq!(
        span.unwrap(),
        NumberingSpan {
            count: 3,
            highest: 4
        }
    );
}
