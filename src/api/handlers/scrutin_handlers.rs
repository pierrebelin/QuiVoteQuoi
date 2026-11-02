use axum::extract::{Path, Query, State};
use axum::Json;

use crate::api::error::ApiError;
use crate::api::scrutin_dto::{
    DossierScrutinsResponse, ScrutinDetailDto, ScrutinListQuery, ScrutinListResponse,
    ScrutinSummaryDto, ScrutinsRefreshResponse, SHOW_OF_HANDS_NOTE,
};
use crate::application::ports::scrutin_repository::ScrutinFilter;
use crate::application::use_cases::get_scrutin_detail::GetScrutinDetail;
use crate::application::use_cases::list_scrutins::ListScrutins;
use crate::application::use_cases::refresh_scrutins::RefreshScrutins;
use crate::domain::scrutin::ScrutinUid;
use crate::AppState;

/// CU-02 — Liste des scrutins.
pub async fn list_scrutins(
    State(state): State<AppState>,
    Query(params): Query<ScrutinListQuery>,
) -> Result<Json<ScrutinListResponse>, ApiError> {
    let filter: ScrutinFilter = params.into();
    let offset = filter.offset.max(0);

    let page = ListScrutins::new(state.scrutin_repository.as_ref())
        .execute(filter)
        .await
        .map_err(ApiError::internal)?;

    Ok(Json(ScrutinListResponse::from((page, offset))))
}

/// CU-03 — Detail d'un scrutin.
pub async fn get_scrutin_detail(
    State(state): State<AppState>,
    Path(uid): Path<String>,
) -> Result<Json<ScrutinDetailDto>, ApiError> {
    let uid = ScrutinUid::new(uid).map_err(ApiError::bad_request)?;

    let detail = GetScrutinDetail::new(
        state.scrutin_repository.as_ref(),
        state.actor_repository.as_ref(),
    )
    .execute(&uid)
    .await
    .map_err(ApiError::internal)?
    .ok_or_else(|| ApiError::not_found("Scrutin not found"))?;

    Ok(Json(ScrutinDetailDto::from(detail)))
}

/// CU-04 — Scrutins d'un dossier. La section existe meme vide: la source peut
/// ne rattacher aucun scrutin a un dossier, et le taire laisserait croire a une
/// absence de vote.
pub async fn get_dossier_scrutins(
    State(state): State<AppState>,
    Path(uid): Path<String>,
    Query(params): Query<ScrutinListQuery>,
) -> Result<Json<DossierScrutinsResponse>, ApiError> {
    let mut filter: ScrutinFilter = params.into();
    filter.dossier_uid = Some(uid);
    let offset = filter.offset.max(0);
    let page = ListScrutins::new(state.scrutin_repository.as_ref())
        .execute(filter)
        .await
        .map_err(ApiError::internal)?;

    let scrutins: Vec<ScrutinSummaryDto> = page
        .items
        .into_iter()
        .map(ScrutinSummaryDto::from)
        .collect();

    Ok(Json(DossierScrutinsResponse {
        count: scrutins.len(),
        total: page.total,
        offset,
        scrutins,
        coverage_note: SHOW_OF_HANDS_NOTE,
    }))
}

/// CU-01 — Ingerer les scrutins.
///
/// RM-11: le referentiel doit avoir ete rafraichi avant. Le use case detecte un
/// referentiel vide et remonte l'anomalie plutot que de reconstruire a vide.
/// `POST /api/refresh` enchaine les trois dans l'ordre.
pub async fn refresh_scrutins(
    State(state): State<AppState>,
) -> Result<Json<ScrutinsRefreshResponse>, ApiError> {
    let summary = RefreshScrutins::new(
        state.scrutin_source.as_ref(),
        state.scrutin_repository.as_ref(),
        state.actor_repository.as_ref(),
    )
    .execute()
    .await
    .map_err(ApiError::internal)?;

    Ok(Json(ScrutinsRefreshResponse::from(summary)))
}
