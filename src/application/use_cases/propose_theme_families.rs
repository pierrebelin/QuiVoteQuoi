use chrono::NaiveDate;

use crate::application::ports::theme_classifier::ThemeClassifier;
use crate::application::ports::theme_repository::{
    AttemptOutcome, RepositoryError, ThemeRepository,
};
use crate::domain::theme::{ProposedFamily, SubjectRef, ThemeProposal};
use crate::domain::theme_rules::rule_for_label;

/// Resultat d'une passe de rattachement. Chiffres comptes ici, pas rendus par
/// le modele (RM-10).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProposalRun {
    pub attempted: usize,
    /// Rattaches par regle publiee, sans un jeton depense (RM-13).
    pub ruled: usize,
    pub proposed: usize,
    /// Le modele a repondu sans retenir de famille.
    pub without_family: usize,
    /// Le modele n'a pas repondu. Re-tente a la passe suivante.
    pub failed: usize,
    /// Appels au modele reellement passes. Une passe de 100 objets qui n'en
    /// compte que 3 est une passe qui a bien travaille (RM-14).
    pub model_calls: usize,
}

/// Un objet a rattacher: un texte debattu, ou un dossier qu'aucun scrutin ne
/// relie a un texte.
struct Pending {
    subject: SubjectRef,
    label: String,
}

/// CU-02 — Rattacher les objets en attente.
///
/// Trois passages, du moins cher au plus cher (RM-14) :
/// 1. les scrutins et les dossiers relies a un texte n'arrivent jamais ici —
///    ils heritent de leur texte (RM-06), et c'est de loin le plus gros levier :
///    8 434 scrutins tiennent en 322 textes ;
/// 2. ce qu'une regle publiee sait rattacher l'est sans appel au modele (RM-13) ;
/// 3. le reste part au modele **par lot**, le cadrage n'etant paye qu'une fois
///    par lot au lieu d'une fois par texte.
///
/// Un echec du modele n'en fait echouer aucun autre: l'objet reste non rattache
/// et consultable (RM-01).
pub struct ProposeThemeFamilies<'a> {
    repository: &'a dyn ThemeRepository,
    classifier: &'a dyn ThemeClassifier,
}

impl<'a> ProposeThemeFamilies<'a> {
    pub fn new(repository: &'a dyn ThemeRepository, classifier: &'a dyn ThemeClassifier) -> Self {
        Self {
            repository,
            classifier,
        }
    }

    pub async fn execute(
        &self,
        batch: i64,
        today: NaiveDate,
    ) -> Result<ProposalRun, RepositoryError> {
        let mut run = ProposalRun::default();
        let pending = self.pending(batch).await?;

        // 1. Les regles d'abord: ce qu'elles rattachent ne part pas au modele.
        let mut for_model: Vec<Pending> = Vec::with_capacity(pending.len());
        for item in pending {
            run.attempted += 1;
            match rule_for_label(&item.label) {
                Some(rule) => {
                    let assignments = rule
                        .assignments(&item.subject, today)
                        .map_err(|e| RepositoryError::Database(e.to_string()))?;
                    self.repository
                        .replace_assignments(&item.subject, today, &assignments)
                        .await?;
                    self.repository
                        .record_attempt(&item.subject, today, AttemptOutcome::Ruled)
                        .await?;
                    run.ruled += 1;
                }
                None => for_model.push(item),
            }
        }

        // 2. Le reste, par lot.
        let size = self.classifier.batch_size().max(1);
        for chunk in for_model.chunks(size) {
            self.classify_chunk(chunk, today, &mut run).await?;
        }

        Ok(run)
    }

    /// Textes d'abord — ils portent les scrutins, donc le plus de votes par
    /// rattachement — puis les dossiers sans texte porteur, sur ce qui reste du
    /// plafond de la passe.
    async fn pending(&self, batch: i64) -> Result<Vec<Pending>, RepositoryError> {
        let texts = self.repository.texts_awaiting_proposal(batch).await?;
        let mut pending: Vec<Pending> = texts
            .into_iter()
            .map(|text| Pending {
                subject: SubjectRef::Text(text.key().clone()),
                label: text.label().to_string(),
            })
            .collect();

        let remaining = batch - pending.len() as i64;
        if remaining > 0 {
            let dossiers = self
                .repository
                .dossiers_awaiting_proposal(remaining)
                .await?;
            pending.extend(dossiers.into_iter().map(|dossier| Pending {
                subject: SubjectRef::Dossier(dossier.uid),
                label: dossier.title,
            }));
        }
        Ok(pending)
    }

    async fn classify_chunk(
        &self,
        chunk: &[Pending],
        today: NaiveDate,
        run: &mut ProposalRun,
    ) -> Result<(), RepositoryError> {
        let mut batches: Vec<Vec<&Pending>> = vec![chunk.iter().collect()];

        while let Some(items) = batches.pop() {
            let labels: Vec<String> = items.iter().map(|item| item.label.clone()).collect();
            run.model_calls += 1;

            let answers = match self.classifier.propose_batch(&labels).await {
                Ok(answers) => answers,
                // Un lot en erreur est laisse au prochain rafraichissement :
                // le fractionner multiplierait les appels alors que le
                // fournisseur est indisponible pour tout le lot.
                Err(error) => {
                    tracing::warn!(batch = items.len(), %error, "theme proposal batch failed");
                    self.record_failed(&items, today, run).await?;
                    continue;
                }
            };

            let mut missing = Vec::new();
            for (index, item) in items.iter().enumerate() {
                match answers.get(index).and_then(Option::as_ref) {
                    Some(families) => self.save(item, families, today, run).await?,
                    None => missing.push(*item),
                }
            }

            if missing.is_empty() {
                continue;
            }

            if items.len() == 1 {
                tracing::warn!(label = missing[0].label, "no answer after isolated retry");
                self.record_failed(&missing, today, run).await?;
                continue;
            }

            // Le modele a deja repondu pour une partie du lot : ne lui
            // resoumettre que les libelles manquants. La division borne la
            // repetition du cadrage tout en isolant un libelle defaillant.
            if missing.len() == 1 {
                batches.push(missing);
            } else {
                let middle = missing.len().div_ceil(2);
                batches.push(missing[middle..].to_vec());
                batches.push(missing[..middle].to_vec());
            }
        }

        Ok(())
    }

    async fn record_failed(
        &self,
        items: &[&Pending],
        today: NaiveDate,
        run: &mut ProposalRun,
    ) -> Result<(), RepositoryError> {
        for item in items {
            run.failed += 1;
            self.repository
                .record_attempt(&item.subject, today, AttemptOutcome::Failed)
                .await?;
        }
        Ok(())
    }

    async fn save(
        &self,
        item: &Pending,
        families: &[ProposedFamily],
        today: NaiveDate,
        run: &mut ProposalRun,
    ) -> Result<(), RepositoryError> {
        let proposal = match ThemeProposal::new(
            item.subject.clone(),
            families.to_vec(),
            self.classifier.model().to_string(),
            self.classifier.prompt_version().to_string(),
            today,
        ) {
            Ok(proposal) => proposal,
            // Liste vide ou refusee par le domaine: le modele a repondu, il n'a
            // rien retenu. Ce n'est pas un echec, l'objet reste consultable.
            Err(error) => {
                if !families.is_empty() {
                    tracing::warn!(label = item.label, %error, "theme proposal refused");
                }
                run.without_family += 1;
                self.repository
                    .record_attempt(&item.subject, today, AttemptOutcome::NoFamily)
                    .await?;
                return Ok(());
            }
        };

        let assignments = proposal
            .into_assignments()
            .map_err(|e| RepositoryError::Database(e.to_string()))?;

        self.repository.save_proposal(&proposal).await?;
        self.repository
            .replace_assignments(&item.subject, today, &assignments)
            .await?;
        self.repository
            .record_attempt(&item.subject, today, AttemptOutcome::Proposed)
            .await?;
        run.proposed += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::theme_repository::PendingDossier;
    use crate::application::use_cases::theme_fakes::{InMemoryThemeRepository, StubClassifier};
    use crate::domain::dossier::DossierUid;
    use crate::domain::theme::{DebatedText, FamilyCode};

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 8, 9).unwrap()
    }

    fn text(label: &str) -> DebatedText {
        DebatedText::new(label.to_string()).unwrap()
    }

    #[tokio::test]
    async fn a_proposal_is_published_as_a_proposal() {
        let repository = InMemoryThemeRepository::default();
        let logement =
            text("proposition de loi de simplification du droit de l'urbanisme et du logement");
        *repository.awaiting.lock().unwrap() = vec![logement.clone()];
        let classifier = StubClassifier::new().answering(
            logement.label(),
            vec![(
                FamilyCode::Logement,
                "Le texte porte sur l'urbanisme et le logement.",
            )],
        );

        let run = ProposeThemeFamilies::new(&repository, &classifier)
            .execute(10, today())
            .await
            .unwrap();

        assert_eq!(run.proposed, 1);
        let subject = SubjectRef::Text(logement.key().clone());
        assert_eq!(
            repository.current_families(&subject),
            vec![FamilyCode::Logement]
        );
        let assignments = repository.assignments.lock().unwrap();
        assert_eq!(
            assignments[0].motive(),
            Some("Le texte porte sur l'urbanisme et le logement.")
        );
    }

    #[tokio::test]
    async fn a_ruled_text_never_reaches_the_model() {
        // Le texte le plus vote de la legislature: c'est exactement celui qu'il
        // ne faut pas payer au modele.
        let repository = InMemoryThemeRepository::default();
        let plf = text("projet de loi de finances pour 2026");
        *repository.awaiting.lock().unwrap() = vec![plf.clone()];
        let classifier = StubClassifier::new();

        let run = ProposeThemeFamilies::new(&repository, &classifier)
            .execute(10, today())
            .await
            .unwrap();

        assert_eq!(run.ruled, 1);
        assert_eq!(run.model_calls, 0);
        assert!(classifier.calls.lock().unwrap().is_empty());

        let subject = SubjectRef::Text(plf.key().clone());
        assert_eq!(
            repository.current_families(&subject),
            vec![FamilyCode::EconomieFinancesPubliques]
        );
        let assignments = repository.assignments.lock().unwrap();
        assert_eq!(assignments[0].author(), "règle « loi de finances »");
    }

    #[tokio::test]
    async fn a_ruled_text_is_never_recorded_as_a_model_proposal() {
        // Une regle n'est pas une proposition de modele: enregistrer l'un pour
        // l'autre ferait mentir le site sur sa propre methode.
        let repository = InMemoryThemeRepository::default();
        *repository.awaiting.lock().unwrap() = vec![text("projet de loi de finances pour 2026")];

        ProposeThemeFamilies::new(&repository, &StubClassifier::new())
            .execute(10, today())
            .await
            .unwrap();

        assert!(repository.proposals.lock().unwrap().is_empty());
        assert_eq!(
            repository.attempts.lock().unwrap()[0].2,
            AttemptOutcome::Ruled
        );
    }

    #[tokio::test]
    async fn texts_left_to_the_model_travel_in_one_call() {
        let repository = InMemoryThemeRepository::default();
        let first = text("proposition de loi relative au droit à l'aide à mourir");
        let second = text("proposition de loi visant à réguler les meublés de tourisme");
        *repository.awaiting.lock().unwrap() = vec![first.clone(), second.clone()];
        let classifier = StubClassifier::new()
            .with_batch_size(10)
            .answering(first.label(), vec![(FamilyCode::Sante, "fin de vie")])
            .answering(second.label(), vec![(FamilyCode::Logement, "meublés")]);

        let run = ProposeThemeFamilies::new(&repository, &classifier)
            .execute(10, today())
            .await
            .unwrap();

        assert_eq!(run.proposed, 2);
        assert_eq!(run.model_calls, 1);
        assert_eq!(*classifier.batches.lock().unwrap(), vec![2]);
    }

    #[tokio::test]
    async fn a_pass_is_split_along_the_classifier_batch_size() {
        let repository = InMemoryThemeRepository::default();
        *repository.awaiting.lock().unwrap() = (0..5)
            .map(|i| {
                text(&format!(
                    "proposition de loi n° {i} relative à la vie associative"
                ))
            })
            .collect();
        let classifier = StubClassifier::new()
            .with_batch_size(2)
            .defaulting_to_nothing();

        let run = ProposeThemeFamilies::new(&repository, &classifier)
            .execute(10, today())
            .await
            .unwrap();

        assert_eq!(run.attempted, 5);
        assert_eq!(run.model_calls, 3);
        assert_eq!(*classifier.batches.lock().unwrap(), vec![2, 2, 1]);
    }

    #[tokio::test]
    async fn more_than_three_families_are_truncated() {
        let repository = InMemoryThemeRepository::default();
        let text = text("proposition de loi portant diverses dispositions d'adaptation");
        *repository.awaiting.lock().unwrap() = vec![text.clone()];
        let classifier = StubClassifier::new().answering(
            text.label(),
            vec![
                (FamilyCode::PouvoirAchat, "fiscalité"),
                (FamilyCode::InstitutionsViePolitique, "procédure"),
                (FamilyCode::Sante, "social"),
                (FamilyCode::Logement, "logement"),
                (FamilyCode::Numerique, "numérique"),
            ],
        );

        ProposeThemeFamilies::new(&repository, &classifier)
            .execute(10, today())
            .await
            .unwrap();

        let subject = SubjectRef::Text(text.key().clone());
        assert_eq!(repository.current_families(&subject).len(), 3);
    }

    #[tokio::test]
    async fn a_failing_batch_leaves_every_text_unassigned_and_retriable() {
        let repository = InMemoryThemeRepository::default();
        let first = text("proposition de loi relative au droit à l'aide à mourir");
        let second = text("proposition de loi visant à réguler les meublés de tourisme");
        *repository.awaiting.lock().unwrap() = vec![first.clone(), second.clone()];
        let classifier = StubClassifier::new().failing_batches();

        let run = ProposeThemeFamilies::new(&repository, &classifier)
            .execute(10, today())
            .await
            .unwrap();

        assert_eq!(run.failed, 2);
        assert_eq!(run.proposed, 0);
        assert!(repository
            .current_families(&SubjectRef::Text(first.key().clone()))
            .is_empty());
        let attempts = repository.attempts.lock().unwrap();
        assert!(attempts
            .iter()
            .all(|(_, _, outcome)| *outcome == AttemptOutcome::Failed));
    }

    #[tokio::test]
    async fn a_label_the_model_skipped_is_retried_alone_not_counted_as_unassignable() {
        let repository = InMemoryThemeRepository::default();
        let answered = text("proposition de loi relative au droit à l'aide à mourir");
        let skipped = text("proposition de loi visant à réguler les meublés de tourisme");
        *repository.awaiting.lock().unwrap() = vec![answered.clone(), skipped.clone()];
        let classifier = StubClassifier::new()
            .with_batch_size(10)
            .answering(answered.label(), vec![(FamilyCode::Sante, "fin de vie")])
            .skipping(skipped.label());

        let run = ProposeThemeFamilies::new(&repository, &classifier)
            .execute(10, today())
            .await
            .unwrap();

        assert_eq!(run.proposed, 1);
        assert_eq!(run.failed, 1);
        assert_eq!(run.without_family, 0);
        assert_eq!(run.model_calls, 2);
        assert_eq!(
            *classifier.calls.lock().unwrap(),
            vec![
                answered.label().to_string(),
                skipped.label().to_string(),
                skipped.label().to_string()
            ]
        );
        assert_eq!(*classifier.batches.lock().unwrap(), vec![2, 1]);
        let attempts = repository.attempts.lock().unwrap();
        assert!(attempts.contains(&(
            skipped.key().as_str().to_string(),
            today(),
            AttemptOutcome::Failed
        )));
    }

    #[tokio::test]
    async fn a_model_retaining_nothing_is_not_an_error() {
        let repository = InMemoryThemeRepository::default();
        let odd = text("proposition de loi portant diverses dispositions d'ordre technique");
        *repository.awaiting.lock().unwrap() = vec![odd.clone()];
        let classifier = StubClassifier::new().defaulting_to_nothing();

        let run = ProposeThemeFamilies::new(&repository, &classifier)
            .execute(10, today())
            .await
            .unwrap();

        assert_eq!(run.without_family, 1);
        assert_eq!(run.failed, 0);
        assert_eq!(
            repository.attempts.lock().unwrap()[0].2,
            AttemptOutcome::NoFamily
        );
    }

    #[tokio::test]
    async fn the_model_only_ever_sees_the_labels() {
        let repository = InMemoryThemeRepository::default();
        let text = text("proposition de loi relative au droit à l'aide à mourir");
        *repository.awaiting.lock().unwrap() = vec![text.clone()];
        let classifier =
            StubClassifier::new().answering(text.label(), vec![(FamilyCode::Sante, "fin de vie")]);

        ProposeThemeFamilies::new(&repository, &classifier)
            .execute(10, today())
            .await
            .unwrap();

        assert_eq!(
            *classifier.calls.lock().unwrap(),
            vec![text.label().to_string()]
        );
    }

    #[tokio::test]
    async fn a_dossier_without_scrutin_is_classified_on_its_own_title() {
        let repository = InMemoryThemeRepository::default();
        *repository.awaiting_dossiers.lock().unwrap() = vec![PendingDossier {
            uid: DossierUid::new("DLR5L17N12345".into()).unwrap(),
            title: "Accès aux soins dans les zones rurales".into(),
        }];
        let classifier = StubClassifier::new().answering(
            "Accès aux soins dans les zones rurales",
            vec![(FamilyCode::Sante, "Le dossier porte sur l'accès aux soins.")],
        );

        let run = ProposeThemeFamilies::new(&repository, &classifier)
            .execute(10, today())
            .await
            .unwrap();

        assert_eq!(run.proposed, 1);
        let subject = SubjectRef::Dossier(DossierUid::new("DLR5L17N12345".into()).unwrap());
        assert_eq!(
            repository.current_families(&subject),
            vec![FamilyCode::Sante]
        );
    }

    #[tokio::test]
    async fn texts_take_the_pass_before_dossiers() {
        // Un texte porte des scrutins, un dossier sans scrutin n'en porte aucun:
        // a plafond egal, le texte rapporte davantage de votes rattaches.
        let repository = InMemoryThemeRepository::default();
        let pending = text("proposition de loi relative au droit à l'aide à mourir");
        *repository.awaiting.lock().unwrap() = vec![pending.clone()];
        *repository.awaiting_dossiers.lock().unwrap() = vec![PendingDossier {
            uid: DossierUid::new("DLR5L17N12345".into()).unwrap(),
            title: "Accès aux soins dans les zones rurales".into(),
        }];
        let classifier = StubClassifier::new().defaulting_to_nothing();

        let run = ProposeThemeFamilies::new(&repository, &classifier)
            .execute(1, today())
            .await
            .unwrap();

        assert_eq!(run.attempted, 1);
        assert_eq!(
            *classifier.calls.lock().unwrap(),
            vec![pending.label().to_string()]
        );
    }
}
