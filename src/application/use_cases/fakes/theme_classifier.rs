//! Doublure du port d'effet `ThemeClassifier`: reponse programmee.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;

use crate::application::ports::theme_classifier::{ClassifierError, ThemeClassifier};
use crate::domain::theme::{FamilyCode, ProposedFamily};

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
