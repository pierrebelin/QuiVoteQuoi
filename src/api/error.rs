use std::fmt::Display;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Message rendu au client pour toute panne interne. Le texte de l'erreur
/// reste dans le journal: il peut nommer une table, une requete ou un hote.
pub const INTERNAL_ERROR_MESSAGE: &str = "internal server error";

/// Erreur d'un handler, convertie en reponse HTTP.
///
/// Seules les erreurs du visiteur (demande mal formee, ressource absente)
/// renvoient leur texte; une panne interne est journalisee et repond un
/// message generique.
#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    NotFound(String),
    Internal(String),
}

impl ApiError {
    pub fn bad_request(error: impl Display) -> Self {
        Self::BadRequest(error.to_string())
    }

    pub fn not_found(error: impl Display) -> Self {
        Self::NotFound(error.to_string())
    }

    pub fn internal(error: impl Display) -> Self {
        Self::Internal(error.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message).into_response(),
            Self::NotFound(message) => (StatusCode::NOT_FOUND, message).into_response(),
            Self::Internal(error) => {
                tracing::error!(%error, "request failed");
                (StatusCode::INTERNAL_SERVER_ERROR, INTERNAL_ERROR_MESSAGE).into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn body_of(response: Response) -> String {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn an_internal_error_hides_its_text_from_the_client() {
        let response = ApiError::internal("database error: relation \"dossiers\" does not exist")
            .into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body_of(response).await, INTERNAL_ERROR_MESSAGE);
    }

    #[tokio::test]
    async fn a_visitor_error_keeps_its_text() {
        let response = ApiError::bad_request("unknown parliamentary group: PS").into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(body_of(response).await, "unknown parliamentary group: PS");
    }

    #[tokio::test]
    async fn a_missing_resource_answers_not_found() {
        let response = ApiError::not_found("Dossier not found").into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
