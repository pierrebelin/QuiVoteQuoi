//! Import d'un exercice du budget de l'État depuis son manifeste : le câblage
//! du binaire `import-budget`, gardé dans la bibliothèque pour être testé.
//!
//! Le manifeste désigne les deux fichiers officiels (crédits votés,
//! nomenclature), leurs empreintes et les attendus tirés de la loi. Tout
//! contrôle qui échoue arrête l'import avant la moindre écriture. Rejouer le
//! même import remplace l'exercice à l'identique.

use std::path::Path;

use async_trait::async_trait;

use crate::application::ports::budget_repository::{BudgetRepository, RepositoryError};
use crate::application::use_cases::import_budget_exercise::{
    ImportBudgetExercise, ImportBudgetExerciseCommand, ImportReport,
};
use crate::domain::budget::{BudgetExercise, BudgetYear};
use crate::infrastructure::budget::lfi_workbook::read_credit_lines;
use crate::infrastructure::budget::manifest::Manifest;
use crate::infrastructure::budget::nomenclature_workbook::{
    complete_with_nomenclature, read_nomenclature,
};
use crate::infrastructure::config::try_connect_database;
use crate::infrastructure::persistence::pg_budget_repository::PgBudgetRepository;

pub const DRY_RUN_FLAG: &str = "--dry-run";

/// Ce que l'import a fait, à afficher.
pub struct ImportOutcome {
    pub report: ImportReport,
    /// Types de ligne du fichier des crédits écartés (`BA`, `CAS`…), avec leur nombre.
    pub discarded_types: Vec<(String, usize)>,
    pub dry_run: bool,
}

/// Dépôt du mode `--dry-run` : accepte l'écriture sans rien garder, pour que
/// le même use case contrôle l'exercice sans base.
struct DiscardingRepository;

#[async_trait]
impl BudgetRepository for DiscardingRepository {
    async fn replace_exercise(&self, _exercise: &BudgetExercise) -> Result<(), RepositoryError> {
        Ok(())
    }

    async fn find_exercise(
        &self,
        _year: BudgetYear,
    ) -> Result<Option<BudgetExercise>, RepositoryError> {
        Ok(None)
    }

    async fn available_years(&self) -> Result<Vec<BudgetYear>, RepositoryError> {
        Ok(Vec::new())
    }
}

/// Lit, contrôle et, hors `dry_run`, écrit l'exercice du manifeste.
pub async fn import_budget(manifest_path: &Path, dry_run: bool) -> Result<ImportOutcome, String> {
    let manifest = Manifest::read(manifest_path).map_err(|error| error.to_string())?;
    let credits_path = manifest
        .verified_source_path(manifest_path)
        .map_err(|error| error.to_string())?;
    let nomenclature_path = manifest
        .verified_nomenclature_path(manifest_path)
        .map_err(|error| error.to_string())?;
    let resolved = manifest.resolve().map_err(|error| error.to_string())?;

    let credits =
        read_credit_lines(&credits_path, resolved.year).map_err(|error| error.to_string())?;
    let discarded_types = credits.discarded_types.into_iter().collect();
    let nomenclature =
        read_nomenclature(&nomenclature_path, resolved.year).map_err(|error| error.to_string())?;
    let lines = complete_with_nomenclature(
        credits.lines,
        &nomenclature,
        &manifest.nomenclature.absent_from_law,
    )
    .map_err(|error| error.to_string())?;

    let command = ImportBudgetExerciseCommand {
        year: resolved.year,
        stage: resolved.stage,
        law: resolved.law,
        source: resolved.source,
        lines,
        expectations: resolved.expectations,
    };

    let report = if dry_run {
        ImportBudgetExercise::new(&DiscardingRepository)
            .execute(command)
            .await
    } else {
        let repository = PgBudgetRepository::new(try_connect_database().await?);
        ImportBudgetExercise::new(&repository)
            .execute(command)
            .await
    }
    .map_err(|error| error.to_string())?;

    Ok(ImportOutcome {
        report,
        discarded_types,
        dry_run,
    })
}

/// Compte rendu lisible, une information par ligne.
pub fn describe(outcome: &ImportOutcome) -> Vec<String> {
    let report = &outcome.report;
    let verb = if outcome.dry_run {
        "contrôlé (rien écrit)"
    } else {
        "importé"
    };
    let mut lines = vec![format!(
        "exercice {} {verb} : {} missions, {} programmes, {} € au budget général",
        report.year,
        report.general_budget_missions,
        report.general_budget_programmes,
        report.general_budget_total
    )];
    lines.push(format!(
        "{} actions au budget général",
        report.general_budget_actions
    ));
    for share in report.general_budget_nature.natures() {
        lines.push(format!(
            "titre {} ({}) : {} €",
            share.nature.title(),
            share.nature.official_label(),
            share.cp
        ));
    }
    for account in &report.off_budget {
        lines.push(format!(
            "hors budget général : {} — {} programmes, {} €",
            account.mission, account.programmes, account.total
        ));
    }
    for (kind, rows) in &outcome.discarded_types {
        lines.push(format!("lignes de type {kind} écartées : {rows}"));
    }
    if report.discarded_lines > 0 {
        lines.push(format!(
            "programmes hors attendus écartés : {} ({})",
            report.discarded_lines,
            report.discarded_missions.join(" ; ")
        ));
    }
    lines
}
