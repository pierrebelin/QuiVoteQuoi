use crate::application::ports::actor_repository::ActorRepository;
use crate::application::ports::amendment_citation_repository::AmendmentCitationRepository;
use crate::application::ports::scrutin_repository::{
    RepositoryError, ScrutinFilter, ScrutinRepository, ScrutinSummary,
};
use crate::application::use_cases::browse_dossier_amendments::{
    author_uids, cited_view_of, CitedAmendmentView,
};
use crate::domain::actor::ActorDirectory;
use crate::domain::scrutin::{ScrutinUid, ShortLabel};

/// Ligne de la liste: le scrutin publie, son libelle court (RM-09) et les
/// amendements que son objet cite (RM-05, RM-08).
#[derive(Debug, Clone)]
pub struct ScrutinListItem {
    pub summary: ScrutinSummary,
    pub short_label: ShortLabel,
    pub cited: Vec<CitedAmendmentView>,
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
}

impl<'a> ListScrutins<'a> {
    pub fn new(
        repository: &'a dyn ScrutinRepository,
        citations: &'a dyn AmendmentCitationRepository,
        actors: &'a dyn ActorRepository,
    ) -> Self {
        Self {
            repository,
            citations,
            actors,
        }
    }

    pub async fn execute(&self, filter: ScrutinFilter) -> Result<ScrutinListPage, RepositoryError> {
        let page = self.repository.list(&filter).await?;
        let uids: Vec<ScrutinUid> = page
            .items
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

        let items = page
            .items
            .into_iter()
            .map(|summary| {
                let cited = rows
                    .iter()
                    .filter(|row| row.scrutin.as_str() == summary.uid)
                    .filter_map(|row| cited_view_of(row.clone(), &directory))
                    .collect();
                ScrutinListItem {
                    short_label: ShortLabel::from_subject(&summary.subject),
                    cited,
                    summary,
                }
            })
            .collect();
        Ok(ScrutinListPage {
            items,
            total: page.total,
        })
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
        amendment_row, scrutin_row, FakeActors, FakeScrutins,
    };
    use crate::domain::amendment::SummaryPreview;
    use crate::domain::amendment_citation::{LinkMethod, UnlinkedCause};
    use crate::domain::dossier::DossierUid;
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
        let repository = FakeScrutins { rows, total };
        let citations = citations(cited);
        ListScrutins::new(&repository, &citations, &actors())
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
        };

        let page = ListScrutins::new(&repository, &citations(vec![]), &actors())
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
}
