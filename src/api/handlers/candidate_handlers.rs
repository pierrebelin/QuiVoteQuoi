use axum::extract::{Query, State};
use axum::Json;
use chrono::{Duration, Utc};

use crate::api::candidate_dto::{
    CandidateComparisonResponse, CandidateListQuery, ChangedSourcesQuery, ChangedSourcesResponse,
};
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

/// Sources de programme dont le contenu a changé : file de revue humaine,
/// jamais une publication automatique (README.md §8.2).
pub async fn list_changed_sources(
    State(state): State<AppState>,
    Query(params): Query<ChangedSourcesQuery>,
) -> Result<Json<ChangedSourcesResponse>, ApiError> {
    let days = params.jours.unwrap_or(7).clamp(1, 90);
    let records = state
        .candidate_repository
        .changed_sources(Utc::now() - Duration::days(days))
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(records.into()))
}

fn status_of(error: BrowseCandidatesError) -> ApiError {
    match error {
        BrowseCandidatesError::Repository(_) => ApiError::internal(error),
        _ => ApiError::bad_request(error),
    }
}
