use chrono::NaiveDate;

use crate::application::ports::actor_repository::{ActorRepository, RegistrySummary};
use crate::application::ports::actor_source::ActorSource;
use crate::application::ports::amendment_repository::AmendmentRepository;
use crate::application::ports::amendment_source::AmendmentSource;
use crate::application::ports::assembly_source::AssemblySource;
use crate::application::ports::dossier_repository::DossierRepository;
use crate::application::ports::scrutin_repository::ScrutinRepository;
use crate::application::ports::scrutin_source::ScrutinSource;
use crate::application::ports::theme_classifier::ThemeClassifier;
use crate::application::ports::theme_repository::ThemeRepository;

use super::extract_debated_texts::{ExtractDebatedTexts, ExtractionReport};
use super::propose_theme_families::{ProposalRun, ProposeThemeFamilies};
use super::refresh_actor_registry::RefreshActorRegistry;
use super::refresh_amendments::{AmendmentsSummary, RefreshAmendments};
use super::refresh_dossiers::{DossiersSummary, RefreshDossiers, RefreshError, RefreshScope};
use super::refresh_scrutins::{RefreshScrutins, ScrutinsSummary};

#[derive(Debug, Clone)]
pub struct RefreshOutcome {
    pub registry: Option<RegistrySummary>,
    /// Anomalie signalee plutot que silencieuse: le referentiel n'a pas pu etre
    /// rafraichi, les rattachements reposent sur la version precedente.
    pub registry_anomaly: Option<String>,
    pub dossiers: DossiersSummary,
    pub scrutins: Option<ScrutinsSummary>,
    /// Idem cote scrutins: source indisponible, les scrutins deja stockes
    /// restent en place et la lacune est signalee.
    pub scrutins_anomaly: Option<String>,
    /// Extraction des textes debattus, deterministe (RM-02).
    pub extraction: Option<ExtractionReport>,
    pub extraction_anomaly: Option<String>,
    /// Rattachement des objets encore en attente, plafonne a chaque passe.
    pub themes: Option<ProposalRun>,
    pub themes_anomaly: Option<String>,
    /// Amendements. Passe la plus longue, donc la derniere: une troncature ne
    /// coute jamais le reste du rafraichissement.
    pub amendments: Option<AmendmentsSummary>,
    pub amendments_anomaly: Option<String>,
}

/// Rafraichissement complet, dans l'ordre impose: referentiel d'abord, dossiers
/// et scrutins ensuite. L'inverse calculerait les rattachements et les
/// repartitions reconstruites sur des appartenances perimees (RM-11).
///
/// La categorisation ferme la marche, et dans cet ordre-la seulement: elle
/// s'appuie sur les objets des scrutins qui viennent d'etre ecrits. Elle se
/// fait en deux temps — extraction deterministe des textes (RM-02), puis
/// rattachement des seuls objets encore en attente (RM-14). Un objet deja
/// rattache n'est jamais resoumis: le cout d'un rafraichissement suit ce qui
/// est nouveau, pas la taille de la base.
///
/// Ni l'un ni l'autre ne fait echouer le rafraichissement: un modele
/// indisponible laisse des objets non rattaches, tous consultables (RM-01).
pub struct RefreshAll<'a> {
    actor_source: &'a dyn ActorSource,
    actor_repository: &'a dyn ActorRepository,
    assembly_source: &'a dyn AssemblySource,
    dossier_repository: &'a dyn DossierRepository,
    scrutin_source: &'a dyn ScrutinSource,
    scrutin_repository: &'a dyn ScrutinRepository,
    amendment_source: &'a dyn AmendmentSource,
    amendment_repository: &'a dyn AmendmentRepository,
    theme_repository: &'a dyn ThemeRepository,
    theme_classifier: &'a dyn ThemeClassifier,
    /// Objets soumis au rattachement par passe. Plafond, pas objectif: le
    /// reliquat est repris au rafraichissement suivant.
    theme_batch: i64,
    /// Amendements ecrits par passe. Meme regle: un plafond, pas un objectif.
    amendment_batch: usize,
}

impl<'a> RefreshAll<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        actor_source: &'a dyn ActorSource,
        actor_repository: &'a dyn ActorRepository,
        assembly_source: &'a dyn AssemblySource,
        dossier_repository: &'a dyn DossierRepository,
        scrutin_source: &'a dyn ScrutinSource,
        scrutin_repository: &'a dyn ScrutinRepository,
        amendment_source: &'a dyn AmendmentSource,
        amendment_repository: &'a dyn AmendmentRepository,
        theme_repository: &'a dyn ThemeRepository,
        theme_classifier: &'a dyn ThemeClassifier,
        theme_batch: i64,
        amendment_batch: usize,
    ) -> Self {
        Self {
            actor_source,
            actor_repository,
            assembly_source,
            dossier_repository,
            scrutin_source,
            scrutin_repository,
            amendment_source,
            amendment_repository,
            theme_repository,
            theme_classifier,
            theme_batch,
            amendment_batch,
        }
    }

    pub async fn execute(&self, today: NaiveDate) -> Result<RefreshOutcome, RefreshError> {
        self.execute_with(RefreshScope::Incremental, today).await
    }

    pub async fn execute_with(
        &self,
        scope: RefreshScope,
        today: NaiveDate,
    ) -> Result<RefreshOutcome, RefreshError> {
        let registry_result = RefreshActorRegistry::new(self.actor_source, self.actor_repository)
            .execute()
            .await;

        // CU-01: source indisponible, le referentiel precedent est conserve et
        // le rafraichissement des dossiers continue.
        let (registry, registry_anomaly) = match registry_result {
            Ok(summary) => (Some(summary), None),
            Err(e) => {
                tracing::warn!("Actor registry refresh failed, keeping the previous one: {e}");
                (None, Some(e.to_string()))
            }
        };

        let dossiers = RefreshDossiers::new(
            self.assembly_source,
            self.dossier_repository,
            self.actor_repository,
        )
        .execute_with(scope)
        .await?;

        // Meme regle que pour le referentiel: une source indisponible ne fait
        // pas echouer le reste, elle se signale (README.md §2).
        let (scrutins, scrutins_anomaly) = match RefreshScrutins::new(
            self.scrutin_source,
            self.scrutin_repository,
            self.actor_repository,
        )
        .execute()
        .await
        {
            Ok(summary) => (Some(summary), None),
            Err(e) => {
                tracing::warn!("Scrutins refresh failed, keeping the stored ones: {e}");
                (None, Some(e.to_string()))
            }
        };

        // L'extraction relit les objets de scrutin deja stockes: elle reste
        // utile meme quand la source des scrutins n'a pas repondu.
        let (extraction, extraction_anomaly) = match ExtractDebatedTexts::new(self.theme_repository)
            .execute()
            .await
        {
            Ok(report) => (Some(report), None),
            Err(e) => {
                tracing::warn!("Debated text extraction failed: {e}");
                (None, Some(e.to_string()))
            }
        };

        // Sans extraction, aucun texte nouveau n'est en attente: soumettre
        // reviendrait a rejouer l'ancien etat.
        let (themes, themes_anomaly) = if extraction.is_none() {
            (
                None,
                Some("extraction indisponible, aucun rattachement tenté".to_string()),
            )
        } else {
            match ProposeThemeFamilies::new(self.theme_repository, self.theme_classifier)
                .execute(self.theme_batch, today)
                .await
            {
                Ok(run) => (Some(run), None),
                Err(e) => {
                    tracing::warn!("Theme assignment pass failed: {e}");
                    (None, Some(e.to_string()))
                }
            }
        };

        // Les amendements ferment la marche. Deux raisons: leur ingestion est la
        // plus longue, et une passe coupee par un delai ne doit rien couter au
        // reste; et le groupe de leurs signataires est date depuis le
        // referentiel, qui vient d'etre rafraichi (RM-11).
        let (amendments, amendments_anomaly) = match RefreshAmendments::new(
            self.amendment_source,
            self.amendment_repository,
            self.actor_repository,
            self.amendment_batch,
        )
        .execute()
        .await
        {
            Ok(summary) => (Some(summary), None),
            Err(e) => {
                tracing::warn!("Amendments refresh failed, keeping the stored ones: {e}");
                (None, Some(e.to_string()))
            }
        };

        Ok(RefreshOutcome {
            registry,
            registry_anomaly,
            dossiers,
            scrutins,
            scrutins_anomaly,
            extraction,
            extraction_anomaly,
            themes,
            themes_anomaly,
            amendments,
            amendments_anomaly,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::NaiveDate;
    use std::sync::Mutex;

    use crate::application::ports::actor_source::SourceError as ActorSourceError;
    use crate::application::ports::amendment_repository::{
        AmendmentGroupOption, AmendmentPage, AmendmentPageRequest, DossierAmendmentCoverage,
        RepositoryError as AmendmentRepositoryError, SignatoryRow,
    };
    use crate::application::ports::amendment_source::{
        AmendmentBatch, AmendmentBatches, AmendmentFeed, ArchiveScan,
    };
    use crate::application::ports::assembly_source::SourceError;
    use crate::application::ports::scrutin_repository::{
        RepositoryError as ScrutinRepositoryError, ScrutinFilter, ScrutinPage, ScrutinSummary,
    };
    use crate::application::ports::scrutin_source::SourceError as ScrutinSourceError;
    use crate::application::ports::theme_repository::ScrutinSubject;
    use crate::application::use_cases::theme_fakes::{InMemoryThemeRepository, StubClassifier};
    use crate::domain::actor::{ActorRegistry, ActorUid};
    use crate::domain::amendment::Amendment;
    use crate::domain::dataset::DatasetShape;
    use crate::domain::dossier::{DossierUid, LegislativeDossier};
    use crate::domain::scrutin::{Scrutin, ScrutinUid};
    use crate::domain::theme::{DebatedText, FamilyCode, SubjectRef};

    use super::super::refresh_dossiers::tests::{
        dossier, InMemoryActorRepository, InMemoryDossierRepository,
    };

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    struct FakeAssemblySource;

    #[async_trait]
    impl AssemblySource for FakeAssemblySource {
        async fn fetch_dossiers_since(
            &self,
            _since: Option<NaiveDate>,
        ) -> Result<Vec<LegislativeDossier>, SourceError> {
            unreachable!()
        }

        async fn fetch_dossiers_since_with_refs(
            &self,
            _since: Option<NaiveDate>,
        ) -> Result<Vec<(LegislativeDossier, Vec<String>)>, SourceError> {
            Ok(vec![(
                dossier("D1", Some(date(2025, 5, 13)), date(2025, 5, 13)),
                vec!["PA111111".into()],
            )])
        }

        async fn fetch_dossier_by_uid(
            &self,
            _uid: &DossierUid,
        ) -> Result<Option<LegislativeDossier>, SourceError> {
            unreachable!()
        }

        async fn fetch_dossier_by_uid_with_refs(
            &self,
            _uid: &DossierUid,
        ) -> Result<Option<(LegislativeDossier, Vec<String>)>, SourceError> {
            unreachable!()
        }
    }

    struct FakeScrutinSource {
        available: bool,
    }

    #[async_trait]
    impl ScrutinSource for FakeScrutinSource {
        async fn fetch_scrutins(
            &self,
            _legislature: u16,
        ) -> Result<Vec<Scrutin>, ScrutinSourceError> {
            if !self.available {
                return Err(ScrutinSourceError::Download("source unavailable".into()));
            }
            Ok(vec![])
        }
    }

    struct InMemoryScrutinRepository {
        saved: Mutex<usize>,
    }

    #[async_trait]
    impl ScrutinRepository for InMemoryScrutinRepository {
        async fn save_scrutins(
            &self,
            scrutins: &[Scrutin],
        ) -> Result<usize, ScrutinRepositoryError> {
            *self.saved.lock().unwrap() += scrutins.len();
            Ok(scrutins.len())
        }
        async fn list(
            &self,
            _filter: &ScrutinFilter,
        ) -> Result<ScrutinPage, ScrutinRepositoryError> {
            unreachable!()
        }
        async fn by_uid(
            &self,
            _uid: &ScrutinUid,
        ) -> Result<Option<Scrutin>, ScrutinRepositoryError> {
            unreachable!()
        }
        async fn by_dossier(
            &self,
            _uid: &str,
        ) -> Result<Vec<ScrutinSummary>, ScrutinRepositoryError> {
            unreachable!()
        }

        async fn dataset_shape(&self) -> Result<DatasetShape, ScrutinRepositoryError> {
            unreachable!()
        }
    }

    /// Source d'amendements muette: le rafraichissement des amendements n'est pas
    /// l'objet de ces tests, seule son innocuite sur le reste l'est.
    struct EmptyAmendmentSource;

    #[async_trait]
    impl AmendmentSource for EmptyAmendmentSource {
        async fn fetch_amendments(
            &self,
            _legislature: u16,
            _batch_size: usize,
        ) -> Result<AmendmentFeed, crate::application::ports::SourceError> {
            Ok(AmendmentFeed {
                archive_id: None,
                batches: AmendmentBatches::from_batches(vec![Ok(AmendmentBatch::Done(
                    ArchiveScan::default(),
                ))]),
            })
        }
    }

    struct NoopAmendmentRepository;

    #[async_trait]
    impl AmendmentRepository for NoopAmendmentRepository {
        async fn save_amendments(
            &self,
            _amendments: &[Amendment],
        ) -> Result<usize, AmendmentRepositoryError> {
            Ok(0)
        }

        async fn by_dossier(
            &self,
            _dossier_uid: &str,
            _page: &AmendmentPageRequest,
        ) -> Result<AmendmentPage, AmendmentRepositoryError> {
            unreachable!()
        }

        async fn groups_by_dossier(
            &self,
            _dossier_uid: &str,
        ) -> Result<Vec<AmendmentGroupOption>, AmendmentRepositoryError> {
            unreachable!()
        }

        async fn dossier_coverage(
            &self,
            _dossier_uid: &str,
        ) -> Result<DossierAmendmentCoverage, AmendmentRepositoryError> {
            unreachable!()
        }

        async fn signatories_of(
            &self,
            _amendment_uid: &str,
        ) -> Result<Vec<SignatoryRow>, AmendmentRepositoryError> {
            unreachable!()
        }

        async fn last_archive_id(
            &self,
            _label: &str,
        ) -> Result<Option<String>, AmendmentRepositoryError> {
            Ok(None)
        }

        async fn remember_archive(
            &self,
            _label: &str,
            _id: &str,
        ) -> Result<(), AmendmentRepositoryError> {
            Ok(())
        }
    }

    struct RecordingActorSource {
        available: bool,
        order: Mutex<Vec<&'static str>>,
    }

    #[async_trait]
    impl ActorSource for RecordingActorSource {
        async fn fetch_registry(
            &self,
            _legislature: u16,
        ) -> Result<ActorRegistry, ActorSourceError> {
            self.order.lock().unwrap().push("registry");
            if !self.available {
                return Err(ActorSourceError::Download("source unavailable".into()));
            }
            Ok(ActorRegistry {
                actors: vec![],
                groups: vec![],
                memberships: vec![],
            })
        }
    }

    /// Le rafraichissement porte neuf collaborateurs; seuls les deux derniers
    /// varient d'un test a l'autre.
    macro_rules! refresh {
        ($actor_source:expr, $actor_repository:expr, $assembly_source:expr,
         $dossier_repository:expr, $scrutin_source:expr, $scrutin_repository:expr,
         $theme_repository:expr, $theme_classifier:expr) => {
            RefreshAll::new(
                $actor_source,
                $actor_repository,
                $assembly_source,
                $dossier_repository,
                $scrutin_source,
                $scrutin_repository,
                &EmptyAmendmentSource,
                &NoopAmendmentRepository,
                $theme_repository,
                $theme_classifier,
                50,
                0,
            )
            .execute(date(2026, 8, 9))
            .await
            .unwrap()
        };
    }

    #[tokio::test]
    async fn refreshes_the_registry_before_the_dossiers() {
        let actor_source = RecordingActorSource {
            available: true,
            order: Mutex::new(vec![]),
        };
        let actor_repository = InMemoryActorRepository::with_deputy_changing_group();
        let assembly_source = FakeAssemblySource;
        let dossier_repository = InMemoryDossierRepository::new();
        let scrutin_source = FakeScrutinSource { available: true };
        let scrutin_repository = InMemoryScrutinRepository {
            saved: Mutex::new(0),
        };
        let theme_repository = InMemoryThemeRepository::default();
        let theme_classifier = StubClassifier::new();

        let outcome = refresh!(
            &actor_source,
            &actor_repository,
            &assembly_source,
            &dossier_repository,
            &scrutin_source,
            &scrutin_repository,
            &theme_repository,
            &theme_classifier
        );

        assert_eq!(*actor_source.order.lock().unwrap(), vec!["registry"]);
        assert!(outcome.scrutins.is_some());
        assert!(outcome.scrutins_anomaly.is_none());
        assert!(actor_repository
            .requested_uids
            .lock()
            .unwrap()
            .contains(&ActorUid::new("PA111111".into()).unwrap()));
        assert_eq!(outcome.dossiers.written, 1);
        assert!(outcome.registry_anomaly.is_none());
    }

    #[tokio::test]
    async fn keeps_refreshing_dossiers_when_the_registry_source_is_down() {
        let actor_source = RecordingActorSource {
            available: false,
            order: Mutex::new(vec![]),
        };
        let actor_repository = InMemoryActorRepository::with_deputy_changing_group();
        let assembly_source = FakeAssemblySource;
        let dossier_repository = InMemoryDossierRepository::new();
        let scrutin_source = FakeScrutinSource { available: false };
        let scrutin_repository = InMemoryScrutinRepository {
            saved: Mutex::new(0),
        };
        let theme_repository = InMemoryThemeRepository::default();
        let theme_classifier = StubClassifier::new();

        let outcome = refresh!(
            &actor_source,
            &actor_repository,
            &assembly_source,
            &dossier_repository,
            &scrutin_source,
            &scrutin_repository,
            &theme_repository,
            &theme_classifier
        );

        assert!(outcome.registry.is_none());
        // Source scrutins indisponible: le reste passe, la lacune est signalee.
        assert!(outcome.scrutins.is_none());
        assert!(outcome.scrutins_anomaly.is_some());
        assert_eq!(*scrutin_repository.saved.lock().unwrap(), 0);
        assert!(outcome.registry_anomaly.is_some());
        assert_eq!(outcome.dossiers.written, 1);

        // Le referentiel precedent a bien servi au rattachement.
        let store = dossier_repository.dossiers.lock().unwrap();
        assert_eq!(store["D1"].initiators()[0].group().unwrap().abbrev, "A");
    }

    #[tokio::test]
    async fn categorises_after_the_scrutins_have_been_written() {
        let actor_source = RecordingActorSource {
            available: true,
            order: Mutex::new(vec![]),
        };
        let actor_repository = InMemoryActorRepository::with_deputy_changing_group();
        let assembly_source = FakeAssemblySource;
        let dossier_repository = InMemoryDossierRepository::new();
        let scrutin_source = FakeScrutinSource { available: true };
        let scrutin_repository = InMemoryScrutinRepository {
            saved: Mutex::new(0),
        };

        let theme_repository = InMemoryThemeRepository::default();
        *theme_repository.subjects.lock().unwrap() = vec![ScrutinSubject {
            uid: "VTANR5L17V1".into(),
            subject: "l'article 12 du projet de loi de finances pour 2026 (première lecture)."
                .into(),
        }];
        let awaiting = DebatedText::new("projet de loi de finances pour 2026".into()).unwrap();
        *theme_repository.awaiting.lock().unwrap() = vec![awaiting.clone()];
        let theme_classifier = StubClassifier::new();

        let outcome = refresh!(
            &actor_source,
            &actor_repository,
            &assembly_source,
            &dossier_repository,
            &scrutin_source,
            &scrutin_repository,
            &theme_repository,
            &theme_classifier
        );

        let extraction = outcome.extraction.expect("extraction produite");
        assert_eq!(extraction.texts_found, 1);
        assert_eq!(extraction.scrutins_linked, 1);

        // La loi de finances releve d'une regle publiee: le rafraichissement la
        // rattache sans depenser un jeton (RM-13).
        let themes = outcome.themes.expect("passe de rattachement produite");
        assert_eq!(themes.ruled, 1);
        assert_eq!(themes.model_calls, 0);
        assert!(theme_classifier.calls.lock().unwrap().is_empty());
        assert_eq!(
            theme_repository.current_families(&SubjectRef::Text(awaiting.key().clone())),
            vec![
                FamilyCode::PouvoirAchatFiscalite,
                FamilyCode::InstitutionsProcedure
            ]
        );
    }

    #[tokio::test]
    async fn an_unreachable_model_does_not_fail_the_refresh() {
        let actor_source = RecordingActorSource {
            available: true,
            order: Mutex::new(vec![]),
        };
        let actor_repository = InMemoryActorRepository::with_deputy_changing_group();
        let assembly_source = FakeAssemblySource;
        let dossier_repository = InMemoryDossierRepository::new();
        let scrutin_source = FakeScrutinSource { available: true };
        let scrutin_repository = InMemoryScrutinRepository {
            saved: Mutex::new(0),
        };

        let theme_repository = InMemoryThemeRepository::default();
        *theme_repository.awaiting.lock().unwrap() =
            vec![
                DebatedText::new("proposition de loi relative au droit à l'aide à mourir".into())
                    .unwrap(),
            ];
        let theme_classifier = StubClassifier::new().failing_batches();

        let outcome = refresh!(
            &actor_source,
            &actor_repository,
            &assembly_source,
            &dossier_repository,
            &scrutin_source,
            &scrutin_repository,
            &theme_repository,
            &theme_classifier
        );

        // Le rafraichissement rend un resultat: dossiers et scrutins sont a jour,
        // le texte reste non rattache et sera repris (RM-01).
        assert_eq!(outcome.dossiers.written, 1);
        assert!(outcome.themes_anomaly.is_none());
        assert_eq!(outcome.themes.expect("passe produite").failed, 1);
    }
}
