//! Fake d'etat du port `ThemeRepository`: on verifie l'etat persiste.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::application::ports::theme_repository::{
    AssignedFamily, AttemptOutcome, MethodReport, PendingDossier, RepositoryError, ScrutinSubject,
    TextLink, TextPage, TextScrutin, TextSummary, ThemeRepository,
};
use crate::domain::theme::{
    DebatedText, FamilyCode, SubjectRef, TextKey, ThemeAssignment, ThemeProposal,
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
    /// Familles courantes semees par uid de dossier, servies par `families_of_dossier`.
    pub dossier_families: Mutex<HashMap<String, Vec<AssignedFamily>>>,
    /// Page servie par famille; une famille absente rend une page vide.
    pub family_pages: Mutex<HashMap<FamilyCode, TextPage>>,
    /// Page des non rattaches; `None` rend une page vide.
    pub unassigned_page: Mutex<Option<TextPage>>,
    /// `(limit, offset)` de chaque lecture paginee, dans l'ordre.
    pub page_requests: Mutex<Vec<(i64, i64)>>,
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
        family: FamilyCode,
        limit: i64,
        offset: i64,
    ) -> Result<TextPage, RepositoryError> {
        self.page_requests.lock().unwrap().push((limit, offset));
        Ok(self
            .family_pages
            .lock()
            .unwrap()
            .get(&family)
            .cloned()
            .unwrap_or(TextPage {
                items: vec![],
                total: 0,
            }))
    }

    async fn unassigned_texts(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<TextPage, RepositoryError> {
        self.page_requests.lock().unwrap().push((limit, offset));
        Ok(self
            .unassigned_page
            .lock()
            .unwrap()
            .clone()
            .unwrap_or(TextPage {
                items: vec![],
                total: 0,
            }))
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
        dossier_uid: &str,
    ) -> Result<Vec<AssignedFamily>, RepositoryError> {
        Ok(self
            .dossier_families
            .lock()
            .unwrap()
            .get(dossier_uid)
            .cloned()
            .unwrap_or_default())
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

/// Ligne de liste d'un texte, sans scrutin ni famille.
pub fn text_summary(key: &str, label: &str) -> TextSummary {
    TextSummary {
        key: key.to_string(),
        label: label.to_string(),
        scrutin_count: 0,
        first_vote: None,
        last_vote: None,
        dossier_uid: None,
        dossier_label: None,
        families: vec![],
        last_attempt_outcome: None,
    }
}

/// Famille courante ouverte le `opened_on`, sans motif.
pub fn assigned(family: FamilyCode, opened_on: NaiveDate) -> AssignedFamily {
    AssignedFamily {
        family,
        opened_on,
        motive: None,
    }
}
