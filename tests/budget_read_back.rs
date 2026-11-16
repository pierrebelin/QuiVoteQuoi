//! Un exercice budgétaire s'écrit en une transaction et se relit à l'identique ;
//! en remplacer un n'en laisse rien de l'ancien ; un remplacement qui échoue
//! laisse l'ancien intact.
//!
//! Seul un vrai Postgres met à l'épreuve la transaction, les clés étrangères en
//! cascade et les contraintes de la migration. La CI n'en a pas, d'où `#[ignore]`.
//! Chaque test pose sa fixture dans une année qui lui est propre (2091 à 2096),
//! loin des exercices réels : les tests tournent en parallèle sur la même base.
//! Les noms de mission sont en ASCII, pour que l'ordre de lecture (`ORDER BY
//! name`) ne dépende pas de la collation de la base.
//!
//! Les migrations sont jouées sur la base visée, qui doit donc être jetable :
//!
//! ```text
//! DATABASE_URL=postgresql://... \
//!   cargo test --test budget_read_back -- --ignored
//! ```

use chrono::NaiveDate;
use quivotequoi::application::ports::budget_repository::BudgetRepository;
use quivotequoi::domain::budget::{
    AccountKind, AdoptionProcedure, BudgetExercise, BudgetStage, BudgetYear, Euros, LawReference,
    Mission, NewBudgetExercise, Programme, ProgrammeNumber, SourceReference,
};
use quivotequoi::infrastructure::persistence::pg_budget_repository::PgBudgetRepository;
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

async fn remove_fixture(pool: &PgPool, years: &[i16]) {
    sqlx::query("DELETE FROM budget_exercises WHERE year = ANY($1)")
        .bind(years)
        .execute(pool)
        .await
        .unwrap();
}

fn year(value: u16) -> BudgetYear {
    BudgetYear::new(value).unwrap()
}

fn programme(number: u16, name: &str, cp: i64) -> Programme {
    Programme::new(
        ProgrammeNumber::new(number).unwrap(),
        name,
        Euros::new(cp).unwrap(),
    )
    .unwrap()
}

fn mission(name: &str, kind: AccountKind, programmes: Vec<Programme>) -> Mission {
    Mission::new(name, kind, programmes).unwrap()
}

/// Missions en ordre de nom, programmes en ordre de numéro : l'ordre de lecture.
fn exercise(year_value: u16, missions: Vec<Mission>) -> BudgetExercise {
    BudgetExercise::new(NewBudgetExercise {
        year: year(year_value),
        stage: BudgetStage::Lfi,
        law: LawReference {
            number: "2091-001".to_string(),
            date: NaiveDate::from_ymd_opt(2091, 2, 19).unwrap(),
            credits_article: 148,
            adoption_procedure: AdoptionProcedure::Article49_3,
            legifrance_url: "https://www.legifrance.example/loi".to_string(),
        },
        source: SourceReference {
            title: "Crédits AE et CP votés".to_string(),
            url: "https://www.budget.example/fichier".to_string(),
            published_on: NaiveDate::from_ymd_opt(2091, 3, 3).unwrap(),
            downloaded_on: NaiveDate::from_ymd_opt(2091, 11, 16).unwrap(),
            sha256: "ab".repeat(32),
        },
        missions,
    })
    .unwrap()
}

fn sample(year_value: u16) -> BudgetExercise {
    exercise(
        year_value,
        vec![
            mission(
                "Alpha mission",
                AccountKind::GeneralBudget,
                vec![
                    programme(101, "Premier", 600),
                    programme(102, "Second", 400),
                ],
            ),
            mission(
                "Beta account",
                AccountKind::FinancialAssistance,
                vec![programme(841, "Avances", 50)],
            ),
        ],
    )
}

#[tokio::test]
#[ignore = "base Postgres jetable à poser dans DATABASE_URL"]
async fn reads_back_the_exercise_it_wrote() {
    let pool = connect().await;
    remove_fixture(&pool, &[2091]).await;
    let repository = PgBudgetRepository::new(pool.clone());
    let written = sample(2091);

    repository.replace_exercise(&written).await.unwrap();
    let read = repository.find_exercise(year(2091)).await;
    remove_fixture(&pool, &[2091]).await;

    assert_eq!(read.unwrap(), Some(written));
}

#[tokio::test]
#[ignore = "base Postgres jetable à poser dans DATABASE_URL"]
async fn replacing_an_exercise_leaves_nothing_of_the_previous_one() {
    let pool = connect().await;
    remove_fixture(&pool, &[2092]).await;
    let repository = PgBudgetRepository::new(pool.clone());
    let replacement = exercise(
        2092,
        vec![mission(
            "Gamma mission",
            AccountKind::GeneralBudget,
            vec![programme(103, "Troisième", 10)],
        )],
    );

    repository.replace_exercise(&sample(2092)).await.unwrap();
    repository.replace_exercise(&replacement).await.unwrap();
    let read = repository.find_exercise(year(2092)).await;
    let missions: i64 =
        sqlx::query_scalar("SELECT count(*) FROM budget_missions WHERE year = 2092")
            .fetch_one(&pool)
            .await
            .unwrap();
    let programmes: i64 =
        sqlx::query_scalar("SELECT count(*) FROM budget_programmes WHERE year = 2092")
            .fetch_one(&pool)
            .await
            .unwrap();
    remove_fixture(&pool, &[2092]).await;

    assert_eq!(read.unwrap(), Some(replacement));
    assert_eq!((missions, programmes), (1, 1));
}

#[tokio::test]
#[ignore = "base Postgres jetable à poser dans DATABASE_URL"]
async fn a_failed_replacement_keeps_the_previous_exercise() {
    let pool = connect().await;
    remove_fixture(&pool, &[2093]).await;
    let repository = PgBudgetRepository::new(pool.clone());
    let previous = sample(2093);
    // Le même numéro de programme dans deux missions : le domaine le construit,
    // c'est `verify` qui le refuse en amont ; la clé primaire (année, numéro) le
    // refuse ici, à la deuxième insertion, après la suppression de l'ancien.
    let broken = exercise(
        2093,
        vec![
            mission(
                "Alpha mission",
                AccountKind::GeneralBudget,
                vec![programme(101, "Premier", 1)],
            ),
            mission(
                "Beta account",
                AccountKind::FinancialAssistance,
                vec![programme(101, "Doublon", 2)],
            ),
        ],
    );

    repository.replace_exercise(&previous).await.unwrap();
    let failed = repository.replace_exercise(&broken).await;
    let read = repository.find_exercise(year(2093)).await;
    remove_fixture(&pool, &[2093]).await;

    assert!(failed.is_err());
    assert_eq!(read.unwrap(), Some(previous));
}

#[tokio::test]
#[ignore = "base Postgres jetable à poser dans DATABASE_URL"]
async fn lists_the_imported_years_most_recent_first() {
    let pool = connect().await;
    remove_fixture(&pool, &[2094, 2095]).await;
    let repository = PgBudgetRepository::new(pool.clone());

    repository.replace_exercise(&sample(2094)).await.unwrap();
    repository.replace_exercise(&sample(2095)).await.unwrap();
    let years = repository.available_years().await;
    remove_fixture(&pool, &[2094, 2095]).await;

    let ours: Vec<u16> = years
        .unwrap()
        .into_iter()
        .map(|y| y.get())
        .filter(|y| *y == 2094 || *y == 2095)
        .collect();
    assert_eq!(ours, [2095, 2094]);
}

#[tokio::test]
#[ignore = "base Postgres jetable à poser dans DATABASE_URL"]
async fn reads_nothing_for_a_year_that_was_never_imported() {
    let pool = connect().await;
    remove_fixture(&pool, &[2096]).await;

    let read = PgBudgetRepository::new(pool)
        .find_exercise(year(2096))
        .await;

    assert_eq!(read.unwrap(), None);
}

#[tokio::test]
#[ignore = "base Postgres jetable à poser dans DATABASE_URL"]
async fn the_database_refuses_a_negative_amount() {
    let pool = connect().await;
    remove_fixture(&pool, &[2090]).await;
    let repository = PgBudgetRepository::new(pool.clone());
    repository.replace_exercise(&sample(2090)).await.unwrap();

    let refused = sqlx::query(
        "INSERT INTO budget_programmes (year, number, name, mission_name, cp_euros)
         VALUES (2090, 999, 'Négatif', 'Alpha mission', -1)",
    )
    .execute(&pool)
    .await;
    remove_fixture(&pool, &[2090]).await;

    assert!(refused.is_err());
}
