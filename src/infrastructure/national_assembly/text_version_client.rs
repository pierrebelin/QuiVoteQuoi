//! Versions des textes de loi et texte intégral de celles que l'Assemblée
//! publie (SPEC-en-bref CU-01, RM-01, RM-14).
//!
//! Les versions viennent des entrées `document/*.json` de l'archive des dossiers
//! (`dossierRef`, `classification.statutAdoption`, dates, intitulé), lues avec
//! un `ArchiveFetcher` propre : l'ingestion des dossiers n'est pas touchée. Le
//! texte d'une version se lit à son lien officiel (`TextVersion::official_url`).

use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::time::Duration;

use async_trait::async_trait;
use chrono::NaiveDate;
use reqwest::StatusCode;
use serde::Deserialize;

use crate::application::ports::text_version_source::{SourceError, TextVersionSource};
use crate::domain::dossier::DossierUid;
use crate::domain::law_text::VersionArticle;
use crate::domain::text_version::{DocumentUid, SourceDocument, TextVersion};

use super::archive_fetcher::ArchiveFetcher;
use super::client::DOSSIERS_URL;
use super::law_text_parsing::split_articles;

/// Durée maximale du téléchargement du texte d'une version. Le projet de loi de
/// finances pèse près de quatre mégaoctets.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

/// Identité du client auprès de l'Assemblée, comme celle de `ArchiveFetcher`.
const USER_AGENT: &str = concat!(
    "QuiVoteQuoi/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/pierrebelin/QuiVoteQuoi)"
);

#[derive(Deserialize)]
struct RawDocumentWrapper {
    document: RawDocument,
}

#[derive(Deserialize)]
struct RawDocument {
    uid: String,
    #[serde(rename = "dossierRef")]
    dossier_ref: Option<String>,
    classification: Option<RawClassification>,
    #[serde(rename = "cycleDeVie")]
    life_cycle: Option<RawLifeCycle>,
    titres: Option<RawTitles>,
}

#[derive(Deserialize)]
struct RawClassification {
    #[serde(rename = "statutAdoption")]
    adoption_status: Option<String>,
}

#[derive(Deserialize)]
struct RawLifeCycle {
    chrono: Option<RawChrono>,
}

#[derive(Deserialize)]
struct RawChrono {
    #[serde(rename = "dateDepot")]
    deposit_date: Option<String>,
    #[serde(rename = "datePublication")]
    publication_date: Option<String>,
}

#[derive(Deserialize)]
struct RawTitles {
    #[serde(rename = "titrePrincipal")]
    main_title: Option<String>,
}

pub struct TextVersionClient {
    archive: ArchiveFetcher,
    /// Client HTTP, ou la raison pour laquelle il n'a pas pu être construit :
    /// le serveur démarre quand même, et chaque lecture de texte le dit.
    http: Result<reqwest::Client, String>,
}

impl Default for TextVersionClient {
    fn default() -> Self {
        Self::new()
    }
}

impl TextVersionClient {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| e.to_string());
        if let Err(error) = &http {
            tracing::error!(%error, "text versions: HTTP client unavailable, text downloads will fail");
        }
        Self {
            archive: ArchiveFetcher::new(DOSSIERS_URL, "text versions"),
            http,
        }
    }

    /// RM-01 : les versions que l'archive des dossiers publie. Un document
    /// illisible est écarté sans interrompre la lecture des autres.
    fn versions_from_archive(data: &[u8]) -> Result<Vec<TextVersion>, SourceError> {
        let mut archive = zip::ZipArchive::new(Cursor::new(data))
            .map_err(|e| SourceError::Parse(e.to_string()))?;
        let mut versions: HashMap<String, TextVersion> = HashMap::new();

        for index in 0..archive.len() {
            let mut file = match archive.by_index(index) {
                Ok(file) => file,
                Err(error) => {
                    tracing::warn!("Skipping unreadable zip entry #{index}: {error}");
                    continue;
                }
            };
            let name = file.name().to_string();
            if !name.contains("document/") || !name.ends_with(".json") {
                continue;
            }
            let mut content = String::new();
            if let Err(error) = file.read_to_string(&mut content) {
                tracing::warn!("Skipping undecodable document file {name}: {error}");
                continue;
            }
            let Ok(wrapper) = serde_json::from_str::<RawDocumentWrapper>(&content) else {
                continue;
            };
            if let Some(version) = Self::version_of(wrapper.document) {
                versions.insert(version.document_uid().as_str().to_string(), version);
            }
        }

        let mut versions: Vec<TextVersion> = versions.into_values().collect();
        versions.sort_by(|a, b| a.document_uid().as_str().cmp(b.document_uid().as_str()));
        tracing::info!("Found {} text versions", versions.len());
        Ok(versions)
    }

    fn version_of(raw: RawDocument) -> Option<TextVersion> {
        let chrono = raw.life_cycle.and_then(|life_cycle| life_cycle.chrono);
        // Un texte adopté n'a pas de date de dépôt: sa date de publication est
        // la seule que la source en donne.
        let date = chrono.as_ref().and_then(|chrono| {
            chrono
                .deposit_date
                .as_deref()
                .and_then(parse_date)
                .or_else(|| chrono.publication_date.as_deref().and_then(parse_date))
        });
        TextVersion::from_document(SourceDocument {
            document_uid: DocumentUid::new(raw.uid).ok()?,
            dossier_uid: DossierUid::new(raw.dossier_ref?).ok()?,
            adoption_status: raw
                .classification
                .and_then(|classification| classification.adoption_status),
            deposited_on: date,
            title: raw
                .titres
                .and_then(|titles| titles.main_title)
                .unwrap_or_default(),
        })
    }

    /// Le texte brut d'un document; `None` si l'Assemblée ne le publie pas (404).
    async fn fetch_text(&self, url: &str) -> Result<Option<String>, SourceError> {
        let http = self.http.as_ref().map_err(|error| {
            SourceError::Download(format!("text versions: HTTP client unavailable: {error}"))
        })?;
        let response = http
            .get(url)
            .send()
            .await
            .map_err(|e| SourceError::Download(format!("{url}: {e}")))?;
        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !status.is_success() {
            return Err(SourceError::Download(format!("{url}: HTTP {status}")));
        }
        response
            .text()
            .await
            .map(Some)
            .map_err(|e| SourceError::Download(format!("{url}: {e}")))
    }
}

/// « 2025-05-27T00:00:00.000+02:00 » → 2025-05-27.
fn parse_date(raw: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(raw.get(..10)?, "%Y-%m-%d").ok()
}

#[async_trait]
impl TextVersionSource for TextVersionClient {
    async fn fetch_versions(&self) -> Result<Vec<TextVersion>, SourceError> {
        let data = self.archive.fetch().await?;
        tokio::task::spawn_blocking(move || Self::versions_from_archive(&data))
            .await
            .map_err(|e| SourceError::Parse(e.to_string()))?
    }

    async fn version_articles(
        &self,
        version: &TextVersion,
    ) -> Result<Option<Vec<VersionArticle>>, SourceError> {
        // Le Sénat publie ses textes sur son site: rien à lire ici (RM-14).
        let Some(url) = version.official_url() else {
            return Ok(None);
        };
        let Some(html) = self.fetch_text(&url).await? else {
            return Ok(None);
        };
        tokio::task::spawn_blocking(move || split_articles(&html))
            .await
            .map(Some)
            .map_err(|e| SourceError::Parse(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    use crate::domain::text_version::{Chamber, VersionKind};

    fn document_json(
        uid: &str,
        status: Option<&str>,
        chrono: &str,
        dossier: Option<&str>,
    ) -> String {
        let status = status.map_or("null".to_string(), |s| format!("\"{s}\""));
        let dossier = dossier.map_or(String::new(), |d| format!("\"dossierRef\": \"{d}\","));
        format!(
            r#"{{"document": {{"uid": "{uid}", {dossier}
                "classification": {{"statutAdoption": {status}}},
                "cycleDeVie": {{"chrono": {chrono}}},
                "titres": {{"titrePrincipal": "proposition de loi relative au droit à l'aide à mourir"}}}}}}"#
        )
    }

    fn archive_of(entries: &[(&str, String)]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, content) in entries {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(content.as_bytes()).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn keeps_deposited_and_adopted_texts_and_leaves_the_rest_when_reading_the_archive() {
        let deposit_chrono =
            r#"{"dateDepot": "2025-05-27T00:00:00.000+02:00", "datePublication": null}"#;
        let adopted_chrono =
            r#"{"dateDepot": null, "datePublication": "2025-06-03T00:00:00.000+02:00"}"#;
        let archive = archive_of(&[
            (
                "json/document/PIONANR5L17B1100.json",
                document_json(
                    "PIONANR5L17B1100",
                    None,
                    deposit_chrono,
                    Some("DLR5L17N51670"),
                ),
            ),
            (
                "json/document/PIONANR5L17BTA0122.json",
                document_json(
                    "PIONANR5L17BTA0122",
                    Some("ADOPTSEANCE"),
                    adopted_chrono,
                    Some("DLR5L17N51670"),
                ),
            ),
            (
                "json/document/PIONANR5L17BTC1364.json",
                document_json(
                    "PIONANR5L17BTC1364",
                    Some("ADOPTCOM"),
                    deposit_chrono,
                    Some("DLR5L17N51670"),
                ),
            ),
            (
                "json/document/RAPPANR5L17B1364.json",
                document_json(
                    "RAPPANR5L17B1364",
                    None,
                    deposit_chrono,
                    Some("DLR5L17N51670"),
                ),
            ),
            (
                "json/document/PIONSNR5S479B0440.json",
                document_json(
                    "PIONSNR5S479B0440",
                    None,
                    deposit_chrono,
                    Some("DLS5S479N1"),
                ),
            ),
            (
                "json/document/PIONANR5L17B0001.json",
                document_json("PIONANR5L17B0001", None, deposit_chrono, None),
            ),
            (
                "json/dossierParlementaire/DLR5L17N51670.json",
                "{}".to_string(),
            ),
            (
                "json/document/PIONANR5L17B0002.json",
                "not json".to_string(),
            ),
        ]);

        let versions = TextVersionClient::versions_from_archive(&archive).unwrap();

        let uids: Vec<&str> = versions.iter().map(|v| v.document_uid().as_str()).collect();
        assert_eq!(
            uids,
            vec![
                "PIONANR5L17B1100",
                "PIONANR5L17BTA0122",
                "PIONSNR5S479B0440"
            ]
        );
        let deposit = &versions[0];
        assert_eq!(deposit.dossier_uid().as_str(), "DLR5L17N51670");
        assert_eq!(deposit.kind(), VersionKind::Deposited);
        assert_eq!(
            deposit.deposited_on(),
            NaiveDate::from_ymd_opt(2025, 5, 27).unwrap()
        );
        let adopted = &versions[1];
        assert_eq!(adopted.kind(), VersionKind::Adopted);
        assert_eq!(
            adopted.deposited_on(),
            NaiveDate::from_ymd_opt(2025, 6, 3).unwrap(),
            "an adopted text is dated by its publication"
        );
        assert_eq!(versions[2].chamber(), Chamber::Senate);
        assert_eq!(
            adopted.title(),
            "proposition de loi relative au droit à l'aide à mourir"
        );
    }

    #[test]
    fn refuses_an_archive_that_is_not_a_zip() {
        assert!(matches!(
            TextVersionClient::versions_from_archive(b"not a zip"),
            Err(SourceError::Parse(_))
        ));
    }

    #[test]
    fn reads_a_date_from_a_timestamp_and_nothing_from_garbage() {
        assert_eq!(
            parse_date("2025-05-27T00:00:00.000+02:00"),
            NaiveDate::from_ymd_opt(2025, 5, 27)
        );
        assert_eq!(parse_date("2025"), None);
        assert_eq!(parse_date("pas une date"), None);
    }

    /// Lit l'archive réelle désignée par `TEXT_VERSION_ARCHIVE` (le fichier
    /// `Dossiers_Legislatifs.json.zip` de data.assemblee-nationale.fr).
    #[test]
    #[ignore = "reads a real archive from the file named by TEXT_VERSION_ARCHIVE"]
    fn reads_every_version_of_a_real_archive_when_path_is_given() {
        let path = std::env::var("TEXT_VERSION_ARCHIVE").expect("TEXT_VERSION_ARCHIVE");
        let data = std::fs::read(&path).expect("readable archive");

        let versions = TextVersionClient::versions_from_archive(&data).unwrap();

        let count = |chamber: Chamber, kind: VersionKind| {
            versions
                .iter()
                .filter(|v| v.chamber() == chamber && v.kind() == kind)
                .count()
        };
        println!(
            "{} versions: AN deposited {}, AN adopted {}, SN deposited {}, SN adopted {}",
            versions.len(),
            count(Chamber::Assembly, VersionKind::Deposited),
            count(Chamber::Assembly, VersionKind::Adopted),
            count(Chamber::Senate, VersionKind::Deposited),
            count(Chamber::Senate, VersionKind::Adopted),
        );
        assert!(!versions.is_empty(), "no version recognised in {path}");
    }
}
