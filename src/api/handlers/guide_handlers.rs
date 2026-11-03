use axum::extract::State;
use axum::Json;
use chrono::Utc;

use crate::api::error::ApiError;
use crate::api::freshness_dto::FreshnessResponse;
use crate::api::guide_dto::DatasetResponse;
use crate::application::use_cases::describe_dataset::DescribeDataset;
use crate::application::use_cases::describe_freshness::DescribeFreshness;
use crate::AppState;

/// Chiffres du guide de lecture (page « Comprendre »).
pub async fn get_dataset(State(state): State<AppState>) -> Result<Json<DatasetResponse>, ApiError> {
    let overview = DescribeDataset::new(
        state.scrutin_repository.as_ref(),
        state.theme_repository.as_ref(),
    )
    .execute()
    .await
    .map_err(ApiError::internal)?;

    Ok(Json(DatasetResponse::from(overview)))
}

/// Fraicheur des quatre sources (RM-F6, RM-F7). L'horloge est lue ici pour que
/// le use case reste rejouable a l'identique.
pub async fn get_freshness(
    State(state): State<AppState>,
) -> Result<Json<FreshnessResponse>, ApiError> {
    let overview = DescribeFreshness::new(
        state.ingestion_journal.as_ref(),
        state.scrutin_repository.as_ref(),
    )
    .execute(Utc::now())
    .await
    .map_err(ApiError::internal)?;

    Ok(Json(FreshnessResponse::from(overview)))
}
