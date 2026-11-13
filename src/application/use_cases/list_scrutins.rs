use std::collections::HashMap;

use crate::application::ports::actor_repository::ActorRepository;
use crate::application::ports::amendment_citation_repository::AmendmentCitationRepository;
use crate::application::ports::dossier_repository::DossierRepository;
use crate::application::ports::scrutin_repository::{
    RepositoryError, ScrutinFilter, ScrutinRepository, ScrutinSummary,
};
use crate::application::use_cases::browse_dossier_amendments::{
    author_uids, cited_view_of, CitedAmendmentView,
};
use crate::domain::actor::ActorDirectory;
use crate::domain::dossier::{DossierUid, LegislativeDossier};
use crate::domain::scrutin::{ScrutinUid, ShortLabel};

/// Ligne de la liste: le scrutin publie, son libelle court (RM-09), les
/// amendements que son objet cite (RM-05, RM-08) et le dossier auquel la
/// source le rattache, pour en dire le sort et l'etape.
#[derive(Debug, Clone)]
pub struct ScrutinListItem {
    pub summary: ScrutinSummary,
    pub short_label: ShortLabel,
    pub cited: Vec<CitedAmendmentView>,
    /// Dossier rattache, sans actes ni documents. Absent quand le scrutin n'a
    /// pas de dossier, ou que ce dossier n'est pas en base.
    pub dossier: Option<LegislativeDossier>,
}

#[derive(Debug, Clone)]
pub struct ScrutinListPage {
    pub items: Vec<ScrutinListItem>,
    pub total: i64,
}

/// CU-02 — Consulter la liste des scrutins.
///
/// Les filtres restreignent l'affichage demande, jamais le contenu de la base
/// (RM-01). Le total renvoye est celui du filtre: le visiteur voit combien de
/// scrutins il regarde.
pub struct ListScrutins<'a> {
    repository: &'a dyn ScrutinRepository,
    citations: &'a dyn AmendmentCitationRepository,
    actors: &'a dyn ActorRepository,
    dossiers: &'a dyn DossierRepository,
}

impl<'a> ListScrutins<'a> {
    pub fn new(
        repository: &'a dyn ScrutinRepository,
        citations: &'a dyn AmendmentCitationRepository,
        actors: &'a dyn ActorRepository,
        dossiers: &'a dyn DossierRepository,
    ) -> Self {
        Self {
            repository,
            citations,
            actors,
            dossiers,
        }
    }

    pub async fn execute(&self, filter: ScrutinFilter) -> Result<ScrutinListPage, RepositoryError> {
        let page = self.repository.list(&filter).await?;
        let items = self.enrich(page.items).await?;
        Ok(ScrutinListPage {
            items,
            total: page.total,
        })
    }

    /// Ajoute a chaque ligne son libelle court, ses amendements cites et son
    /// dossier, en lectures groupees sur l'ensemble des lignes.
    async fn enrich(
        &self,
        summaries: Vec<ScrutinSummary>,
    ) -> Result<Vec<ScrutinListItem>, RepositoryError> {
        let uids: Vec<ScrutinUid> = summaries
            .iter()
            .filter_map(|summary| ScrutinUid::new(summary.uid.clone()).ok())
            .collect();
        let mut rows = if uids.is_empty() {
            Vec::new()
        } else {
            self.citations.citations_of(&uids).await?
        };
        rows.sort_by_key(|row| row.rank);

        // Le referentiel n'est charge que pour les auteurs cites de la page.
        let authors = author_uids(
            rows.iter()
                .filter_map(|row| row.amendment.as_ref())
                .map(|cited| &cited.summary),
        );
        let directory = if authors.is_empty() {
            ActorDirectory::new(vec![], vec![], vec![])
        } else {
            self.actors.load_directory_for(&authors).await?
        };

        let dossiers = self.dossiers_of(&summaries).await?;

        let items = summaries
            .into_iter()
            .map(|summary| {
                let dossier = summary
                    .dossier_uid
                    .as_ref()
                    .and_then(|uid| dossiers.get(uid).cloned());
                let cited = rows
                    .iter()
                    .filter(|row| row.scrutin.as_str() == summary.uid)
                    .filter_map(|row| cited_view_of(row.clone(), &directory))
                    .collect();
                ScrutinListItem {
                    short_label: ShortLabel::from_subject(&summary.subject),
                    cited,
                    dossier,
                    summary,
                }
            })
            .collect();
        Ok(items)
    }

    /// Tous les scrutins de la periode, enrichis comme une ligne de liste.
    pub async fn execute_between(
        &self,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    ) -> Result<Vec<ScrutinListItem>, RepositoryError> {
        let summaries = self.repository.between(from, to).await?;
        self.enrich(summaries).await
    }

    /// Les dossiers que la page nomme, en une seule lecture: la liste affiche
    /// cinquante scrutins, souvent d'une poignee de dossiers.
    async fn dossiers_of(
        &self,
        summaries: &[ScrutinSummary],
    ) -> Result<HashMap<String, LegislativeDossier>, RepositoryError> {
        let mut uids: Vec<DossierUid> = summaries
            .iter()
            .filter_map(|summary| summary.dossier_uid.clone())
            .filter_map(|uid| DossierUid::new(uid).ok())
            .collect();
        uids.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        uids.dedup_by(|a, b| a.as_str() == b.as_str());
        if uids.is_empty() {
            return Ok(HashMap::new());
        }
        let found = self.dossiers.find_by_uids(&uids).await?;
        Ok(found
            .into_iter()
            .map(|dossier| (dossier.uid().as_str().to_string(), dossier))
            .collect())
    }

    /// CU-04 — Scrutins d'un dossier.
    pub async fn for_dossier(
        &self,
        dossier_uid: &str,
    ) -> Result<Vec<ScrutinSummary>, RepositoryError> {
        self.repository.by_dossier(dossier_uid).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::amendment_citation_repository::{CitationRow, CitedAmendmentRow};
    use crate::application::use_cases::citation_fakes::InMemoryCitationRepository;
    use crate::application::use_cases::read_fakes::{
        amendment_row, listed_dossier, scrutin_row, FakeActors, FakeDossiers, FakeScrutins,
    };
    use crate::domain::amendment::SummaryPreview;
    use crate::domain::dataset::DatasetShape;
    use crate::domain::amendment_citation::{LinkMethod, UnlinkedCause};
    use crate::domain::dossier::{DossierOutcome, DossierUid, LegislativeStage};
    use crate::domain::scrutin::ScrutinUid;

    fn actors() -> FakeActors {
        FakeActors { group_color: None }
    }

    fn linked_row(scrutin: &str, rank: u16, number: &str, title: &str) -> CitationRow {
        let mut summary = amendment_row();
        summary.uid = format!("AM{number}");
        summary.number = number.into();
        summary.target_title = title.into();
        CitationRow {
            scrutin: ScrutinUid::new(scrutin.into()).unwrap(),
            rank,
            cited_number: Some(number.into()),
            amendment: Some(CitedAmendmentRow {
                dossier_uid: DossierUid::new("DLR5L17N1".into()).unwrap(),
                summary,
            }),
            cause: None,
            method: LinkMethod {
                by_cited_article: false,
            },
        }
    }

    fn unlinked_row(scrutin: &str) -> CitationRow {
        CitationRow {
            scrutin: ScrutinUid::new(scrutin.into()).unwrap(),
            rank: 0,
            cited_number: Some("99".into()),
            amendment: None,
            cause: Some(UnlinkedCause::Unresolved),
            method: LinkMethod {
                by_cited_article: false,
            },
        }
    }

    fn citations(rows: Vec<CitationRow>) -> InMemoryCitationRepository {
        InMemoryCitationRepository {
            rows: std::sync::Mutex::new(rows),
            ..Default::default()
        }
    }

    async fn list(
        rows: Vec<ScrutinSummary>,
        cited: Vec<CitationRow>,
    ) -> ScrutinListPage {
        let total = rows.len() as i64;
        let repository = FakeScrutins {
            rows,
            total,
            shape: DatasetShape::default(),
        };
        let citations = citations(cited);
        ListScrutins::new(&repository, &citations, &actors(), &FakeDossiers::default())
            .execute(ScrutinFilter::default())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn should_give_each_scrutin_its_short_label_when_list_is_read() {
        let page = list(
            vec![
                scrutin_row("S1", "l'article 3 du projet de loi de finances"),
                scrutin_row("S2", "l'ensemble du projet de loi"),
                scrutin_row("S3", "objet inconnu de la source"),
            ],
            vec![],
        )
        .await;

        let labels: Vec<&str> = page.items.iter().map(|i| i.short_label.as_str()).collect();
        assert_eq!(
            labels,
            vec![
                "Vote sur l'article 3",
                "Vote sur l'ensemble du texte",
                "objet inconnu de la source"
            ]
        );
    }

    #[tokio::test]
    async fn should_keep_source_order_and_total_when_list_is_read() {
        let repository = FakeScrutins {
            rows: vec![
                scrutin_row("S3", "l'ensemble du projet de loi"),
                scrutin_row("S1", "l'ensemble du projet de loi"),
                scrutin_row("S2", "l'ensemble du projet de loi"),
            ],
            total: 7,
            shape: DatasetShape::default(),
        };

        let page = ListScrutins::new(
            &repository,
            &citations(vec![]),
            &actors(),
            &FakeDossiers::default(),
        )
        .execute(ScrutinFilter::default())
        .await
        .unwrap();

        let uids: Vec<&str> = page.items.iter().map(|i| i.summary.uid.as_str()).collect();
        assert_eq!(uids, vec!["S3", "S1", "S2"]);
        assert_eq!(page.total, 7);
    }

    #[tokio::test]
    async fn should_present_cited_amendment_with_author_article_and_preview_when_scrutin_is_linked() {
        let page = list(
            vec![scrutin_row("S1", "l'amendement n° 12 de M. Dupont")],
            vec![linked_row("S1", 0, "12", "Article 3")],
        )
        .await;

        let cited = &page.items[0].cited;
        assert_eq!(cited.len(), 1);
        assert_eq!(cited[0].number, "12");
        assert_eq!(cited[0].target_title, "Article 3");
        assert_eq!(cited[0].author.name.as_deref(), Some("Jean Dupont"));
        assert_eq!(
            cited[0].preview,
            SummaryPreview::Text {
                text: "Court expos\u{e9}.".into(),
                truncated: false
            }
        );
    }

    #[tokio::test]
    async fn should_list_cited_amendments_in_citation_order_when_subject_cites_several() {
        let page = list(
            vec![scrutin_row("S1", "les amendements n° 45 et 12 de M. Dupont")],
            vec![
                linked_row("S1", 0, "45", "Article 3"),
                linked_row("S1", 1, "12", "Article 4"),
            ],
        )
        .await;

        let numbers: Vec<&str> = page.items[0]
            .cited
            .iter()
            .map(|c| c.number.as_str())
            .collect();
        assert_eq!(numbers, vec!["45", "12"]);
    }

    #[tokio::test]
    async fn should_give_no_cited_amendment_when_scrutin_is_not_linked() {
        let page = list(
            vec![
                scrutin_row("S1", "l'amendement n° 99 de M. Dupont"),
                scrutin_row("S2", "l'amendement n° 12 de M. Dupont"),
            ],
            vec![unlinked_row("S1"), linked_row("S2", 0, "12", "Article 3")],
        )
        .await;

        assert!(page.items[0].cited.is_empty());
        assert_eq!(page.items[1].cited.len(), 1);
    }

    #[tokio::test]
    async fn should_keep_source_order_when_some_scrutins_are_linked_and_others_not() {
        let page = list(
            vec![
                scrutin_row("S3", "l'amendement n° 12 de M. Dupont"),
                scrutin_row("S1", "l'ensemble du projet de loi"),
                scrutin_row("S2", "l'amendement n° 45 de M. Dupont"),
            ],
            vec![
                linked_row("S2", 0, "45", "Article 3"),
                linked_row("S3", 0, "12", "Article 3"),
            ],
        )
        .await;

        let shape: Vec<(&str, usize)> = page
            .items
            .iter()
            .map(|i| (i.summary.uid.as_str(), i.cited.len()))
            .collect();
        assert_eq!(shape, vec![("S3", 1), ("S1", 0), ("S2", 1)]);
    }

    fn in_dossier(uid: &str, dossier: &str) -> ScrutinSummary {
        let mut row = scrutin_row(uid, "l'article 3 du projet de loi de finances");
        row.dossier_uid = Some(dossier.into());
        row.dossier_label = Some("Projet de loi de finances pour 2027".into());
        row
    }

    async fn list_with_dossiers(
        rows: Vec<ScrutinSummary>,
        dossiers: &FakeDossiers,
    ) -> ScrutinListPage {
        let total = rows.len() as i64;
        let repository = FakeScrutins {
            rows,
            total,
            shape: DatasetShape::default(),
        };
        ListScrutins::new(&repository, &citations(vec![]), &actors(), dossiers)
            .execute(ScrutinFilter::default())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn should_attach_its_dossier_to_each_scrutin_when_source_links_one() {
        let rejected = DossierOutcome::Rejected {
            date: chrono::NaiveDate::from_ymd_opt(2026, 11, 4).unwrap(),
            label: "rejet\u{e9}".into(),
        };
        let dossiers = FakeDossiers {
            dossiers: vec![listed_dossier(
                "DLR5L17N54629",
                rejected.clone(),
                Some(LegislativeStage::FirstReadingAN),
            )],
            ..Default::default()
        };

        let page = list_with_dossiers(
            vec![
                in_dossier("S1", "DLR5L17N54629"),
                scrutin_row("S2", "l'ensemble du projet de loi"),
            ],
            &dossiers,
        )
        .await;

        let first = page.items[0].dossier.as_ref().unwrap();
        assert_eq!(first.uid().as_str(), "DLR5L17N54629");
        assert_eq!(first.outcome(), &rejected);
        assert_eq!(
            first.current_stage(),
            Some(LegislativeStage::FirstReadingAN)
        );
        assert!(page.items[1].dossier.is_none());
    }

    #[tokio::test]
    async fn should_read_dossiers_once_per_page_when_several_scrutins_share_one() {
        let dossiers = FakeDossiers {
            dossiers: vec![
                listed_dossier("DLR5L17N1", DossierOutcome::NoRecordedConclusion, None),
                listed_dossier("DLR5L17N2", DossierOutcome::NoRecordedConclusion, None),
            ],
            ..Default::default()
        };

        let page = list_with_dossiers(
            vec![
                in_dossier("S1", "DLR5L17N2"),
                in_dossier("S2", "DLR5L17N1"),
                in_dossier("S3", "DLR5L17N2"),
            ],
            &dossiers,
        )
        .await;

        assert_eq!(
            *dossiers.lookups.lock().unwrap(),
            vec![vec!["DLR5L17N1".to_string(), "DLR5L17N2".to_string()]]
        );
        assert!(page.items.iter().all(|i| i.dossier.is_some()));
    }

    #[tokio::test]
    async fn should_not_read_dossiers_when_no_scrutin_of_page_has_one() {
        let dossiers = FakeDossiers::default();

        list_with_dossiers(
            vec![scrutin_row("S1", "l'ensemble du projet de loi")],
            &dossiers,
        )
        .await;

        assert!(dossiers.lookups.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn should_give_no_dossier_when_linked_dossier_is_not_stored() {
        let page = list_with_dossiers(
            vec![in_dossier("S1", "DLR5L17N404")],
            &FakeDossiers::default(),
        )
        .await;

        assert!(page.items[0].dossier.is_none());
    }

    fn on(date: (i32, u32, u32), mut row: ScrutinSummary) -> ScrutinSummary {
        row.date = chrono::NaiveDate::from_ymd_opt(date.0, date.1, date.2).unwrap();
        row
    }

    fn day(d: u32) -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(2026, 11, d).unwrap()
    }

    fn fake_scrutins(rows: Vec<ScrutinSummary>) -> FakeScrutins {
        FakeScrutins {
            rows,
            total: 0,
            shape: DatasetShape::default(),
        }
    }

    #[tokio::test]
    async fn lists_every_scrutin_of_a_period_beyond_the_page_cap() {
        let rows: Vec<ScrutinSummary> = (0..362)
            .map(|i| on((2026, 11, 3), scrutin_row(&format!("S{i}"), "l'ensemble du projet de loi")))
            .collect();
        let repository = fake_scrutins(rows);

        let items = ListScrutins::new(
            &repository,
            &citations(vec![]),
            &actors(),
            &FakeDossiers::default(),
        )
        .execute_between(day(2), day(8))
        .await
        .unwrap();

        assert_eq!(items.len(), 362);
    }

    #[tokio::test]
    async fn leaves_out_the_scrutins_outside_the_period() {
        let repository = fake_scrutins(vec![
            on((2026, 11, 1), scrutin_row("S0", "l'ensemble du projet de loi")),
            on((2026, 11, 2), scrutin_row("S1", "l'ensemble du projet de loi")),
            on((2026, 11, 8), scrutin_row("S2", "l'ensemble du projet de loi")),
            on((2026, 11, 9), scrutin_row("S3", "l'ensemble du projet de loi")),
        ]);

        let items = ListScrutins::new(
            &repository,
            &citations(vec![]),
            &actors(),
            &FakeDossiers::default(),
        )
        .execute_between(day(2), day(8))
        .await
        .unwrap();

        let uids: Vec<&str> = items.iter().map(|i| i.summary.uid.as_str()).collect();
        assert_eq!(uids, vec!["S1", "S2"]);
    }

    #[tokio::test]
    async fn enriches_a_scrutin_of_a_period_with_its_short_label_and_dossier() {
        let repository = fake_scrutins(vec![on(
            (2026, 11, 4),
            in_dossier("S1", "DLR5L17N54629"),
        )]);
        let dossiers = FakeDossiers {
            dossiers: vec![listed_dossier(
                "DLR5L17N54629",
                DossierOutcome::NoRecordedConclusion,
                Some(LegislativeStage::FirstReadingAN),
            )],
            ..Default::default()
        };

        let items = ListScrutins::new(&repository, &citations(vec![]), &actors(), &dossiers)
            .execute_between(day(2), day(8))
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].short_label.as_str(), "Vote sur l'article 3");
        assert_eq!(
            items[0].dossier.as_ref().unwrap().uid().as_str(),
            "DLR5L17N54629"
        );
    }
}
