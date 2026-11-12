use axum::extract::{Query, State};
use axum::Json;

use crate::api::error::ApiError;
use crate::api::final_vote_dto::{
    ExplanationCoverageDto, FinalVoteListQuery, FinalVoteListResponse,
};
use crate::application::use_cases::browse_final_votes::{
    BrowseFinalVotes, BrowseFinalVotesCommand, BrowseFinalVotesError,
};
use crate::application::use_cases::describe_explanation_coverage::DescribeExplanationCoverage;
use crate::AppState;

/// RM-10 — Compteurs du rattachement des explications, pour la page méthode.
pub async fn get_final_votes_method(
    State(state): State<AppState>,
) -> Result<Json<ExplanationCoverageDto>, ApiError> {
    let coverage = DescribeExplanationCoverage::new(state.vote_explanation_repository.as_ref())
        .execute()
        .await
        .map_err(ApiError::internal)?;

    Ok(Json(coverage.into()))
}

/// CU-07 — Votes sur l'ensemble d'un texte, groupe par groupe.
pub async fn list_final_votes(
    State(state): State<AppState>,
    Query(params): Query<FinalVoteListQuery>,
) -> Result<Json<FinalVoteListResponse>, ApiError> {
    let command: BrowseFinalVotesCommand = params.into();
    let offset = command.offset.unwrap_or(0).max(0);

    let view = BrowseFinalVotes::new(state.final_vote_repository.as_ref())
        .execute(command)
        .await
        .map_err(status_of)?;

    Ok(Json(FinalVoteListResponse::from((view, offset))))
}

/// Une demande mal formee est une erreur du visiteur, pas du serveur: le
/// distinguer evite de faire passer un sigle inconnu pour une panne.
///
/// Un vote que le domaine refuse vient de la base, pas de la demande: c'est une
/// panne, pas une erreur du visiteur.
fn status_of(error: BrowseFinalVotesError) -> ApiError {
    match error {
        BrowseFinalVotesError::Repository(_)
        | BrowseFinalVotesError::Scrutin(_)
        | BrowseFinalVotesError::Dossier(_)
        | BrowseFinalVotesError::Actor(_) => ApiError::internal(error),
        BrowseFinalVotesError::TooManyGroups(_)
        | BrowseFinalVotesError::ConflictingGroupSelection
        | BrowseFinalVotesError::UnknownGroup(_)
        | BrowseFinalVotesError::UnknownFamily(_) => ApiError::bad_request(error),
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::scrutin::ScrutinError;

    use super::*;

    #[test]
    fn an_unknown_group_is_the_visitor_s_error() {
        let error = status_of(BrowseFinalVotesError::UnknownGroup("PS".to_string()));

        assert!(matches!(error, ApiError::BadRequest(message) if message.contains("PS")));
    }

    #[test]
    fn a_conflicting_group_selection_is_the_visitor_s_error() {
        let error = status_of(BrowseFinalVotesError::ConflictingGroupSelection);

        assert!(matches!(error, ApiError::BadRequest(_)));
    }

    #[test]
    fn a_vote_refused_by_the_domain_is_an_internal_failure() {
        let error = status_of(BrowseFinalVotesError::Scrutin(
            ScrutinError::EmptyOutcomeCode,
        ));

        assert!(matches!(error, ApiError::Internal(_)));
    }
}
