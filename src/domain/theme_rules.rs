//! Rattachement par regle publiee, sans modele de langage (RM-13).
//!
//! Certains textes portent leur famille dans leur nature juridique: un projet
//! de loi de finances est un texte budgetaire, un projet de loi autorisant la
//! ratification d'un accord est un texte international. Les rattacher par
//! regle plutot que par modele coute zero jeton et se verifie a la lecture:
//! la table ci-dessous est publiee telle quelle sur la page methode.
//!
//! Trois garde-fous:
//! - la regle porte sur la **nature** du texte, jamais sur son orientation ni
//!   sur ses effets supposes (README.md §6);
//! - elle rend les memes familles a chaque passage, sur la meme cle normalisee
//!   (RM-02);
//! - son rattachement reste revisable par arbitrage humain comme n'importe
//!   quel autre (RM-07).
//!
//! Une regle qui ne s'applique pas laisse le texte au modele. Le silence est
//! le comportement par defaut: mieux vaut un appel de plus qu'un rattachement
//! force.

use chrono::NaiveDate;

use super::theme::{FamilyCode, ProposedFamily, SubjectRef, TextKey, ThemeAssignment, ThemeError};

/// Une regle du referentiel publie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemeRule {
    /// Nom court, affiche sur la page methode et conserve comme auteur du
    /// rattachement: l'historique dit quelle regle a decide.
    id: &'static str,
    /// Fragment cherche dans la cle normalisee. La cle etant deja mise en
    /// minuscules et ses apostrophes unifiees (RM-02), le fragment l'est aussi.
    marker: &'static str,
    /// Le marqueur doit former un mot entier: « enseignement » ne doit
    /// capter ni « renseignement » ni « tirer les enseignements ».
    whole_word: bool,
    families: &'static [FamilyCode],
    /// Enonce publie, conserve comme justification du rattachement (RM-05).
    statement: &'static str,
}

impl ThemeRule {
    pub fn id(&self) -> &'static str {
        self.id
    }

    pub fn marker(&self) -> &'static str {
        self.marker
    }

    pub fn whole_word(&self) -> bool {
        self.whole_word
    }

    /// La regle s'applique-t-elle a cette cle normalisee ?
    fn applies_to(&self, key: &str) -> bool {
        if !self.whole_word {
            return key.contains(self.marker);
        }
        key.match_indices(self.marker).any(|(start, _)| {
            let end = start + self.marker.len();
            let joined_before = key[..start]
                .chars()
                .next_back()
                .is_some_and(char::is_alphanumeric);
            let joined_after = key[end..].chars().next().is_some_and(char::is_alphanumeric);
            !joined_before && !joined_after
        })
    }

    pub fn families(&self) -> &'static [FamilyCode] {
        self.families
    }

    pub fn statement(&self) -> &'static str {
        self.statement
    }

    /// Auteur porte par les rattachements produits. Nomme la regle, pas un
    /// humain et pas un modele: l'historique ne doit pas mentir sur ce qui a
    /// ouvert la ligne (RM-07).
    pub fn author(&self) -> String {
        format!("règle « {} »", self.id)
    }

    /// Familles rendues sous la meme forme qu'une proposition de modele, pour
    /// que la suite du traitement soit identique.
    pub fn proposed_families(&self) -> Vec<ProposedFamily> {
        self.families
            .iter()
            .filter_map(|family| ProposedFamily::new(*family, self.statement.to_string()).ok())
            .collect()
    }

    pub fn assignments(
        &self,
        subject: &SubjectRef,
        opened_on: NaiveDate,
    ) -> Result<Vec<ThemeAssignment>, ThemeError> {
        self.families
            .iter()
            .map(|family| {
                ThemeAssignment::open(
                    subject.clone(),
                    *family,
                    opened_on,
                    self.author(),
                    Some(self.statement.to_string()),
                )
            })
            .collect()
    }
}

/// Table publiee, ordonnee du plus specifique au plus general: la premiere
/// regle qui s'applique decide, les suivantes ne sont pas essayees.
///
/// « projet de loi de financement » et « projet de loi de finances » ne se
/// chevauchent pas — « financement » ne contient pas « finances » — mais
/// l'ordre reste celui de la specificite pour que l'ajout d'une regle ne
/// depende pas de cette coincidence.
pub const RULES: [ThemeRule; 16] = [
    ThemeRule {
        id: "financement de la sécurité sociale",
        marker: "financement de la sécurité sociale",
        whole_word: false,
        families: &[FamilyCode::SanteSocial, FamilyCode::PouvoirAchatFiscalite],
        statement: "Un projet de loi de financement de la sécurité sociale fixe les recettes et \
                    les dépenses des régimes de sécurité sociale pour l'année.",
    },
    ThemeRule {
        id: "comptes de la sécurité sociale",
        marker: "approbation des comptes de la sécurité sociale",
        whole_word: false,
        families: &[FamilyCode::SanteSocial, FamilyCode::InstitutionsProcedure],
        statement: "Un projet de loi d'approbation des comptes de la sécurité sociale arrête les \
                    comptes de l'exercice écoulé des régimes de sécurité sociale.",
    },
    ThemeRule {
        id: "loi de finances",
        marker: "projet de loi de finances",
        whole_word: false,
        families: &[
            FamilyCode::PouvoirAchatFiscalite,
            FamilyCode::InstitutionsProcedure,
        ],
        statement: "Un projet de loi de finances fixe les recettes et les charges de l'État pour \
                    l'année, et porte les dispositions fiscales qui s'y rattachent.",
    },
    ThemeRule {
        id: "règlement du budget",
        marker: "règlement du budget",
        whole_word: false,
        families: &[
            FamilyCode::PouvoirAchatFiscalite,
            FamilyCode::InstitutionsProcedure,
        ],
        statement: "Un projet de loi de règlement du budget arrête les comptes de l'État pour \
                    l'exercice écoulé.",
    },
    ThemeRule {
        id: "programmation des finances publiques",
        marker: "programmation des finances publiques",
        whole_word: false,
        families: &[
            FamilyCode::PouvoirAchatFiscalite,
            FamilyCode::InstitutionsProcedure,
        ],
        statement: "Une loi de programmation des finances publiques fixe la trajectoire \
                    pluriannuelle des recettes et des dépenses publiques.",
    },
    ThemeRule {
        id: "ratification d'un engagement international",
        marker: "autorisant la ratification",
        whole_word: false,
        families: &[FamilyCode::InternationalDefense],
        statement: "Un projet de loi autorisant la ratification porte sur un traité ou un accord \
                    conclu avec un État tiers ou une organisation internationale.",
    },
    ThemeRule {
        id: "approbation d'un engagement international",
        marker: "autorisant l'approbation",
        whole_word: false,
        families: &[FamilyCode::InternationalDefense],
        statement: "Un projet de loi autorisant l'approbation porte sur un accord ou une \
                    convention conclus avec un État tiers ou une organisation internationale.",
    },
    ThemeRule {
        id: "programmation militaire",
        marker: "programmation militaire",
        whole_word: false,
        families: &[FamilyCode::InternationalDefense],
        statement: "Une loi de programmation militaire fixe la trajectoire pluriannuelle des \
                    moyens des armées.",
    },
    ThemeRule {
        id: "règlement de l'Assemblée",
        marker: "règlement de l'assemblée nationale",
        whole_word: false,
        families: &[FamilyCode::InstitutionsProcedure],
        statement: "Une proposition de résolution modifiant le règlement de l'Assemblée nationale \
                    porte sur la procédure interne de l'Assemblée.",
    },
    ThemeRule {
        id: "motion de censure",
        marker: "motion de censure",
        whole_word: false,
        families: &[FamilyCode::InstitutionsProcedure],
        statement: "Une motion de censure met en jeu la responsabilité du Gouvernement devant \
                    l'Assemblée nationale, selon une procédure constitutionnelle.",
    },
    ThemeRule {
        id: "assurance chômage",
        marker: "assurance chômage",
        whole_word: false,
        families: &[FamilyCode::TravailEmploi],
        statement: "Un texte relatif à l'assurance chômage porte sur l'indemnisation et \
                    l'accompagnement des personnes privées d'emploi.",
    },
    ThemeRule {
        id: "dialogue social",
        marker: "dialogue social",
        whole_word: false,
        families: &[FamilyCode::TravailEmploi],
        statement: "Un texte relatif au dialogue social porte sur les relations collectives de \
                    travail entre salariés, employeurs et leurs représentants.",
    },
    ThemeRule {
        id: "enseignement",
        marker: "enseignement",
        whole_word: true,
        families: &[FamilyCode::EducationCulture],
        statement: "Un texte relatif à l'enseignement porte sur l'organisation ou les personnels \
                    du système éducatif.",
    },
    ThemeRule {
        id: "jeux olympiques et paralympiques",
        marker: "jeux olympiques et paralympiques",
        whole_word: false,
        families: &[FamilyCode::EducationCulture],
        statement: "Un texte relatif aux jeux Olympiques et Paralympiques porte sur le sport et \
                    son organisation.",
    },
    ThemeRule {
        id: "souveraineté agricole",
        marker: "souveraineté agricole",
        whole_word: false,
        families: &[FamilyCode::AgricultureAlimentation],
        statement: "Un texte relatif à la protection ou à la souveraineté agricoles porte sur \
                    l'agriculture et l'alimentation.",
    },
    ThemeRule {
        id: "fraudes sociales et fiscales",
        marker: "fraudes sociales et fiscales",
        whole_word: false,
        families: &[FamilyCode::SanteSocial, FamilyCode::PouvoirAchatFiscalite],
        statement: "Un texte relatif aux fraudes sociales et fiscales porte à la fois sur les \
                    prestations sociales et sur les obligations fiscales.",
    },
];

/// Regle applicable a une cle de texte, s'il y en a une.
pub fn rule_for(key: &TextKey) -> Option<&'static ThemeRule> {
    RULES.iter().find(|rule| rule.applies_to(key.as_str()))
}

/// Regle applicable a un libelle libre — titre de dossier, notamment. Le
/// libelle passe par la meme normalisation que la cle d'un texte pour que la
/// table se comporte pareil des deux cotes.
pub fn rule_for_label(label: &str) -> Option<&'static ThemeRule> {
    rule_for(&TextKey::from_raw(label))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(raw: &str) -> TextKey {
        TextKey::from_raw(raw)
    }

    #[test]
    fn a_finance_bill_is_ruled_without_a_model() {
        let rule = rule_for(&key("projet de loi de finances pour 2026")).unwrap();
        assert_eq!(rule.id(), "loi de finances");
        assert_eq!(
            rule.families(),
            [
                FamilyCode::PouvoirAchatFiscalite,
                FamilyCode::InstitutionsProcedure
            ]
        );
    }

    #[test]
    fn a_rectifying_finance_bill_falls_under_the_same_rule() {
        for label in [
            "projet de loi de finances rectificative pour 2026",
            "projet de loi de finances de fin de gestion pour 2026",
        ] {
            assert_eq!(rule_for(&key(label)).unwrap().id(), "loi de finances");
        }
    }

    #[test]
    fn social_security_funding_is_not_caught_by_the_finance_rule() {
        // « financement » ne contient pas « finances »: les deux textes les plus
        // votes de la legislature ne doivent pas se confondre.
        let rule = rule_for(&key(
            "projet de loi de financement de la sécurité sociale pour 2026",
        ))
        .unwrap();
        assert_eq!(rule.id(), "financement de la sécurité sociale");
        assert_eq!(
            rule.families(),
            [FamilyCode::SanteSocial, FamilyCode::PouvoirAchatFiscalite]
        );
    }

    #[test]
    fn a_treaty_bill_goes_to_international() {
        for label in [
            "projet de loi autorisant la ratification du traité d'amitié franco-allemand",
            "projet de loi autorisant l'approbation de l'accord entre la France et le Canada",
        ] {
            assert_eq!(
                rule_for(&key(label)).unwrap().families(),
                [FamilyCode::InternationalDefense]
            );
        }
    }

    #[test]
    fn unambiguous_titles_from_the_unassigned_queue_are_ruled() {
        let cases = [
            (
                "motion de censure déposée en application de l'article 49, alinéa 3",
                &[FamilyCode::InstitutionsProcedure][..],
            ),
            (
                "projet de loi relatif à l'enseignement supérieur et à la recherche",
                &[FamilyCode::EducationCulture][..],
            ),
            (
                "projet de loi d'urgence pour la protection et la souveraineté agricoles",
                &[FamilyCode::AgricultureAlimentation][..],
            ),
            (
                "projet de loi portant transposition d'un accord relatif à l'assurance chômage",
                &[FamilyCode::TravailEmploi][..],
            ),
            (
                "projet de loi relatif aux jeux olympiques et paralympiques de 2030",
                &[FamilyCode::EducationCulture][..],
            ),
            (
                "projet de loi relatif à la lutte contre les fraudes sociales et fiscales",
                &[FamilyCode::SanteSocial, FamilyCode::PouvoirAchatFiscalite][..],
            ),
        ];

        for (label, expected) in cases {
            assert_eq!(
                rule_for_label(label).unwrap().families(),
                expected,
                "{label}"
            );
        }
    }

    #[test]
    fn a_whole_word_marker_ignores_longer_words() {
        for label in [
            "projet de loi relatif au renseignement",
            "proposition de loi tirant les enseignements de la crise sanitaire",
        ] {
            assert!(rule_for_label(label).is_none(), "{label}");
        }
    }

    #[test]
    fn a_constitutional_bill_is_left_to_the_model() {
        // Une revision constitutionnelle peut porter sur n'importe quel
        // domaine: seule sa lecture dit lequel.
        assert!(rule_for_label(
            "projet de loi constitutionnelle relatif à la liberté de recourir à l'interruption \
             volontaire de grossesse"
        )
        .is_none());
    }

    #[test]
    fn the_typographic_apostrophe_does_not_defeat_a_rule() {
        // Meme normalisation que la cle (RM-02): l'apostrophe courbe de la
        // source ne doit pas faire manquer la regle.
        assert!(rule_for_label(
            "proposition de résolution tendant à modifier le règlement de l’Assemblée nationale"
        )
        .is_some());
    }

    #[test]
    fn an_ordinary_bill_is_left_to_the_model() {
        assert!(rule_for(&key(
            "proposition de loi relative au droit à l'aide à mourir"
        ))
        .is_none());
        assert!(rule_for(&key(
            "projet de loi de simplification du droit de l'urbanisme et du logement"
        ))
        .is_none());
    }

    #[test]
    fn a_rule_names_itself_in_the_assignments_it_opens() {
        let rule = rule_for(&key("projet de loi de finances pour 2026")).unwrap();
        let subject = SubjectRef::Text(key("projet de loi de finances pour 2026"));
        let opened_on = NaiveDate::from_ymd_opt(2026, 8, 9).unwrap();
        let assignments = rule.assignments(&subject, opened_on).unwrap();

        assert_eq!(assignments.len(), 2);
        for assignment in &assignments {
            assert_eq!(assignment.author(), "règle « loi de finances »");
            assert_eq!(assignment.motive(), Some(rule.statement()));
            assert!(assignment.is_current());
        }
    }

    #[test]
    fn every_rule_is_published_whole() {
        for rule in RULES {
            assert!(!rule.id().is_empty());
            assert!(!rule.statement().is_empty());
            assert!(!rule.families().is_empty());
            assert!(rule.families().len() <= super::super::theme::MAX_FAMILIES);
            // La regle se cherche dans une cle normalisee: un marqueur qui ne
            // l'est pas ne s'appliquerait jamais.
            assert_eq!(rule.marker(), TextKey::from_raw(rule.marker()).as_str());
            assert_eq!(rule.proposed_families().len(), rule.families().len());
        }
    }

    #[test]
    fn no_rule_shadows_another() {
        // Une regle dont le marqueur contient celui d'une regle qui la precede
        // ne serait jamais atteinte.
        for (i, rule) in RULES.iter().enumerate() {
            for earlier in &RULES[..i] {
                assert!(
                    !rule.marker().contains(earlier.marker()),
                    "« {} » est masquée par « {} »",
                    rule.id(),
                    earlier.id()
                );
            }
        }
    }
}
