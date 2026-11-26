use crate::application::ports::theme_repository::{RepositoryError, TextSummary, ThemeRepository};
use crate::domain::theme::{SubjectRef, TextKey, ThemeAssignment, ThemeProposal};

/// Detail d'un texte: ses familles courantes, son historique, la proposition
/// qui l'a produit.
#[derive(Debug, Clone)]
pub struct TextDetail {
    pub summary: TextSummary,
    pub history: Vec<ThemeAssignment>,
    pub proposal: Option<ThemeProposal>,
}

/// CU-04 — Fiche d'un texte.
pub struct GetTextDetail<'a> {
    repository: &'a dyn ThemeRepository,
}

impl<'a> GetTextDetail<'a> {
    pub fn new(repository: &'a dyn ThemeRepository) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, key: &TextKey) -> Result<Option<TextDetail>, RepositoryError> {
        let Some(summary) = self.repository.text_by_key(key).await? else {
            return Ok(None);
        };
        let subject = SubjectRef::Text(key.clone());
        let history = self.repository.assignment_history(&subject).await?;
        let proposal = self.repository.latest_proposal(&subject).await?;
        Ok(Some(TextDetail {
            summary,
            history,
            proposal,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::use_cases::fakes::theme_repository::InMemoryThemeRepository;
    use crate::domain::theme::{DebatedText, FamilyCode, ProposedFamily};
    use chrono::NaiveDate;

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, d).unwrap()
    }

    fn proposal(subject: &SubjectRef, family: FamilyCode, on: NaiveDate) -> ThemeProposal {
        ThemeProposal::new(
            subject.clone(),
            vec![ProposedFamily::new(family, "justification".into()).unwrap()],
            "model".into(),
            "v1".into(),
            on,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn answers_none_for_an_unknown_text() {
        let repository = InMemoryThemeRepository::default();

        let detail = GetTextDetail::new(&repository)
            .execute(&TextKey::from_raw("inconnu"))
            .await
            .unwrap();

        assert!(detail.is_none());
    }

    #[tokio::test]
    async fn gives_the_text_its_own_history_and_latest_proposal_only() {
        let repository = InMemoryThemeRepository::default();
        let text = DebatedText::new("projet de loi de finances pour 2026".into()).unwrap();
        let other = DebatedText::new("proposition de loi sur la chasse".into()).unwrap();
        repository
            .texts
            .lock()
            .unwrap()
            .insert(text.key().as_str().to_string(), text.clone());
        let subject = SubjectRef::Text(text.key().clone());
        let elsewhere = SubjectRef::Text(other.key().clone());
        let [first, second, ..] = FamilyCode::ALL;
        let older = proposal(&subject, first, day(1));
        let latest = proposal(&subject, second, day(2));
        repository.proposals.lock().unwrap().extend([
            older.clone(),
            latest.clone(),
            proposal(&elsewhere, first, day(3)),
        ]);
        repository
            .assignments
            .lock()
            .unwrap()
            .extend(latest.into_assignments().unwrap());
        repository
            .assignments
            .lock()
            .unwrap()
            .extend(proposal(&elsewhere, first, day(3)).into_assignments().unwrap());

        let detail = GetTextDetail::new(&repository)
            .execute(text.key())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(detail.summary.key, text.key().as_str());
        assert_eq!(detail.history.len(), 1);
        assert!(detail.history.iter().all(|a| a.subject() == &subject));
        let proposal = detail.proposal.unwrap();
        assert_eq!(proposal.subject(), &subject);
        assert_eq!(proposal.families()[0].family(), second);
    }
}
