//! Client de l'API Légifrance (PISTE) : rédaction en vigueur d'un article de
//! code ou de loi à une date (SPEC-en-bref RM-07, T3).
//!
//! Deux requêtes par article cité : `/search` donne l'identifiant `LEGIARTI…`
//! de l'article à la date, `/consult/getArticle` son texte. Le jeton OAuth
//! `client_credentials` dure une heure ; il est renouvelé avant son échéance.

use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::NaiveDate;
use reqwest::StatusCode;
use serde_json::{json, Value};

use crate::application::ports::legal_text_source::{LegalTextSource, SourceError};
use crate::domain::cited_article::{CitedArticle, CitedText, CitedWording};
use crate::infrastructure::national_assembly::html_text::normalize_source_text;

pub const OAUTH_URL: &str = "https://oauth.piste.gouv.fr/api/oauth/token";
pub const API_BASE: &str = "https://api.piste.gouv.fr/dila/legifrance/lf-engine-app";
pub const SANDBOX_OAUTH_URL: &str = "https://sandbox-oauth.piste.gouv.fr/api/oauth/token";
pub const SANDBOX_API_BASE: &str =
    "https://sandbox-api.piste.gouv.fr/dila/legifrance/lf-engine-app";

/// Borne de temps d'une requête, jeton compris.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

const SEARCH_PATH: &str = "/search";
const ARTICLE_PATH: &str = "/consult/getArticle";
const SEARCH_PAGE_SIZE: u32 = 10;

/// Préfixe de l'identifiant d'une version d'article chez Légifrance.
const ARTICLE_ID_PREFIX: &str = "LEGIARTI";

/// Avance sur l'échéance à laquelle le jeton est renouvelé.
const TOKEN_RENEWAL_MARGIN: Duration = Duration::from_secs(60);

/// Durée retenue d'un jeton dont la réponse n'indique pas l'échéance.
const FALLBACK_TOKEN_LIFETIME: Duration = Duration::from_secs(300);

/// Nombre de caractères du corps d'une erreur HTTP repris dans le message.
const ERROR_DETAIL_MAX_CHARS: usize = 300;

const CODE_URL_PREFIX: &str = "https://www.legifrance.gouv.fr/codes/article_lc/";
const LAW_URL_PREFIX: &str = "https://www.legifrance.gouv.fr/loda/article_lc/";

pub struct LegifranceClient {
    http: reqwest::Client,
    client_id: String,
    client_secret: String,
    oauth_url: &'static str,
    api_base: &'static str,
    token: tokio::sync::Mutex<Option<AccessToken>>,
}

struct AccessToken {
    value: String,
    renew_at: Instant,
}

impl LegifranceClient {
    /// `None` sans `LEGIFRANCE_CLIENT_ID` ou `LEGIFRANCE_CLIENT_SECRET`.
    /// `LEGIFRANCE_SANDBOX=true` vise l'environnement d'essai de PISTE.
    pub fn from_env() -> Option<Self> {
        let client_id = std::env::var("LEGIFRANCE_CLIENT_ID")
            .ok()
            .filter(|value| !value.trim().is_empty())?;
        let client_secret = std::env::var("LEGIFRANCE_CLIENT_SECRET")
            .ok()
            .filter(|value| !value.trim().is_empty())?;
        let sandbox = is_sandbox(std::env::var("LEGIFRANCE_SANDBOX").ok().as_deref());
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .ok()?;
        Some(Self::new(http, client_id, client_secret, sandbox))
    }

    fn new(http: reqwest::Client, client_id: String, client_secret: String, sandbox: bool) -> Self {
        let (oauth_url, api_base) = if sandbox {
            (SANDBOX_OAUTH_URL, SANDBOX_API_BASE)
        } else {
            (OAUTH_URL, API_BASE)
        };
        Self {
            http,
            client_id,
            client_secret,
            oauth_url,
            api_base,
            token: tokio::sync::Mutex::new(None),
        }
    }

    /// Le jeton en cours, ou un nouveau s'il est échu ou si `renew` l'exige.
    async fn bearer(&self, renew: bool) -> Result<String, SourceError> {
        let mut cached = self.token.lock().await;
        if !renew {
            if let Some(token) = cached.as_ref().filter(|t| Instant::now() < t.renew_at) {
                return Ok(token.value.clone());
            }
        }
        let fresh = self.request_token().await?;
        let value = fresh.value.clone();
        *cached = Some(fresh);
        Ok(value)
    }

    async fn request_token(&self) -> Result<AccessToken, SourceError> {
        let response = self
            .http
            .post(self.oauth_url)
            .form(&[
                ("grant_type", "client_credentials"),
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("scope", "openid"),
            ])
            .send()
            .await
            .map_err(|error| SourceError::Download(format!("legifrance token: {error}")))?;
        let status = response.status();
        if !status.is_success() {
            let detail = response.text().await.unwrap_or_default();
            return Err(SourceError::Download(format!(
                "legifrance token: HTTP {status}: {}",
                truncate_detail(&detail)
            )));
        }
        let payload: Value = response
            .json()
            .await
            .map_err(|error| SourceError::Parse(format!("legifrance token: {error}")))?;
        let (value, lifetime) = token_from(&payload).ok_or_else(|| {
            SourceError::Parse("legifrance token: no access_token in response".to_string())
        })?;
        Ok(AccessToken {
            value,
            renew_at: Instant::now() + lifetime.saturating_sub(TOKEN_RENEWAL_MARGIN),
        })
    }

    /// Appelle l'API ; sur un 401 le jeton est renouvelé et l'appel refait une
    /// seule fois.
    async fn post_api(&self, path: &str, body: &Value) -> Result<Value, SourceError> {
        let url = format!("{}{path}", self.api_base);
        let mut renewed = false;
        loop {
            let token = self.bearer(renewed).await?;
            let response = self
                .http
                .post(&url)
                .bearer_auth(&token)
                .header("accept", "application/json")
                .json(body)
                .send()
                .await
                .map_err(|error| SourceError::Download(format!("legifrance {path}: {error}")))?;
            let status = response.status();
            if status == StatusCode::UNAUTHORIZED && !renewed {
                renewed = true;
                continue;
            }
            if !status.is_success() {
                let detail = response.text().await.unwrap_or_default();
                return Err(SourceError::Download(format!(
                    "legifrance {path}: HTTP {status}: {}",
                    truncate_detail(&detail)
                )));
            }
            return response
                .json()
                .await
                .map_err(|error| SourceError::Parse(format!("legifrance {path}: {error}")));
        }
    }
}

#[async_trait]
impl LegalTextSource for LegifranceClient {
    async fn wording(
        &self,
        cited: &CitedArticle,
        on: NaiveDate,
    ) -> Result<CitedWording, SourceError> {
        let found = self.post_api(SEARCH_PATH, &search_body(cited, on)).await?;
        let Some(id) = first_article_id(&found, cited) else {
            return Ok(if cited.inserted() {
                CitedWording::Created
            } else {
                CitedWording::NotFound
            });
        };
        let consulted = self.post_api(ARTICLE_PATH, &json!({ "id": id })).await?;
        let text = article_text(&consulted)
            .ok_or_else(|| SourceError::Parse(format!("legifrance {id}: article without text")))?;
        Ok(CitedWording::InForce {
            text,
            official_url: official_url(cited, &id),
        })
    }
}

/// Tronque le corps d'une erreur HTTP avant de l'insérer dans un message.
fn truncate_detail(body: &str) -> String {
    body.chars().take(ERROR_DETAIL_MAX_CHARS).collect()
}

/// Corps de la recherche de l'article cité à la date `on` : fond `CODE_DATE`
/// filtré sur le nom du code, ou `LODA_DATE` sur le numéro de la loi.
pub fn search_body(cited: &CitedArticle, on: NaiveDate) -> Value {
    let date = json!({
        "facette": "DATE_VERSION",
        "singleDate": on.format("%Y-%m-%d").to_string(),
    });
    let (fond, fields, filters) = match cited.text() {
        CitedText::Code(name) => (
            "CODE_DATE",
            vec![exact_field("NUM_ARTICLE", cited.number())],
            vec![json!({ "facette": "NOM_CODE", "valeurs": [name] }), date],
        ),
        CitedText::Law(number) => (
            "LODA_DATE",
            vec![
                exact_field("NUM", number),
                exact_field("NUM_ARTICLE", cited.number()),
            ],
            vec![date],
        ),
    };
    json!({
        "fond": fond,
        "recherche": {
            "champs": fields,
            "filtres": filters,
            "pageNumber": 1,
            "pageSize": SEARCH_PAGE_SIZE,
            "operateur": "ET",
            "sort": "PERTINENCE",
            "typePagination": "ARTICLE",
        }
    })
}

fn exact_field(kind: &str, value: &str) -> Value {
    json!({
        "typeChamp": kind,
        "criteres": [{ "typeRecherche": "EXACTE", "valeur": value, "operateur": "ET" }],
        "operateur": "ET",
    })
}

/// Identifiant `LEGIARTI…` du premier extrait qui porte le numéro cité.
///
/// La forme de la réponse n'est pas garantie : elle est lue défensivement. Un
/// extrait d'un autre numéro n'est jamais pris, même s'il est le seul.
pub fn first_article_id(response: &Value, cited: &CitedArticle) -> Option<String> {
    array_of(response, "results")
        .flat_map(extracts_of)
        .find_map(|extract| {
            let id = extract.get("id")?.as_str()?;
            let number = extract.get("num")?.as_str()?;
            (id.starts_with(ARTICLE_ID_PREFIX) && cited.has_number(number)).then(|| id.to_string())
        })
}

/// Les extraits d'un résultat : ceux de ses sections, puis les siens.
fn extracts_of(result: &Value) -> impl Iterator<Item = &Value> {
    array_of(result, "sections")
        .flat_map(|section| array_of(section, "extracts"))
        .chain(array_of(result, "extracts"))
}

fn array_of<'a>(value: &'a Value, key: &str) -> impl Iterator<Item = &'a Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

/// Texte de l'article dans la réponse de consultation : `texte`, sinon
/// `texteHtml` débarrassé de son balisage.
pub fn article_text(response: &Value) -> Option<String> {
    let article = response.get("article")?;
    let plain = article
        .get("texte")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty());
    if let Some(text) = plain {
        return Some(text.to_string());
    }
    let html = article.get("texteHtml").and_then(Value::as_str)?;
    let text = normalize_source_text(html);
    (!text.trim().is_empty()).then_some(text)
}

/// Jeton et durée de vie lus dans la réponse de l'authentification.
fn token_from(response: &Value) -> Option<(String, Duration)> {
    let token = response
        .get("access_token")?
        .as_str()
        .filter(|token| !token.trim().is_empty())?;
    let lifetime = response
        .get("expires_in")
        .and_then(Value::as_u64)
        .map_or(FALLBACK_TOKEN_LIFETIME, Duration::from_secs);
    Some((token.to_string(), lifetime))
}

/// Lien public vers l'article chez Légifrance.
fn official_url(cited: &CitedArticle, id: &str) -> String {
    let prefix = match cited.text() {
        CitedText::Code(_) => CODE_URL_PREFIX,
        CitedText::Law(_) => LAW_URL_PREFIX,
    };
    format!("{prefix}{id}")
}

/// `LEGIFRANCE_SANDBOX=true` vise l'environnement d'essai.
fn is_sandbox(value: Option<&str>) -> bool {
    value.is_some_and(|flag| flag.trim().eq_ignore_ascii_case("true"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(year: i32, month: u32, date: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, date).unwrap()
    }

    fn code_article(name: &str, number: &str) -> CitedArticle {
        CitedArticle::new(
            CitedText::Code(name.to_string()),
            number.to_string(),
            number.to_string(),
            false,
        )
    }

    fn law_article(law: &str, number: &str) -> CitedArticle {
        CitedArticle::new(
            CitedText::Law(law.to_string()),
            number.to_string(),
            number.to_string(),
            false,
        )
    }

    #[test]
    fn should_search_code_article_by_code_name_and_date_when_citation_is_a_code() {
        let cited = code_article(
            "Code des postes et des communications électroniques",
            "L36-11",
        );

        let body = search_body(&cited, day(2018, 1, 1));

        assert_eq!(
            body,
            json!({
                "fond": "CODE_DATE",
                "recherche": {
                    "champs": [{
                        "typeChamp": "NUM_ARTICLE",
                        "criteres": [{
                            "typeRecherche": "EXACTE",
                            "valeur": "L36-11",
                            "operateur": "ET"
                        }],
                        "operateur": "ET"
                    }],
                    "filtres": [
                        {
                            "facette": "NOM_CODE",
                            "valeurs": ["Code des postes et des communications électroniques"]
                        },
                        { "facette": "DATE_VERSION", "singleDate": "2018-01-01" }
                    ],
                    "pageNumber": 1,
                    "pageSize": 10,
                    "operateur": "ET",
                    "sort": "PERTINENCE",
                    "typePagination": "ARTICLE"
                }
            })
        );
    }

    #[test]
    fn should_search_law_article_by_law_number_and_date_when_citation_is_a_law() {
        let cited = law_article("2024-322", "51");

        let body = search_body(&cited, day(2026, 3, 9));

        assert_eq!(
            body,
            json!({
                "fond": "LODA_DATE",
                "recherche": {
                    "champs": [
                        {
                            "typeChamp": "NUM",
                            "criteres": [{
                                "typeRecherche": "EXACTE",
                                "valeur": "2024-322",
                                "operateur": "ET"
                            }],
                            "operateur": "ET"
                        },
                        {
                            "typeChamp": "NUM_ARTICLE",
                            "criteres": [{
                                "typeRecherche": "EXACTE",
                                "valeur": "51",
                                "operateur": "ET"
                            }],
                            "operateur": "ET"
                        }
                    ],
                    "filtres": [
                        { "facette": "DATE_VERSION", "singleDate": "2026-03-09" }
                    ],
                    "pageNumber": 1,
                    "pageSize": 10,
                    "operateur": "ET",
                    "sort": "PERTINENCE",
                    "typePagination": "ARTICLE"
                }
            })
        );
    }

    #[test]
    fn should_read_first_article_id_when_search_returns_extracts() {
        let cited = code_article("Code de la santé publique", "L1110-5");
        let response = json!({
            "totalResultNumber": 1,
            "results": [{
                "titles": [{ "id": "LEGITEXT000006072665", "title": "Code de la santé publique" }],
                "sections": [{
                    "id": "LEGISCTA000006170574",
                    "title": "Chapitre préliminaire",
                    "extracts": [
                        { "id": "LEGISCTA000006170999", "num": "L1110-5" },
                        { "id": "LEGIARTI000000000001", "num": "L1110-4" },
                        { "id": "LEGIARTI000006685784", "num": "L. 1110-5" },
                        { "id": "LEGIARTI000000000002", "num": "L1110-5" }
                    ]
                }]
            }]
        });

        assert_eq!(
            first_article_id(&response, &cited),
            Some("LEGIARTI000006685784".to_string())
        );
    }

    #[test]
    fn should_read_no_article_id_when_search_returns_no_result() {
        let cited = code_article("Code de la santé publique", "L1110-5");

        assert_eq!(
            first_article_id(&json!({ "totalResultNumber": 0, "results": [] }), &cited),
            None
        );
        assert_eq!(
            first_article_id(&json!({ "results": [{ "sections": [] }] }), &cited),
            None
        );
        assert_eq!(first_article_id(&json!({}), &cited), None);
        assert_eq!(first_article_id(&Value::Null, &cited), None);
    }

    #[test]
    fn should_read_article_text_when_consultation_returns_an_article() {
        let plain = json!({
            "article": {
                "id": "LEGIARTI000006685784",
                "num": "L1110-5",
                "texte": "Toute personne a le droit de recevoir des soins.",
                "texteHtml": "<p>Autre texte</p>"
            }
        });
        let html_only = json!({
            "article": { "texte": "  ", "texteHtml": "<p>Premier alinéa.</p><p>Second &amp; dernier.</p>" }
        });

        assert_eq!(
            article_text(&plain),
            Some("Toute personne a le droit de recevoir des soins.".to_string())
        );
        assert_eq!(
            article_text(&html_only),
            Some("Premier alinéa.\n\nSecond & dernier.".to_string())
        );
        assert_eq!(article_text(&json!({ "article": { "texte": " " } })), None);
        assert_eq!(article_text(&json!({})), None);
    }

    #[test]
    fn should_refuse_article_id_when_extract_carries_another_number() {
        let cited = code_article("Code de la santé publique", "L1110-5");
        let response = json!({
            "results": [{
                "sections": [{
                    "extracts": [{ "id": "LEGIARTI000006685790", "num": "L1110-6" }]
                }]
            }]
        });

        assert_eq!(first_article_id(&response, &cited), None);
    }

    #[test]
    fn should_read_token_and_lifetime_when_authentication_succeeds() {
        let response = json!({
            "access_token": "jeton",
            "token_type": "Bearer",
            "expires_in": 3600
        });

        assert_eq!(
            token_from(&response),
            Some(("jeton".to_string(), Duration::from_secs(3600)))
        );
        assert_eq!(
            token_from(&json!({ "access_token": "jeton" })),
            Some(("jeton".to_string(), FALLBACK_TOKEN_LIFETIME))
        );
        assert_eq!(token_from(&json!({ "access_token": " " })), None);
        assert_eq!(token_from(&json!({ "error": "invalid_client" })), None);
    }

    #[test]
    fn should_link_code_and_law_articles_to_their_official_page() {
        let code = code_article("Code de la santé publique", "L1110-5");
        let law = law_article("2024-322", "51");

        assert_eq!(
            official_url(&code, "LEGIARTI000006685784"),
            "https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000006685784"
        );
        assert_eq!(
            official_url(&law, "LEGIARTI000049000001"),
            "https://www.legifrance.gouv.fr/loda/article_lc/LEGIARTI000049000001"
        );
    }

    #[test]
    fn should_target_sandbox_only_when_flag_says_true() {
        assert!(is_sandbox(Some("true")));
        assert!(is_sandbox(Some(" TRUE ")));
        assert!(!is_sandbox(Some("false")));
        assert!(!is_sandbox(Some("")));
        assert!(!is_sandbox(None));
    }

    /// Essai réel : exige des identifiants PISTE dans l'environnement.
    #[tokio::test]
    #[ignore = "appelle Légifrance : LEGIFRANCE_CLIENT_ID et LEGIFRANCE_CLIENT_SECRET requis"]
    async fn should_read_wording_from_legifrance_when_credentials_are_set() {
        let client = LegifranceClient::from_env()
            .expect("LEGIFRANCE_CLIENT_ID and LEGIFRANCE_CLIENT_SECRET must be set");
        let cited = code_article(
            "Code des postes et des communications électroniques",
            "L36-11",
        );

        let wording = client.wording(&cited, day(2018, 1, 1)).await.unwrap();

        assert!(
            matches!(wording, CitedWording::InForce { .. }),
            "{wording:?}"
        );
    }
}
