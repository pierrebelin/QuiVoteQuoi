pub mod api;
pub mod application;
pub mod domain;
pub mod infrastructure;

use std::sync::Arc;

use application::ports::actor_repository::ActorRepository;
use application::ports::actor_source::ActorSource;
use application::ports::amendment_citation_repository::AmendmentCitationRepository;
use application::ports::amendment_repository::AmendmentRepository;
use application::ports::amendment_source::AmendmentSource;
use application::ports::assembly_source::AssemblySource;
use application::ports::candidate_repository::CandidateRepository;
use application::ports::dossier_group_actions_repository::{
    DossierGroupActionsRepository, DossierSummaryRepository,
};
use application::ports::dossier_repository::DossierRepository;
use application::ports::dossier_summary_generator::DossierSummaryGenerator;
use application::ports::final_vote_repository::FinalVoteRepository;
use application::ports::group_repository::GroupRepository;
use application::ports::health_probe::HealthProbe;
use application::ports::scrutin_repository::ScrutinRepository;
use application::ports::scrutin_source::ScrutinSource;
use application::ports::theme_classifier::ThemeClassifier;
use application::ports::theme_repository::ThemeRepository;

#[derive(Clone)]
pub struct AppState {
    pub health_probe: Arc<dyn HealthProbe>,
    pub assembly_source: Arc<dyn AssemblySource>,
    pub candidate_repository: Arc<dyn CandidateRepository>,
    pub dossier_repository: Arc<dyn DossierRepository>,
    pub dossier_group_actions_repository: Arc<dyn DossierGroupActionsRepository>,
    pub dossier_summary_repository: Arc<dyn DossierSummaryRepository>,
    pub dossier_summary_generator: Arc<dyn DossierSummaryGenerator>,
    pub actor_source: Arc<dyn ActorSource>,
    pub actor_repository: Arc<dyn ActorRepository>,
    pub scrutin_source: Arc<dyn ScrutinSource>,
    pub amendment_source: Arc<dyn AmendmentSource>,
    pub scrutin_repository: Arc<dyn ScrutinRepository>,
    pub amendment_repository: Arc<dyn AmendmentRepository>,
    pub amendment_citation_repository: Arc<dyn AmendmentCitationRepository>,
    pub final_vote_repository: Arc<dyn FinalVoteRepository>,
    pub group_repository: Arc<dyn GroupRepository>,
    pub theme_repository: Arc<dyn ThemeRepository>,
    pub theme_classifier: Arc<dyn ThemeClassifier>,
}
