use std::collections::HashMap;

use crate::application::ports::theme_repository::{RepositoryError, TextLink, ThemeRepository};
use crate::domain::theme::DebatedText;

/// Ce que l'extraction a produit, en chiffres lus depuis les donnees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionReport {
    pub scrutins_read: usize,
    pub texts_found: usize,
    pub scrutins_linked: usize,
    /// Objets ne nommant aucun texte. Ils restent consultables (RM-01).
    pub scrutins_without_text: usize,
    pub dossiers_linked: usize,
}

/// CU-01 — Extraire les textes debattus.
///
/// L'extraction est une regle, pas un jugement: aucun modele n'intervient
/// (RM-02). Elle est rejouable a l'identique — meme objet, meme cle.
pub struct ExtractDebatedTexts<'a> {
    repository: &'a dyn ThemeRepository,
}

impl<'a> ExtractDebatedTexts<'a> {
    pub fn new(repository: &'a dyn ThemeRepository) -> Self {
        Self { repository }
    }

    pub async fn execute(&self) -> Result<ExtractionReport, RepositoryError> {
        let subjects = self.repository.scrutin_subjects().await?;

        let mut texts: HashMap<String, DebatedText> = HashMap::new();
        let mut links: Vec<TextLink> = Vec::with_capacity(subjects.len());
        let mut without_text = 0usize;

        for subject in &subjects {
            let Some(text) = DebatedText::from_subject(&subject.subject) else {
                without_text += 1;
                continue;
            };
            links.push(TextLink {
                scrutin_uid: subject.uid.clone(),
                text_key: text.key().as_str().to_string(),
            });
            texts.entry(text.key().as_str().to_string()).or_insert(text);
        }

        let texts: Vec<DebatedText> = texts.into_values().collect();
        self.repository.save_texts(&texts).await?;
        let scrutins_linked = self.repository.link_scrutins(&links).await?;
        let dossiers_linked = self.repository.link_dossiers_through_scrutins().await?;

        Ok(ExtractionReport {
            scrutins_read: subjects.len(),
            texts_found: texts.len(),
            scrutins_linked,
            scrutins_without_text: without_text,
            dossiers_linked,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::theme_repository::ScrutinSubject;
    use crate::application::use_cases::fakes::theme_repository::InMemoryThemeRepository;

    fn subject(uid: &str, subject: &str) -> ScrutinSubject {
        ScrutinSubject {
            uid: uid.to_string(),
            subject: subject.to_string(),
        }
    }

    #[tokio::test]
    async fn scrutins_of_one_text_share_a_single_key() {
        let repository = InMemoryThemeRepository::default();
        *repository.subjects.lock().unwrap() = vec![
            subject(
                "S1",
                "l'amendement n° 12 de M. Tanguy à l'article 3 du projet de loi de finances \
                 pour 2026 (première lecture).",
            ),
            subject(
                "S2",
                "l'article 33 du projet de loi de finances pour 2026 (nouvelle lecture).",
            ),
        ];

        let report = ExtractDebatedTexts::new(&repository)
            .execute()
            .await
            .unwrap();

        assert_eq!(report.texts_found, 1);
        assert_eq!(report.scrutins_linked, 2);
        let links = repository.links.lock().unwrap();
        assert_eq!(links["S1"], links["S2"]);
    }

    #[tokio::test]
    async fn a_scrutin_naming_no_text_is_counted_not_dropped() {
        let repository = InMemoryThemeRepository::default();
        *repository.subjects.lock().unwrap() = vec![
            subject("S1", "l'ensemble du texte mis aux voix."),
            subject(
                "S2",
                "l'ensemble de la proposition de loi relative au droit à l'aide à mourir \
                 (première lecture).",
            ),
        ];

        let report = ExtractDebatedTexts::new(&repository)
            .execute()
            .await
            .unwrap();

        assert_eq!(report.scrutins_read, 2);
        assert_eq!(report.scrutins_without_text, 1);
        assert_eq!(report.texts_found, 1);
        assert!(!repository.links.lock().unwrap().contains_key("S1"));
    }

    #[tokio::test]
    async fn extraction_is_repeatable() {
        let repository = InMemoryThemeRepository::default();
        *repository.subjects.lock().unwrap() = vec![subject(
            "S1",
            "l'article premier du projet de loi de simplification de la vie économique \
             (première lecture).",
        )];

        let first = ExtractDebatedTexts::new(&repository)
            .execute()
            .await
            .unwrap();
        let second = ExtractDebatedTexts::new(&repository)
            .execute()
            .await
            .unwrap();

        assert_eq!(first, second);
        assert_eq!(repository.texts.lock().unwrap().len(), 1);
    }
}
