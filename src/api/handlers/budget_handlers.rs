use axum::extract::{Path, State};
use axum::Json;

use crate::api::budget_dto::{requested_year, BudgetExerciseResponse};
use crate::api::error::ApiError;
use crate::application::use_cases::get_budget_exercise::{
    GetBudgetExercise, GetBudgetExerciseError,
};
use crate::domain::budget::BudgetYear;
use crate::AppState;

/// Exercice le plus récent.
pub async fn get_latest_budget(
    State(state): State<AppState>,
) -> Result<Json<BudgetExerciseResponse>, ApiError> {
    budget_of(&state, None).await
}

/// Exercice d'une année. Une année illisible ou hors bornes est une demande
/// mal formée (400) ; une année lisible mais jamais importée est une adresse
/// qui n'existe pas (404).
pub async fn get_budget(
    State(state): State<AppState>,
    Path(year): Path<String>,
) -> Result<Json<BudgetExerciseResponse>, ApiError> {
    let year = requested_year(&year).ok_or_else(|| ApiError::bad_request("invalid budget year"))?;
    budget_of(&state, Some(year)).await
}

async fn budget_of(
    state: &AppState,
    year: Option<BudgetYear>,
) -> Result<Json<BudgetExerciseResponse>, ApiError> {
    let view = GetBudgetExercise::new(state.budget_repository.as_ref())
        .execute(year)
        .await
        .map_err(status_of)?;

    Ok(Json(view.into()))
}

/// Une année absente est une adresse qui n'existe pas, pas une panne.
fn status_of(error: GetBudgetExerciseError) -> ApiError {
    match error {
        GetBudgetExerciseError::UnknownYear(_) | GetBudgetExerciseError::NoExercise => {
            ApiError::not_found(error)
        }
        GetBudgetExerciseError::Repository(_) => ApiError::internal(error),
    }
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    use super::*;
    use crate::api::error::INTERNAL_ERROR_MESSAGE;
    use crate::application::ports::RepositoryError;

    async fn answer_of(error: GetBudgetExerciseError) -> (StatusCode, String) {
        let response = status_of(error).into_response();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, String::from_utf8(bytes.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn a_year_never_imported_answers_not_found() {
        let (status, _) = answer_of(GetBudgetExerciseError::UnknownYear(
            BudgetYear::new(2030).unwrap(),
        ))
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn an_empty_base_answers_not_found() {
        let (status, _) = answer_of(GetBudgetExerciseError::NoExercise).await;

        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn a_database_failure_answers_a_generic_internal_error() {
        let (status, body) = answer_of(GetBudgetExerciseError::Repository(
            RepositoryError::Database("relation \"budget_exercises\" does not exist".to_string()),
        ))
        .await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body, INTERNAL_ERROR_MESSAGE);
    }
}
