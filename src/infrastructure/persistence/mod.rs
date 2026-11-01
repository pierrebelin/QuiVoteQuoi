pub mod pg_actor_repository;
pub mod pg_amendment_repository;
pub mod pg_candidate_repository;
pub mod pg_dossier_group_actions_repository;
pub mod pg_dossier_repository;
pub mod pg_dossier_summary_repository;
pub mod pg_final_vote_repository;
pub mod pg_group_repository;
pub mod pg_scrutin_repository;
pub mod pg_theme_repository;

use crate::application::ports::RepositoryError;

/// Convertit une erreur sqlx en `RepositoryError::Database`, message inchangé.
pub(crate) fn db(error: sqlx::Error) -> RepositoryError {
    RepositoryError::Database(error.to_string())
}
