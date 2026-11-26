use crate::domain::theme::FamilyCode;

/// Une famille et son perimetre publie, tels qu'affiches sur le site.
#[derive(Debug, Clone)]
pub struct FamilyDescription {
    pub code: FamilyCode,
    pub label: &'static str,
    pub scope: &'static str,
}

/// CU-06 — Referentiel ferme, servi tel quel: le site publie les familles et
/// leur perimetre (RM-08).
pub struct ListFamilies;

impl ListFamilies {
    pub fn new() -> Self {
        Self
    }

    pub fn execute(&self) -> Vec<FamilyDescription> {
        FamilyCode::ALL
            .into_iter()
            .map(|code| FamilyDescription {
                code,
                label: code.label(),
                scope: code.scope(),
            })
            .collect()
    }
}

impl Default for ListFamilies {
    fn default() -> Self {
        Self::new()
    }
}
