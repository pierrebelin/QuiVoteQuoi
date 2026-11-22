use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::NaiveDate;

use crate::api::error::ApiError;
use crate::api::reel_dto::{ReelQuery, ReelResponse};
use crate::api::week_dto::requested_date;
use crate::application::use_cases::prepare_reel::{PrepareReel, PresentedSubjects};
use crate::AppState;

/// Jour de l'adresse du Reel. Une date illisible est une adresse qui n'existe pas.
pub fn requested_day(raw: &str) -> Result<NaiveDate, ApiError> {
    requested_date(raw).ok_or_else(|| ApiError::not_found(format!("reel unavailable: {raw}")))
}

/// Sujets deja presentes de l'adresse; un code ou une liste hors borne est une mauvaise requete.
pub fn presented_subjects(query: ReelQuery) -> Result<PresentedSubjects, ApiError> {
    PresentedSubjects::try_from(query).map_err(ApiError::bad_request)
}

/// Reel prepare le jour de l'adresse (CU-01, CU-02).
pub async fn get_reel(
    State(state): State<AppState>,
    Path(date): Path<String>,
    Query(query): Query<ReelQuery>,
) -> Result<Json<ReelResponse>, ApiError> {
    let today = requested_day(&date)?;
    let presented = presented_subjects(query)?;
    let prepared = PrepareReel::new(
        state.scrutin_repository.as_ref(),
        state.final_vote_repository.as_ref(),
        state.ingestion_journal.as_ref(),
        state.candidate_repository.as_ref(),
        state.theme_repository.as_ref(),
        state.site_news_catalog.as_ref(),
    )
    .execute(today, &presented)
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
    fn answers_bad_request_for_an_unknown_theme_code() {
        let query = ReelQuery {
            dernier_theme: Some("astrologie".into()),
            nouveautes_presentees: None,
        };

        let status = match presented_subjects(query) {
            Ok(_) => StatusCode::OK,
            Err(error) => error.into_response().status(),
        };

        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn answers_bad_request_beyond_the_presented_site_news_limit() {
        let query = |count: usize| ReelQuery {
            dernier_theme: None,
            nouveautes_presentees: Some(
                (0..count).map(|n| format!("n{n}")).collect::<Vec<_>>().join(","),
            ),
        };

        let at_limit = presented_subjects(query(500));
        let beyond = match presented_subjects(query(501)) {
            Ok(_) => StatusCode::OK,
            Err(error) => error.into_response().status(),
        };

        assert!(at_limit.is_ok());
        assert_eq!(beyond, StatusCode::BAD_REQUEST);
    }

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
