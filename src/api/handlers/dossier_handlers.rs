use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;

use crate::api::dossier_group_actions_dto::DossierGroupActionsResponse;
use crate::api::dto::{
    CurateBody, DossierDetailDto, DossierDto, DossierPageQuery, DossierPageResponse,
    RecentActivityQuery, RecentDossiersResponse, RefreshQuery, RefreshResponse, RegistryResponse,
    SuggestionsQuery, SuggestionsResponse,
};
use crate::api::error::ApiError;
use crate::application::ports::dossier_repository::DossierCriteria;
use crate::application::use_cases::browse_dossiers::{BrowseDossiers, DossierQuery};
use crate::application::use_cases::curate_dossier::CurateDossier;
use crate::application::use_cases::fetch_recent_dossiers::FetchRecentDossiers;
use crate::application::use_cases::get_dossier_detail::GetDossierDetail;
use crate::application::use_cases::get_dossier_group_actions::GetDossierGroupActions;
use crate::application::use_cases::refresh_actor_registry::RefreshActorRegistry;
use crate::application::use_cases::refresh_all::RefreshAll;
use crate::application::use_cases::refresh_dossier_group_summaries::RefreshDossierGroupSummaries;
use crate::application::use_cases::refresh_dossiers::RefreshScope;
use crate::application::use_cases::save_dossier::SaveDossier;
use crate::application::use_cases::suggest_dossiers::SuggestDossiers;
use crate::domain::dossier::DossierUid;
use crate::infrastructure::config;
use crate::AppState;

pub async fn get_recent_dossiers(
    State(state): State<AppState>,
    Query(params): Query<RecentActivityQuery>,
) -> Result<Json<RecentDossiersResponse>, ApiError> {
    let uc = FetchRecentDossiers::new(state.dossier_repository.as_ref());

    let dossiers = uc
        .execute(params.days, Utc::now().date_naive())
        .await
        .map_err(ApiError::internal)?;

    let dtos: Vec<DossierDto> = dossiers.into_iter().map(DossierDto::from).collect();

    Ok(Json(RecentDossiersResponse {
        count: dtos.len(),
        dossiers: dtos,
    }))
}

/// Liste paginée des dossiers, du plus récent au plus ancien. Les critères de
/// la requête restreignent l'affichage, jamais le contenu de la base.
pub async fn browse_dossiers(
    State(state): State<AppState>,
    Query(params): Query<DossierPageQuery>,
) -> Result<Json<DossierPageResponse>, ApiError> {
    let query = DossierQuery::new(params.page, params.per_page, DossierCriteria::from(&params));
    let uc = BrowseDossiers::new(state.dossier_repository.as_ref());

    let per_page = query.per_page();
    let page_number = query.page();

    let page = uc.execute(query).await.map_err(ApiError::internal)?;

    Ok(Json(DossierPageResponse {
        page: page_number,
        per_page,
        total: page.total,
        total_pages: page.total.div_euclid(per_page)
            + i64::from(page.total.rem_euclid(per_page) > 0),
        dossiers: page.items.into_iter().map(DossierDto::from).collect(),
    }))
}

pub async fn get_dossier_detail(
    State(state): State<AppState>,
    Path(uid): Path<String>,
) -> Result<Json<DossierDetailDto>, ApiError> {
    let uid = DossierUid::new(uid).map_err(ApiError::bad_request)?;

    let uc = GetDossierDetail::new(
        state.dossier_repository.as_ref(),
        state.assembly_source.as_ref(),
        state.actor_repository.as_ref(),
    );

    let result = uc
        .execute(&uid)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("Dossier not found"))?;

    Ok(Json(DossierDetailDto::from_result(result)))
}

pub async fn get_dossier_group_actions(
    State(state): State<AppState>,
    Path(uid): Path<String>,
) -> Result<Json<DossierGroupActionsResponse>, ApiError> {
    let result = GetDossierGroupActions::new(
        state.dossier_group_actions_repository.as_ref(),
        state.dossier_summary_repository.as_ref(),
    )
    .execute(&uid)
    .await
    .map_err(ApiError::internal)?
    .ok_or_else(|| ApiError::not_found("Dossier not found"))?;

    Ok(Json(DossierGroupActionsResponse::from(result)))
}

pub async fn save_dossier(
    State(state): State<AppState>,
    Path(uid): Path<String>,
) -> Result<StatusCode, ApiError> {
    let uid = DossierUid::new(uid).map_err(ApiError::bad_request)?;

    let uc = SaveDossier::new(
        state.assembly_source.as_ref(),
        state.dossier_repository.as_ref(),
        state.actor_repository.as_ref(),
    );

    uc.execute(&uid).await.map_err(ApiError::internal)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_suggestions(
    State(state): State<AppState>,
    Query(params): Query<SuggestionsQuery>,
) -> Result<Json<SuggestionsResponse>, ApiError> {
    let uc = SuggestDossiers::new(state.dossier_repository.as_ref());

    let dossiers = uc.execute(params.count).await.map_err(ApiError::internal)?;

    let dtos: Vec<DossierDto> = dossiers.into_iter().map(DossierDto::from).collect();

    Ok(Json(SuggestionsResponse {
        count: dtos.len(),
        suggestions: dtos,
    }))
}

pub async fn curate_dossier(
    State(state): State<AppState>,
    Path(uid): Path<String>,
    Json(body): Json<CurateBody>,
) -> Result<StatusCode, ApiError> {
    let uid = DossierUid::new(uid).map_err(ApiError::bad_request)?;

    let uc = CurateDossier::new(state.dossier_repository.as_ref());

    uc.execute(&uid, body.status).await.map_err(|e| match &e {
        crate::application::use_cases::curate_dossier::CurateError::NotFound(_) => {
            ApiError::not_found(e)
        }
        _ => ApiError::internal(e),
    })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Rafraichit le referentiel puis les dossiers, dans cet ordre.
///
/// Par defaut seuls les dossiers dont la source a bouge sont reecrits.
/// `?full=true` force la reecriture complete, necessaire apres un changement
/// de regle de derivation (score, sort, rattachement).
pub async fn refresh_dossiers(
    State(state): State<AppState>,
    Query(query): Query<RefreshQuery>,
) -> Result<Json<RefreshResponse>, ApiError> {
    let scope = if query.full {
        RefreshScope::Full
    } else {
        RefreshScope::Incremental
    };

    let uc = RefreshAll::new(
        state.actor_source.as_ref(),
        state.actor_repository.as_ref(),
        state.assembly_source.as_ref(),
        state.dossier_repository.as_ref(),
        state.scrutin_source.as_ref(),
        state.scrutin_repository.as_ref(),
        state.amendment_source.as_ref(),
        state.amendment_repository.as_ref(),
        state.amendment_citation_repository.as_ref(),
        state.theme_repository.as_ref(),
        state.theme_classifier.as_ref(),
        config::theme_batch_per_refresh(),
        config::amendment_batch_per_refresh(),
    );

    let outcome = uc
        .execute_with(scope, Utc::now().date_naive())
        .await
        .map_err(ApiError::internal)?;

    let summary_report = RefreshDossierGroupSummaries::new(
        state.dossier_group_actions_repository.as_ref(),
        state.dossier_summary_repository.as_ref(),
        state.dossier_summary_generator.as_ref(),
    )
    .execute(config::dossier_summary_batch_per_refresh())
    .await
    .map_err(ApiError::internal)?;

    Ok(Json(RefreshResponse::from((outcome, summary_report))))
}

pub async fn refresh_actor_registry(
    State(state): State<AppState>,
) -> Result<Json<RegistryResponse>, ApiError> {
    let uc =
        RefreshActorRegistry::new(state.actor_source.as_ref(), state.actor_repository.as_ref());

    let summary = uc.execute().await.map_err(ApiError::internal)?;

    Ok(Json(RegistryResponse::from(summary)))
}
