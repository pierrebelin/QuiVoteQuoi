//! Port d'ingestion des amendements.
//!
//! La source publie une archive complete par legislature, et SPEC-amendements
//! RM-01 impose de tout prendre. Mais la ou 8 434 scrutins tiennent en memoire,
//! une legislature d'amendements ne tient pas: chacun porte un expose sommaire,
//! et il y en a plusieurs centaines de milliers. Le port rend donc des **lots**
//! plutot qu'une collection, pour que la memoire tenue soit bornee par le lot et
//! non par l'archive.
//!
//! Le decoupage a une deuxieme raison, structurelle: le parcours d'un ZIP est
//! synchrone et l'ecriture en base est asynchrone. L'adapter choisit la couture
//! entre les deux (`BatchStream`).

use std::collections::BTreeMap;

use async_trait::async_trait;

use crate::domain::amendment::Amendment;

pub use super::SourceError;

/// Ce qu'un parcours d'archive a vu, y compris ce qu'il n'a pas su lire.
///
/// Toute lacune y figure. Un ecart entre le publie et l'ingere est une lacune,
/// pas un detail d'implementation: il doit se voir (README.md §2).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArchiveScan {
    /// Entrees `.json` rencontrees dans l'archive.
    pub json_entries: usize,
    pub parsed: usize,
    /// Entrees dont le contenu n'est pas du texte UTF-8.
    pub undecodable: usize,
    /// Entrees dont la structure ne correspond pas au JSON attendu.
    pub malformed: usize,
    /// Entrees refusees par les invariants du domaine.
    pub refused: usize,
    /// Echantillon borne des causes rencontrees pendant le parcours.
    pub failures: BTreeMap<String, usize>,
    pub other_legislature: usize,
    /// Amendements que la source ne rattache a aucun texte legislatif. Ils
    /// entrent quand meme (RM-01), la lacune est comptee.
    pub without_text_ref: usize,
    /// Sorts publies hors referentiel, avec leur nombre d'occurrences (RM-04).
    pub unknown_fates: BTreeMap<String, usize>,
    /// Premiers segments de chemin rencontres dans le ZIP, avec leur nombre.
    /// Diagnostic d'arborescence uniquement: le parseur ne s'en sert pas, il les
    /// journalise. Une source qui renomme ses repertoires doit se voir dans le
    /// journal, pas vider l'ingestion en silence.
    pub top_level: BTreeMap<String, usize>,
}

impl ArchiveScan {
    pub fn unreadable(&self) -> usize {
        self.undecodable + self.malformed + self.refused
    }
}

#[derive(Debug)]
pub enum AmendmentBatch {
    Items(Vec<Amendment>),
    /// Dernier message du flux: bilan complet du parcours.
    Done(ArchiveScan),
}

/// Lecture d'un flux de lots. L'adapter choisit la couture (canal, fichier,
/// file en memoire) ; le port n'en connait que le prochain message.
#[async_trait]
pub trait BatchStream: Send {
    async fn next(&mut self) -> Option<Result<AmendmentBatch, SourceError>>;
}

/// Flux de lots. Se consomme jusqu'a `None`; le dernier message porte le bilan.
pub struct AmendmentBatches {
    stream: Box<dyn BatchStream>,
}

impl AmendmentBatches {
    pub fn new(stream: Box<dyn BatchStream>) -> Self {
        Self { stream }
    }

    /// Flux pret a consommer, sans tache ni reseau. Pour les tests de use case.
    #[cfg(test)]
    pub fn from_batches(batches: Vec<Result<AmendmentBatch, SourceError>>) -> Self {
        Self::new(Box::new(QueuedBatches(batches.into())))
    }

    pub async fn next(&mut self) -> Option<Result<AmendmentBatch, SourceError>> {
        self.stream.next().await
    }
}

#[cfg(test)]
struct QueuedBatches(std::collections::VecDeque<Result<AmendmentBatch, SourceError>>);

#[cfg(test)]
#[async_trait]
impl BatchStream for QueuedBatches {
    async fn next(&mut self) -> Option<Result<AmendmentBatch, SourceError>> {
        self.0.pop_front()
    }
}

/// Ce qu'une ouverture d'archive rend: son identite, et ses lots.
pub struct AmendmentFeed {
    /// Identite de l'archive servie, telle que la source la publie (`ETag`, ou
    /// `Last-Modified` a defaut). `None` quand la source n'en publie aucune.
    ///
    /// Permet de reconnaitre une archive deja ingeree et de ne pas la
    /// reparcourir. Abandonner `batches` sans le consommer interrompt le
    /// parcours: au plus un lot aura ete lu pour rien.
    pub archive_id: Option<String>,
    pub batches: AmendmentBatches,
}

#[async_trait]
pub trait AmendmentSource: Send + Sync {
    /// Ouvre l'archive et rend les amendements par lots d'au plus `batch_size`.
    ///
    /// L'archive n'est jamais materialisee en memoire sous forme d'agregats.
    async fn fetch_amendments(
        &self,
        legislature: u16,
        batch_size: usize,
    ) -> Result<AmendmentFeed, SourceError>;
}
