use crate::application::ports::scrutin_repository::{RepositoryError, ScrutinRepository};
use crate::application::ports::theme_repository::ThemeRepository;
use crate::domain::dataset::DatasetShape;

/// Ce que la page « Comprendre » affiche en chiffres.
///
/// Les grandeurs dérivées sont calculées ici, pas dans la page : un rapport
/// affiché doit être reproductible et testable, et il ne doit exister qu'une
/// seule formule par chiffre (README.md §6).
#[derive(Debug, Clone, PartialEq)]
pub struct DatasetOverview {
    pub shape: DatasetShape,
    pub texts_total: i64,
    /// `scrutins_total - scrutins_with_dossier`.
    pub scrutins_without_dossier: i64,
    /// Part des scrutins sans dossier, en pourcentage entier. `None` quand la
    /// base est vide : une part n'a alors aucun sens, et afficher 0 % ferait
    /// croire à une couverture complète.
    pub scrutins_without_dossier_share: Option<i64>,
    /// Nombre moyen de scrutins par texte débattu, à la décimale. `None` si
    /// aucun texte n'a encore été extrait.
    pub scrutins_per_text: Option<f64>,
}

/// Guide de lecture — volumétrie et répartitions du jeu de données.
pub struct DescribeDataset<'a> {
    scrutins: &'a dyn ScrutinRepository,
    themes: &'a dyn ThemeRepository,
}

impl<'a> DescribeDataset<'a> {
    pub fn new(scrutins: &'a dyn ScrutinRepository, themes: &'a dyn ThemeRepository) -> Self {
        Self { scrutins, themes }
    }

    pub async fn execute(&self) -> Result<DatasetOverview, RepositoryError> {
        let shape = self.scrutins.dataset_shape().await?;
        let texts_total = self.themes.text_count().await?;

        let scrutins_without_dossier = shape.scrutins_without_dossier();
        let scrutins_without_dossier_share = shape.share_without_dossier_percent();
        let scrutins_per_text = shape.scrutins_per_text(texts_total);

        Ok(DatasetOverview {
            shape,
            texts_total,
            scrutins_without_dossier,
            scrutins_without_dossier_share,
            scrutins_per_text,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;

    use super::*;
    use crate::application::ports::scrutin_repository::{
        ScrutinFilter, ScrutinPage, ScrutinSummary,
    };
    use crate::application::use_cases::theme_fakes::InMemoryThemeRepository;
    use crate::domain::scrutin::{Scrutin, ScrutinUid};
    use crate::domain::theme::DebatedText;

    /// Port d'état réduit à ce que le use case lit : le reste du contrat n'est
    /// pas exercé ici et une doublure complète masquerait cette portée.
    #[derive(Default)]
    struct StubScrutinRepository {
        shape: Mutex<DatasetShape>,
    }

    impl StubScrutinRepository {
        fn with(shape: DatasetShape) -> Self {
            Self {
                shape: Mutex::new(shape),
            }
        }
    }

    #[async_trait]
    impl ScrutinRepository for StubScrutinRepository {
        async fn save_scrutins(&self, _: &[Scrutin]) -> Result<usize, RepositoryError> {
            unimplemented!("hors portée du guide de lecture")
        }

        async fn list(&self, _: &ScrutinFilter) -> Result<ScrutinPage, RepositoryError> {
            unimplemented!("hors portée du guide de lecture")
        }

        async fn by_uid(&self, _: &ScrutinUid) -> Result<Option<Scrutin>, RepositoryError> {
            unimplemented!("hors portée du guide de lecture")
        }

        async fn by_dossier(&self, _: &str) -> Result<Vec<ScrutinSummary>, RepositoryError> {
            unimplemented!("hors portée du guide de lecture")
        }

        async fn sitting_keys(
            &self,
        ) -> Result<Vec<crate::domain::sitting_record::ScrutinSittingKey>, RepositoryError> {
            unimplemented!("hors portée du guide de lecture")
        }

        async fn dataset_shape(&self) -> Result<DatasetShape, RepositoryError> {
            Ok(self.shape.lock().unwrap().clone())
        }
    }

    async fn themes_with(count: usize) -> InMemoryThemeRepository {
        let repository = InMemoryThemeRepository::default();
        let texts: Vec<DebatedText> = (0..count)
            .map(|index| DebatedText::new(format!("projet de loi n° {index}")).unwrap())
            .collect();
        repository.save_texts(&texts).await.unwrap();
        repository
    }

    #[tokio::test]
    async fn derives_share_and_ratio_from_the_shape() {
        let scrutins = StubScrutinRepository::with(DatasetShape {
            scrutins_total: 8_434,
            scrutins_with_dossier: 2_614,
            ..DatasetShape::default()
        });
        let themes = themes_with(322).await;

        let overview = DescribeDataset::new(&scrutins, &themes)
            .execute()
            .await
            .unwrap();

        assert_eq!(overview.scrutins_without_dossier, 5_820);
        assert_eq!(overview.scrutins_without_dossier_share, Some(69));
        assert_eq!(overview.scrutins_per_text, Some(26.2));
        assert_eq!(overview.texts_total, 322);
    }

    /// Base vide : aucune part, aucun ratio. Un 0 % affiché se lirait comme
    /// « tous les scrutins portent un dossier », l'inverse du fait.
    #[tokio::test]
    async fn reports_no_share_when_there_is_nothing_to_divide() {
        let scrutins = StubScrutinRepository::default();
        let themes = InMemoryThemeRepository::default();

        let overview = DescribeDataset::new(&scrutins, &themes)
            .execute()
            .await
            .unwrap();

        assert_eq!(overview.scrutins_without_dossier, 0);
        assert_eq!(overview.scrutins_without_dossier_share, None);
        assert_eq!(overview.scrutins_per_text, None);
    }

    /// Des scrutins mais aucun texte extrait : la part reste calculable, le
    /// ratio non.
    #[tokio::test]
    async fn reports_share_without_any_extracted_text() {
        let scrutins = StubScrutinRepository::with(DatasetShape {
            scrutins_total: 10,
            scrutins_with_dossier: 3,
            ..DatasetShape::default()
        });
        let themes = InMemoryThemeRepository::default();

        let overview = DescribeDataset::new(&scrutins, &themes)
            .execute()
            .await
            .unwrap();

        assert_eq!(overview.scrutins_without_dossier_share, Some(70));
        assert_eq!(overview.scrutins_per_text, None);
    }
}
