//! Doublures partagées par les tests des use cases « En bref ».

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::application::ports::brief_generator::{
    BriefGenerator, BriefRequest, GeneratedLine, SummaryGeneratorError,
};
use crate::application::ports::brief_line_repository::BriefLineRepository;
use crate::application::ports::legal_text_source::LegalTextSource;
use crate::application::ports::text_version_repository::{TextReading, TextVersionRepository};
use crate::application::ports::text_version_source::TextVersionSource;
use crate::application::ports::{RepositoryError, SourceError};
use crate::domain::cited_article::{CitedArticle, CitedText, CitedWording};
use crate::domain::dossier::DossierUid;
use crate::domain::law_text::{ArticleFingerprint, VersionArticle};
use crate::domain::text_brief::{BriefProgress, BriefStatus, StoredBriefLine};
use crate::domain::text_version::{BriefCandidate, DocumentUid, ReadingStatus, TextVersion};

/// Source des versions : ce qu'elle publie est semé par le test.
#[derive(Default)]
pub struct FakeTextVersionSource {
    /// Versions publiées par `fetch_versions`.
    pub versions: Mutex<Vec<TextVersion>>,
    /// Articles servis par référence ; une référence absente n'est pas publiée.
    pub texts: Mutex<HashMap<String, Vec<VersionArticle>>>,
    /// `fetch_versions` échoue.
    pub versions_unavailable: Mutex<bool>,
    /// Références dont la lecture du texte échoue.
    pub failing_texts: Mutex<Vec<String>>,
    /// Références dont le texte a été demandé, dans l'ordre.
    pub text_requests: Mutex<Vec<String>>,
}

#[async_trait]
impl TextVersionSource for FakeTextVersionSource {
    async fn fetch_versions(&self) -> Result<Vec<TextVersion>, SourceError> {
        if *self.versions_unavailable.lock().unwrap() {
            return Err(SourceError::Download("archive indisponible".to_string()));
        }
        Ok(self.versions.lock().unwrap().clone())
    }

    async fn version_articles(
        &self,
        version: &TextVersion,
    ) -> Result<Option<Vec<VersionArticle>>, SourceError> {
        let uid = version.document_uid().as_str().to_string();
        self.text_requests.lock().unwrap().push(uid.clone());
        if self.failing_texts.lock().unwrap().contains(&uid) {
            return Err(SourceError::Download(format!("{uid}: texte illisible")));
        }
        Ok(self.texts.lock().unwrap().get(&uid).cloned())
    }
}

/// Dépôt des versions en mémoire, avec le contrat du dépôt réel : une version
/// déjà connue garde son état de lecture quand elle est enregistrée à nouveau.
#[derive(Default)]
pub struct InMemoryTextVersionRepository {
    pub versions: Mutex<Vec<TextVersion>>,
    pub articles: Mutex<HashMap<String, Vec<VersionArticle>>>,
    /// Dossiers votés semés : le dossier et la date de son dernier scrutin.
    pub votes: Mutex<Vec<(DossierUid, NaiveDate)>>,
    /// Toute opération échoue.
    pub unreachable: Mutex<bool>,
}

impl InMemoryTextVersionRepository {
    fn check_reachable(&self) -> Result<(), RepositoryError> {
        if *self.unreachable.lock().unwrap() {
            return Err(RepositoryError::Database("base injoignable".to_string()));
        }
        Ok(())
    }

    /// L'état de lecture enregistré d'une version.
    pub fn reading_of(&self, document_uid: &str) -> Option<ReadingStatus> {
        self.versions
            .lock()
            .unwrap()
            .iter()
            .find(|version| version.document_uid().as_str() == document_uid)
            .map(TextVersion::reading)
    }
}

#[async_trait]
impl TextVersionRepository for InMemoryTextVersionRepository {
    async fn save_versions(&self, versions: &[TextVersion]) -> Result<(), RepositoryError> {
        self.check_reachable()?;
        let mut stored = self.versions.lock().unwrap();
        for version in versions {
            match stored
                .iter_mut()
                .find(|known| known.document_uid() == version.document_uid())
            {
                Some(known) => *known = version.with_reading(known.reading()),
                None => stored.push(version.clone()),
            }
        }
        Ok(())
    }

    async fn versions_of(&self, dossier: &DossierUid) -> Result<Vec<TextVersion>, RepositoryError> {
        self.check_reachable()?;
        Ok(self
            .versions
            .lock()
            .unwrap()
            .iter()
            .filter(|version| version.dossier_uid() == dossier)
            .cloned()
            .collect())
    }

    async fn voted_dossiers(&self) -> Result<Vec<BriefCandidate>, RepositoryError> {
        self.check_reachable()?;
        Ok(self
            .votes
            .lock()
            .unwrap()
            .iter()
            .map(|(dossier_uid, last_vote_on)| BriefCandidate {
                dossier_uid: dossier_uid.clone(),
                last_vote_on: *last_vote_on,
            })
            .collect())
    }

    async fn versions_of_voted(&self) -> Result<Vec<TextVersion>, RepositoryError> {
        self.check_reachable()?;
        let votes = self.votes.lock().unwrap();
        Ok(self
            .versions
            .lock()
            .unwrap()
            .iter()
            .filter(|version| {
                votes
                    .iter()
                    .any(|(dossier, _)| dossier == version.dossier_uid())
            })
            .cloned()
            .collect())
    }

    async fn articles_of(
        &self,
        document_uid: &DocumentUid,
    ) -> Result<Option<Vec<VersionArticle>>, RepositoryError> {
        self.check_reachable()?;
        Ok(self
            .articles
            .lock()
            .unwrap()
            .get(document_uid.as_str())
            .cloned())
    }

    async fn save_reading(
        &self,
        document_uid: &DocumentUid,
        reading: &TextReading,
    ) -> Result<(), RepositoryError> {
        self.check_reachable()?;
        let status = match reading {
            TextReading::Articles(articles) => {
                self.articles
                    .lock()
                    .unwrap()
                    .insert(document_uid.as_str().to_string(), articles.clone());
                ReadingStatus::Read
            }
            TextReading::Missing => {
                self.articles.lock().unwrap().remove(document_uid.as_str());
                ReadingStatus::Missing
            }
        };
        let mut stored = self.versions.lock().unwrap();
        if let Some(known) = stored
            .iter_mut()
            .find(|known| known.document_uid() == document_uid)
        {
            *known = known.with_reading(status);
        }
        Ok(())
    }
}

/// Source de Légifrance : ce qu'elle rend est semé par le test. Un article non
/// semé est inconnu, ou créé quand le texte l'insère, comme chez la source
/// réelle.
#[derive(Default)]
pub struct FakeLegalTextSource {
    /// Rédactions servies, par texte et numéro normalisé.
    pub wordings: Mutex<HashMap<(CitedText, String), CitedWording>>,
    /// Citations (texte, numéro normalisé) dont la lecture échoue.
    pub failing: Mutex<Vec<(CitedText, String)>>,
    /// Demandes reçues, dans l'ordre : texte, numéro normalisé, date.
    pub requests: Mutex<Vec<(CitedText, String, NaiveDate)>>,
}

#[async_trait]
impl LegalTextSource for FakeLegalTextSource {
    async fn wording(
        &self,
        cited: &CitedArticle,
        on: NaiveDate,
    ) -> Result<CitedWording, SourceError> {
        let key = (cited.text().clone(), cited.number().to_string());
        self.requests
            .lock()
            .unwrap()
            .push((key.0.clone(), key.1.clone(), on));
        if self.failing.lock().unwrap().contains(&key) {
            return Err(SourceError::Download(format!(
                "{} {}: legifrance indisponible",
                key.0.label(),
                key.1
            )));
        }
        if let Some(wording) = self.wordings.lock().unwrap().get(&key) {
            return Ok(wording.clone());
        }
        Ok(if cited.inserted() {
            CitedWording::Created
        } else {
            CitedWording::NotFound
        })
    }
}

/// Modèle de langage : ce qu'il rend est semé par le test, les requêtes reçues
/// sont gardées. Un appel est numéroté à partir de 0.
pub struct FakeBriefGenerator {
    pub model: String,
    pub prompt_version: String,
    /// Lignes rendues à chaque appel, quelles que soient les clés demandées.
    pub lines: Mutex<Vec<GeneratedLine>>,
    /// Quand posé, chaque clé demandée reçoit en plus cette ligne.
    pub line_for_every_key: Mutex<Option<String>>,
    /// Numéros des appels qui échouent.
    pub failing_calls: Mutex<Vec<usize>>,
    /// Tous les appels échouent.
    pub failing: Mutex<bool>,
    /// Le fournisseur n'a pas de clé : tous les appels sont indisponibles.
    pub unavailable: Mutex<bool>,
    /// Requêtes reçues, dans l'ordre.
    pub requests: Mutex<Vec<BriefRequest>>,
}

impl Default for FakeBriefGenerator {
    fn default() -> Self {
        Self {
            model: "modele-essai".to_string(),
            prompt_version: "prompt-essai-v1".to_string(),
            lines: Mutex::new(Vec::new()),
            line_for_every_key: Mutex::new(None),
            failing_calls: Mutex::new(Vec::new()),
            failing: Mutex::new(false),
            unavailable: Mutex::new(false),
            requests: Mutex::new(Vec::new()),
        }
    }
}

impl FakeBriefGenerator {
    /// Fait rendre cette ligne pour cette clé.
    pub fn answer(&self, key: &str, line: &str) {
        self.lines.lock().unwrap().push(GeneratedLine {
            key: key.to_string(),
            line: line.to_string(),
        });
    }

    /// Les clés de chaque requête reçue, dans l'ordre.
    pub fn requested_keys(&self) -> Vec<Vec<String>> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .map(|request| request.articles.iter().map(|a| a.key.clone()).collect())
            .collect()
    }
}

#[async_trait]
impl BriefGenerator for FakeBriefGenerator {
    async fn generate(
        &self,
        request: &BriefRequest,
    ) -> Result<Vec<GeneratedLine>, SummaryGeneratorError> {
        let call = {
            let mut requests = self.requests.lock().unwrap();
            requests.push(request.clone());
            requests.len() - 1
        };
        if *self.unavailable.lock().unwrap() {
            return Err(SummaryGeneratorError::Unavailable(
                "clé du fournisseur absente".to_string(),
            ));
        }
        if *self.failing.lock().unwrap() || self.failing_calls.lock().unwrap().contains(&call) {
            return Err(SummaryGeneratorError::Call(format!(
                "appel {call}: fournisseur injoignable"
            )));
        }
        let mut answered = self.lines.lock().unwrap().clone();
        if let Some(line) = self.line_for_every_key.lock().unwrap().as_ref() {
            answered.extend(request.articles.iter().map(|article| GeneratedLine {
                key: article.key.clone(),
                line: line.clone(),
            }));
        }
        Ok(answered)
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn prompt_version(&self) -> &str {
        &self.prompt_version
    }
}

/// Dépôt des lignes en mémoire, avec le contrat du dépôt réel : une ligne
/// publiable n'est jamais réécrite.
#[derive(Default)]
pub struct InMemoryBriefLineRepository {
    pub lines: Mutex<HashMap<String, StoredBriefLine>>,
    /// Échecs consécutifs par version, avec le dernier message.
    pub failures: Mutex<HashMap<String, (u8, String)>>,
    pub progress: Mutex<Vec<(DossierUid, BriefProgress)>>,
    /// Nombre de lignes de chaque appel à `save_lines`, dans l'ordre.
    pub saved_batches: Mutex<Vec<usize>>,
    /// Toute opération échoue.
    pub unreachable: Mutex<bool>,
}

impl InMemoryBriefLineRepository {
    fn check_reachable(&self) -> Result<(), RepositoryError> {
        if *self.unreachable.lock().unwrap() {
            return Err(RepositoryError::Database("base injoignable".to_string()));
        }
        Ok(())
    }

    /// La ligne enregistrée sous cette empreinte.
    pub fn line(&self, fingerprint: &str) -> Option<StoredBriefLine> {
        self.lines.lock().unwrap().get(fingerprint).cloned()
    }

    /// Les échecs enregistrés pour cette version.
    pub fn failure_count(&self, document_uid: &str) -> u8 {
        self.failures
            .lock()
            .unwrap()
            .get(document_uid)
            .map_or(0, |(count, _)| *count)
    }

    /// Pose des échecs déjà enregistrés pour cette version.
    pub fn seed_failures(&self, document_uid: &str, count: u8) {
        self.failures
            .lock()
            .unwrap()
            .insert(document_uid.to_string(), (count, "essai".to_string()));
    }

    /// L'avancement enregistré pour ce dossier.
    pub fn progress_of(&self, dossier: &str) -> Option<BriefProgress> {
        self.progress
            .lock()
            .unwrap()
            .iter()
            .find(|(known, _)| known.as_str() == dossier)
            .map(|(_, progress)| progress.clone())
    }
}

#[async_trait]
impl BriefLineRepository for InMemoryBriefLineRepository {
    async fn lines_for(
        &self,
        fingerprints: &[ArticleFingerprint],
    ) -> Result<Vec<StoredBriefLine>, RepositoryError> {
        self.check_reachable()?;
        let stored = self.lines.lock().unwrap();
        Ok(fingerprints
            .iter()
            .filter_map(|fingerprint| stored.get(fingerprint.as_str()).cloned())
            .collect())
    }

    async fn save_lines(&self, lines: &[StoredBriefLine]) -> Result<(), RepositoryError> {
        self.check_reachable()?;
        self.saved_batches.lock().unwrap().push(lines.len());
        let mut stored = self.lines.lock().unwrap();
        for line in lines {
            let key = line.fingerprint.as_str().to_string();
            if stored
                .get(&key)
                .is_some_and(|known| known.status == BriefStatus::Ready)
            {
                continue;
            }
            stored.insert(key, line.clone());
        }
        Ok(())
    }

    async fn failures_of(&self, document_uid: &DocumentUid) -> Result<u8, RepositoryError> {
        self.check_reachable()?;
        Ok(self.failure_count(document_uid.as_str()))
    }

    async fn record_failure(
        &self,
        document_uid: &DocumentUid,
        error: &str,
    ) -> Result<(), RepositoryError> {
        self.check_reachable()?;
        let mut failures = self.failures.lock().unwrap();
        let entry = failures
            .entry(document_uid.as_str().to_string())
            .or_insert((0, String::new()));
        entry.0 += 1;
        entry.1 = error.to_string();
        Ok(())
    }

    async fn clear_failures(&self, document_uid: &DocumentUid) -> Result<(), RepositoryError> {
        self.check_reachable()?;
        self.failures.lock().unwrap().remove(document_uid.as_str());
        Ok(())
    }

    async fn save_progress(
        &self,
        dossier: &DossierUid,
        progress: &BriefProgress,
    ) -> Result<(), RepositoryError> {
        self.check_reachable()?;
        let mut stored = self.progress.lock().unwrap();
        stored.retain(|(known, _)| known != dossier);
        stored.push((dossier.clone(), progress.clone()));
        Ok(())
    }

    async fn progress(&self) -> Result<Vec<(DossierUid, BriefProgress)>, RepositoryError> {
        self.check_reachable()?;
        Ok(self.progress.lock().unwrap().clone())
    }
}
