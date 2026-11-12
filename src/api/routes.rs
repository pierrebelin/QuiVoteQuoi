use axum::routing::{get, post};
use axum::{middleware, Router};
use tower_http::cors::CorsLayer;

use crate::api::security::{require_admin_token, AdminGuard};
use crate::infrastructure::config;
use crate::AppState;

use super::handlers::{
    amendment_handlers, candidate_handlers, dossier_handlers, final_vote_handlers, group_handlers,
    guide_handlers, health_handlers, scrutin_handlers, theme_handlers,
};

pub fn create_router(state: AppState, guard: AdminGuard) -> Router {
    read_routes()
        .merge(write_routes(guard))
        .layer(cors_layer())
        .with_state(state)
}

/// Consultation. Ouverte : le site publie de la donnee publique de
/// l'Assemblee nationale, et un jeton dans un bundle JavaScript public ne
/// protegerait rien (README.md §2).
fn read_routes() -> Router<AppState> {
    Router::new()
        .route("/api/health", get(health_handlers::health))
        .route(
            "/api/candidats-2027",
            get(candidate_handlers::list_candidates),
        )
        .route(
            "/api/candidats-2027/sources-modifiees",
            get(candidate_handlers::list_changed_sources),
        )
        // Liste principale : tous les dossiers, paginés (README.md §2).
        .route("/api/dossiers", get(dossier_handlers::browse_dossiers))
        // Activité récente, utilisée par la sélection éditoriale.
        .route(
            "/api/dossiers/recent",
            get(dossier_handlers::get_recent_dossiers),
        )
        .route(
            "/api/dossiers/{uid}",
            get(dossier_handlers::get_dossier_detail),
        )
        .route(
            "/api/dossiers/{uid}/lecture-groupes",
            get(dossier_handlers::get_dossier_group_actions),
        )
        // « En bref » : route fille, chargée à l'affichage de la section
        // (SPEC-en-bref CU-03).
        .route(
            "/api/dossiers/{uid}/en-bref",
            get(dossier_handlers::get_dossier_brief),
        )
        .route(
            "/api/dossiers/{uid}/scrutins",
            get(scrutin_handlers::get_dossier_scrutins),
        )
        // Amendements d'un dossier: route fille, comme les scrutins. Un dossier
        // budgetaire en porte des milliers (SPEC-amendements RM-07).
        .route(
            "/api/dossiers/{uid}/amendements",
            get(amendment_handlers::get_dossier_amendments),
        )
        .route("/api/scrutins", get(scrutin_handlers::list_scrutins))
        // Votes sur l'ensemble d'un texte, groupe par groupe (CU-07).
        .route(
            "/api/votes-finaux",
            get(final_vote_handlers::list_final_votes),
        )
        .route(
            "/api/votes-finaux/methode",
            get(final_vote_handlers::get_final_votes_method),
        )
        // Groupes parlementaires: liste et fiche. Un groupe repond a son
        // identifiant comme a son sigle, ancien ou courant (README.md §8.1).
        .route("/api/groupes", get(group_handlers::list_groups))
        .route(
            "/api/groupes/{group}",
            get(group_handlers::get_group_detail),
        )
        .route(
            "/api/scrutins/{uid}",
            get(scrutin_handlers::get_scrutin_detail),
        )
        // Guide de lecture: chiffres de la page « Comprendre ».
        .route("/api/dataset", get(guide_handlers::get_dataset))
        // Fraicheur des sources: pied de page, bandeau et workflow de controle.
        .route("/api/freshness", get(guide_handlers::get_freshness))
        // Thematisation (todo/SPEC-thematisation.md).
        .route("/api/themes", get(theme_handlers::list_families))
        .route("/api/themes/method", get(theme_handlers::get_method))
        .route(
            "/api/themes/unassigned",
            get(theme_handlers::list_unassigned_texts),
        )
        .route("/api/themes/{code}", get(theme_handlers::list_family_texts))
        .route("/api/texts/{key}", get(theme_handlers::get_text_detail))
}

/// Administration. Ingestion depuis l'Assemblee nationale et
/// thematisation LLM : aucune n'est appelee par un parcours de consultation,
/// toutes coutent du temps machine ou de la cle Anthropic. Jeton du jour
/// obligatoire (`api::security`).
fn write_routes(guard: AdminGuard) -> Router<AppState> {
    Router::new()
        .route(
            "/api/dossiers/{uid}/save",
            post(dossier_handlers::save_dossier),
        )
        .route(
            "/api/scrutins/refresh",
            post(scrutin_handlers::refresh_scrutins),
        )
        .route(
            "/api/amendements/refresh",
            post(amendment_handlers::refresh_amendments),
        )
        .route("/api/themes/extract", post(theme_handlers::extract_texts))
        .route(
            "/api/themes/propose",
            post(theme_handlers::propose_families),
        )
        .route("/api/themes/arbitrate", post(theme_handlers::arbitrate))
        .route("/api/refresh", post(dossier_handlers::refresh_dossiers))
        .route(
            "/api/registry/refresh",
            post(dossier_handlers::refresh_actor_registry),
        )
        // `route_layer` et non `layer` : une methode inconnue sur un chemin
        // connu doit rendre 405, sans passer par la garde.
        .route_layer(middleware::from_fn_with_state(guard, require_admin_token))
}

/// CORS. Le front est servi par le meme hote que l'API en production, et par
/// le proxy Vite en developpement : aucune requete inter-origines legitime
/// n'existe. `ALLOWED_ORIGINS` (liste separee par des virgules) ouvre une
/// exception quand une origine tierce apparait ; sans elle, rien n'est
/// autorise. Remplace `CorsLayer::permissive()`, qui laissait n'importe quelle
/// page du web lire les reponses de l'API avec les identifiants du visiteur.
fn cors_layer() -> CorsLayer {
    let origins: Vec<_> = config::allowed_origins()
        .into_iter()
        .filter_map(|origin| match origin.parse() {
            Ok(value) => Some(value),
            Err(_) => {
                tracing::warn!(%origin, "ALLOWED_ORIGINS: origine ignorée, valeur illisible");
                None
            }
        })
        .collect();

    if origins.is_empty() {
        CorsLayer::new()
    } else {
        tracing::info!(count = origins.len(), "CORS: origines tierces autorisées");
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([axum::http::Method::GET, axum::http::Method::HEAD])
            .allow_headers([axum::http::header::CONTENT_TYPE])
    }
}
