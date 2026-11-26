//! Prompt, schéma et lecture de la réponse du classifieur thématique, communs
//! aux trois fournisseurs. Les adapters Anthropic, OpenAI et Mistral n'en font
//! que le transport.
//!
//! Le modele ne recoit que les libelles des textes (RM-04) et ne rend que des
//! familles et du texte de justification (RM-10): la reponse est contrainte par
//! un schema qui ne porte aucun nombre.
//!
//! Les libelles partent **par lot** (RM-14). Le cadrage — seize familles avec
//! leur perimetre, plus les regles de rattachement — pese bien plus lourd qu'un
//! libelle de loi; l'envoyer une fois par texte revient a payer le cadrage 322
//! fois. Un lot de vingt le paie seize fois en tout.

use serde::Deserialize;
use serde_json::{json, Value};

use crate::application::ports::theme_classifier::ClassifierError;
use crate::domain::theme::{FamilyCode, ProposedFamily, MAX_FAMILIES};

/// Version de l'instruction, conservee avec chaque proposition pour la rendre
/// inspectable. A incrementer des que le texte ci-dessous change.
pub(crate) const PROMPT_VERSION: &str = "thematisation-v3";

/// Libelles soumis en un appel.
///
/// Vingt: assez pour amortir le cadrage, assez peu pour qu'une reponse tienne
/// largement sous le plafond de jetons — une reponse tronquee perd le lot
/// entier, pas un seul texte.
pub(crate) const BATCH_SIZE: usize = 20;

/// Plafond de sortie, dimensionne sur le lot: chaque texte peut porter jusqu'a
/// trois familles justifiees.
const TOKENS_PER_LABEL: usize = 400;
const TOKENS_FLOOR: usize = 2_000;

/// Plafond de jetons de sortie d'un lot de `labels` libellés, commun aux trois
/// fournisseurs.
pub(crate) fn max_output_tokens(labels: usize) -> usize {
    TOKENS_FLOOR + TOKENS_PER_LABEL * labels
}

/// Libelles numerotes. Le numero sert a rendre la reponse reattribuable sans
/// dependre de l'ordre rendu par le modele.
pub(crate) fn numbered_labels(labels: &[String]) -> String {
    labels
        .iter()
        .enumerate()
        .map(|(i, label)| format!("{}. {label}", i + 1))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Referentiel ferme decrit au modele. Construit depuis le domaine: la page
/// methode et l'instruction ne peuvent pas diverger (RM-08).
pub(crate) fn system_prompt() -> String {
    let familles = FamilyCode::ALL
        .iter()
        .map(|f| format!("- `{}` — {} : {}", f.as_str(), f.label(), f.scope()))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "Tu rattaches des textes soumis au vote de l'Assemblée nationale à une ou plusieurs \
         familles thématiques.\n\n\
         Familles disponibles :\n{familles}\n\n\
         Règles :\n\
         - Tu reçois une liste numérotée de libellés de textes, et rien d'autre. Tu ne disposes \
         d'aucun résultat de vote, d'aucun décompte, d'aucun nom de groupe parlementaire, et tu \
         n'en demandes pas.\n\
         - Tu traites chaque libellé indépendamment des autres et tu rends une entrée par \
         libellé, en reprenant son numéro. N'omets aucun numéro et n'en invente aucun.\n\
         - Pour chaque texte tu retiens de une à {MAX_FAMILIES} familles, de la plus centrale à \
         la moins centrale. N'en retiens plusieurs que si le texte porte réellement sur \
         plusieurs de ces domaines.\n\
         - Tu rattaches sur le sujet du texte, jamais sur son orientation, ses effets supposés \
         ou sa valeur, ni sur le code qu'il modifie, ni sur le public qu'il vise.\n\
         - La justification dit ce que le libellé du texte porte, en une à deux phrases. \
         Aucun jugement, aucune évaluation, aucun chiffre, aucune estimation, aucun \
         qualificatif d'ampleur.\n\
         - Si aucune famille ne convient à un texte, rends pour lui une liste de familles vide \
         seulement si son libellé, pris littéralement, ne relève d'aucun périmètre du référentiel. \
         Une liste vide est exceptionnelle : tu renseignes alors obligatoirement `motif_absence` \
         avec `hors_referentiel` ou `libelle_insuffisant`.\n\
         - Ne rends pas une liste vide parce que le texte est juridique, transversal ou porte sur \
         plusieurs domaines : retiens alors la famille la plus centrale. Par exemple, une motion \
         de censure relève de `institutions-vie-politique` ; une loi constitutionnelle relève de \
         `institutions-vie-politique` et, s'il y en a un, du domaine qu'elle inscrit dans la \
         Constitution ; un texte sur le chômage ou le dialogue social relève du travail ; un \
         texte agricole relève de l'agriculture.\n\
         - N'invente aucune famille : seuls les codes listés ci-dessus sont acceptés."
    )
}

pub(crate) fn response_schema(expected: usize) -> Value {
    let codes: Vec<&str> = FamilyCode::ALL.iter().map(|f| f.as_str()).collect();
    let numbers: Vec<usize> = (1..=expected).collect();
    json!({
        "type": "object",
        "properties": {
            "textes": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "numero": { "type": "integer", "enum": numbers },
                        "familles": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "famille": { "type": "string", "enum": codes },
                                    "justification": { "type": "string" }
                                },
                                "required": ["famille", "justification"],
                                "additionalProperties": false
                            }
                        },
                        "motif_absence": {
                            "type": "string",
                            "enum": ["hors_referentiel", "libelle_insuffisant"]
                        }
                    },
                    "required": ["numero", "familles"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["textes"],
        "additionalProperties": false
    })
}

#[derive(Deserialize)]
struct Answer {
    textes: Vec<AnsweredText>,
}

#[derive(Deserialize)]
struct AnsweredText {
    numero: usize,
    familles: Vec<AnsweredFamily>,
    motif_absence: Option<String>,
}

#[derive(Deserialize)]
struct AnsweredFamily {
    famille: String,
    justification: String,
}

pub(crate) fn parse_text(
    text: &str,
    expected: usize,
) -> Result<Vec<Option<Vec<ProposedFamily>>>, ClassifierError> {
    let answer: Answer =
        serde_json::from_str(text).map_err(|e| ClassifierError::Answer(e.to_string()))?;

    let mut out: Vec<Option<Vec<ProposedFamily>>> = vec![None; expected];
    for entry in answer.textes {
        // Numerotation rendue au modele: 1..=expected.
        let Some(slot) = entry.numero.checked_sub(1).filter(|i| *i < expected) else {
            tracing::warn!(numero = entry.numero, "numéro hors du lot, réponse écartée");
            continue;
        };
        if out[slot].is_some() {
            tracing::warn!(
                numero = entry.numero,
                "numéro rendu deux fois, doublon écarté"
            );
            continue;
        }

        if entry.familles.is_empty() {
            match entry.motif_absence.as_deref() {
                Some("hors_referentiel" | "libelle_insuffisant") => {
                    out[slot] = Some(vec![]);
                }
                Some(reason) => {
                    tracing::warn!(%reason, "motif d'absence de famille invalide");
                }
                None => {
                    tracing::warn!("liste de familles vide sans motif, réponse à reprendre");
                }
            }
            continue;
        }

        if entry.motif_absence.is_some() {
            tracing::warn!("motif d'absence présent avec des familles, réponse à reprendre");
            continue;
        }

        let mut proposed = Vec::with_capacity(entry.familles.len());
        let mut invalid = false;
        for family in entry.familles {
            let code = match FamilyCode::parse(&family.famille) {
                Ok(code) => code,
                Err(_) => {
                    tracing::warn!(famille = family.famille, "famille hors référentiel écartée");
                    invalid = true;
                    continue;
                }
            };
            match ProposedFamily::new(code, family.justification) {
                Ok(entry) => proposed.push(entry),
                Err(error) => {
                    tracing::warn!(%error, "proposition écartée");
                    invalid = true;
                }
            }
        }
        if invalid {
            continue;
        }
        out[slot] = Some(proposed);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_prompt_lists_every_family_of_the_referential() {
        let prompt = system_prompt();
        for family in FamilyCode::ALL {
            assert!(prompt.contains(family.as_str()), "{}", family.as_str());
            assert!(prompt.contains(family.label()));
        }
    }

    #[test]
    fn the_prompt_attaches_on_the_subject_not_on_the_code_or_the_audience() {
        let prompt = system_prompt();
        assert!(prompt.contains("ni sur le code qu'il modifie"));
        assert!(prompt.contains("ni sur le public qu'il vise"));
    }

    #[test]
    fn the_prompt_makes_an_unassigned_answer_exceptional() {
        let prompt = system_prompt();
        assert!(prompt.contains("Une liste vide est exceptionnelle"));
        assert!(prompt.contains("motif_absence"));
        assert!(prompt.contains("motion de censure"));
    }

    #[test]
    fn the_schema_only_accepts_referential_codes() {
        let schema = response_schema(2);
        let codes = schema["properties"]["textes"]["items"]["properties"]["familles"]["items"]
            ["properties"]["famille"]["enum"]
            .as_array()
            .unwrap();
        assert_eq!(codes.len(), FamilyCode::ALL.len());
        assert!(codes.iter().any(|c| c == "immigration"));
    }

    #[test]
    fn the_schema_only_accepts_numbers_of_the_current_batch_without_array_bounds() {
        let schema = response_schema(3);
        let texts = &schema["properties"]["textes"];

        assert!(texts.get("minItems").is_none());
        assert!(texts.get("maxItems").is_none());
        assert_eq!(
            texts["items"]["properties"]["numero"]["enum"],
            json!([1, 2, 3])
        );
        assert_eq!(
            texts["items"]["properties"]["motif_absence"]["enum"],
            json!(["hors_referentiel", "libelle_insuffisant"])
        );
    }

    #[test]
    fn labels_are_numbered_from_one() {
        let numbered = numbered_labels(&["premier texte".into(), "second texte".into()]);
        assert_eq!(numbered, "1. premier texte\n2. second texte");
    }

    #[test]
    fn each_answer_goes_back_to_its_own_label() {
        let parsed = parse_text(
            r#"{"textes":[
                    {"numero":2,"familles":[{"famille":"logement","justification":"urbanisme"}]},
                    {"numero":1,"familles":[{"famille":"immigration","justification":"séjour"}]}
                ]}"#,
            2,
        )
        .unwrap();

        assert_eq!(
            parsed[0].as_ref().unwrap()[0].family(),
            FamilyCode::Immigration
        );
        assert_eq!(
            parsed[1].as_ref().unwrap()[0].family(),
            FamilyCode::Logement
        );
    }

    #[test]
    fn a_label_the_model_skipped_stays_retriable() {
        // `None`, pas `Some(vec![])`: le texte n'a pas eu de reponse, il n'a pas
        // ete juge sans famille.
        let parsed = parse_text(
            r#"{"textes":[{"numero":1,"familles":[{"famille":"logement","justification":"urbanisme"}]}]}"#,
            3,
        )
        .unwrap();
        assert!(parsed[0].is_some());
        assert!(parsed[1].is_none());
        assert!(parsed[2].is_none());
    }

    #[test]
    fn a_number_outside_the_batch_is_dropped_without_failing() {
        let parsed = parse_text(
            r#"{"textes":[
                    {"numero":7,"familles":[{"famille":"logement","justification":"urbanisme"}]},
                    {"numero":1,"familles":[{"famille":"numerique","justification":"données"}]}
                ]}"#,
            2,
        )
        .unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(
            parsed[0].as_ref().unwrap()[0].family(),
            FamilyCode::Numerique
        );
        assert!(parsed[1].is_none());
    }

    #[test]
    fn a_repeated_number_keeps_the_first_answer() {
        let parsed = parse_text(
            r#"{"textes":[
                    {"numero":1,"familles":[{"famille":"logement","justification":"urbanisme"}]},
                    {"numero":1,"familles":[{"famille":"numerique","justification":"données"}]}
                ]}"#,
            1,
        )
        .unwrap();
        assert_eq!(parsed[0].as_ref().unwrap().len(), 1);
        assert_eq!(
            parsed[0].as_ref().unwrap()[0].family(),
            FamilyCode::Logement
        );
    }

    #[test]
    fn an_invalid_family_leaves_the_whole_answer_retriable() {
        let parsed = parse_text(
            r#"{"textes":[{"numero":1,"familles":[
                    {"famille":"securite","justification":"hors référentiel"},
                    {"famille":"sante","justification":"soins palliatifs"}
                ]}]}"#,
            1,
        )
        .unwrap();
        assert!(parsed[0].is_none());
    }

    #[test]
    fn a_family_without_justification_leaves_the_answer_retriable() {
        let parsed = parse_text(
            r#"{"textes":[{"numero":1,"familles":[{"famille":"numerique","justification":"  "}]}]}"#,
            1,
        )
        .unwrap();
        assert!(parsed[0].is_none());
    }

    #[test]
    fn an_empty_family_list_with_an_explicit_reason_is_an_answer() {
        let parsed = parse_text(
            r#"{"textes":[{"numero":1,"familles":[],"motif_absence":"hors_referentiel"}]}"#,
            1,
        )
        .unwrap();
        assert_eq!(parsed[0].as_ref().unwrap().len(), 0);
    }

    #[test]
    fn an_empty_family_list_without_a_reason_is_retriable() {
        let parsed = parse_text(r#"{"textes":[{"numero":1,"familles":[]}]}"#, 1).unwrap();
        assert!(parsed[0].is_none());
    }
}
