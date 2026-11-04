//! Prompt, schéma et lecture de la réponse du modèle « En bref », communs aux
//! trois fournisseurs (SPEC-en-bref RM-09, README §6 et §8.1).
//!
//! Les adaptateurs Anthropic, OpenAI et Mistral n'en font que le transport.

use serde::Deserialize;
use serde_json::{json, Value};

use crate::application::ports::brief_generator::{
    BriefRequest, GeneratedLine, SummaryGeneratorError,
};
use crate::domain::cited_article::CitedWording;

/// Version du prompt, enregistrée avec chaque ligne (RM-16, RM-17).
pub const PROMPT_VERSION: &str = "text-brief-v2";

/// Plafond de jetons de sortie d'un appel, commun aux trois fournisseurs : un
/// paquet de 40 articles rend 40 clés (une empreinte) et 40 lignes.
pub(crate) const BRIEF_MAX_OUTPUT_TOKENS: u32 = 8_000;

/// Ce que dit le modèle d'un article cité que le texte crée (RM-07).
const CREATED_BY_THE_TEXT: &str = "article créé par le texte";

pub(crate) fn system_prompt() -> &'static str {
    concat!(
        "Tu résumes les articles d'un texte de loi français, un par un, pour des lecteurs qui ne connaissent pas le droit.\n",
        "\n",
        "Tu reçois un objet JSON {\"articles\": [...]}. Chaque article porte : \"key\" (un identifiant à recopier tel quel), \"content\" (le texte de l'article) et \"cited_articles\" (les articles de code ou de loi que l'article modifie, avec leur rédaction en vigueur dans \"text_in_force\", ou la mention \"article créé par le texte\" dans \"status\").\n",
        "\n",
        "Rends exactement une ligne par article reçu, au format JSON {\"lines\": [{\"key\": \"...\", \"line\": \"...\"}]}, avec la clé recopiée à l'identique.\n",
        "\n",
        "Règles de chaque ligne :\n",
        "- Français courant, sans jargon juridique, en une seule phrase de 25 mots au plus.\n",
        "- Dis ce que l'article change. Quand il modifie une règle existante, dis la règle avant et la règle après, en t'appuyant sur la rédaction en vigueur fournie.\n",
        "- Un article de pure forme se dit comme tel (par exemple : date d'entrée en vigueur).\n",
        "- Ne reprends pas le numéro de l'article.\n",
        "- Aucune intention (pas de \"vise à\", \"a pour objectif\", \"a pour but\", \"afin de\") et aucun jugement (pas de \"important\", \"significatif\", \"historique\", \"ambitieux\", \"efficace\", \"injuste\").\n",
        "- N'écris aucun nombre qui ne soit recopié des sources (le contenu de l'article et les rédactions fournies), avec le même mot qui le qualifie (unité, mois, jours...). Aucun calcul, aucun arrondi, aucune conversion : \"trois mois\" reste \"trois mois\".\n",
        "- Recopie chaque nombre sous la forme des sources : \"2027\" reste \"2027\", \"60 milliards\" reste \"60 milliards\". N'écris jamais en lettres un nombre écrit en chiffres.\n",
        "- Si tout ne tient pas en 25 mots, cite moins de montants plutôt que d'allonger la phrase.\n",
        "- Ne nomme aucune taxe, aucun dispositif ni aucun organisme que les sources ne nomment pas.\n",
        "- N'utilise que les sources fournies : aucune connaissance extérieure, aucun contexte politique, aucune supposition. Quand une rédaction n'est pas fournie, ne la décris pas.",
    )
}

pub(crate) fn response_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "lines": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "key": { "type": "string" },
                        "line": { "type": "string" }
                    },
                    "required": ["key", "line"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["lines"],
        "additionalProperties": false
    })
}

/// L'entrée du modèle : pour chaque article, sa clé, son contenu et ses articles
/// cités avec leur rédaction en vigueur, ou la mention qu'ils sont créés par le
/// texte (RM-09). Ni intitulé, ni exposé des motifs, ni numéro d'article.
pub(crate) fn request_input(request: &BriefRequest) -> String {
    let articles: Vec<Value> = request
        .articles
        .iter()
        .map(|article| {
            let cited: Vec<Value> = article
                .cited
                .iter()
                .filter_map(|citation| match &citation.wording {
                    CitedWording::InForce { text, .. } => Some(json!({
                        "article": citation.label,
                        "text_in_force": text,
                    })),
                    CitedWording::Created => Some(json!({
                        "article": citation.label,
                        "status": CREATED_BY_THE_TEXT,
                    })),
                    CitedWording::NotFound => None,
                })
                .collect();
            json!({
                "key": article.key,
                "content": article.content,
                "cited_articles": cited,
            })
        })
        .collect();
    json!({ "articles": articles }).to_string()
}

#[derive(Debug, Deserialize)]
struct Answer {
    lines: Vec<AnswerLine>,
}

#[derive(Debug, Deserialize)]
struct AnswerLine {
    key: String,
    line: String,
}

/// Lit la réponse du modèle. Un JSON qui ne suit pas le schéma rejette toute la
/// réponse : les articles seront repris par la passe suivante. Les clés
/// inconnues ou répétées ne sont pas écartées ici, c'est l'affaire de la passe.
pub(crate) fn parse_lines(text: &str) -> Result<Vec<GeneratedLine>, SummaryGeneratorError> {
    let answer: Answer = serde_json::from_str(text)
        .map_err(|error| SummaryGeneratorError::Answer(format!("json invalide: {error}")))?;
    Ok(answer
        .lines
        .into_iter()
        .map(|entry| GeneratedLine {
            key: entry.key,
            line: entry.line,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::brief_generator::{BriefArticle, BriefCitation};

    fn article(key: &str, content: &str, cited: Vec<BriefCitation>) -> BriefArticle {
        BriefArticle {
            key: key.to_string(),
            content: content.to_string(),
            cited,
        }
    }

    fn input_of(articles: Vec<BriefArticle>) -> Value {
        let text = request_input(&BriefRequest { articles });
        serde_json::from_str(&text).unwrap()
    }

    #[test]
    fn should_give_each_article_its_key_content_and_cited_wordings_when_input_is_built() {
        let input = input_of(vec![
            article(
                "sha256:aaa",
                "Le II de l'article L. 1110-5 est abrogé.",
                vec![BriefCitation {
                    label: "code de la santé publique, article L. 1110-5".to_string(),
                    wording: CitedWording::InForce {
                        text: "Toute personne a le droit d'être informée.".to_string(),
                        official_url: "https://www.legifrance.gouv.fr/x".to_string(),
                    },
                }],
            ),
            article(
                "sha256:bbb",
                "La présente loi entre en vigueur le 1er janvier 2027.",
                vec![],
            ),
        ]);

        let articles = input["articles"].as_array().unwrap();
        assert_eq!(articles.len(), 2);
        assert_eq!(articles[0]["key"], "sha256:aaa");
        assert_eq!(
            articles[0]["content"],
            "Le II de l'article L. 1110-5 est abrogé."
        );
        let cited = articles[0]["cited_articles"].as_array().unwrap();
        assert_eq!(cited.len(), 1);
        assert_eq!(
            cited[0]["article"],
            "code de la santé publique, article L. 1110-5"
        );
        assert_eq!(
            cited[0]["text_in_force"],
            "Toute personne a le droit d'être informée."
        );
        assert_eq!(articles[1]["key"], "sha256:bbb");
        assert_eq!(articles[1]["cited_articles"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn should_send_only_the_sources_of_each_article_when_input_is_built() {
        let input = input_of(vec![article("sha256:aaa", "Contenu.", vec![])]);

        let mut top: Vec<&String> = input.as_object().unwrap().keys().collect();
        top.sort();
        assert_eq!(top, vec!["articles"]);
        let mut fields: Vec<&String> = input["articles"][0].as_object().unwrap().keys().collect();
        fields.sort();
        assert_eq!(fields, vec!["cited_articles", "content", "key"]);
    }

    #[test]
    fn should_mark_created_article_when_citation_has_no_wording() {
        let input = input_of(vec![article(
            "sha256:aaa",
            "Après l'article L. 114-1, il est inséré un article L. 114-1-1.",
            vec![BriefCitation {
                label: "code de la sécurité sociale, article L. 114-1-1".to_string(),
                wording: CitedWording::Created,
            }],
        )]);

        let cited = &input["articles"][0]["cited_articles"][0];
        assert_eq!(cited["status"], "article créé par le texte");
        assert!(cited.get("text_in_force").is_none());
    }

    #[test]
    fn should_leave_out_citation_when_the_source_does_not_know_the_article() {
        let input = input_of(vec![article(
            "sha256:aaa",
            "Contenu.",
            vec![BriefCitation {
                label: "code de l'énergie, article L. 1".to_string(),
                wording: CitedWording::NotFound,
            }],
        )]);

        assert_eq!(
            input["articles"][0]["cited_articles"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
    }

    #[test]
    fn should_read_one_line_per_key_when_answer_matches_schema() {
        let text = r#"{"lines":[
            {"key":"sha256:aaa","line":"Supprime le droit à une information écrite."},
            {"key":"sha256:bbb","line":"Fixe la date d'entrée en vigueur."}
        ]}"#;

        let lines = parse_lines(text).unwrap();

        assert_eq!(
            lines,
            vec![
                GeneratedLine {
                    key: "sha256:aaa".to_string(),
                    line: "Supprime le droit à une information écrite.".to_string(),
                },
                GeneratedLine {
                    key: "sha256:bbb".to_string(),
                    line: "Fixe la date d'entrée en vigueur.".to_string(),
                },
            ]
        );
    }

    #[test]
    fn should_reject_answer_when_json_does_not_parse() {
        let error = parse_lines("{\"lines\":[{\"key\":\"sha256:aaa\"").unwrap_err();

        assert!(matches!(error, SummaryGeneratorError::Answer(_)));
    }

    #[test]
    fn should_reject_answer_when_a_line_has_no_text() {
        let error = parse_lines(r#"{"lines":[{"key":"sha256:aaa"}]}"#).unwrap_err();

        assert!(matches!(error, SummaryGeneratorError::Answer(_)));
    }

    #[test]
    fn should_keep_the_rules_of_a_line_in_the_system_prompt() {
        let prompt = system_prompt().to_lowercase();

        for rule in [
            "25 mots",
            "avant",
            "pure forme",
            "numéro de l'article",
            "vise à",
            "afin de",
            "aucun calcul",
            "aucune conversion",
            "sous la forme des sources",
            "cite moins de montants",
            "ne nomme aucune taxe",
        ] {
            assert!(prompt.contains(rule), "missing rule {rule:?}");
        }
    }

    #[test]
    fn should_change_the_prompt_version_with_the_rules_on_numbers_and_names() {
        assert_eq!(PROMPT_VERSION, "text-brief-v2");
    }

    #[test]
    fn should_ask_for_an_object_of_lines_with_key_and_line_when_schema_is_built() {
        let schema = response_schema();

        let item = &schema["properties"]["lines"]["items"];
        assert_eq!(item["required"], json!(["key", "line"]));
        assert_eq!(schema["required"], json!(["lines"]));
        assert_eq!(schema["additionalProperties"], json!(false));
    }
}
