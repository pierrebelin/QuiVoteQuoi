//! Une synthèse prête se relit depuis Postgres, de bout en bout de la route
//! `GET /api/dossiers/{uid}/lecture-groupes`.
//!
//! `Row::get` panique quand le type Rust ne correspond pas à celui de la
//! colonne, mais seulement sur une valeur non nulle : tant qu'aucune synthèse
//! n'est prête, rien ne se voit. La panique coupe la connexion et nginx répond
//! 502. Un faux dépôt remplacerait justement le décodage fautif : seul un vrai
//! Postgres le met à l'épreuve. La CI n'en a pas, d'où `#[ignore]` et le test
//! sans base qui accompagne `pg_dossier_summary_repository.rs`.
//!
//! Les migrations sont jouées sur la base visée, qui doit donc être jetable :
//!
//! ```text
//! DATABASE_URL=postgresql://... \
//!   cargo test --test dossier_summary_read_back -- --ignored
//! ```

use quivotequoi::api::dossier_group_actions_dto::DossierGroupActionsResponse;
use quivotequoi::application::ports::dossier_group_actions_repository::{
    DossierGroupActionsRepository, DossierSummaryRepository, GeneratedGroupSummary,
};
use quivotequoi::application::use_cases::get_dossier_group_actions::{
    fingerprint, GetDossierGroupActions,
};
use quivotequoi::infrastructure::persistence::pg_dossier_group_actions_repository::PgDossierGroupActionsRepository;
use quivotequoi::infrastructure::persistence::pg_dossier_summary_repository::PgDossierSummaryRepository;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

const DOSSIER_UID: &str = "DLRREADBACK";
const GROUP_UID: &str = "POREADBACK";

async fn remove_fixture(pool: &PgPool) {
    sqlx::query("DELETE FROM legislative_dossiers WHERE uid = $1")
        .bind(DOSSIER_UID)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM parliamentary_groups WHERE uid = $1")
        .bind(GROUP_UID)
        .execute(pool)
        .await
        .unwrap();
}

async fn insert_fixture(pool: &PgPool) {
    sqlx::query(
        "INSERT INTO legislative_dossiers
            (uid, title, procedure_label, last_activity_date, last_activity_label,
             score_progress, score_magnitude, score_momentum, score_total)
         VALUES ($1, 'Dossier de test', 'Proposition de loi', '2026-10-01', 'Dépôt', 0, 0, 0, 0)",
    )
    .bind(DOSSIER_UID)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO parliamentary_groups (uid, legislature, label, abbrev)
         VALUES ($1, 17, 'Groupe de test', 'TEST')",
    )
    .bind(GROUP_UID)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
#[ignore = "base Postgres jetable à poser dans DATABASE_URL"]
async fn a_ready_summary_is_read_back_with_its_generation_date() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();
    remove_fixture(&pool).await;
    insert_fixture(&pool).await;

    let facts = PgDossierGroupActionsRepository::new(pool.clone());
    let summaries = PgDossierSummaryRepository::new(pool.clone());
    let current = facts.load_facts(DOSSIER_UID).await.unwrap().unwrap();
    summaries
        .save_ready(
            DOSSIER_UID,
            fingerprint(&current).as_str(),
            "modele-de-test",
            "v-test",
            &[GeneratedGroupSummary {
                group_uid: GROUP_UID.to_string(),
                paragraph: "Le groupe a déposé des amendements.".to_string(),
                sources: Vec::new(),
            }],
        )
        .await
        .unwrap();

    let actions = GetDossierGroupActions::new(&facts, &summaries)
        .execute(DOSSIER_UID)
        .await
        .unwrap()
        .unwrap();
    let body = serde_json::to_value(DossierGroupActionsResponse::from(actions)).unwrap();
    remove_fixture(&pool).await;

    let summary = &body["groups"][0]["summary"];
    assert_eq!(summary["text"], "Le groupe a déposé des amendements.");
    assert!(
        summary["generated_at"].is_string(),
        "generated_at missing from {summary}"
    );
}
