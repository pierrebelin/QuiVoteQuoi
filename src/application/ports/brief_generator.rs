use async_trait::async_trait;

use crate::domain::cited_article::CitedWording;

pub use super::dossier_summary_generator::SummaryGeneratorError;

/// Les articles d'une version à résumer en un appel (SPEC-en-bref RM-09).
///
/// Le modèle ne reçoit que les sources de chaque article : ni l'intitulé du
/// texte, ni l'exposé des motifs, ni le numéro de l'article.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BriefRequest {
    pub articles: Vec<BriefArticle>,
}

/// Un article à résumer et ses sources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BriefArticle {
    /// Identifiant que le modèle recopie dans sa réponse : l'empreinte.
    pub key: String,
    pub content: String,
    /// Articles de code ou de loi que l'article modifie (RM-07).
    pub cited: Vec<BriefCitation>,
}

/// Un article cité, tel que le modèle le reçoit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BriefCitation {
    /// Le texte et le numéro de l'article cité, en langage courant.
    pub label: String,
    /// `InForce` : sa rédaction à la date de la version ; `Created` : le texte
    /// le crée. Un article que la source ne connaît pas n'est pas transmis.
    pub wording: CitedWording,
}

/// Ligne rendue par le modèle pour un article, avant tout contrôle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedLine {
    pub key: String,
    pub line: String,
}

/// Fournisseur de modèle qui résume les articles d'une version (SPEC-en-bref
/// CU-02).
#[async_trait]
pub trait BriefGenerator: Send + Sync {
    /// Une ligne par article reçu, au mieux : le code contrôle chaque ligne et
    /// traite l'absence d'une clé (RM-13).
    async fn generate(
        &self,
        request: &BriefRequest,
    ) -> Result<Vec<GeneratedLine>, SummaryGeneratorError>;

    fn model(&self) -> &str;

    fn prompt_version(&self) -> &str;
}
