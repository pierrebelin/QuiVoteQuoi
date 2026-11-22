//! Route de la methode des Reels : `/api/reels/methode` sert la methode
//! (RM-15, RM-16), elle n'est pas lue comme la date d'un jour
//! (`/api/reels/{date}`).
//!
//! Pool Postgres en `connect_lazy` : la methode n'emet aucune requete SQL, le
//! test tourne sans base.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

use quivotequoi::api::routes::create_router;
use quivotequoi::api::security::AdminGuard;
use quivotequoi::infrastructure::legifrance::unavailable::UnavailableLegalTextSource;
use quivotequoi::infrastructure::llm::unavailable_brief_generator::UnavailableBriefGenerator;
use quivotequoi::infrastructure::llm::unavailable_classifier::UnavailableThemeClassifier;
use quivotequoi::infrastructure::llm::unavailable_dossier_summary::UnavailableDossierSummaryGenerator;
use quivotequoi::infrastructure::national_assembly::actor_client::AmoActorClient;
use quivotequoi::infrastructure::national_assembly::amendment_client::AmendmentClient;
use quivotequoi::infrastructure::national_assembly::client::NationalAssemblyClient;
use quivotequoi::infrastructure::national_assembly::scrutin_client::ScrutinClient;
use quivotequoi::infrastructure::national_assembly::sitting_record_client::SittingRecordClient;
use quivotequoi::infrastructure::national_assembly::text_version_client::TextVersionClient;
use quivotequoi::infrastructure::persistence::pg_actor_repository::PgActorRepository;
use quivotequoi::infrastructure::persistence::pg_amendment_repository::PgAmendmentRepository;
use quivotequoi::infrastructure::persistence::pg_brief_line_repository::PgBriefLineRepository;
use quivotequoi::infrastructure::persistence::pg_budget_repository::PgBudgetRepository;
use quivotequoi::infrastructure::persistence::pg_candidate_repository::PgCandidateRepository;
use quivotequoi::infrastructure::persistence::pg_dossier_repository::PgDossierRepository;
use quivotequoi::infrastructure::persistence::pg_final_vote_repository::PgFinalVoteRepository;
use quivotequoi::infrastructure::persistence::pg_group_repository::PgGroupRepository;
use quivotequoi::infrastructure::persistence::pg_scrutin_repository::PgScrutinRepository;
use quivotequoi::infrastructure::persistence::pg_text_version_repository::PgTextVersionRepository;
use quivotequoi::infrastructure::persistence::pg_theme_repository::PgThemeRepository;
use quivotequoi::infrastructure::persistence::pg_vote_explanation_repository::PgVoteExplanationRepository;
use quivotequoi::AppState;

fn state() -> AppState {
    let db = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_millis(200))
        .connect_lazy("postgresql://unused:unused@127.0.0.1:1/unused")
        .expect("chaîne de connexion valide, aucune connexion ouverte");

    AppState {
        health_probe: Arc::new(
            quivotequoi::infrastructure::persistence::pg_health_probe::PgHealthProbe::new(db.clone()),
        ),
        assembly_source: Arc::new(NationalAssemblyClient::new()),
        candidate_repository: Arc::new(PgCandidateRepository::new(db.clone())),
        dossier_repository: Arc::new(PgDossierRepository::new(db.clone())),
        decision_vote_repository: Arc::new(PgDossierRepository::new(db.clone())),
        dossier_group_actions_repository: Arc::new(
            quivotequoi::infrastructure::persistence::pg_dossier_group_actions_repository::PgDossierGroupActionsRepository::new(db.clone()),
        ),
        dossier_summary_repository: Arc::new(
            quivotequoi::infrastructure::persistence::pg_dossier_summary_repository::PgDossierSummaryRepository::new(db.clone()),
        ),
        dossier_summary_generator: Arc::new(UnavailableDossierSummaryGenerator),
        actor_source: Arc::new(AmoActorClient::new()),
        actor_repository: Arc::new(PgActorRepository::new(db.clone())),
        scrutin_source: Arc::new(ScrutinClient::new()),
        amendment_source: Arc::new(AmendmentClient::new()),
        scrutin_repository: Arc::new(PgScrutinRepository::new(db.clone())),
        amendment_repository: Arc::new(PgAmendmentRepository::new(db.clone())),
        amendment_citation_repository: Arc::new(
            quivotequoi::infrastructure::persistence::pg_amendment_citation_repository::PgAmendmentCitationRepository::new(db.clone()),
        ),
        ingestion_journal: Arc::new(
            quivotequoi::infrastructure::persistence::pg_ingestion_journal::PgIngestionJournal::new(db.clone()),
        ),
        final_vote_repository: Arc::new(PgFinalVoteRepository::new(db.clone())),
        group_repository: Arc::new(PgGroupRepository::new(db.clone())),
        theme_repository: Arc::new(PgThemeRepository::new(db.clone())),
        site_news_catalog: Arc::new(
            quivotequoi::infrastructure::content::json_site_news_catalog::JsonSiteNewsCatalog::embedded()
                .expect("content/nouveautes.json is valid"),
        ),
        theme_classifier: Arc::new(UnavailableThemeClassifier),
        text_version_source: Arc::new(TextVersionClient::new()),
        text_version_repository: Arc::new(PgTextVersionRepository::new(db.clone())),
        legal_text_source: Arc::new(UnavailableLegalTextSource),
        brief_generator: Arc::new(UnavailableBriefGenerator),
        brief_line_repository: Arc::new(PgBriefLineRepository::new(db.clone())),
        budget_repository: Arc::new(PgBudgetRepository::new(db.clone())),
        sitting_record_source: Arc::new(SittingRecordClient::new()),
        vote_explanation_repository: Arc::new(PgVoteExplanationRepository::new(db)),
    }
}

#[tokio::test]
async fn serves_the_method_and_not_a_date() {
    let router = create_router(state(), AdminGuard::closed());
    let request = Request::builder()
        .method("GET")
        .uri("/api/reels/methode")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(json.get("theme_rotation").is_some(), "corps : {json}");
}
