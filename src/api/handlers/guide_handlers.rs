use axum::extract::State;
use axum::Json;

use crate::api::error::ApiError;
use crate::api::guide_dto::DatasetResponse;
use crate::application::use_cases::describe_dataset::DescribeDataset;
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
