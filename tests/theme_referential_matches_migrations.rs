//! Le catalogue en base et le referentiel du domaine sont le meme referentiel.
//!
//! `theme_assignments.family_code` porte une cle etrangere vers
//! `theme_families`. Une famille ajoutee au domaine et oubliee en migration ne
//! casse rien a la compilation: elle casse l'ecriture du premier rattachement
//! qui la porte, en production, pendant un rafraichissement. Ce test fait
//! echouer la divergence au plus tot.

use quivotequoi::domain::theme::FamilyCode;
use std::collections::BTreeSet;

/// Codes inseres dans `theme_families` par l'ensemble des migrations.
///
/// Lecture volontairement litterale: dans un bloc `INSERT INTO theme_families`,
/// toute ligne ouvrant un tuple porte le code en tete, et le bloc s'arrete au
/// point-virgule. Un `INSERT` ecrit autrement echapperait a ce test — c'est le
/// prix d'un test qui ne parse pas du SQL.
fn codes_inserted_by_migrations() -> BTreeSet<String> {
    let mut codes = BTreeSet::new();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("migrations lisibles")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .collect();
    files.sort();

    for file in files {
        let sql = std::fs::read_to_string(&file).expect("migration lisible");
        let mut inserting = false;
        for line in sql.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("INSERT INTO theme_families") {
                inserting = true;
                continue;
            }
            if !inserting {
                continue;
            }
            // Les valeurs s'etalent sur plusieurs lignes: seule celle qui ouvre
            // le tuple porte le code.
            if let Some(rest) = trimmed.strip_prefix("('") {
                if let Some(code) = rest.split('\'').next() {
                    codes.insert(code.to_string());
                }
            }
            if trimmed.ends_with(';') {
                inserting = false;
            }
        }
    }
    codes
}

#[test]
fn database_catalogue_holds_exactly_the_domain_families() {
    let expected: BTreeSet<String> = FamilyCode::ALL
        .iter()
        .map(|family| family.as_str().to_string())
        .collect();

    assert_eq!(
        codes_inserted_by_migrations(),
        expected,
        "le catalogue `theme_families` et `FamilyCode::ALL` ont divergé"
    );
}

#[test]
fn every_family_has_a_publishable_label_and_scope() {
    for family in FamilyCode::ALL {
        assert!(!family.label().trim().is_empty(), "{}", family.as_str());
        assert!(!family.scope().trim().is_empty(), "{}", family.as_str());
        assert_eq!(FamilyCode::parse(family.as_str()).unwrap(), family);
    }
}

#[test]
fn sensitive_families_state_their_attachment_rule() {
    // RM-11: sur ces familles, la justification porte sur l'objet du texte et
    // jamais sur son orientation. Le perimetre publie doit le dire, sinon la
    // regle ne vit que dans le code.
    for family in FamilyCode::ALL.iter().filter(|f| f.is_sensitive()) {
        assert!(
            family.scope().contains("jamais sur son orientation"),
            "le périmètre publié de « {} » ne porte pas la règle RM-11",
            family.label()
        );
    }
}
