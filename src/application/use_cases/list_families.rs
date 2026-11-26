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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_every_family_of_the_closed_referential_with_its_published_scope() {
        let families = ListFamilies::new().execute();

        let codes: Vec<FamilyCode> = families.iter().map(|f| f.code).collect();
        assert_eq!(codes, FamilyCode::ALL.to_vec());
        for family in &families {
            assert_eq!(family.label, family.code.label());
            assert_eq!(family.scope, family.code.scope());
        }
    }
}
