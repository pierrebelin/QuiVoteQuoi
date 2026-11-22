use axum::extract::{Path, State};
use axum::Json;
use chrono::NaiveDate;

use crate::api::error::ApiError;
use crate::api::reel_dto::ReelResponse;
use crate::api::week_dto::requested_date;
use crate::application::use_cases::prepare_reel::PrepareReel;
use crate::AppState;

/// Jour de l'adresse du Reel. Une date illisible est une adresse qui n'existe pas.
pub fn requested_day(raw: &str) -> Result<NaiveDate, ApiError> {
    requested_date(raw).ok_or_else(|| ApiError::not_found(format!("reel unavailable: {raw}")))
}

/// Reel prepare le jour de l'adresse (CU-01, CU-02).
pub async fn get_reel(
    State(state): State<AppState>,
    Path(date): Path<String>,
) -> Result<Json<ReelResponse>, ApiError> {
    let today = requested_day(&date)?;
    let prepared = PrepareReel::new(
        state.scrutin_repository.as_ref(),
        state.final_vote_repository.as_ref(),
        state.ingestion_journal.as_ref(),
    )
    .execute(today)
    .await
    .map_err(ApiError::internal)?;

    Ok(Json(prepared.into()))
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    use super::*;

    #[test]
    fn answers_not_found_for_an_unreadable_date() {
        for raw in ["2026-2-3", "demain", "2026-13-01", ""] {
            let status = match requested_day(raw) {
                Ok(_) => StatusCode::OK,
                Err(error) => error.into_response().status(),
            };

            assert_eq!(status, StatusCode::NOT_FOUND, "{raw}");
        }
    }
}
