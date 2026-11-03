//! La contrainte de source du journal et les sources du domaine sont la meme liste.
//!
//! `ingestion_checks.source` porte une contrainte `CHECK (source IN (...))`.
//! Une source ajoutee au domaine et oubliee en migration ne casse rien a la
//! compilation: elle casse l'ecriture de la premiere ligne de journal qui la
//! porte, en production. Ce test fait echouer la divergence au plus tot.

use quivotequoi::domain::ingestion::IngestionSource;
use std::collections::BTreeSet;

/// Codes listes par la contrainte `CHECK (source IN (...))` de la table
/// `ingestion_checks`, migrations lues dans l'ordre.
///
/// Lecture volontairement litterale: dans le bloc `CREATE TABLE
/// ingestion_checks`, la ligne contenant `source IN` ouvre la liste, tout ce
/// qui est entre apostrophes est un code, jusqu'a la ligne qui contient `')`.
fn codes_held_by_check_constraint() -> BTreeSet<String> {
    let mut codes = BTreeSet::new();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("migrations lisibles")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .collect();
    files.sort();

    for file in files {
        let sql = std::fs::read_to_string(&file).expect("migration lisible");
        let mut in_table = false;
        let mut collecting = false;
        for line in sql.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("CREATE TABLE") && trimmed.contains("ingestion_checks") {
                in_table = true;
            }
            if !in_table {
                continue;
            }
            if trimmed.contains("source IN") {
                collecting = true;
            }
            if collecting {
                for code in trimmed.split('\'').skip(1).step_by(2) {
                    codes.insert(code.to_string());
                }
                if trimmed.contains("')") {
                    collecting = false;
                }
            }
            if trimmed.ends_with(");") {
                in_table = false;
            }
        }
    }
    codes
}

fn domain_codes() -> BTreeSet<String> {
    IngestionSource::ALL
        .iter()
        .map(|source| source.as_str().to_string())
        .collect()
}

#[test]
fn should_list_every_domain_source_in_check_constraint_when_migrations_are_read() {
    let held = codes_held_by_check_constraint();

    let missing: Vec<_> = domain_codes().difference(&held).cloned().collect();

    assert!(
        missing.is_empty(),
        "sources du domaine absentes de la contrainte `ingestion_checks`: {missing:?}"
    );
}

#[test]
fn should_list_no_unknown_source_in_check_constraint_when_migrations_are_read() {
    let held = codes_held_by_check_constraint();

    assert!(
        !held.is_empty(),
        "aucune contrainte `source IN` trouvée pour `ingestion_checks`"
    );
    let unknown: Vec<_> = held.difference(&domain_codes()).cloned().collect();
    assert!(
        unknown.is_empty(),
        "codes de la contrainte `ingestion_checks` inconnus du domaine: {unknown:?}"
    );
}
