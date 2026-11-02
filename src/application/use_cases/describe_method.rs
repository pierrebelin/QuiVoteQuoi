use crate::application::ports::amendment_citation_repository::AmendmentCitationRepository;
use crate::application::ports::theme_repository::{MethodReport, RepositoryError, ThemeRepository};
use crate::domain::amendment_citation::LinkingReport;

/// Ce que la page méthode publie : thématisation et rapprochement.
#[derive(Debug, Clone)]
pub struct MethodOverview {
    pub report: MethodReport,
    pub linking: LinkingReport,
}

/// CU-01 — Publier les compteurs de la page méthode.
pub struct DescribeMethod<'a> {
    themes: &'a dyn ThemeRepository,
    citations: &'a dyn AmendmentCitationRepository,
}

impl<'a> DescribeMethod<'a> {
    pub fn new(
        themes: &'a dyn ThemeRepository,
        citations: &'a dyn AmendmentCitationRepository,
    ) -> Self {
        Self { themes, citations }
    }

    pub async fn execute(&self) -> Result<MethodOverview, RepositoryError> {
        let report = self.themes.method_report().await?;
        let counts = self.citations.citation_counts().await?;
        Ok(MethodOverview {
            report,
            linking: LinkingReport::from_counts(&counts),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::amendment_citation_repository::StoredCitation;
    use crate::application::use_cases::citation_fakes::InMemoryCitationRepository;
    use crate::application::use_cases::theme_fakes::InMemoryThemeRepository;
    use crate::domain::amendment::AmendmentUid;
    use crate::domain::amendment_citation::{
        CitationOutcome, CitationResult, LinkMethod, UnlinkedCause,
    };
    use crate::domain::scrutin::ScrutinUid;

    fn report() -> MethodReport {
        MethodReport {
            families: vec![],
            texts_total: 322,
            texts_assigned: 300,
            texts_without_family: 10,
            texts_attempt_failed: 2,
            texts_never_attempted: 10,
            scrutins_total: 8_434,
            scrutins_with_text: 6_000,
            scrutins_assigned: 5_900,
            dossiers_total: 400,
            dossiers_linked_to_text: 350,
            dossiers_assigned: 340,
            dossiers_without_text: 50,
        }
    }

    fn stored(rank: u16, result: CitationResult) -> StoredCitation {
        StoredCitation {
            scrutin: ScrutinUid::new("VTANR5L17V1".to_string()).unwrap(),
            outcome: CitationOutcome {
                rank,
                cited_number: None,
                result,
            },
        }
    }

    fn linked(by_cited_article: bool) -> CitationResult {
        CitationResult::Linked {
            amendment: AmendmentUid::new("AMANR1".to_string()).unwrap(),
            method: LinkMethod { by_cited_article },
        }
    }

    #[tokio::test]
    async fn should_publish_links_by_method_and_absences_by_cause_beside_theme_report_when_citations_are_stored(
    ) {
        let themes = InMemoryThemeRepository::default();
        *themes.method_report.lock().unwrap() = Some(report());
        let citations = InMemoryCitationRepository::default();
        *citations.citations.lock().unwrap() = vec![
            stored(0, linked(false)),
            stored(1, linked(true)),
            stored(2, linked(true)),
            stored(3, CitationResult::Unlinked(UnlinkedCause::NoNumber)),
            stored(4, CitationResult::Unlinked(UnlinkedCause::Ambiguous)),
        ];

        let overview = DescribeMethod::new(&themes, &citations)
            .execute()
            .await
            .unwrap();

        assert_eq!(overview.report.texts_total, 322);
        assert_eq!(overview.report.dossiers_without_text, 50);
        assert_eq!(
            overview.linking,
            LinkingReport {
                linked: 3,
                linked_by_cited_article: 2,
                no_number: 1,
                no_dossier: 0,
                unresolved: 0,
                ambiguous: 1,
            }
        );
    }
}
