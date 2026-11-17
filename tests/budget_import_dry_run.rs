//! Les exercices 2025 et 2026 versionnés (`data/budget/`) passent tous les
//! contrôles d'import : empreintes des deux fichiers officiels, total de
//! l'article de crédits, missions et programmes de l'état B, audiovisuel public
//! de l'état D, actions qui redonnent leur programme, titre conforme à la
//! catégorie. Sans base : `--dry-run`.

use std::path::Path;

use quivotequoi::application::use_cases::import_budget_exercise::ImportReport;
use quivotequoi::budget_import::import_budget;

/// CP du budget général par titre, en dizaines de millions d'euros arrondies
/// (273,17 Md€ = 27 317) : sommes du fichier officiel par `Code Titre`,
/// recalculées hors du code d'import.
fn titles_in_tens_of_millions(report: &ImportReport) -> Vec<(u8, i64)> {
    report
        .general_budget_nature
        .natures()
        .iter()
        .map(|share| {
            (
                share.nature.title(),
                (share.cp.get() + 5_000_000) / 10_000_000,
            )
        })
        .collect()
}

#[tokio::test]
async fn the_2026_finance_law_passes_every_import_check() {
    let outcome = import_budget(Path::new("data/budget/lfi-2026.json"), true)
        .await
        .expect("the 2026 exercise should pass its checks");

    let report = outcome.report;
    assert_eq!(report.general_budget_missions, 32);
    assert_eq!(report.general_budget_programmes, 131);
    assert_eq!(report.general_budget_total.get(), 593_890_071_649);
    assert_eq!(report.general_budget_actions, 603);
    assert_eq!(report.general_budget_nature.total().get(), 593_890_071_649);
    assert_eq!(report.off_budget.len(), 1);
    assert_eq!(report.off_budget[0].programmes, 7);
    assert_eq!(report.off_budget[0].total.get(), 3_863_312_945);
}

#[tokio::test]
async fn the_2026_titles_match_the_sums_of_the_official_file() {
    let outcome = import_budget(Path::new("data/budget/lfi-2026.json"), true)
        .await
        .expect("the 2026 exercise should pass its checks");

    assert_eq!(
        titles_in_tens_of_millions(&outcome.report),
        vec![
            (1, 114),
            (2, 16_041),
            (3, 7_012),
            (4, 5_928),
            (5, 2_917),
            (6, 27_317),
            (7, 60)
        ]
    );
}

#[tokio::test]
async fn the_2025_finance_law_passes_every_import_check() {
    let outcome = import_budget(Path::new("data/budget/lfi-2025.json"), true)
        .await
        .expect("the 2025 exercise should pass its checks");

    let report = outcome.report;
    assert_eq!(report.general_budget_missions, 34);
    assert_eq!(report.general_budget_programmes, 140);
    assert_eq!(report.general_budget_total.get(), 582_396_983_244);
    assert_eq!(report.general_budget_nature.total().get(), 582_396_983_244);
}
