use async_trait::async_trait;

use crate::domain::budget::{BudgetExercise, BudgetYear};

pub use super::RepositoryError;

/// Exercices budgétaires publiés, un par année.
#[async_trait]
pub trait BudgetRepository: Send + Sync {
    /// Remplace l'exercice de la même année, missions et programmes compris.
    ///
    /// Tout ou rien : un exercice n'est jamais lu à moitié remplacé. Rejouer le
    /// même exercice laisse la base dans le même état.
    async fn replace_exercise(&self, exercise: &BudgetExercise) -> Result<(), RepositoryError>;

    /// L'exercice d'une année, `None` quand elle n'a jamais été importée.
    async fn find_exercise(
        &self,
        year: BudgetYear,
    ) -> Result<Option<BudgetExercise>, RepositoryError>;

    /// Années importées, de la plus récente à la plus ancienne.
    async fn available_years(&self) -> Result<Vec<BudgetYear>, RepositoryError>;
}
