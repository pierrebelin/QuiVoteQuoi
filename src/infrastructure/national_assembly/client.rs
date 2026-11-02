use std::collections::HashMap;
use std::io::{Cursor, Read};

use async_trait::async_trait;
use chrono::NaiveDate;
use sha2::{Digest, Sha256};

use crate::application::ports::assembly_source::{AssemblySource, SourceError};
use crate::domain::dossier::{
    Committee, CurationStatus, DossierUid, Initiative, Initiator, LegislativeAct,
    LegislativeDocument, LegislativeDossier,
};
use crate::domain::legislature::CURRENT_LEGISLATURE;
use crate::domain::scoring::compute_score;

use super::archive_fetcher::ArchiveFetcher;
use super::committees::resolve_committee;
use super::parsing::{
    collect_all_acts, extract_document_refs, extract_initiator_refs, find_committee_organe_ref,
    find_current_stage, find_deposit_date, find_latest_act, find_outcome, RawDocumentWrapper,
    RawDossierWrapper,
};

const DOSSIERS_URL: &str = "https://data.assemblee-nationale.fr/static/openData/repository/17/loi/dossiers_legislatifs/Dossiers_Legislatifs.json.zip";
const OPEN_LICENSE: &str = "Licence Ouverte / Open Licence (Etalab)";

/// Le lecteur peut ouvrir la version précise dont la notice est publiée par
/// l'Assemblée. Le suffixe `.raw` est celui utilisé par les pages dynamiques
/// de l'Assemblée pour embarquer le document, quel que soit son type.
fn official_document_url(document_uid: &str) -> String {
    format!("https://www.assemblee-nationale.fr/dyn/docs/{document_uid}.raw")
}

struct DocumentMeta {
    title: String,
    short_title: Option<String>,
    doc_type: String,
    date: Option<NaiveDate>,
    metadata_fingerprint: String,
}

pub struct NationalAssemblyClient {
    archive: ArchiveFetcher,
}

impl NationalAssemblyClient {
    pub fn new() -> Self {
        Self {
            archive: ArchiveFetcher::new(DOSSIERS_URL, "dossiers"),
        }
    }

    async fn get_zip(&self) -> Result<bytes::Bytes, SourceError> {
        self.archive.fetch().await
    }

    fn parse_raw_dossier(
        raw: &super::parsing::RawDossier,
        doc_index: &HashMap<String, DocumentMeta>,
    ) -> Option<(LegislativeDossier, Vec<String>)> {
        let uid = DossierUid::new(raw.uid.clone()).ok()?;
        let act_info = find_latest_act(&raw.legislative_acts)?;
        let date = NaiveDate::parse_from_str(&act_info.date, "%Y-%m-%d").ok()?;

        let legislature: u16 = raw
            .legislature
            .as_deref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(CURRENT_LEGISLATURE);

        let url = raw.dossier_title.titre_chemin.as_ref().map(|chemin| {
            format!("https://www.assemblee-nationale.fr/dyn/{legislature}/dossiers/{chemin}")
        });

        let all_acts: Vec<LegislativeAct> = collect_all_acts(&raw.legislative_acts)
            .into_iter()
            .filter_map(|a| {
                NaiveDate::parse_from_str(&a.date, "%Y-%m-%d")
                    .ok()
                    .map(|d| LegislativeAct {
                        date: d,
                        label: a.label,
                        code: a.code,
                    })
            })
            .collect();

        let doc_refs = extract_document_refs(&raw.legislative_acts);
        let documents: Vec<LegislativeDocument> = doc_refs
            .iter()
            .filter_map(|doc_uid| {
                let meta = doc_index.get(doc_uid)?;
                Some(LegislativeDocument {
                    document_uid: doc_uid.clone(),
                    title: meta.title.clone(),
                    short_title: meta.short_title.clone(),
                    doc_type: meta.doc_type.clone(),
                    date: meta.date,
                    official_url: Some(official_document_url(doc_uid)),
                    source_archive_url: Some(DOSSIERS_URL.to_string()),
                    source_license: Some(OPEN_LICENSE.to_string()),
                    source_metadata_fingerprint: Some(meta.metadata_fingerprint.clone()),
                    source_retrieved_at: None,
                })
            })
            .collect();

        // Date de reference du rattachement des initiateurs (RM-01). L'acte de
        // depot la porte quand il existe; sinon la date de depot du plus ancien
        // document du dossier, qui coincide avec elle partout ou les deux sont
        // publiees.
        let deposit_date = find_deposit_date(&raw.legislative_acts)
            .and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())
            .or_else(|| documents.iter().filter_map(|d| d.date).min());

        let score = compute_score(&raw.dossier_title.titre, &act_info.label, all_acts.len());
        let current_stage = find_current_stage(&raw.legislative_acts);
        let outcome = find_outcome(&raw.legislative_acts, raw.fusion.as_ref());
        let committee = find_committee_organe_ref(&raw.legislative_acts)
            .and_then(|ref_id| resolve_committee(&ref_id).map(String::from))
            .and_then(|c| Committee::new(c).ok());
        let initiator_refs = extract_initiator_refs(&raw.initiator);

        let initiators = Initiator::default_for(
            Initiative::from_procedure(&raw.parliamentary_procedure.libelle),
            !initiator_refs.is_empty(),
        );

        Some((
            LegislativeDossier {
                uid,
                title: raw.dossier_title.titre.clone(),
                procedure: raw.parliamentary_procedure.libelle.clone(),
                legislature,
                url,
                summary: None,
                deposit_date,
                last_activity_date: date,
                last_activity_label: act_info.label,
                acts: all_acts,
                documents,
                score,
                current_stage,
                initiators,
                committee,
                curation_status: CurationStatus::New,
                outcome,
            },
            initiator_refs,
        ))
    }

    fn build_document_index(
        archive: &mut zip::ZipArchive<Cursor<&[u8]>>,
    ) -> HashMap<String, DocumentMeta> {
        let mut index = HashMap::new();

        for i in 0..archive.len() {
            let mut file = match archive.by_index(i) {
                Ok(f) => f,
                Err(_) => continue,
            };

            let name = file.name().to_string();
            if !name.contains("document/") || !name.ends_with(".json") {
                continue;
            }

            let mut content = String::new();
            if file.read_to_string(&mut content).is_err() {
                continue;
            }

            let wrapper: RawDocumentWrapper = match serde_json::from_str(&content) {
                Ok(w) => w,
                Err(_) => continue,
            };

            let doc = &wrapper.document;
            let title = doc
                .titres
                .as_ref()
                .and_then(|t| t.titre_principal.clone())
                .unwrap_or_default();

            if title.is_empty() {
                continue;
            }

            let short_title = doc
                .titres
                .as_ref()
                .and_then(|t| t.titre_principal_court.clone());

            let doc_type = doc
                .denomination
                .clone()
                .or_else(|| doc.provenance.clone())
                .unwrap_or_else(|| "Document".to_string());

            let date = doc
                .cycle_de_vie
                .as_ref()
                .and_then(|c| c.chrono.as_ref())
                .and_then(|c| c.date_depot.as_deref())
                .and_then(|d| NaiveDate::parse_from_str(&d[..10.min(d.len())], "%Y-%m-%d").ok());

            index.insert(
                doc.uid.clone(),
                DocumentMeta {
                    title,
                    short_title,
                    doc_type,
                    date,
                    metadata_fingerprint: format!("{:x}", Sha256::digest(content.as_bytes())),
                },
            );
        }

        tracing::info!("Built document index with {} entries", index.len());
        index
    }

    fn parse_dossiers(
        data: &[u8],
        since: Option<NaiveDate>,
    ) -> Result<Vec<(LegislativeDossier, Vec<String>)>, SourceError> {
        let cursor = Cursor::new(data);
        let mut archive =
            zip::ZipArchive::new(cursor).map_err(|e| SourceError::Parse(e.to_string()))?;

        let doc_index = Self::build_document_index(&mut archive);

        let mut dossiers = Vec::new();

        for i in 0..archive.len() {
            let mut file = match archive.by_index(i) {
                Ok(f) => f,
                Err(error) => {
                    tracing::warn!("Skipping unreadable zip entry #{i}: {error}");
                    continue;
                }
            };

            let name = file.name().to_string();
            if !name.contains("dossierParlementaire/") || !name.ends_with(".json") {
                continue;
            }

            let mut content = String::new();
            if let Err(error) = file.read_to_string(&mut content) {
                tracing::warn!("Skipping undecodable dossier file {name}: {error}");
                continue;
            }

            let wrapper: RawDossierWrapper = match serde_json::from_str(&content) {
                Ok(w) => w,
                Err(_) => continue,
            };

            let (dossier, refs) =
                match Self::parse_raw_dossier(&wrapper.parliamentary_dossier, &doc_index) {
                    Some(d) => d,
                    None => continue,
                };

            if since.is_some_and(|since| dossier.last_activity_date < since) {
                continue;
            }

            dossiers.push((dossier, refs));
        }

        Ok(dossiers)
    }

    fn find_dossier_by_uid(
        data: &[u8],
        uid: &str,
    ) -> Result<Option<(LegislativeDossier, Vec<String>)>, SourceError> {
        let cursor = Cursor::new(data);
        let mut archive =
            zip::ZipArchive::new(cursor).map_err(|e| SourceError::Parse(e.to_string()))?;

        let doc_index = Self::build_document_index(&mut archive);

        for i in 0..archive.len() {
            let mut file = match archive.by_index(i) {
                Ok(f) => f,
                Err(error) => {
                    tracing::warn!("Skipping unreadable zip entry #{i}: {error}");
                    continue;
                }
            };

            let name = file.name().to_string();
            if !name.contains("dossierParlementaire/") || !name.ends_with(".json") {
                continue;
            }

            let mut content = String::new();
            if let Err(error) = file.read_to_string(&mut content) {
                tracing::warn!("Skipping undecodable dossier file {name}: {error}");
                continue;
            }

            let wrapper: RawDossierWrapper = match serde_json::from_str(&content) {
                Ok(w) => w,
                Err(_) => continue,
            };

            if wrapper.parliamentary_dossier.uid != uid {
                continue;
            }

            return Ok(Self::parse_raw_dossier(
                &wrapper.parliamentary_dossier,
                &doc_index,
            ));
        }

        Ok(None)
    }
}

#[async_trait]
impl AssemblySource for NationalAssemblyClient {
    async fn fetch_dossiers_since(
        &self,
        since: Option<NaiveDate>,
    ) -> Result<Vec<LegislativeDossier>, SourceError> {
        let results = self.fetch_dossiers_since_with_refs(since).await?;
        Ok(results.into_iter().map(|(d, _)| d).collect())
    }

    async fn fetch_dossiers_since_with_refs(
        &self,
        since: Option<NaiveDate>,
    ) -> Result<Vec<(LegislativeDossier, Vec<String>)>, SourceError> {
        let zip_data = self.get_zip().await?;

        let parsed = tokio::task::spawn_blocking(move || Self::parse_dossiers(&zip_data, since))
            .await
            .map_err(|e| SourceError::Parse(e.to_string()))??;

        match since {
            Some(since) => tracing::info!("Found {} dossiers since {since}", parsed.len()),
            None => tracing::info!("Found {} dossiers, no lower date bound", parsed.len()),
        }
        Ok(parsed)
    }

    async fn fetch_dossier_by_uid(
        &self,
        uid: &DossierUid,
    ) -> Result<Option<LegislativeDossier>, SourceError> {
        let zip_data = self.get_zip().await?;
        let uid = uid.as_str().to_string();

        let result =
            tokio::task::spawn_blocking(move || Self::find_dossier_by_uid(&zip_data, &uid))
                .await
                .map_err(|e| SourceError::Parse(e.to_string()))??;

        Ok(result.map(|(d, _refs)| d))
    }

    async fn fetch_dossier_by_uid_with_refs(
        &self,
        uid: &DossierUid,
    ) -> Result<Option<(LegislativeDossier, Vec<String>)>, SourceError> {
        let zip_data = self.get_zip().await?;
        let uid = uid.as_str().to_string();

        tokio::task::spawn_blocking(move || Self::find_dossier_by_uid(&zip_data, &uid))
            .await
            .map_err(|e| SourceError::Parse(e.to_string()))?
    }
}

#[cfg(test)]
mod tests {
    use super::official_document_url;

    #[test]
    fn points_to_the_exact_official_document() {
        assert_eq!(
            official_document_url("PIONANR5L17B1560"),
            "https://www.assemblee-nationale.fr/dyn/docs/PIONANR5L17B1560.raw"
        );
    }
}
