//! Les scrutins d'une période se lisent depuis Postgres, bornes incluses, sans
//! plafond, dans l'ordre date puis numéro (RM-01), avec leur nombre et la
//! dernière date publiée (RM-04) ; les votes sur l'ensemble se lisent sur la
//! même période (RM-07).
//!
//! Seul un vrai Postgres met à l'épreuve le `BETWEEN`, le tri sur le numéro
//! (texte en base) et l'absence de `LIMIT`. La CI n'en a pas, d'où `#[ignore]`.
//! Chaque test pose sa fixture dans une année qui lui est propre, sous un
//! préfixe d'uid propre : les tests tournent en parallèle sur la même base.
//! Seul `counts_the_scrutins_of_a_period_and_gives_the_latest_scrutin_date`
//! vit en 2099, pour que la dernière date de la base soit celle de sa fixture.
//!
//! Les migrations sont jouées sur la base visée, qui doit donc être jetable :
//!
//! ```text
//! DATABASE_URL=postgresql://... \
//!   cargo test --test week_votes_read_back -- --ignored
//! ```

use chrono::{Datelike, Duration, NaiveDate};
use quivotequoi::application::ports::final_vote_repository::{
    FinalVoteFilter, FinalVoteRepository,
};
use quivotequoi::application::ports::scrutin_repository::ScrutinRepository;
use quivotequoi::infrastructure::persistence::pg_final_vote_repository::PgFinalVoteRepository;
use quivotequoi::infrastructure::persistence::pg_scrutin_repository::PgScrutinRepository;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

fn monday_of(year: i32, month: u32, day: u32) -> NaiveDate {
    let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
    date - Duration::days(i64::from(date.weekday().num_days_from_monday()))
}

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
    let like = format!("{prefix}%");
    sqlx::query("DELETE FROM scrutins WHERE uid LIKE $1")
        .bind(&like)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM debated_texts WHERE text_key LIKE $1")
        .bind(&like)
        .execute(pool)
        .await
        .unwrap();
}

async fn insert_scrutin(pool: &PgPool, uid: &str, number: &str, date: NaiveDate, subject: &str) {
    sqlx::query(
        "INSERT INTO scrutins
            (uid, number, legislature, scrutin_date, ballot_type_code, ballot_type_label,
             outcome_code, outcome_label, subject, voters, expressed, required,
             announcement, votes_for, votes_against, abstentions, not_voting,
             voluntary_not_voting)
         VALUES ($1, $2, 17, $3, 'SPO', 'Scrutin public ordinaire',
                 'adopté', 'adopté', $4, 10, 10, 6, 'Adopté', 6, 4, 0, 0, 0)",
    )
    .bind(uid)
    .bind(number)
    .bind(date)
    .bind(subject)
    .execute(pool)
    .await
    .unwrap();
}

/// Un vote sur l'ensemble, avec le texte débattu que la jointure de
/// `list_final_votes` exige.
async fn insert_final_vote(pool: &PgPool, uid: &str, number: &str, date: NaiveDate) {
    insert_scrutin(
        pool,
        uid,
        number,
        date,
        "l'ensemble du projet de loi de test",
    )
    .await;
    sqlx::query("INSERT INTO debated_texts (text_key, label) VALUES ($1, 'Texte de test')")
        .bind(uid)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO scrutin_debated_texts (scrutin_uid, text_key) VALUES ($1, $1)")
        .bind(uid)
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "base Postgres jetable à poser dans DATABASE_URL"]
async fn reads_every_scrutin_from_monday_to_sunday_in_date_then_number_order() {
    const PREFIX: &str = "WVRBORDER";
    let pool = connect().await;
    remove_fixture(&pool, PREFIX).await;
    let monday = monday_of(2097, 6, 2);
    let sunday = monday + Duration::days(6);
    // Insérés hors ordre ; « 100 » après « 10 » et « 9 » à l'ordre numérique.
    insert_scrutin(&pool, "WVRBORDER-C", "100", sunday, "Amendement C").await;
    insert_scrutin(&pool, "WVRBORDER-A", "9", monday, "Amendement A").await;
    insert_scrutin(&pool, "WVRBORDER-B", "10", monday, "Amendement B").await;
    insert_scrutin(
        &pool,
        "WVRBORDER-N",
        "101",
        sunday + Duration::days(1),
        "Amendement du lundi suivant",
    )
    .await;

    let read = PgScrutinRepository::new(pool.clone())
        .between(monday, sunday)
        .await;
    remove_fixture(&pool, PREFIX).await;

    let uids: Vec<String> = read.unwrap().into_iter().map(|s| s.uid).collect();
    assert_eq!(uids, ["WVRBORDER-A", "WVRBORDER-B", "WVRBORDER-C"]);
}

#[tokio::test]
#[ignore = "base Postgres jetable à poser dans DATABASE_URL"]
async fn reads_more_than_two_hundred_scrutins_of_one_week() {
    const PREFIX: &str = "WVRMANY";
    let pool = connect().await;
    remove_fixture(&pool, PREFIX).await;
    let monday = monday_of(2098, 3, 3);
    sqlx::query(
        "INSERT INTO scrutins
            (uid, number, legislature, scrutin_date, ballot_type_code, ballot_type_label,
             outcome_code, outcome_label, subject, voters, expressed, required,
             announcement, votes_for, votes_against, abstentions, not_voting,
             voluntary_not_voting)
         SELECT 'WVRMANY-' || n, n::text, 17, $1, 'SPO', 'Scrutin public ordinaire',
                'adopté', 'adopté', 'Amendement ' || n, 10, 10, 6, 'Adopté', 6, 4, 0, 0, 0
         FROM generate_series(1, 201) AS n",
    )
    .bind(monday)
    .execute(&pool)
    .await
    .unwrap();

    let read = PgScrutinRepository::new(pool.clone())
        .between(monday, monday + Duration::days(6))
        .await;
    remove_fixture(&pool, PREFIX).await;

    assert_eq!(read.unwrap().len(), 201);
}

#[tokio::test]
#[ignore = "base Postgres jetable à poser dans DATABASE_URL"]
async fn counts_the_scrutins_of_a_period_and_gives_the_latest_scrutin_date() {
    const PREFIX: &str = "WVRCOUNT";
    let pool = connect().await;
    remove_fixture(&pool, PREFIX).await;
    let monday = monday_of(2099, 6, 1);
    let sunday = monday + Duration::days(6);
    let next_monday = sunday + Duration::days(1);
    insert_scrutin(&pool, "WVRCOUNT-A", "9", monday, "Amendement A").await;
    insert_scrutin(
        &pool,
        "WVRCOUNT-B",
        "10",
        monday + Duration::days(2),
        "Amendement B",
    )
    .await;
    insert_scrutin(&pool, "WVRCOUNT-C", "100", sunday, "Amendement C").await;
    insert_scrutin(
        &pool,
        "WVRCOUNT-N",
        "101",
        next_monday,
        "Amendement suivant",
    )
    .await;

    let repository = PgScrutinRepository::new(pool.clone());
    let count = repository.count_between(monday, sunday).await;
    let latest = repository.latest_scrutin_date().await;
    remove_fixture(&pool, PREFIX).await;

    assert_eq!(count.unwrap(), 3);
    assert_eq!(latest.unwrap(), Some(next_monday));
}

#[tokio::test]
#[ignore = "base Postgres jetable à poser dans DATABASE_URL"]
async fn lists_only_the_final_votes_of_the_period() {
    const PREFIX: &str = "WVRFINAL";
    let pool = connect().await;
    remove_fixture(&pool, PREFIX).await;
    let monday = monday_of(2096, 6, 3);
    let sunday = monday + Duration::days(6);
    insert_final_vote(&pool, "WVRFINAL-IN", "5", monday + Duration::days(3)).await;
    insert_final_vote(&pool, "WVRFINAL-NEXT", "6", sunday + Duration::days(1)).await;
    insert_scrutin(&pool, "WVRFINAL-AMDT", "4", monday, "Amendement sans rapport").await;

    let filter = FinalVoteFilter {
        from: Some(monday),
        to: Some(sunday),
        limit: 1000,
        ..FinalVoteFilter::default()
    };
    let page = PgFinalVoteRepository::new(pool.clone())
        .list_final_votes(&filter)
        .await;
    remove_fixture(&pool, PREFIX).await;

    let uids: Vec<String> = page
        .unwrap()
        .items
        .into_iter()
        .map(|item| item.scrutin_uid)
        .collect();
    assert_eq!(uids, ["WVRFINAL-IN"]);
}
