use axum::extract::{Path, State};
use axum::Json;
use chrono::Utc;

use crate::api::error::ApiError;
use crate::api::group_dto::{GroupDetailResponse, GroupListResponse};
use crate::application::use_cases::browse_groups::{BrowseGroups, BrowseGroupsCommand};
use crate::application::use_cases::get_group_detail::{
    GetGroupDetail, GetGroupDetailCommand, GetGroupDetailError,
};
use crate::AppState;

/// Liste des groupes parlementaires de la legislature.
pub async fn list_groups(
    State(state): State<AppState>,
) -> Result<Json<GroupListResponse>, ApiError> {
    let view = BrowseGroups::new(state.group_repository.as_ref())
        .execute(BrowseGroupsCommand { today: today() })
        .await
        .map_err(ApiError::internal)?;

    Ok(Json(view.into()))
}

/// Fiche d'un groupe, par identifiant ou par sigle.
pub async fn get_group_detail(
    State(state): State<AppState>,
    Path(group): Path<String>,
) -> Result<Json<GroupDetailResponse>, ApiError> {
    let view = GetGroupDetail::new(state.group_repository.as_ref())
        .execute(GetGroupDetailCommand {
            group,
            today: today(),
        })
        .await
        .map_err(status_of)?;

    Ok(Json(view.into()))
}

/// Date de reference des effectifs. Lue ici plutot que dans le use case, qui
/// reste rejouable a l'identique.
fn today() -> chrono::NaiveDate {
    Utc::now().date_naive()
}

/// Un sigle inconnu est une adresse qui n'existe pas, pas une panne: le
/// distinguer evite de faire passer l'un pour l'autre.
fn status_of(error: GetGroupDetailError) -> ApiError {
    match error {
        GetGroupDetailError::UnknownGroup(_) => ApiError::not_found(error),
        GetGroupDetailError::Repository(_) => ApiError::internal(error),
    }
}
