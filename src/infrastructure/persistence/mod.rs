pub mod pg_actor_repository;
pub mod pg_amendment_repository;
pub mod pg_amendment_citation_repository;
pub mod pg_candidate_repository;
pub mod pg_dossier_group_actions_repository;
pub mod pg_dossier_repository;
pub mod pg_dossier_summary_repository;
pub mod pg_final_vote_repository;
pub mod pg_group_repository;
pub mod pg_health_probe;
pub mod pg_ingestion_journal;
pub mod pg_scrutin_repository;
pub mod pg_theme_repository;

use crate::application::ports::RepositoryError;

/// Convertit une erreur sqlx en `RepositoryError::Database`, message inchangé.
pub(crate) fn db(error: sqlx::Error) -> RepositoryError {
    RepositoryError::Database(error.to_string())
}

/// Selection SQL des votes sur l'ensemble d'un texte, alias `s` sur `scrutins`.
///
/// La condition reprend `domain::final_vote::FINAL_VOTE_MARKERS`, ecrite ici en
/// SQL pour rester indexable: la meme regle appliquee en Rust obligerait a
/// charger les 8 434 scrutins pour en garder 222. Les deux formes d'apostrophe
/// sont testees, la source melange les deux. Le `LIKE` est sensible a la casse
/// et ne retire pas les blancs de tete, contrairement a `is_final_vote_subject`.
pub(crate) const FINAL_VOTE_PREDICATE: &str =
    "(s.subject LIKE 'l''ensemble %' OR s.subject LIKE 'l\u{2019}ensemble %')";

/// Une ligne lue que le domaine refuse: la base est sous le controle des
/// migrations, une telle ligne est une incoherence a signaler, pas a masquer.
pub(crate) fn invalid_row(error: impl std::fmt::Display) -> RepositoryError {
    RepositoryError::Database(format!("invalid row: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::final_vote::FINAL_VOTE_MARKERS;

    #[test]
    fn final_vote_predicate_tests_every_domain_marker_as_a_prefix() {
        for marker in FINAL_VOTE_MARKERS {
            let escaped = marker.replace('\'', "''");

            assert!(
                FINAL_VOTE_PREDICATE.contains(&format!("LIKE '{escaped}%'")),
                "marker {marker:?} missing from {FINAL_VOTE_PREDICATE}"
            );
        }
    }

    #[test]
    fn final_vote_predicate_has_one_like_per_domain_marker() {
        assert_eq!(
            FINAL_VOTE_PREDICATE.matches("LIKE").count(),
            FINAL_VOTE_MARKERS.len()
        );
    }
}
