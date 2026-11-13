use axum::extract::{Path, State};
use axum::Json;
use chrono::NaiveDate;

use crate::api::clock::paris_today;
use crate::api::error::ApiError;
use crate::api::week_dto::{requested_date, WeekResponse};
use crate::application::use_cases::get_week_of_votes::{GetWeekOfVotes, GetWeekOfVotesError};
use crate::AppState;

/// Semaine par defaut: la courante, ou la derniere publiee quand elle est vide (RM-07).
pub async fn get_current_week(
    State(state): State<AppState>,
) -> Result<Json<WeekResponse>, ApiError> {
    week_of(&state, None).await
}

/// Semaine contenant la date de l'adresse (RM-03). Une date illisible est une
/// adresse qui n'existe pas, comme une date hors bornes.
pub async fn get_week(
    State(state): State<AppState>,
    Path(date): Path<String>,
) -> Result<Json<WeekResponse>, ApiError> {
    let requested = requested_date(&date)
        .ok_or_else(|| ApiError::not_found(format!("week unavailable: {date}")))?;
    week_of(&state, Some(requested)).await
}

/// Le jour courant est lu ici, en heure de Paris, et passe au cas d'usage.
async fn week_of(
    state: &AppState,
    requested: Option<NaiveDate>,
) -> Result<Json<WeekResponse>, ApiError> {
    let view = GetWeekOfVotes::new(
        state.scrutin_repository.as_ref(),
        state.amendment_citation_repository.as_ref(),
        state.actor_repository.as_ref(),
        state.dossier_repository.as_ref(),
        state.final_vote_repository.as_ref(),
        state.theme_repository.as_ref(),
    )
    .execute(requested, paris_today())
    .await
    .map_err(status_of)?;

    Ok(Json(view.into()))
}

/// Une semaine hors bornes est une adresse qui n'existe pas, pas une panne (RM-03).
fn status_of(error: GetWeekOfVotesError) -> ApiError {
    match error {
        GetWeekOfVotesError::Unavailable(_) => ApiError::not_found(error),
        GetWeekOfVotesError::Repository(_) | GetWeekOfVotesError::FinalVotes(_) => {
            ApiError::internal(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use chrono::NaiveDate;

    use super::*;
    use crate::api::error::INTERNAL_ERROR_MESSAGE;
    use crate::application::ports::RepositoryError;
    use crate::application::use_cases::browse_final_votes::BrowseFinalVotesError;
    use crate::domain::week::WeekError;

    async fn answer_of(error: GetWeekOfVotesError) -> (StatusCode, String) {
        let response = status_of(error).into_response();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, String::from_utf8(bytes.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn answers_not_found_for_an_unavailable_week_and_a_generic_error_otherwise() {
        let date = NaiveDate::from_ymd_opt(2030, 1, 1).unwrap();

        let (status, _) = answer_of(GetWeekOfVotesError::Unavailable(WeekError::Unavailable(
            date,
        )))
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        let (status, body) = answer_of(GetWeekOfVotesError::Repository(
            RepositoryError::Database("relation \"scrutins\" does not exist".into()),
        ))
        .await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body, INTERNAL_ERROR_MESSAGE);

        let (status, body) = answer_of(GetWeekOfVotesError::FinalVotes(
            BrowseFinalVotesError::TooManyGroups(5),
        ))
        .await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body, INTERNAL_ERROR_MESSAGE);
    }
}
