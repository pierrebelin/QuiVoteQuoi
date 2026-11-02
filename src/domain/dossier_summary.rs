//! Invariants applicables au texte publie par le generateur de syntheses, et
//! regles de fraicheur des syntheses stockees.
//!
//! Le modele ne produit ni chiffre ni position globale. Ces regles sont
//! revalidees apres la reponse du fournisseur, meme si celui-ci annonce un
//! schema JSON strict.

use chrono::NaiveDateTime;

use super::actor::GroupUid;

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

/// Empreinte des faits d'un dossier au moment ou une synthese a ete produite.
///
/// Valeur opaque pour le domaine: le calcul (hachage de la serialisation des
/// faits) reste a l'application, le domaine ne fait que la comparer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactsFingerprint(String);

impl FactsFingerprint {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SummaryStatus {
    Pending,
    Ready,
}

#[derive(Debug, Clone)]
pub struct SummarySource {
    pub source_id: String,
    pub kind: String,
    pub uid: String,
    pub label: String,
    pub official_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StoredGroupSummary {
    pub group_uid: GroupUid,
    pub status: SummaryStatus,
    pub paragraph: Option<String>,
    pub facts_fingerprint: String,
    pub model: Option<String>,
    pub prompt_version: Option<String>,
    pub generated_at: Option<NaiveDateTime>,
    pub sources: Vec<SummarySource>,
}

impl StoredGroupSummary {
    /// Regle de lecture: la synthese est montrable si elle est prete et produite
    /// pour les faits courants (meme empreinte).
    ///
    /// Le modele et la version de prompt ne sont volontairement pas verifies: une
    /// synthese d'un ancien modele reste montrable tant que les faits n'ont pas
    /// bouge. La regenerer est l'affaire du rafraichissement (`is_up_to_date`),
    /// jamais d'une lecture. L'appartenance du groupe aux faits reste verifiee
    /// par l'appelant.
    pub fn is_shown_for(&self, current: &FactsFingerprint) -> bool {
        self.status == SummaryStatus::Ready && self.facts_fingerprint == current.as_str()
    }

    /// Regle d'ecriture: la synthese n'a pas besoin d'etre regeneree si elle est
    /// montrable pour les faits courants et produite par le modele et la version
    /// de prompt du generateur actuel. Un modele ou une version inconnus (`None`)
    /// ne comptent jamais comme a jour.
    pub fn is_up_to_date(
        &self,
        current: &FactsFingerprint,
        model: &str,
        prompt_version: &str,
    ) -> bool {
        self.is_shown_for(current)
            && self.model.as_deref() == Some(model)
            && self.prompt_version.as_deref() == Some(prompt_version)
    }
}

/// Etat de la synthese d'un groupe tel que la fiche dossier le presente.
///
/// Priorite: pas d'acte (`NoData`) avant synthese montrable (`Ready`) avant
/// synthese en attente (`SummaryPending`); le reste est `SummaryUnavailable`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupSummaryState {
    NoData,
    Ready,
    SummaryPending,
    SummaryUnavailable,
}

impl GroupSummaryState {
    /// `has_actions`: le groupe porte au moins un acte (`GroupFacts::has_actions`).
    /// `has_summary`: une synthese montrable existe pour lui. `stored_status`:
    /// statut de la synthese stockee, s'il y en a une.
    pub fn new(
        has_actions: bool,
        has_summary: bool,
        stored_status: Option<&SummaryStatus>,
    ) -> Self {
        if !has_actions {
            Self::NoData
        } else if has_summary {
            Self::Ready
        } else if stored_status == Some(&SummaryStatus::Pending) {
            Self::SummaryPending
        } else {
            Self::SummaryUnavailable
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NoData => "no_data",
            Self::Ready => "ready",
            Self::SummaryPending => "summary_pending",
            Self::SummaryUnavailable => "summary_unavailable",
        }
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

    fn fingerprint(value: &str) -> FactsFingerprint {
        FactsFingerprint::new(value.into())
    }

    fn stored(status: SummaryStatus, facts_fingerprint: &str) -> StoredGroupSummary {
        StoredGroupSummary {
            group_uid: GroupUid::new("G1".to_string()).unwrap(),
            status,
            paragraph: Some("Texte descriptif".into()),
            facts_fingerprint: facts_fingerprint.into(),
            model: Some("model-a".into()),
            prompt_version: Some("v1".into()),
            generated_at: None,
            sources: vec![],
        }
    }

    #[test]
    fn a_ready_summary_with_the_current_fingerprint_is_shown() {
        assert!(stored(SummaryStatus::Ready, "abc").is_shown_for(&fingerprint("abc")));
    }

    #[test]
    fn a_pending_summary_is_not_shown() {
        assert!(!stored(SummaryStatus::Pending, "abc").is_shown_for(&fingerprint("abc")));
    }

    #[test]
    fn a_summary_made_for_other_facts_is_not_shown() {
        assert!(!stored(SummaryStatus::Ready, "old").is_shown_for(&fingerprint("abc")));
    }

    #[test]
    fn a_summary_from_another_model_is_still_shown() {
        let mut summary = stored(SummaryStatus::Ready, "abc");
        summary.model = Some("former-model".into());
        summary.prompt_version = Some("former-version".into());
        assert!(summary.is_shown_for(&fingerprint("abc")));
    }

    #[test]
    fn a_summary_matching_facts_model_and_prompt_is_up_to_date() {
        assert!(stored(SummaryStatus::Ready, "abc").is_up_to_date(
            &fingerprint("abc"),
            "model-a",
            "v1"
        ));
    }

    #[test]
    fn a_summary_from_another_model_is_not_up_to_date() {
        assert!(!stored(SummaryStatus::Ready, "abc").is_up_to_date(
            &fingerprint("abc"),
            "model-b",
            "v1"
        ));
    }

    #[test]
    fn a_summary_from_another_prompt_version_is_not_up_to_date() {
        assert!(!stored(SummaryStatus::Ready, "abc").is_up_to_date(
            &fingerprint("abc"),
            "model-a",
            "v2"
        ));
    }

    #[test]
    fn a_summary_without_model_or_prompt_version_is_not_up_to_date() {
        let mut no_model = stored(SummaryStatus::Ready, "abc");
        no_model.model = None;
        let mut no_prompt = stored(SummaryStatus::Ready, "abc");
        no_prompt.prompt_version = None;
        assert!(!no_model.is_up_to_date(&fingerprint("abc"), "model-a", "v1"));
        assert!(!no_prompt.is_up_to_date(&fingerprint("abc"), "model-a", "v1"));
    }

    #[test]
    fn a_summary_not_shown_is_never_up_to_date() {
        assert!(!stored(SummaryStatus::Pending, "abc").is_up_to_date(
            &fingerprint("abc"),
            "model-a",
            "v1"
        ));
        assert!(!stored(SummaryStatus::Ready, "old").is_up_to_date(
            &fingerprint("abc"),
            "model-a",
            "v1"
        ));
    }

    #[test]
    fn a_group_without_actions_has_no_data_whatever_is_stored() {
        assert_eq!(
            GroupSummaryState::new(false, true, Some(&SummaryStatus::Ready)),
            GroupSummaryState::NoData
        );
        assert_eq!(
            GroupSummaryState::new(false, false, Some(&SummaryStatus::Pending)),
            GroupSummaryState::NoData
        );
    }

    #[test]
    fn a_group_with_a_shown_summary_is_ready_even_if_a_pending_status_is_stored() {
        assert_eq!(
            GroupSummaryState::new(true, true, Some(&SummaryStatus::Ready)),
            GroupSummaryState::Ready
        );
        assert_eq!(
            GroupSummaryState::new(true, true, Some(&SummaryStatus::Pending)),
            GroupSummaryState::Ready
        );
    }

    #[test]
    fn a_group_with_actions_and_a_pending_summary_is_summary_pending() {
        assert_eq!(
            GroupSummaryState::new(true, false, Some(&SummaryStatus::Pending)),
            GroupSummaryState::SummaryPending
        );
    }

    #[test]
    fn a_group_with_actions_and_no_usable_summary_is_summary_unavailable() {
        assert_eq!(
            GroupSummaryState::new(true, false, None),
            GroupSummaryState::SummaryUnavailable
        );
        assert_eq!(
            GroupSummaryState::new(true, false, Some(&SummaryStatus::Ready)),
            GroupSummaryState::SummaryUnavailable
        );
    }
}
