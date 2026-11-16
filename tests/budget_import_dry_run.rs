//! L'exercice 2026 versionné (`data/budget/`) passe tous les contrôles d'import :
//! empreintes des deux fichiers officiels, total de l'article 148, 32 missions,
//! 131 programmes, audiovisuel public de l'état D. Sans base : `--dry-run`.

use std::path::Path;

use quivotequoi::budget_import::import_budget;

#[tokio::test]
async fn the_2026_finance_law_passes_every_import_check() {
    let outcome = import_budget(Path::new("data/budget/lfi-2026.json"), true)
        .await
        .expect("the 2026 exercise should pass its checks");

    let report = outcome.report;
    assert_eq!(report.general_budget_missions, 32);
    assert_eq!(report.general_budget_programmes, 131);
    assert_eq!(report.general_budget_total.get(), 593_890_071_649);
    assert_eq!(report.off_budget.len(), 1);
    assert_eq!(report.off_budget[0].programmes, 7);
    assert_eq!(report.off_budget[0].total.get(), 3_863_312_945);
}
