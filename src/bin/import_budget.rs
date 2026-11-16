//! Importe un exercice du budget de l'État depuis son manifeste.
//!
//! ```text
//! import-budget data/budget/lfi-2026.json            # contrôle puis écrit en base
//! import-budget data/budget/lfi-2026.json --dry-run  # contrôle seulement, sans base
//! ```
//!
//! Câblage seul : la lecture, les contrôles et l'écriture sont dans
//! `quivotequoi::budget_import`.

use std::path::Path;
use std::process::ExitCode;

use quivotequoi::budget_import::{describe, import_budget, DRY_RUN_FLAG};

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dry_run = args.iter().any(|arg| arg == DRY_RUN_FLAG);
    let Some(manifest) = args.iter().find(|arg| arg.as_str() != DRY_RUN_FLAG) else {
        eprintln!("usage: import-budget <manifest.json> [{DRY_RUN_FLAG}]");
        return ExitCode::FAILURE;
    };

    match import_budget(Path::new(manifest), dry_run).await {
        Ok(outcome) => {
            for line in describe(&outcome) {
                println!("{line}");
            }
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("import refusé : {message}");
            ExitCode::FAILURE
        }
    }
}
