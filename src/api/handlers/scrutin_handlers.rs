use axum::extract::{Path, Query, State};
use axum::Json;

use crate::api::error::ApiError;
use crate::api::scrutin_dto::{
    DossierScrutinsResponse, ScrutinDetailDto, ScrutinListQuery, ScrutinListResponse,
    ScrutinSummaryDto, ScrutinsRefreshResponse, SHOW_OF_HANDS_NOTE,
};
use crate::application::ports::scrutin_repository::ScrutinFilter;
use crate::application::use_cases::browse_themes::BrowseThemes;
use crate::application::use_cases::get_scrutin_detail::GetScrutinDetail;
use crate::application::use_cases::journaling::Journaling;
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

    let page = ListScrutins::new(
        state.scrutin_repository.as_ref(),
        state.amendment_citation_repository.as_ref(),
        state.actor_repository.as_ref(),
        state.dossier_repository.as_ref(),
    )
    .execute(filter)
    .await
    .map_err(ApiError::internal)?;

    let mut response = ScrutinListResponse::from((page, offset));
    attach_families(&state, &mut response.scrutins).await?;
    Ok(Json(response))
}

/// Pose sur chaque scrutin ses familles, en une seule lecture pour la page.
async fn attach_families(
    state: &AppState,
    scrutins: &mut [ScrutinSummaryDto],
) -> Result<(), ApiError> {
    let uids: Vec<String> = scrutins.iter().map(|s| s.uid.clone()).collect();
    let mut families = BrowseThemes::new(state.theme_repository.as_ref())
        .families_of_scrutins(&uids)
        .await
        .map_err(ApiError::internal)?;
    for scrutin in scrutins.iter_mut() {
        scrutin.families = families
            .remove(&scrutin.uid)
            .unwrap_or_default()
            .into_iter()
            .map(Into::into)
            .collect();
    }
    Ok(())
}

/// CU-03 — Detail d'un scrutin.
pub async fn get_scrutin_detail(
    State(state): State<AppState>,
    Path(uid): Path<String>,
) -> Result<Json<ScrutinDetailDto>, ApiError> {
    let uid = ScrutinUid::new(uid).map_err(ApiError::bad_request)?;

    let detail = GetScrutinDetail::new(
        state.scrutin_repository.as_ref(),
        state.amendment_citation_repository.as_ref(),
        state.actor_repository.as_ref(),
    )
    .execute(&uid)
    .await
    .map_err(ApiError::internal)?
    .ok_or_else(|| ApiError::not_found("Scrutin not found"))?;

    let families = BrowseThemes::new(state.theme_repository.as_ref())
        .families_of_scrutins(&[uid.as_str().to_string()])
        .await
        .map_err(ApiError::internal)?
        .remove(uid.as_str())
        .unwrap_or_default();

    let mut dto = ScrutinDetailDto::from(detail);
    dto.families = families.into_iter().map(Into::into).collect();
    Ok(Json(dto))
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
    let page = ListScrutins::new(
        state.scrutin_repository.as_ref(),
        state.amendment_citation_repository.as_ref(),
        state.actor_repository.as_ref(),
        state.dossier_repository.as_ref(),
    )
    .execute(filter)
    .await
    .map_err(ApiError::internal)?;

    let mut scrutins: Vec<ScrutinSummaryDto> = page
        .items
        .into_iter()
        .map(ScrutinSummaryDto::from)
        .collect();
    attach_families(&state, &mut scrutins).await?;

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
        Journaling::to(state.ingestion_journal.as_ref(), chrono::Utc::now()),
    )
    .execute()
    .await
    .map_err(ApiError::internal)?;

    Ok(Json(ScrutinsRefreshResponse::from(summary)))
}
