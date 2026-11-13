//! Doublures partagees par les tests des use cases de thematisation.
//!
//! Ports d'etat: fake in-memory, on verifie l'etat persiste. Le classifieur est
//! un port d'effet: sa doublure joue une reponse programmee.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::application::ports::theme_classifier::{ClassifierError, ThemeClassifier};
use crate::application::ports::theme_repository::{
    AssignedFamily, AttemptOutcome, MethodReport, PendingDossier, RepositoryError, ScrutinSubject,
    TextLink, TextPage, TextScrutin, TextSummary, ThemeRepository,
};
use crate::domain::theme::{
    DebatedText, FamilyCode, ProposedFamily, SubjectRef, TextKey, ThemeAssignment, ThemeProposal,
};

#[derive(Default)]
pub struct InMemoryThemeRepository {
    pub subjects: Mutex<Vec<ScrutinSubject>>,
    pub texts: Mutex<HashMap<String, DebatedText>>,
    pub links: Mutex<HashMap<String, String>>,
    pub attempts: Mutex<Vec<(String, NaiveDate, AttemptOutcome)>>,
    pub proposals: Mutex<Vec<ThemeProposal>>,
    pub assignments: Mutex<Vec<ThemeAssignment>>,
    pub awaiting: Mutex<Vec<DebatedText>>,
    pub awaiting_dossiers: Mutex<Vec<PendingDossier>>,
    pub method_report: Mutex<Option<MethodReport>>,
    /// Familles courantes semees par uid de scrutin, servies par `families_of_scrutins`.
    pub scrutin_families: Mutex<HashMap<String, Vec<AssignedFamily>>>,
}

impl InMemoryThemeRepository {
    pub fn current_families(&self, subject: &SubjectRef) -> Vec<FamilyCode> {
        let mut families: Vec<FamilyCode> = self
            .assignments
            .lock()
            .unwrap()
            .iter()
            .filter(|a| a.subject() == subject && a.is_current())
            .map(|a| a.family())
            .collect();
        families.sort();
        families
    }

    pub fn closed_count(&self, subject: &SubjectRef) -> usize {
        self.assignments
            .lock()
            .unwrap()
            .iter()
            .filter(|a| a.subject() == subject && !a.is_current())
            .count()
    }
}

#[async_trait]
impl ThemeRepository for InMemoryThemeRepository {
    async fn scrutin_subjects(&self) -> Result<Vec<ScrutinSubject>, RepositoryError> {
        Ok(self.subjects.lock().unwrap().clone())
    }

    async fn save_texts(&self, texts: &[DebatedText]) -> Result<usize, RepositoryError> {
        let mut stored = self.texts.lock().unwrap();
        for text in texts {
            stored.insert(text.key().as_str().to_string(), text.clone());
        }
        Ok(texts.len())
    }

    async fn link_scrutins(&self, links: &[TextLink]) -> Result<usize, RepositoryError> {
        let mut stored = self.links.lock().unwrap();
        for link in links {
            stored.insert(link.scrutin_uid.clone(), link.text_key.clone());
        }
        Ok(links.len())
    }

    async fn link_dossiers_through_scrutins(&self) -> Result<usize, RepositoryError> {
        Ok(0)
    }

    async fn texts_awaiting_proposal(
        &self,
        limit: i64,
    ) -> Result<Vec<DebatedText>, RepositoryError> {
        let awaiting = self.awaiting.lock().unwrap();
        Ok(awaiting.iter().take(limit as usize).cloned().collect())
    }

    async fn dossiers_awaiting_proposal(
        &self,
        limit: i64,
    ) -> Result<Vec<PendingDossier>, RepositoryError> {
        let awaiting = self.awaiting_dossiers.lock().unwrap();
        Ok(awaiting.iter().take(limit as usize).cloned().collect())
    }

    async fn record_attempt(
        &self,
        subject: &SubjectRef,
        on: NaiveDate,
        outcome: AttemptOutcome,
    ) -> Result<(), RepositoryError> {
        self.attempts
            .lock()
            .unwrap()
            .push((subject.identifier().to_string(), on, outcome));
        Ok(())
    }

    async fn save_proposal(&self, proposal: &ThemeProposal) -> Result<(), RepositoryError> {
        self.proposals.lock().unwrap().push(proposal.clone());
        Ok(())
    }

    async fn latest_proposal(
        &self,
        subject: &SubjectRef,
    ) -> Result<Option<ThemeProposal>, RepositoryError> {
        Ok(self
            .proposals
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|p| p.subject() == subject)
            .cloned())
    }

    async fn replace_assignments(
        &self,
        subject: &SubjectRef,
        closed_on: NaiveDate,
        opened: &[ThemeAssignment],
    ) -> Result<(), RepositoryError> {
        let mut stored = self.assignments.lock().unwrap();
        for assignment in stored.iter_mut() {
            if assignment.subject() == subject && assignment.is_current() {
                assignment
                    .close(closed_on)
                    .map_err(|e| RepositoryError::Database(e.to_string()))?;
            }
        }
        stored.extend(opened.iter().cloned());
        Ok(())
    }

    async fn assignment_history(
        &self,
        subject: &SubjectRef,
    ) -> Result<Vec<ThemeAssignment>, RepositoryError> {
        Ok(self
            .assignments
            .lock()
            .unwrap()
            .iter()
            .filter(|a| a.subject() == subject)
            .cloned()
            .collect())
    }

    async fn text_by_key(&self, key: &TextKey) -> Result<Option<TextSummary>, RepositoryError> {
        let texts = self.texts.lock().unwrap();
        Ok(texts.get(key.as_str()).map(|text| TextSummary {
            key: text.key().as_str().to_string(),
            label: text.label().to_string(),
            scrutin_count: 0,
            first_vote: None,
            last_vote: None,
            dossier_uid: None,
            dossier_label: None,
            families: vec![],
            last_attempt_outcome: None,
        }))
    }

    async fn scrutins_of_text(
        &self,
        _key: &TextKey,
        _limit: i64,
        _offset: i64,
    ) -> Result<Vec<TextScrutin>, RepositoryError> {
        Ok(vec![])
    }

    async fn texts_by_family(
        &self,
        _family: FamilyCode,
        _limit: i64,
        _offset: i64,
    ) -> Result<TextPage, RepositoryError> {
        Ok(TextPage {
            items: vec![],
            total: 0,
        })
    }

    async fn unassigned_texts(
        &self,
        _limit: i64,
        _offset: i64,
    ) -> Result<TextPage, RepositoryError> {
        Ok(TextPage {
            items: vec![],
            total: 0,
        })
    }

    async fn families_of_scrutins(
        &self,
        scrutin_uids: &[String],
    ) -> Result<HashMap<String, Vec<AssignedFamily>>, RepositoryError> {
        Ok(self
            .scrutin_families
            .lock()
            .unwrap()
            .iter()
            .filter(|(uid, _)| scrutin_uids.contains(uid))
            .map(|(uid, families)| (uid.clone(), families.clone()))
            .collect())
    }

    async fn families_of_dossier(
        &self,
        _dossier_uid: &str,
    ) -> Result<Vec<AssignedFamily>, RepositoryError> {
        Ok(vec![])
    }

    async fn method_report(&self) -> Result<MethodReport, RepositoryError> {
        Ok(self
            .method_report
            .lock()
            .unwrap()
            .clone()
            .expect("method_report non programme"))
    }

    async fn text_count(&self) -> Result<i64, RepositoryError> {
        Ok(self.texts.lock().unwrap().len() as i64)
    }
}

/// Doublure du port d'effet: rend la reponse programmee pour chaque libelle.
///
/// `calls` retient les libelles soumis, dans l'ordre; `batches` retient la
/// taille de chaque appel, ce qui est la grandeur a surveiller — c'est le
/// nombre d'appels, pas le nombre de libelles, qui fait la facture (RM-14).
pub struct StubClassifier {
    answers: Mutex<HashMap<String, Vec<ProposedFamily>>>,
    /// Libelles pour lesquels le modele ne rend rien du tout.
    skipped: Mutex<Vec<String>>,
    default: Mutex<Option<Vec<ProposedFamily>>>,
    failing: Mutex<bool>,
    batch_size: Mutex<usize>,
    pub calls: Mutex<Vec<String>>,
    pub batches: Mutex<Vec<usize>>,
}

impl StubClassifier {
    pub fn new() -> Self {
        Self {
            answers: Mutex::new(HashMap::new()),
            skipped: Mutex::new(vec![]),
            default: Mutex::new(None),
            failing: Mutex::new(false),
            batch_size: Mutex::new(20),
            calls: Mutex::new(vec![]),
            batches: Mutex::new(vec![]),
        }
    }

    pub fn answering(self, label: &str, families: Vec<(FamilyCode, &str)>) -> Self {
        self.answers.lock().unwrap().insert(
            label.to_string(),
            families
                .into_iter()
                .map(|(family, why)| ProposedFamily::new(family, why.to_string()).unwrap())
                .collect(),
        );
        self
    }

    /// Le modele repond au lot, mais omet ce libelle.
    pub fn skipping(self, label: &str) -> Self {
        self.skipped.lock().unwrap().push(label.to_string());
        self
    }

    /// Tout appel echoue: le lot entier est perdu.
    pub fn failing_batches(self) -> Self {
        *self.failing.lock().unwrap() = true;
        self
    }

    pub fn with_batch_size(self, size: usize) -> Self {
        *self.batch_size.lock().unwrap() = size;
        self
    }

    pub fn defaulting_to_nothing(self) -> Self {
        *self.default.lock().unwrap() = Some(vec![]);
        self
    }
}

impl Default for StubClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ThemeClassifier for StubClassifier {
    async fn propose_batch(
        &self,
        labels: &[String],
    ) -> Result<Vec<Option<Vec<ProposedFamily>>>, ClassifierError> {
        self.batches.lock().unwrap().push(labels.len());
        self.calls.lock().unwrap().extend(labels.iter().cloned());

        if *self.failing.lock().unwrap() {
            return Err(ClassifierError::Call("modèle injoignable".to_string()));
        }

        let answers = self.answers.lock().unwrap();
        let skipped = self.skipped.lock().unwrap();
        let default = self.default.lock().unwrap();
        Ok(labels
            .iter()
            .map(|label| {
                if skipped.contains(label) {
                    return None;
                }
                answers
                    .get(label)
                    .cloned()
                    .or_else(|| default.clone())
                    .or(Some(vec![]))
            })
            .collect())
    }

    fn model(&self) -> &str {
        "modele-de-test"
    }

    fn prompt_version(&self) -> &str {
        "test-v1"
    }

    fn batch_size(&self) -> usize {
        *self.batch_size.lock().unwrap()
    }
}
