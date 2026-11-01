use axum::extract::{Query, State};
use axum::Json;

use crate::api::candidate_dto::{CandidateComparisonResponse, CandidateListQuery};
use crate::api::error::ApiError;
use crate::application::use_cases::browse_candidates::{
    BrowseCandidates, BrowseCandidatesCommand, BrowseCandidatesError,
};
use crate::AppState;

/// Candidatures 2027, programmes attribués et groupes associés avec source.
pub async fn list_candidates(
    State(state): State<AppState>,
    Query(params): Query<CandidateListQuery>,
) -> Result<Json<CandidateComparisonResponse>, ApiError> {
    let command: BrowseCandidatesCommand = params.into();
    let view = BrowseCandidates::new(state.candidate_repository.as_ref())
        .execute(command)
        .await
        .map_err(status_of)?;
    Ok(Json(view.into()))
}

fn status_of(error: BrowseCandidatesError) -> ApiError {
    match error {
        BrowseCandidatesError::Repository(_) => ApiError::internal(error),
        _ => ApiError::bad_request(error),
    }
}
