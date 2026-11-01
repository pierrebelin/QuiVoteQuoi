//! Verrou de cablage : les routes d'ecriture sont bien derriere la garde, les
//! routes de consultation restent ouvertes.
//!
//! Les tests unitaires de `api::security` verifient la regle ; ceux-ci
//! verifient qu'elle est branchee sur les bons chemins. Une route d'ecriture
//! ajoutee au routeur de lecture par distraction est exactement le genre
//! d'erreur qu'un test de la seule fonction `verify` ne voit pas.
//!
//! Le pool Postgres est ouvert en `connect_lazy` : aucune connexion n'est
//! etablie tant qu'une requete SQL n'est pas emise, et une requete refusee par
//! la garde n'en emet aucune. Ces tests tournent donc sans base.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

use quivotequoi::api::routes::create_router;
use quivotequoi::api::security::AdminGuard;
use quivotequoi::infrastructure::llm::unavailable_classifier::UnavailableClassifier;
use quivotequoi::infrastructure::llm::unavailable_dossier_summary::UnavailableDossierSummaryGenerator;
use quivotequoi::infrastructure::national_assembly::actor_client::AmoActorClient;
use quivotequoi::infrastructure::national_assembly::amendment_client::AmendmentClient;
use quivotequoi::infrastructure::national_assembly::client::NationalAssemblyClient;
use quivotequoi::infrastructure::national_assembly::scrutin_client::ScrutinClient;
use quivotequoi::infrastructure::persistence::pg_actor_repository::PgActorRepository;
use quivotequoi::infrastructure::persistence::pg_amendment_repository::PgAmendmentRepository;
use quivotequoi::infrastructure::persistence::pg_candidate_repository::PgCandidateRepository;
use quivotequoi::infrastructure::persistence::pg_dossier_repository::PgDossierRepository;
use quivotequoi::infrastructure::persistence::pg_final_vote_repository::PgFinalVoteRepository;
use quivotequoi::infrastructure::persistence::pg_group_repository::PgGroupRepository;
use quivotequoi::infrastructure::persistence::pg_scrutin_repository::PgScrutinRepository;
use quivotequoi::infrastructure::persistence::pg_theme_repository::PgThemeRepository;
use quivotequoi::infrastructure::security::AdminTokenSecret;
use quivotequoi::AppState;

const SECRET: &str = "0123456789abcdef0123456789abcdef";

/// Les neuf routes d'ecriture de l'API. Toute route ajoutee ici sans garde
/// fait echouer `toute_route_d_ecriture_exige_un_jeton`.
const WRITE_ROUTES: [&str; 9] = [
    "/api/refresh",
    "/api/registry/refresh",
    "/api/scrutins/refresh",
    "/api/amendements/refresh",
    "/api/dossiers/DLR5L15N47160/curate",
    "/api/dossiers/DLR5L15N47160/save",
    "/api/themes/extract",
    "/api/themes/propose",
    "/api/themes/arbitrate",
];

fn state() -> AppState {
    let db = PgPoolOptions::new()
        .max_connections(1)
        // Les lectures non authentifiees atteignent le handler et tentent une
        // requete SQL. Sans ce delai court, chacune attend les 30 secondes du
        // defaut avant d'abandonner.
        .acquire_timeout(Duration::from_millis(200))
        .connect_lazy("postgresql://unused:unused@127.0.0.1:1/unused")
        .expect("chaîne de connexion valide, aucune connexion ouverte");

    AppState {
        db: db.clone(),
        assembly_source: Arc::new(NationalAssemblyClient::new()),
        candidate_repository: Arc::new(PgCandidateRepository::new(db.clone())),
        dossier_repository: Arc::new(PgDossierRepository::new(db.clone())),
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
        final_vote_repository: Arc::new(PgFinalVoteRepository::new(db.clone())),
        group_repository: Arc::new(PgGroupRepository::new(db.clone())),
        theme_repository: Arc::new(PgThemeRepository::new(db)),
        theme_classifier: Arc::new(UnavailableClassifier),
    }
}

fn guarded_router() -> axum::Router {
    let secret = AdminTokenSecret::new(SECRET.to_string()).unwrap();
    create_router(state(), AdminGuard::new(Some(secret)))
}

fn token_of_today() -> String {
    AdminTokenSecret::new(SECRET.to_string())
        .unwrap()
        .token_for(Utc::now().date_naive())
}

async fn status_of(router: axum::Router, request: Request<Body>) -> StatusCode {
    router.oneshot(request).await.unwrap().status()
}

fn post(path: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .unwrap()
}

#[tokio::test]
async fn toute_route_d_ecriture_exige_un_jeton() {
    for path in WRITE_ROUTES {
        let status = status_of(guarded_router(), post(path)).await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "route non gardée : {path}"
        );
    }
}

#[tokio::test]
async fn un_jeton_perime_ne_passe_aucune_route_d_ecriture() {
    let stale = AdminTokenSecret::new(SECRET.to_string())
        .unwrap()
        .token_for(Utc::now().date_naive() - chrono::Duration::days(30));

    for path in WRITE_ROUTES {
        let mut request = post(path);
        request
            .headers_mut()
            .insert("x-admin-token", stale.parse().unwrap());

        let status = status_of(guarded_router(), request).await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "jeton périmé accepté : {path}"
        );
    }
}

/// Avec le jeton du jour, la garde s'efface. La requete atteint le handler et
/// echoue en base (le pool ne mene nulle part) : ce qui compte est qu'elle ne
/// soit plus refusee en 401/403.
#[tokio::test]
async fn le_jeton_du_jour_franchit_la_garde() {
    let token = token_of_today();

    let mut request = post("/api/registry/refresh");
    request
        .headers_mut()
        .insert("x-admin-token", token.parse().unwrap());

    let status = status_of(guarded_router(), request).await;
    assert_ne!(status, StatusCode::UNAUTHORIZED);
    assert_ne!(status, StatusCode::FORBIDDEN);
}

/// Sans `ADMIN_TOKEN_SECRET`, l'ecriture est fermee : 403, et aucun jeton ne
/// l'ouvre.
#[tokio::test]
async fn sans_secret_l_ecriture_est_fermee() {
    let router = create_router(state(), AdminGuard::closed());

    let mut request = post("/api/refresh");
    request
        .headers_mut()
        .insert("x-admin-token", token_of_today().parse().unwrap());

    assert_eq!(status_of(router, request).await, StatusCode::FORBIDDEN);
}

/// Le site public ne demande pas de jeton : une lecture non authentifiee ne
/// doit jamais rendre 401 ni 403 (README.md §2).
#[tokio::test]
async fn les_routes_de_consultation_restent_ouvertes() {
    for path in [
        "/api/scrutins",
        "/api/groupes",
        "/api/dossiers",
        "/api/dossiers/D1/lecture-groupes",
        "/api/themes",
        "/api/votes-finaux",
    ] {
        let request = Request::builder()
            .method("GET")
            .uri(path)
            .body(Body::empty())
            .unwrap();

        let status = status_of(guarded_router(), request).await;
        assert_ne!(status, StatusCode::UNAUTHORIZED, "lecture fermée : {path}");
        assert_ne!(status, StatusCode::FORBIDDEN, "lecture fermée : {path}");
    }
}

/// Une methode inconnue sur un chemin d'ecriture est refusee par la garde
/// avant meme le controle de methode : 401, pas 405. C'est le comportement
/// voulu — un client anonyme n'apprend pas quelles methodes le chemin accepte.
#[tokio::test]
async fn une_methode_inconnue_sur_un_chemin_d_ecriture_est_refusee_avant_le_405() {
    let request = Request::builder()
        .method("DELETE")
        .uri("/api/refresh")
        .body(Body::empty())
        .unwrap();

    assert_eq!(
        status_of(guarded_router(), request).await,
        StatusCode::UNAUTHORIZED
    );
}

/// En revanche un chemin inconnu reste un 404 : la garde ne couvre que les
/// routes d'ecriture declarees, elle n'avale pas tout le trafic.
#[tokio::test]
async fn un_chemin_inconnu_rend_404() {
    let request = Request::builder()
        .method("POST")
        .uri("/api/inexistant")
        .body(Body::empty())
        .unwrap();

    assert_eq!(
        status_of(guarded_router(), request).await,
        StatusCode::NOT_FOUND
    );
}
