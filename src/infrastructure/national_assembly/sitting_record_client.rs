use chrono::{DateTime, NaiveDate, Utc};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use std::io::{Cursor, Read};
use std::time::Duration;

use async_trait::async_trait;

use crate::application::ports::sitting_record_source::{
    SittingRecordSource, SittingRecords, SourceError,
};
use crate::domain::actor::ActorUid;

use super::archive_fetcher::{ArchiveFetcher, USER_AGENT};
use crate::domain::sitting_record::{
    AnnouncedResult, NewSittingRecord, SittingEntry, SittingRecord, SittingRecordUid, SittingRef,
    SpeechAnchor,
};

/// Debut du texte d'un paragraphe de resultat annonce par la presidence.
const RESULT_MARKER: &str = "Voici le résultat du scrutin";
const INTERJECTION_GRAMMAR_PREFIX: &str = "INTERRUPTION";
const PRESIDING_ROLE: &str = "president";
/// La plupart des paroles de la presidence n'ont pas d'attribut `roledebat`:
/// seul le nom de l'orateur les distingue.
const PRESIDING_SPEAKER_NAMES: [&str; 2] = ["M. le président", "Mme la présidente"];

/// Archive des comptes rendus de la legislature: pas de sous-ensemble a demander.
const SITTING_RECORDS_URL: &str = "https://data.assemblee-nationale.fr/static/openData/repository/17/vp/syceronbrut/syseron.xml.zip";

/// Compte rendu publie a part, `{uid}.xml`, mis a jour a mesure que le
/// compte rendu provisoire se complete, sans attendre l'archive du lendemain.
const SITTING_RECORD_BASE_URL: &str = "https://www.assemblee-nationale.fr/dyn/opendata/";

/// Un compte rendu pese environ un Mo: une minute couvre une source lente,
/// pas une source bloquee.
const SITTING_RECORD_TIMEOUT: Duration = Duration::from_secs(60);

pub struct SittingRecordClient {
    archive: ArchiveFetcher,
    /// `Err`: client HTTP indisponible, chaque lecture a part echoue avec ce message.
    http: Result<reqwest::Client, String>,
}

/// Ce que l'on est en train de lire dans un element texte.
enum Capture {
    Meta(String),
    Heading(u8),
    ParagraphText,
    SpeakerName,
}

#[derive(Default)]
struct Metadata {
    uid: Option<String>,
    sitting_ref: Option<String>,
    date: Option<String>,
    version: Option<String>,
    generated_at: Option<String>,
}

#[derive(Default)]
struct Paragraph {
    actor: Option<String>,
    anchor: Option<String>,
    presiding: bool,
    interjection: bool,
    text: String,
}

impl Paragraph {
    fn from_start(start: &BytesStart) -> Self {
        let mut paragraph = Self::default();
        for attribute in start.attributes().flatten() {
            let Ok(value) = attribute.unescape_value() else {
                continue;
            };
            match attribute.key.local_name().as_ref() {
                b"id_acteur" => paragraph.actor = Some(value.into_owned()),
                b"id_syceron" => paragraph.anchor = Some(value.into_owned()),
                b"roledebat" => paragraph.presiding = value == PRESIDING_ROLE,
                b"code_grammaire" => {
                    paragraph.interjection = value.starts_with(INTERJECTION_GRAMMAR_PREFIX)
                }
                _ => {}
            }
        }
        paragraph
    }

    fn into_entry(self) -> Option<SittingEntry> {
        if self.text.is_empty() {
            return None;
        }
        if self.text.contains(RESULT_MARKER) {
            if let Some(result) = AnnouncedResult::from_text(&self.text) {
                return Some(SittingEntry::Result(result));
            }
        }
        Some(SittingEntry::Speech {
            actor_uid: ActorUid::new(self.actor?).ok()?,
            anchor: SpeechAnchor::new(self.anchor?).ok()?,
            presiding: self.presiding,
            interjection: self.interjection,
            text: self.text,
        })
    }
}

fn level_of(start: &BytesStart) -> u8 {
    start
        .attributes()
        .flatten()
        .find(|a| a.key.local_name().as_ref() == b"nivpoint")
        .and_then(|a| a.unescape_value().ok()?.trim().parse().ok())
        .unwrap_or(0)
}

fn normalize(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl SittingRecordClient {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(SITTING_RECORD_TIMEOUT)
            .build()
            .map_err(|e| e.to_string());
        if let Err(error) = &http {
            tracing::error!(%error, "Sitting record HTTP client unavailable, single records will fail");
        }
        Self {
            archive: ArchiveFetcher::new(SITTING_RECORDS_URL, "comptes rendus"),
            http,
        }
    }

    fn sitting_record_url(uid: &SittingRecordUid) -> String {
        format!("{SITTING_RECORD_BASE_URL}{uid}.xml")
    }

    /// Une entree par compte rendu de l'archive; un XML illisible n'arrete pas les autres.
    fn parse_archive(data: &[u8]) -> Result<Vec<Result<SittingRecord, SourceError>>, SourceError> {
        let mut archive = zip::ZipArchive::new(Cursor::new(data))
            .map_err(|e| SourceError::Parse(e.to_string()))?;
        let mut records = Vec::new();
        let mut buffer = Vec::new();
        for i in 0..archive.len() {
            let mut entry = match archive.by_index(i) {
                Ok(entry) => entry,
                Err(e) => {
                    records.push(Err(SourceError::Parse(e.to_string())));
                    continue;
                }
            };
            if !entry.name().ends_with(".xml") {
                continue;
            }
            let name = entry.name().to_string();
            buffer.clear();
            let parsed = entry
                .read_to_end(&mut buffer)
                .map_err(|e| e.to_string())
                .and_then(|_| Self::parse_sitting(&buffer));
            records.push(parsed.map_err(|e| SourceError::Parse(format!("{name}: {e}"))));
        }
        Ok(records)
    }

    /// Lit un compte rendu XML. Les paragraphes du sommaire sont ignores; les
    /// textes sont gardes mot pour mot, mentions entre parentheses comprises
    /// (RM-01), seuls les espaces sont ramenes a un (RM-03).
    pub(crate) fn parse_sitting(xml: &[u8]) -> Result<SittingRecord, String> {
        let mut reader = Reader::from_reader(xml);
        let mut buffer = Vec::new();
        let mut stack: Vec<String> = Vec::new();
        let mut levels: Vec<u8> = Vec::new();
        let mut capture: Option<(Capture, usize)> = None;
        let mut text = String::new();
        let mut meta = Metadata::default();
        let mut paragraph: Option<Paragraph> = None;
        let mut entries = Vec::new();

        loop {
            let event = reader
                .read_event_into(&mut buffer)
                .map_err(|e| e.to_string())?;
            match event {
                Event::Start(start) => {
                    let name = String::from_utf8_lossy(start.local_name().as_ref()).into_owned();
                    let parent = stack.last().map(String::as_str);
                    let in_summary = stack.iter().any(|n| n == "sommaire");
                    if capture.is_none() {
                        let next = match (name.as_str(), parent) {
                            ("uid" | "seanceRef", Some(_)) if stack.len() == 1 => {
                                Some(Capture::Meta(name.clone()))
                            }
                            ("dateSeance" | "version" | "heureGeneration", Some("metadonnees")) => {
                                Some(Capture::Meta(name.clone()))
                            }
                            ("texte", Some("point")) if !in_summary => {
                                Some(Capture::Heading(levels.last().copied().unwrap_or(0)))
                            }
                            ("texte", Some("paragraphe")) if paragraph.is_some() => {
                                Some(Capture::ParagraphText)
                            }
                            ("nom", Some("orateur")) if paragraph.is_some() => {
                                Some(Capture::SpeakerName)
                            }
                            _ => None,
                        };
                        if let Some(next) = next {
                            text.clear();
                            capture = Some((next, stack.len() + 1));
                        }
                    }
                    if name == "point" {
                        levels.push(level_of(&start));
                    } else if name == "paragraphe" && !in_summary {
                        paragraph = Some(Paragraph::from_start(&start));
                    }
                    stack.push(name);
                }
                Event::Text(chunk) if capture.is_some() => {
                    match chunk.unescape() {
                        Ok(value) => text.push_str(&value),
                        Err(_) => text.push_str(&String::from_utf8_lossy(&chunk)),
                    };
                }
                Event::End(_) => {
                    if let Some((_, depth)) = &capture {
                        if *depth == stack.len() {
                            let value = normalize(&text);
                            match capture.take().map(|(what, _)| what) {
                                Some(Capture::Meta(field)) => match field.as_str() {
                                    "uid" => meta.uid = Some(value),
                                    "seanceRef" => meta.sitting_ref = Some(value),
                                    "dateSeance" => meta.date = Some(value),
                                    "version" => meta.version = Some(value),
                                    _ => meta.generated_at = Some(value),
                                },
                                Some(Capture::Heading(level)) => {
                                    entries.push(SittingEntry::Heading { level, text: value });
                                }
                                Some(Capture::ParagraphText) => {
                                    if let Some(p) = paragraph.as_mut() {
                                        p.text = value;
                                    }
                                }
                                Some(Capture::SpeakerName) => {
                                    if let Some(p) = paragraph.as_mut() {
                                        p.presiding |=
                                            PRESIDING_SPEAKER_NAMES.contains(&value.as_str());
                                    }
                                }
                                None => {}
                            }
                        }
                    }
                    match stack.pop().as_deref() {
                        Some("point") => {
                            levels.pop();
                        }
                        Some("paragraphe") => {
                            entries.extend(paragraph.take().and_then(Paragraph::into_entry));
                        }
                        _ => {}
                    }
                }
                Event::Eof => break,
                _ => {}
            }
            buffer.clear();
        }

        let sitting_ref = meta.sitting_ref.ok_or("missing seanceRef")?;
        let uid = meta.uid.ok_or("missing uid")?;
        let date = meta.date.ok_or("missing dateSeance")?;
        let date = date
            .get(..8)
            .and_then(|day| NaiveDate::parse_from_str(day, "%Y%m%d").ok())
            .ok_or("invalid dateSeance")?;
        let generated_at = meta.generated_at.ok_or("missing heureGeneration")?;
        let generated_at = DateTime::parse_from_rfc3339(&generated_at)
            .map_err(|_| "invalid heureGeneration")?
            .with_timezone(&Utc);

        Ok(SittingRecord::new(NewSittingRecord {
            uid: SittingRecordUid::new(uid).map_err(|e| e.to_string())?,
            sitting_ref: SittingRef::new(sitting_ref).map_err(|e| e.to_string())?,
            date,
            version: meta.version.ok_or("missing version")?,
            generated_at,
            entries,
        }))
    }
}

impl Default for SittingRecordClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SittingRecordSource for SittingRecordClient {
    async fn fetch_sitting_records(
        &self,
        _legislature: u16,
    ) -> Result<SittingRecords, SourceError> {
        let data = self.archive.fetch().await?;
        let records = tokio::task::spawn_blocking(move || Self::parse_archive(&data))
            .await
            .map_err(|e| SourceError::Parse(e.to_string()))??;
        Ok(Box::new(records.into_iter()))
    }

    async fn fetch_sitting_record(
        &self,
        uid: &SittingRecordUid,
    ) -> Result<SittingRecord, SourceError> {
        let url = Self::sitting_record_url(uid);
        let http = self
            .http
            .as_ref()
            .map_err(|e| SourceError::Download(e.clone()))?;
        let response = http
            .get(&url)
            .send()
            .await
            .map_err(|e| SourceError::Download(format!("{url}: {e}")))?;
        if !response.status().is_success() {
            return Err(SourceError::Download(format!(
                "{url}: HTTP {}",
                response.status()
            )));
        }
        let body = response
            .bytes()
            .await
            .map_err(|e| SourceError::Download(format!("{url}: {e}")))?;
        Self::parse_sitting(&body).map_err(|e| SourceError::Parse(format!("{uid}: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use chrono::{NaiveDate, TimeZone, Utc};

    use super::*;
    use crate::domain::actor::ActorUid;
    use crate::domain::sitting_record::{AnnouncedResult, SittingEntry};

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<compteRendu xmlns="http://schemas.assemblee-nationale.fr/referentiel" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <uid>CRSANR5L17S2025O1N006</uid>
  <seanceRef>RUANR5L17S2025IDS29344</seanceRef>
  <sessionRef>SCR5A2025O1</sessionRef>
  <metadonnees>
    <dateSeance>20251013150000000</dateSeance>
    <numSeance>6</numSeance>
    <legislature>17</legislature>
    <version>8</version>
    <heureGeneration>2025-10-14T12:23:49.000+02:00</heureGeneration>
  </metadonnees>
  <contenu>
    <quantiemes><journee>Deuxième séance du lundi 13 octobre 2025</journee></quantiemes>
    <sommaire>
      <sommaire1 valeur="Titre du sommaire" id_syceron="1"><para niv="1"><sommaire3 id_syceron="2">Sommaire à ignorer</sommaire3></para></sommaire1>
    </sommaire>
    <point nivpoint="1" id_syceron="10">
      <orateurs/>
      <texte>Projet de loi de finances</texte>
      <point nivpoint="3" id_syceron="11">
        <texte>Explications de vote</texte>
        <paragraphe id_acteur="PA1592" id_mandat="M1" code_grammaire="PAROLE_GENERIQUE" roledebat="" id_syceron="100">
          <orateurs><orateur><nom>Mme Marine Tondelier</nom><id>PA1592</id><qualite/></orateur></orateurs>
          <texte stime="1">- Je vous remercie,
            monsieur   le président. (Applaudissements sur les bancs du groupe A.) Nous voterons contre.</texte>
        </paragraphe>
        <paragraphe id_acteur="PA2000" code_grammaire="INTERRUPTION_1_10" roledebat="" id_syceron="101">
          <orateurs><orateur><nom>M. Jean Dupont</nom><id>PA2000</id></orateur></orateurs>
          <texte>- C'est faux !</texte>
        </paragraphe>
        <paragraphe id_acteur="PA3000" code_grammaire="PAROLE_GENERIQUE" id_syceron="102">
          <orateurs><orateur><nom>M. le président</nom><id>PA3000</id></orateur></orateurs>
          <texte>Nous allons procéder au scrutin.</texte>
        </paragraphe>
        <paragraphe id_acteur="PA3000" code_grammaire="PAROLE_GENERIQUE" roledebat="president" id_syceron="103">
          <orateurs><orateur><nom>M. le président</nom><id>PA3000</id></orateur></orateurs>
          <texte>Voici le résultat du scrutin : Nombre de votants 553 Nombre de suffrages exprimés 540 Majorité absolue 271 Pour l’adoption 300 Contre 240</texte>
        </paragraphe>
      </point>
    </point>
  </contenu>
</compteRendu>"#;

    fn parse(xml: &str) -> SittingRecord {
        SittingRecordClient::parse_sitting(xml.as_bytes()).expect("sample must parse")
    }

    #[test]
    fn reads_the_sitting_metadata_of_a_report() {
        let record = parse(SAMPLE);

        assert_eq!(record.uid().as_str(), "CRSANR5L17S2025O1N006");
        assert_eq!(record.sitting_ref().as_str(), "RUANR5L17S2025IDS29344");
        assert_eq!(
            record.date(),
            NaiveDate::from_ymd_opt(2025, 10, 13).unwrap()
        );
        assert_eq!(record.version(), "8");
        assert_eq!(
            record.generated_at(),
            Utc.with_ymd_and_hms(2025, 10, 14, 10, 23, 49).unwrap()
        );
    }

    #[test]
    fn reads_headings_with_their_level_outside_the_summary() {
        let record = parse(SAMPLE);

        let headings: Vec<(u8, &str)> = record
            .entries()
            .iter()
            .filter_map(|entry| match entry {
                SittingEntry::Heading { level, text } => Some((*level, text.as_str())),
                _ => None,
            })
            .collect();
        assert_eq!(
            headings,
            vec![
                (1, "Projet de loi de finances"),
                (3, "Explications de vote")
            ]
        );
    }

    #[test]
    fn reads_speeches_with_speaker_anchor_and_markers() {
        let record = parse(SAMPLE);

        let speeches: Vec<&SittingEntry> = record
            .entries()
            .iter()
            .filter(|entry| matches!(entry, SittingEntry::Speech { .. }))
            .collect();
        assert_eq!(speeches.len(), 3);
        let SittingEntry::Speech {
            actor_uid,
            anchor,
            presiding,
            interjection,
            text,
        } = speeches[0]
        else {
            unreachable!()
        };
        assert_eq!(actor_uid, &ActorUid::new("PA1592".into()).unwrap());
        assert_eq!(anchor.as_str(), "100");
        assert!(!presiding);
        assert!(!interjection);
        assert_eq!(
            text,
            "- Je vous remercie, monsieur le président. (Applaudissements sur les bancs du groupe A.) Nous voterons contre."
        );
        let SittingEntry::Speech {
            anchor,
            presiding,
            interjection,
            text,
            ..
        } = speeches[1]
        else {
            unreachable!()
        };
        assert_eq!(anchor.as_str(), "101");
        assert!(!presiding);
        assert!(interjection);
        assert_eq!(text, "- C'est faux !");
        let SittingEntry::Speech {
            anchor,
            presiding,
            interjection,
            ..
        } = speeches[2]
        else {
            unreachable!()
        };
        assert_eq!(anchor.as_str(), "102");
        assert!(presiding);
        assert!(!interjection);
    }

    #[test]
    fn reads_an_announced_result_paragraph_as_a_result() {
        let record = parse(SAMPLE);

        let results: Vec<&SittingEntry> = record
            .entries()
            .iter()
            .filter(|entry| matches!(entry, SittingEntry::Result(_)))
            .collect();
        assert_eq!(
            results,
            vec![&SittingEntry::Result(AnnouncedResult {
                voters: 553,
                expressed: 540,
                votes_for: 300,
                votes_against: 240
            })]
        );
        assert_eq!(record.entries().len(), 6);
    }

    #[test]
    fn rejects_a_report_without_sitting_reference_as_unreadable() {
        let xml = SAMPLE.replace("<seanceRef>RUANR5L17S2025IDS29344</seanceRef>", "");

        assert_eq!(
            SittingRecordClient::parse_sitting(xml.as_bytes()).unwrap_err(),
            "missing seanceRef"
        );
    }

    #[test]
    fn builds_the_single_record_url_from_its_uid() {
        let uid = SittingRecordUid::new("CRSANR5L17S2027O1N061".into()).unwrap();

        assert_eq!(
            SittingRecordClient::sitting_record_url(&uid),
            "https://www.assemblee-nationale.fr/dyn/opendata/CRSANR5L17S2027O1N061.xml"
        );
    }

    /// Verifie le lecteur contre un compte rendu publie a part, provisoire.
    ///
    /// Ignore par defaut: il lit un fichier telecharge au prealable, par exemple
    ///   curl -o cr.xml https://www.assemblee-nationale.fr/dyn/opendata/CRSANR5L17S2027O1N061.xml
    ///   SITTING_RECORD_FILE=/chemin/cr.xml cargo test -- --ignored
    #[test]
    #[ignore]
    fn parses_a_real_single_record() {
        let path = std::env::var("SITTING_RECORD_FILE")
            .expect("SITTING_RECORD_FILE must point to a sitting record");
        let xml = std::fs::read(path).expect("record must be readable");

        let record = SittingRecordClient::parse_sitting(&xml).unwrap();

        assert!(record.uid().as_str().starts_with("CRSANR5L17"));
        assert!(record
            .entries()
            .iter()
            .any(|entry| matches!(entry, SittingEntry::Result(_))));
    }

    /// Verifie le lecteur contre l'archive officielle.
    ///
    /// Ignore par defaut: l'archive pese des centaines de Mo. Pour l'executer:
    ///   SITTING_RECORDS_ARCHIVE=/chemin/archive.zip cargo test -- --ignored
    #[test]
    #[ignore]
    fn parses_the_real_archive() {
        let path = std::env::var("SITTING_RECORDS_ARCHIVE")
            .expect("SITTING_RECORDS_ARCHIVE must point to the archive");
        let file = std::fs::File::open(path).expect("archive must be readable");
        let mut archive = zip::ZipArchive::new(file).expect("archive must be a zip");

        let mut parsed = 0usize;
        let mut buffer = Vec::new();
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).unwrap();
            if !entry.name().ends_with(".xml") {
                continue;
            }
            buffer.clear();
            entry.read_to_end(&mut buffer).unwrap();
            let record = SittingRecordClient::parse_sitting(&buffer).unwrap();
            assert!(!record.entries().is_empty());
            parsed += 1;
        }

        assert!(parsed > 600);
    }
}
