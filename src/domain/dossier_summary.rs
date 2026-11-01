//! Invariants applicables au texte publie par le generateur de syntheses.
//!
//! Le modele ne produit ni chiffre ni position globale. Ces regles sont
//! revalidees apres la reponse du fournisseur, meme si celui-ci annonce un
//! schema JSON strict.

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SummaryError {
    #[error("summary paragraph is empty")]
    Empty,
    #[error("summary paragraph is too long")]
    TooLong,
    #[error("summary paragraph contains a number")]
    ContainsNumber,
    #[error("summary paragraph contains forbidden positioning vocabulary")]
    ForbiddenPositioning,
    #[error("summary paragraph must be a single paragraph")]
    MultipleParagraphs,
}

const MAX_SUMMARY_CHARS: usize = 900;
const FORBIDDEN_TERMS: &[&str] = &[
    "soutient",
    "s'oppose",
    "s’oppose",
    "favorable",
    "défavorable",
    "position globale",
    "position du groupe",
    "ligne du groupe",
    "cohérent",
    "cohérente",
    "incohérent",
    "incohérente",
    "majoritaire",
    "vote pour",
    "vote contre",
    "a voté pour",
    "a voté contre",
    "constance",
    "classement",
    "important",
    "significatif",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryParagraph(String);

impl SummaryParagraph {
    pub fn new(raw: String) -> Result<Self, SummaryError> {
        let text = raw.trim();
        if text.is_empty() {
            return Err(SummaryError::Empty);
        }
        if text.chars().count() > MAX_SUMMARY_CHARS {
            return Err(SummaryError::TooLong);
        }
        if text.chars().any(char::is_numeric) {
            return Err(SummaryError::ContainsNumber);
        }
        if text.contains('\n') || text.contains('\r') {
            return Err(SummaryError::MultipleParagraphs);
        }

        let lowered = text.to_lowercase();
        if FORBIDDEN_TERMS.iter().any(|term| lowered.contains(term)) {
            return Err(SummaryError::ForbiddenPositioning);
        }

        Ok(Self(text.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_neutral_descriptive_paragraph() {
        let value = SummaryParagraph::new(
            "Les actes associes au groupe sont rattaches aux textes et aux scrutins publies pour ce dossier.".into(),
        );
        assert!(value.is_ok());
    }

    #[test]
    fn a_paragraph_containing_a_number_is_rejected() {
        assert_eq!(
            SummaryParagraph::new("Le groupe apparait sur 2 actes.".into()),
            Err(SummaryError::ContainsNumber)
        );
    }

    #[test]
    fn a_paragraph_using_positioning_vocabulary_is_rejected() {
        assert_eq!(
            SummaryParagraph::new("Le groupe est favorable au texte.".into()),
            Err(SummaryError::ForbiddenPositioning)
        );
    }
}
