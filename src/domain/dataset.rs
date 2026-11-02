//! Forme du jeu de donnees.
//!
//! Ce sont des faits de couverture, pas une mesure d'activite parlementaire:
//! ils disent ce que le site contient et, par difference, ce qu'il ne contient
//! pas (README.md §2, §7). Les grandeurs derivees (part, ratio) sont calculees
//! ici, une seule formule par chiffre, pour qu'un rapport affiche soit
//! reproductible et testable (README.md §6).

use chrono::NaiveDate;

/// Un code de la source, son libelle publie, et le nombre de scrutins qui le
/// portent. Sert a montrer la frequence d'un mecanisme sans la qualifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeCount {
    pub code: String,
    pub label: String,
    pub count: i64,
}

/// Volumetrie et repartitions du jeu de donnees, telles qu'affichees sur la
/// page « Comprendre ».
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DatasetShape {
    pub scrutins_total: i64,
    /// Scrutins que la source rattache a un dossier legislatif. Le complement
    /// n'est pas une anomalie: il est majoritaire (RM-10).
    pub scrutins_with_dossier: i64,
    pub first_scrutin_date: Option<NaiveDate>,
    pub last_scrutin_date: Option<NaiveDate>,
    /// Legislatures couvertes, ordre croissant.
    pub legislatures: Vec<i64>,
    /// Repartition par sort du scrutin, la plus frequente d'abord.
    pub outcomes: Vec<CodeCount>,
    /// Repartition par type de scrutin (ordinaire, solennel, ...).
    pub ballot_types: Vec<CodeCount>,
    pub dossiers_total: i64,
}

impl DatasetShape {
    /// Scrutins que la source ne rattache a aucun dossier.
    pub fn scrutins_without_dossier(&self) -> i64 {
        self.scrutins_total - self.scrutins_with_dossier
    }

    /// Part des scrutins sans dossier, en pourcentage entier. `None` quand la
    /// base est vide: une part n'a alors aucun sens, et afficher 0 % ferait
    /// croire a une couverture complete.
    pub fn share_without_dossier_percent(&self) -> Option<i64> {
        (self.scrutins_total > 0).then(|| {
            (self.scrutins_without_dossier() as f64 * 100.0 / self.scrutins_total as f64).round()
                as i64
        })
    }

    /// Nombre moyen de scrutins par texte debattu, a la decimale. `None` si
    /// aucun texte n'a encore ete extrait.
    pub fn scrutins_per_text(&self, texts_total: i64) -> Option<f64> {
        (texts_total > 0)
            .then(|| (self.scrutins_total as f64 * 10.0 / texts_total as f64).round() / 10.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape_with(scrutins_total: i64, scrutins_with_dossier: i64) -> DatasetShape {
        DatasetShape {
            scrutins_total,
            scrutins_with_dossier,
            ..DatasetShape::default()
        }
    }

    #[test]
    fn counts_the_scrutins_without_dossier_as_the_complement() {
        assert_eq!(shape_with(8_434, 2_614).scrutins_without_dossier(), 5_820);
    }

    #[test]
    fn rounds_the_share_without_dossier_to_a_whole_percent() {
        assert_eq!(
            shape_with(8_434, 2_614).share_without_dossier_percent(),
            Some(69)
        );
    }

    #[test]
    fn reports_share_without_any_extracted_text() {
        let shape = shape_with(10, 3);

        assert_eq!(shape.share_without_dossier_percent(), Some(70));
        assert_eq!(shape.scrutins_per_text(0), None);
    }

    #[test]
    fn reports_no_share_when_there_is_no_scrutin() {
        let shape = DatasetShape::default();

        assert_eq!(shape.scrutins_without_dossier(), 0);
        assert_eq!(shape.share_without_dossier_percent(), None);
        assert_eq!(shape.scrutins_per_text(0), None);
    }

    #[test]
    fn rounds_scrutins_per_text_to_one_decimal() {
        assert_eq!(shape_with(8_434, 2_614).scrutins_per_text(322), Some(26.2));
    }

    #[test]
    fn reports_a_zero_ratio_rather_than_none_when_texts_exist_without_scrutins() {
        assert_eq!(DatasetShape::default().scrutins_per_text(5), Some(0.0));
    }
}
