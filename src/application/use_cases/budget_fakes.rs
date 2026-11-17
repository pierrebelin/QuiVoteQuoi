//! Doublure du dépôt des exercices budgétaires et fixtures partagées par les
//! tests des use cases du budget.
//!
//! Port d'état : un fake en mémoire, on vérifie ce qui est stocké.

use std::collections::BTreeMap;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::application::ports::budget_repository::{BudgetRepository, RepositoryError};
use crate::domain::budget::{
    AccountKind, ActionCode, AdoptionProcedure, BudgetExercise, BudgetStage, BudgetYear,
    CategoryCredit, Euros, LawReference, Mission, NewBudgetExercise, Programme, ProgrammeAction,
    ProgrammeNumber, SourceReference, SpendingCategory,
};

#[derive(Default)]
pub struct InMemoryBudgetRepository {
    pub exercises: Mutex<BTreeMap<u16, BudgetExercise>>,
    /// Nombre d'appels à `replace_exercise`, refusés ou non : un import refusé
    /// doit le laisser à zéro.
    pub replace_calls: Mutex<usize>,
}

impl InMemoryBudgetRepository {
    pub fn with_exercises(exercises: Vec<BudgetExercise>) -> Self {
        let repository = Self::default();
        for exercise in exercises {
            repository
                .exercises
                .lock()
                .unwrap()
                .insert(exercise.year().get(), exercise);
        }
        repository
    }

    pub fn stored(&self, year: u16) -> Option<BudgetExercise> {
        self.exercises.lock().unwrap().get(&year).cloned()
    }

    pub fn replace_calls(&self) -> usize {
        *self.replace_calls.lock().unwrap()
    }
}

#[async_trait]
impl BudgetRepository for InMemoryBudgetRepository {
    async fn replace_exercise(&self, exercise: &BudgetExercise) -> Result<(), RepositoryError> {
        *self.replace_calls.lock().unwrap() += 1;
        self.exercises
            .lock()
            .unwrap()
            .insert(exercise.year().get(), exercise.clone());
        Ok(())
    }

    async fn find_exercise(
        &self,
        year: BudgetYear,
    ) -> Result<Option<BudgetExercise>, RepositoryError> {
        Ok(self.stored(year.get()))
    }

    async fn available_years(&self) -> Result<Vec<BudgetYear>, RepositoryError> {
        let years = self.exercises.lock().unwrap();
        Ok(years
            .keys()
            .rev()
            .map(|year| BudgetYear::new(*year).unwrap())
            .collect())
    }
}

pub fn euros(amount: i64) -> Euros {
    Euros::new(amount).unwrap()
}

/// Programme doté d'une action unique en fonctionnement (catégorie 31), sans
/// action à 0 €.
pub fn programme(number: u16, name: &str, cp: i64) -> Programme {
    let actions = if cp == 0 {
        Vec::new()
    } else {
        vec![action("01", "Action unique", vec![(31, cp)])]
    };
    programme_with(number, name, cp, actions)
}

pub fn programme_with(
    number: u16,
    name: &str,
    cp: i64,
    actions: Vec<ProgrammeAction>,
) -> Programme {
    Programme::with_actions(
        ProgrammeNumber::new(number).unwrap(),
        name,
        euros(cp),
        actions,
    )
    .unwrap()
}

/// Action et ses crédits, donnés en (catégorie, CP).
pub fn action(code: &str, name: &str, credits: Vec<(u8, i64)>) -> ProgrammeAction {
    ProgrammeAction::new(
        ActionCode::new(code).unwrap(),
        name,
        credits
            .into_iter()
            .map(|(category, cp)| CategoryCredit {
                category: SpendingCategory::new(category).unwrap(),
                cp: euros(cp),
            })
            .collect(),
    )
    .unwrap()
}

pub fn mission(name: &str, kind: AccountKind, programmes: Vec<Programme>) -> Mission {
    Mission::new(name, kind, programmes).unwrap()
}

pub fn law() -> LawReference {
    LawReference {
        number: "2026-103".to_string(),
        date: NaiveDate::from_ymd_opt(2026, 2, 19).unwrap(),
        credits_article: 148,
        adoption_procedure: AdoptionProcedure::Article49_3,
        legifrance_url: "https://www.legifrance.example/loi".to_string(),
    }
}

pub fn source() -> SourceReference {
    SourceReference {
        title: "Crédits AE et CP votés".to_string(),
        url: "https://www.budget.example/fichier".to_string(),
        published_on: NaiveDate::from_ymd_opt(2026, 3, 3).unwrap(),
        downloaded_on: NaiveDate::from_ymd_opt(2026, 11, 16).unwrap(),
        sha256: "ab".repeat(32),
    }
}

/// Exercice de l'année donnée, sans contrôle de ses chiffres.
pub fn exercise(year: u16, missions: Vec<Mission>) -> BudgetExercise {
    BudgetExercise::new(NewBudgetExercise {
        year: BudgetYear::new(year).unwrap(),
        stage: BudgetStage::Lfi,
        law: law(),
        source: source(),
        missions,
    })
    .unwrap()
}
