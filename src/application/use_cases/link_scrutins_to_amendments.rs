use crate::application::ports::amendment_citation_repository::{
    AmendmentCitationRepository, StoredCitation,
};
use crate::application::ports::RepositoryError;
use crate::domain::amendment_citation::{LinkingReport, ScrutinCitations, SearchDossier};
use crate::domain::dossier::DossierUid;

/// CU-01 — Rapprocher les scrutins des amendements qu'ils citent.
pub struct LinkScrutinsToAmendments<'a> {
    citations: &'a dyn AmendmentCitationRepository,
}

impl<'a> LinkScrutinsToAmendments<'a> {
    pub fn new(citations: &'a dyn AmendmentCitationRepository) -> Self {
        Self { citations }
    }

    pub async fn execute(&self) -> Result<LinkingReport, RepositoryError> {
        let scrutins = self.citations.citing_scrutins().await?;
        let cited: Vec<ScrutinCitations> = scrutins
            .into_iter()
            .filter_map(|s| {
                let search = SearchDossier::for_scrutin(s.published_dossier.as_ref());
                ScrutinCitations::from_subject(s.uid, s.date, &s.subject, search)
            })
            .collect();

        let mut dossiers: Vec<DossierUid> = Vec::new();
        for citations in &cited {
            if let SearchDossier::Published(dossier) = citations.search() {
                if !dossiers.contains(dossier) {
                    dossiers.push(dossier.clone());
                }
            }
        }
        let candidates = self.citations.candidates_in(&dossiers).await?;

        let mut stored = Vec::new();
        for citations in &cited {
            for outcome in citations.resolve(&candidates) {
                stored.push(StoredCitation {
                    scrutin: citations.scrutin().clone(),
                    outcome,
                });
            }
        }
        self.citations.replace_citations(&stored).await?;

        let outcomes: Vec<_> = stored.into_iter().map(|c| c.outcome).collect();
        Ok(LinkingReport::from_outcomes(&outcomes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::amendment_citation_repository::{CitingScrutin, StoredCitation};
    use crate::application::use_cases::citation_fakes::InMemoryCitationRepository;
    use crate::domain::amendment::{AmendmentNumber, AmendmentTarget, AmendmentUid};
    use crate::domain::amendment_citation::{
        CandidateAmendment, CitationOutcome, CitationResult, UnlinkedCause,
    };
    use crate::domain::dossier::DossierUid;
    use crate::domain::scrutin::ScrutinUid;
    use chrono::NaiveDate;

    fn day() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 3, 12).unwrap()
    }

    fn dossier(uid: &str) -> DossierUid {
        DossierUid::new(uid.to_string()).unwrap()
    }

    fn scrutin(uid: &str, subject: &str, dossier_uid: Option<&str>) -> CitingScrutin {
        CitingScrutin {
            uid: ScrutinUid::new(uid.to_string()).unwrap(),
            date: day(),
            subject: subject.to_string(),
            published_dossier: dossier_uid.map(dossier),
        }
    }

    fn candidate(uid: &str, dossier_uid: &str, number: &str) -> CandidateAmendment {
        CandidateAmendment::new(
            AmendmentUid::new(uid.to_string()).unwrap(),
            dossier(dossier_uid),
            AmendmentNumber::new(number.to_string()).unwrap(),
            &AmendmentTarget::new("ARTICLE 3".to_string(), None).unwrap(),
            None,
        )
    }

    fn stale() -> StoredCitation {
        StoredCitation {
            scrutin: ScrutinUid::new("VTANR5L17V1".to_string()).unwrap(),
            outcome: CitationOutcome {
                rank: 0,
                cited_number: None,
                result: CitationResult::Unlinked(UnlinkedCause::NoNumber),
            },
        }
    }

    #[tokio::test]
    async fn should_replace_all_citations_with_this_pass_outcomes_when_pass_completes() {
        let repository = InMemoryCitationRepository::default();
        *repository.citations.lock().unwrap() = vec![stale(), stale()];
        *repository.scrutins.lock().unwrap() = vec![scrutin(
            "VTANR5L17V490",
            "l'amendement n° 12 de M. Martin",
            Some("DLR5L17N1"),
        )];
        *repository.amendments.lock().unwrap() = vec![candidate("AMANR1", "DLR5L17N1", "12")];

        LinkScrutinsToAmendments::new(&repository)
            .execute()
            .await
            .unwrap();

        let stored = repository.citations.lock().unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].scrutin.as_str(), "VTANR5L17V490");
        assert!(matches!(
            stored[0].outcome.result,
            CitationResult::Linked { .. }
        ));
    }

    #[tokio::test]
    async fn should_search_candidates_only_in_published_dossiers_when_pass_reads_amendments() {
        let repository = InMemoryCitationRepository::default();
        *repository.scrutins.lock().unwrap() = vec![
            scrutin("VTANR5L17V1", "l'amendement n° 1 de M. A", Some("DLR5L17N1")),
            scrutin("VTANR5L17V2", "l'amendement n° 2 de M. B", None),
            scrutin("VTANR5L17V3", "l'amendement n° 3 de M. C", Some("DLR5L17N1")),
            scrutin("VTANR5L17V4", "l'ensemble du texte", Some("DLR5L17N2")),
        ];

        LinkScrutinsToAmendments::new(&repository)
            .execute()
            .await
            .unwrap();

        let reads = repository.candidate_reads.lock().unwrap();
        assert_eq!(*reads, vec![vec![dossier("DLR5L17N1")]]);
    }

    #[tokio::test]
    async fn should_keep_previous_citations_when_candidates_cannot_be_read() {
        let repository = InMemoryCitationRepository::default();
        *repository.citations.lock().unwrap() = vec![stale()];
        *repository.scrutins.lock().unwrap() = vec![scrutin(
            "VTANR5L17V490",
            "l'amendement n° 12 de M. Martin",
            Some("DLR5L17N1"),
        )];
        *repository.candidates_unreadable.lock().unwrap() = true;

        let result = LinkScrutinsToAmendments::new(&repository).execute().await;

        assert!(result.is_err());
        assert_eq!(*repository.citations.lock().unwrap(), vec![stale()]);
    }

    #[tokio::test]
    async fn should_report_links_by_method_and_absences_by_cause_when_pass_completes() {
        let repository = InMemoryCitationRepository::default();
        *repository.scrutins.lock().unwrap() = vec![
            scrutin("VTANR5L17V1", "l'amendement n° 12 de M. A", Some("DLR5L17N1")),
            scrutin("VTANR5L17V2", "l'amendement n° 99 de M. B", Some("DLR5L17N1")),
            scrutin("VTANR5L17V3", "l'amendement n° 12 de M. C", None),
            scrutin("VTANR5L17V4", "l'amendement de M. D", Some("DLR5L17N1")),
        ];
        *repository.amendments.lock().unwrap() = vec![candidate("AMANR1", "DLR5L17N1", "12")];

        let report = LinkScrutinsToAmendments::new(&repository)
            .execute()
            .await
            .unwrap();

        assert_eq!(
            report,
            LinkingReport {
                linked: 1,
                linked_by_cited_article: 0,
                no_number: 1,
                no_dossier: 1,
                unresolved: 1,
                ambiguous: 0,
            }
        );
    }
}
