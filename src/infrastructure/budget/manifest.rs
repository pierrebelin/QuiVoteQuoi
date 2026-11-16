//! Manifeste d'un exercice budgétaire : la loi, le fichier officiel et les
//! attendus tirés de la loi, versionnés à côté du fichier (`data/budget/`).
//!
//! Le manifeste est la seule porte d'entrée des références d'un exercice : ni
//! l'année, ni la loi, ni les attendus ne sont écrits dans le code. Les
//! attendus viennent de la loi (article de crédits, états B et D), jamais du
//! fichier lui-même : un fichier qui se tromperait de la même façon que ses
//! propres totaux passerait sinon tous les contrôles.

use std::path::{Component, Path, PathBuf};

use chrono::NaiveDate;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::domain::budget::{
    AccountKind, AdoptionProcedure, BudgetStage, BudgetYear, Euros, Expectations, LawReference,
    OffBudgetAccount, SourceReference,
};

/// Longueur d'une empreinte SHA-256, en caractères hexadécimaux.
const SHA256_HEX_LEN: usize = 64;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ManifestError {
    #[error("manifest unreadable at {path}: {reason}")]
    Unreadable { path: String, reason: String },
    #[error("manifest malformed: {0}")]
    Malformed(String),
    #[error("manifest field {field} invalid: {reason}")]
    Invalid { field: &'static str, reason: String },
    #[error("source file unreadable at {path}: {reason}")]
    SourceUnreadable { path: String, reason: String },
    #[error("source file {path} has sha256 {actual}, the manifest expects {expected}")]
    ChecksumMismatch {
        path: String,
        expected: String,
        actual: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub year: u16,
    pub stage: String,
    pub law: ManifestLaw,
    pub source: ManifestSource,
    /// Nomenclature des programmes du même exercice : elle seule liste les
    /// programmes dotés de 0 €, que le fichier des crédits omet.
    pub nomenclature: ManifestNomenclature,
    pub expected: ManifestExpected,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestNomenclature {
    /// Chemin du fichier officiel, relatif au dossier du manifeste.
    pub file: String,
    pub title: String,
    pub url: String,
    pub published_on: NaiveDate,
    pub sha256: String,
    /// Programmes de la nomenclature que l'état B de la loi ne cite pas
    /// (programme clos gardé par Chorus, comme le 369 en 2025) : l'import les
    /// écarte au lieu de les montrer à 0 €. Relevés dans la loi, jamais déduits
    /// des fichiers.
    #[serde(default)]
    pub absent_from_law: Vec<u16>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestLaw {
    pub number: String,
    pub date: NaiveDate,
    pub credits_article: u16,
    pub adoption_procedure: String,
    pub legifrance_url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestSource {
    /// Chemin du fichier officiel, relatif au dossier du manifeste.
    pub file: String,
    pub title: String,
    pub url: String,
    pub published_on: NaiveDate,
    pub downloaded_on: NaiveDate,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestExpected {
    pub general_budget_cp_euros: i64,
    pub missions: usize,
    pub programmes: usize,
    pub off_budget: Vec<ManifestOffBudget>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestOffBudget {
    pub mission: String,
    pub kind: String,
    pub programmes: usize,
    pub cp_euros: i64,
}

/// Le manifeste traduit en types du domaine, prêt à nourrir la commande d'import.
#[derive(Debug, Clone)]
pub struct ResolvedManifest {
    pub year: BudgetYear,
    pub stage: BudgetStage,
    pub law: LawReference,
    pub source: SourceReference,
    pub expectations: Expectations,
}

impl Manifest {
    pub fn parse(json: &str) -> Result<Self, ManifestError> {
        serde_json::from_str(json).map_err(|error| ManifestError::Malformed(error.to_string()))
    }

    pub fn read(path: &Path) -> Result<Self, ManifestError> {
        let json = std::fs::read_to_string(path).map_err(|error| ManifestError::Unreadable {
            path: path.display().to_string(),
            reason: error.to_string(),
        })?;
        Self::parse(&json)
    }

    /// Chemin du fichier officiel : `source.file` résolu depuis le dossier du
    /// manifeste. Un chemin absolu ou qui remonte (`..`) est refusé : le fichier
    /// vit à côté du manifeste, dans le dépôt.
    pub fn source_path(&self, manifest_path: &Path) -> Result<PathBuf, ManifestError> {
        beside(manifest_path, &self.source.file, "source.file")
    }

    /// Chemin du fichier officiel, après contrôle de son empreinte SHA-256
    /// contre celle du manifeste. Un fichier modifié, ou remplacé par un autre,
    /// est refusé avant toute lecture de ses lignes.
    pub fn verified_source_path(&self, manifest_path: &Path) -> Result<PathBuf, ManifestError> {
        let path = self.source_path(manifest_path)?;
        verify_checksum(path, &self.source.sha256)
    }

    /// Chemin de la nomenclature, après le même contrôle d'empreinte.
    pub fn verified_nomenclature_path(
        &self,
        manifest_path: &Path,
    ) -> Result<PathBuf, ManifestError> {
        let path = beside(manifest_path, &self.nomenclature.file, "nomenclature.file")?;
        verify_checksum(path, &self.nomenclature.sha256)
    }

    /// Traduit le manifeste en types du domaine, en refusant ce que le domaine
    /// refuse (stade inconnu, procédure inconnue, montant négatif...).
    pub fn resolve(&self) -> Result<ResolvedManifest, ManifestError> {
        let year = BudgetYear::new(self.year).map_err(|error| invalid("year", error))?;
        let stage = BudgetStage::parse(&self.stage).ok_or_else(|| ManifestError::Invalid {
            field: "stage",
            reason: format!("unknown stage `{}`, only `lfi` is published", self.stage),
        })?;
        let adoption_procedure = AdoptionProcedure::parse(&self.law.adoption_procedure)
            .ok_or_else(|| ManifestError::Invalid {
                field: "law.adoption_procedure",
                reason: format!("unknown procedure `{}`", self.law.adoption_procedure),
            })?;

        let off_budget = self
            .expected
            .off_budget
            .iter()
            .map(|account| {
                let kind =
                    AccountKind::parse(&account.kind).ok_or_else(|| ManifestError::Invalid {
                        field: "expected.off_budget.kind",
                        reason: format!("unknown account kind `{}`", account.kind),
                    })?;
                if kind == AccountKind::GeneralBudget {
                    return Err(ManifestError::Invalid {
                        field: "expected.off_budget.kind",
                        reason: format!(
                            "`{}` is the general budget, not an off-budget account",
                            account.mission
                        ),
                    });
                }
                Ok(OffBudgetAccount {
                    mission: account.mission.clone(),
                    kind,
                    programmes: account.programmes,
                    cp: Euros::new(account.cp_euros)
                        .map_err(|error| invalid("expected.off_budget.cp_euros", error))?,
                })
            })
            .collect::<Result<Vec<_>, ManifestError>>()?;

        Ok(ResolvedManifest {
            year,
            stage,
            law: LawReference {
                number: self.law.number.clone(),
                date: self.law.date,
                credits_article: self.law.credits_article,
                adoption_procedure,
                legifrance_url: self.law.legifrance_url.clone(),
            },
            source: SourceReference {
                title: self.source.title.clone(),
                url: self.source.url.clone(),
                published_on: self.source.published_on,
                downloaded_on: self.source.downloaded_on,
                sha256: normalized_sha256(&self.source.sha256)?,
            },
            expectations: Expectations {
                general_budget_cp: Euros::new(self.expected.general_budget_cp_euros)
                    .map_err(|error| invalid("expected.general_budget_cp_euros", error))?,
                missions: self.expected.missions,
                programmes: self.expected.programmes,
                off_budget,
            },
        })
    }
}

/// Empreinte SHA-256 d'un fichier, en hexadécimal minuscule. Sert aussi à écrire
/// le manifeste d'un nouvel exercice.
/// `file` résolu depuis le dossier du manifeste. Un chemin absolu ou qui
/// remonte (`..`) est refusé : le fichier vit à côté du manifeste, dans le dépôt.
fn beside(manifest_path: &Path, file: &str, field: &'static str) -> Result<PathBuf, ManifestError> {
    let relative = Path::new(file);
    let stays_inside = relative
        .components()
        .all(|component| matches!(component, Component::Normal(_)));
    if file.trim().is_empty() || !stays_inside {
        return Err(ManifestError::Invalid {
            field,
            reason: "must be a path relative to the manifest, without `..`".to_string(),
        });
    }
    let directory = manifest_path.parent().unwrap_or_else(|| Path::new(""));
    Ok(directory.join(relative))
}

/// Refuse un fichier dont l'empreinte diffère de celle du manifeste : un
/// fichier modifié, ou remplacé par un autre, l'est avant toute lecture.
fn verify_checksum(path: PathBuf, sha256: &str) -> Result<PathBuf, ManifestError> {
    let expected = normalized_sha256(sha256)?;
    let actual = file_sha256(&path)?;
    if actual != expected {
        return Err(ManifestError::ChecksumMismatch {
            path: path.display().to_string(),
            expected,
            actual,
        });
    }
    Ok(path)
}

pub fn file_sha256(path: &Path) -> Result<String, ManifestError> {
    let bytes = std::fs::read(path).map_err(|error| ManifestError::SourceUnreadable {
        path: path.display().to_string(),
        reason: error.to_string(),
    })?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn normalized_sha256(raw: &str) -> Result<String, ManifestError> {
    let hash = raw.trim().to_ascii_lowercase();
    if hash.len() != SHA256_HEX_LEN || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ManifestError::Invalid {
            field: "source.sha256",
            reason: format!("expected {SHA256_HEX_LEN} hexadecimal characters"),
        });
    }
    Ok(hash)
}

fn invalid(field: &'static str, error: impl std::fmt::Display) -> ManifestError {
    ManifestError::Invalid {
        field,
        reason: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SHA-256 de « abc », vecteur de test du standard.
    const ABC_SHA256: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    fn manifest_json(sha256: &str) -> String {
        format!(
            r#"{{
              "year": 2026, "stage": "lfi",
              "law": {{ "number": "2026-103", "date": "2026-02-19", "credits_article": 148,
                        "adoption_procedure": "article_49_3",
                        "legifrance_url": "https://www.legifrance.example/loi" }},
              "source": {{ "file": "lfi-2026-credits-ae-cp.xls",
                           "title": "Crédits AE et CP votés",
                           "url": "https://www.budget.example/fichier",
                           "published_on": "2026-03-03", "downloaded_on": "2026-11-16",
                           "sha256": "{sha256}" }},
              "nomenclature": {{ "file": "lfi-2026-nomenclature-destination.xls",
                                 "title": "Nomenclature par destination",
                                 "url": "https://www.budget.example/nomenclature",
                                 "published_on": "2026-03-03", "sha256": "{sha256}" }},
              "expected": {{ "general_budget_cp_euros": 593890071649, "missions": 32,
                             "programmes": 131,
                             "off_budget": [ {{ "mission": "Avances à l'audiovisuel public",
                                               "kind": "concours_financiers",
                                               "programmes": 7, "cp_euros": 4000000000 }} ] }}
            }}"#
        )
    }

    fn parsed() -> Manifest {
        Manifest::parse(&manifest_json(ABC_SHA256)).unwrap()
    }

    /// Fichier temporaire propre au test, supprimé en fin de test.
    struct TempFile(PathBuf);

    impl TempFile {
        fn with_content(name: &str, content: &[u8]) -> Self {
            let path = std::env::temp_dir()
                .join(format!("quivotequoi-budget-{}-{name}", std::process::id()));
            std::fs::write(&path, content).unwrap();
            Self(path)
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn resolves_the_manifest_into_domain_types() {
        let resolved = parsed().resolve().unwrap();

        assert_eq!(resolved.year.get(), 2026);
        assert_eq!(resolved.stage, BudgetStage::Lfi);
        assert_eq!(resolved.law.number, "2026-103");
        assert_eq!(resolved.law.credits_article, 148);
        assert_eq!(
            resolved.law.adoption_procedure,
            AdoptionProcedure::Article49_3
        );
        assert_eq!(resolved.source.sha256, ABC_SHA256);
        assert_eq!(
            resolved.expectations.general_budget_cp.get(),
            593_890_071_649
        );
        assert_eq!(resolved.expectations.missions, 32);
        assert_eq!(resolved.expectations.programmes, 131);
        assert_eq!(resolved.expectations.off_budget.len(), 1);
        let account = &resolved.expectations.off_budget[0];
        assert_eq!(account.mission, "Avances à l'audiovisuel public");
        assert_eq!(account.kind, AccountKind::FinancialAssistance);
        assert_eq!(account.programmes, 7);
        assert_eq!(account.cp.get(), 4_000_000_000);
    }

    #[test]
    fn a_field_the_manifest_does_not_know_is_refused() {
        let json =
            manifest_json(ABC_SHA256).replace("\"year\": 2026,", "\"year\": 2026, \"annee\": 1,");

        assert!(matches!(
            Manifest::parse(&json),
            Err(ManifestError::Malformed(_))
        ));
    }

    #[test]
    fn a_stage_other_than_lfi_is_refused() {
        let mut manifest = parsed();
        manifest.stage = "plf".to_string();

        assert!(matches!(
            manifest.resolve(),
            Err(ManifestError::Invalid { field: "stage", .. })
        ));
    }

    #[test]
    fn an_unknown_adoption_procedure_is_refused() {
        let mut manifest = parsed();
        manifest.law.adoption_procedure = "ordonnance".to_string();

        assert!(matches!(
            manifest.resolve(),
            Err(ManifestError::Invalid {
                field: "law.adoption_procedure",
                ..
            })
        ));
    }

    #[test]
    fn a_negative_expected_total_is_refused() {
        let mut manifest = parsed();
        manifest.expected.general_budget_cp_euros = -1;

        assert!(matches!(
            manifest.resolve(),
            Err(ManifestError::Invalid {
                field: "expected.general_budget_cp_euros",
                ..
            })
        ));
    }

    #[test]
    fn an_off_budget_account_of_the_general_budget_kind_is_refused() {
        let mut manifest = parsed();
        manifest.expected.off_budget[0].kind = "budget_general".to_string();

        assert!(matches!(
            manifest.resolve(),
            Err(ManifestError::Invalid {
                field: "expected.off_budget.kind",
                ..
            })
        ));
    }

    #[test]
    fn a_malformed_checksum_is_refused() {
        let manifest = Manifest::parse(&manifest_json("pas-une-empreinte")).unwrap();

        assert!(matches!(
            manifest.resolve(),
            Err(ManifestError::Invalid {
                field: "source.sha256",
                ..
            })
        ));
    }

    #[test]
    fn the_source_file_is_resolved_from_the_manifest_folder() {
        let path = parsed()
            .source_path(Path::new("data/budget/lfi-2026.json"))
            .unwrap();

        assert_eq!(path, Path::new("data/budget/lfi-2026-credits-ae-cp.xls"));
    }

    #[test]
    fn a_source_file_that_leaves_the_manifest_folder_is_refused() {
        for escaping in ["../autre.xls", "/etc/passwd", ""] {
            let mut manifest = parsed();
            manifest.source.file = escaping.to_string();

            assert!(
                matches!(
                    manifest.source_path(Path::new("data/budget/lfi-2026.json")),
                    Err(ManifestError::Invalid {
                        field: "source.file",
                        ..
                    })
                ),
                "chemin {escaping:?}"
            );
        }
    }

    #[test]
    fn computes_the_sha256_of_a_file() {
        let file = TempFile::with_content("sha-abc.bin", b"abc");

        assert_eq!(file_sha256(&file.0).unwrap(), ABC_SHA256);
    }

    #[test]
    fn accepts_a_source_file_whose_checksum_matches() {
        let file = TempFile::with_content("lfi-test.xls", b"abc");
        let mut manifest = parsed();
        manifest.source.file = file.0.file_name().unwrap().to_string_lossy().into_owned();
        let manifest_path = file.0.with_file_name("lfi-test.json");

        let verified = manifest.verified_source_path(&manifest_path).unwrap();

        assert_eq!(verified, file.0);
    }

    #[test]
    fn refuses_a_source_file_whose_checksum_differs() {
        let file = TempFile::with_content("lfi-modified.xls", b"abd");
        let mut manifest = parsed();
        manifest.source.file = file.0.file_name().unwrap().to_string_lossy().into_owned();
        let manifest_path = file.0.with_file_name("lfi-modified.json");

        let error = manifest.verified_source_path(&manifest_path).unwrap_err();

        assert!(matches!(
            error,
            ManifestError::ChecksumMismatch { expected, actual, .. }
                if expected == ABC_SHA256 && actual != ABC_SHA256
        ));
    }

    #[test]
    fn a_missing_source_file_is_reported_as_unreadable() {
        let mut manifest = parsed();
        manifest.source.file = "absent-quivotequoi-budget.xls".to_string();

        let error = manifest
            .verified_source_path(&std::env::temp_dir().join("lfi.json"))
            .unwrap_err();

        assert!(matches!(error, ManifestError::SourceUnreadable { .. }));
    }

    #[test]
    fn reads_a_manifest_from_a_file() {
        let file =
            TempFile::with_content("manifest-read.json", manifest_json(ABC_SHA256).as_bytes());

        let manifest = Manifest::read(&file.0).unwrap();

        assert_eq!(manifest.year, 2026);
        assert_eq!(manifest.expected.programmes, 131);
    }

    #[test]
    fn a_missing_manifest_is_reported_as_unreadable() {
        let error = Manifest::read(&std::env::temp_dir().join("absent-quivotequoi-manifest.json"))
            .unwrap_err();

        assert!(matches!(error, ManifestError::Unreadable { .. }));
    }
}
