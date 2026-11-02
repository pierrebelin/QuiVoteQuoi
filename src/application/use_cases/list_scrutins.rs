use crate::application::ports::scrutin_repository::{
    RepositoryError, ScrutinFilter, ScrutinRepository, ScrutinSummary,
};
use crate::domain::scrutin::ShortLabel;

/// Ligne de la liste: le scrutin publie et son libelle court (RM-09).
#[derive(Debug, Clone)]
pub struct ScrutinListItem {
    pub summary: ScrutinSummary,
    pub short_label: ShortLabel,
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
}

impl<'a> ListScrutins<'a> {
    pub fn new(repository: &'a dyn ScrutinRepository) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, filter: ScrutinFilter) -> Result<ScrutinListPage, RepositoryError> {
        let page = self.repository.list(&filter).await?;
        let items = page
            .items
            .into_iter()
            .map(|summary| ScrutinListItem {
                short_label: ShortLabel::from_subject(&summary.subject),
                summary,
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
    use crate::application::use_cases::read_fakes::{scrutin_row, FakeScrutins};

    #[tokio::test]
    async fn should_give_each_scrutin_its_short_label_when_list_is_read() {
        let repository = FakeScrutins {
            rows: vec![
                scrutin_row("S1", "l'article 3 du projet de loi de finances"),
                scrutin_row("S2", "l'ensemble du projet de loi"),
                scrutin_row("S3", "objet inconnu de la source"),
            ],
            total: 3,
        };

        let page = ListScrutins::new(&repository)
            .execute(ScrutinFilter::default())
            .await
            .unwrap();

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

        let page = ListScrutins::new(&repository)
            .execute(ScrutinFilter::default())
            .await
            .unwrap();

        let uids: Vec<&str> = page.items.iter().map(|i| i.summary.uid.as_str()).collect();
        assert_eq!(uids, vec!["S3", "S1", "S2"]);
        assert_eq!(page.total, 7);
    }
}
