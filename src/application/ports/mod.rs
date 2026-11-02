pub mod actor_repository;
pub mod actor_source;
pub mod amendment_repository;
pub mod amendment_citation_repository;
pub mod amendment_source;
pub mod assembly_source;
pub mod candidate_repository;
pub mod dossier_group_actions_repository;
pub mod dossier_repository;
pub mod dossier_summary_generator;
pub mod final_vote_repository;
pub mod group_repository;
pub mod health_probe;
pub mod scrutin_repository;
pub mod scrutin_source;
pub mod theme_classifier;
pub mod theme_repository;

#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("download failed: {0}")]
    Download(String),
    #[error("parse failed: {0}")]
    Parse(String),
}

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("database error: {0}")]
    Database(String),
}
