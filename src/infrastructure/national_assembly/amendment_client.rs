use std::io::{Cursor, Read};

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::application::ports::amendment_source::{
    AmendmentBatch, AmendmentBatches, AmendmentFeed, AmendmentSource, ArchiveScan, SourceError,
};
use crate::domain::actor::{ActorUid, GroupUid};
use crate::domain::amendment::{
    Amendment, AmendmentFate, AmendmentNumber, AmendmentTarget, AmendmentUid, Author,
    LegislativeTextRef, NewAmendment, Signatory, SignatoryRole,
};

use super::amendment_parsing::{actor_refs_in, RawAmendment, RawAmendmentWrapper};
use super::archive_fetcher::ArchiveFetcher;
use super::html_text::normalize_source_text;
use super::scrutin_parsing::non_empty;

/// Archive complete des amendements de la legislature. Comme les scrutins, il
/// n'existe pas de sous-ensemble a demander: on prend tout (RM-01).
const AMENDMENTS_URL: &str = "https://data.assemblee-nationale.fr/static/openData/repository/17/loi/amendements_div_legis/Amendements.json.zip";

pub struct AmendmentClient {
    archive: ArchiveFetcher,
}

impl AmendmentClient {
    pub fn new() -> Self {
        Self {
            archive: ArchiveFetcher::new(AMENDMENTS_URL, "amendements"),
        }
    }

    async fn get_zip(&self) -> Result<bytes::Bytes, SourceError> {
        self.archive.fetch().await
    }

    /// Parcourt l'archive et pousse les amendements par lots.
    ///
    /// Le seul critere de selection est l'extension `.json`. L'archive est
    /// organisee par texte, mais le parseur ne s'appuie pas sur cette
    /// arborescence: le rattachement se lit dans le **contenu** du fichier, et
    /// les repertoires ne servent qu'au diagnostic (`ArchiveScan::top_level`).
    /// Un renommage a la source doit se voir dans le journal, pas vider
    /// l'ingestion en silence.
    ///
    /// `emit` rend `false` quand le consommateur a disparu: le parcours s'arrete
    /// alors sans erreur.
    pub(crate) fn scan_archive(
        data: &[u8],
        legislature: u16,
        batch_size: usize,
        mut emit: impl FnMut(Vec<Amendment>) -> bool,
    ) -> Result<ArchiveScan, SourceError> {
        let cursor = Cursor::new(data);
        let mut archive =
            zip::ZipArchive::new(cursor).map_err(|e| SourceError::Parse(e.to_string()))?;

        let batch_size = batch_size.max(1);
        let mut scan = ArchiveScan::default();
        let mut batch: Vec<Amendment> = Vec::with_capacity(batch_size);
        let mut buffer = String::new();

        for i in 0..archive.len() {
            let Ok(mut file) = archive.by_index(i) else {
                continue;
            };
            if !file.name().ends_with(".json") {
                continue;
            }
            scan.json_entries += 1;
            scan.count_top_level(file.name());

            buffer.clear();
            if let Err(error) = file.read_to_string(&mut buffer) {
                scan.undecodable += 1;
                scan.count_failure(&format!("undecodable: {error}"));
                tracing::warn!("Skipping undecodable amendment file {}: {error}", file.name());
                continue;
            }

            let wrapper: RawAmendmentWrapper = match serde_json::from_str(&buffer) {
                Ok(w) => w,
                Err(e) => {
                    scan.malformed += 1;
                    scan.count_failure(&format!("malformed: {e}"));
                    tracing::warn!("Skipping malformed amendment file {}: {e}", file.name());
                    continue;
                }
            };

            match Self::to_domain(wrapper.amendement, legislature) {
                Ok(Some(amendment)) => {
                    if amendment.text_ref().is_none() {
                        scan.without_text_ref += 1;
                    }
                    if amendment.fate().is_unknown() {
                        scan.count_unknown_fate(amendment.fate().label());
                    }
                    scan.parsed += 1;
                    batch.push(amendment);
                    if batch.len() >= batch_size && !emit(std::mem::take(&mut batch)) {
                        return Ok(scan);
                    }
                }
                Ok(None) => scan.other_legislature += 1,
                Err(e) => {
                    scan.refused += 1;
                    scan.count_failure(&format!("refused: {e}"));
                    tracing::warn!("Skipping refused amendment: {e}");
                }
            }
        }

        if !batch.is_empty() {
            emit(batch);
        }

        // RM-01: tout ecart entre le publie et l'ingere est une lacune, pas un
        // detail d'implementation. Il doit se voir dans les journaux.
        if scan.unreadable() > 0 {
            tracing::warn!(
                "{} amendment entries skipped: {} undecodable, {} malformed, {} refused",
                scan.unreadable(), scan.undecodable, scan.malformed, scan.refused
            );
        }
        if scan.other_legislature > 0 {
            tracing::info!(
                "{} amendments from another legislature ignored",
                scan.other_legislature
            );
        }
        if scan.without_text_ref > 0 {
            tracing::info!(
                "{} amendments carry no legislative text ref",
                scan.without_text_ref
            );
        }
        for (label, count) in &scan.unknown_fates {
            tracing::warn!("Unknown amendment fate published {count} time(s): {label}");
        }
        tracing::info!(
            "Parsed {} amendments over {} json entries (legislature {legislature}), {} top-level directories",
            scan.parsed,
            scan.json_entries,
            scan.top_level.len()
        );

        Ok(scan)
    }

    fn to_domain(raw: RawAmendment, legislature: u16) -> Result<Option<Amendment>, String> {
        let raw_legislature = raw
            .legislature
            .as_deref()
            .and_then(|l| l.trim().parse::<u16>().ok());
        if raw_legislature.is_some_and(|l| l != legislature) {
            return Ok(None);
        }

        let uid = raw.uid.ok_or_else(|| "entry carries no uid".to_string())?;
        let uid = AmendmentUid::new(uid).map_err(|e| e.to_string())?;

        let identifiant = raw.identification.or(raw.identifiant);
        let number = identifiant
            .as_ref()
            .and_then(|i| {
                i.numero_long
                    .clone()
                    .or_else(|| i.numero.clone())
                    .and_then(|raw| non_empty(Some(raw)))
            })
            .ok_or_else(|| format!("{uid} carries no number"))?;
        let number = AmendmentNumber::new(number).map_err(|e| e.to_string())?;

        let text_ref = non_empty(raw.texte_legislatif_ref)
            .map(LegislativeTextRef::new)
            .transpose()
            .map_err(|e| e.to_string())?;

        let division = raw
            .pointeur_fragment_texte
            .and_then(|pointer| pointer.division)
            .or(raw.division);
        // La source ne publie pas toujours de titre de division: un amendement
        // portant sur l'ensemble du texte n'en a pas. Le libelle de repli est
        // explicite plutot que vide, et n'invente aucune cible.
        let target_title = division
            .as_ref()
            .and_then(|d| non_empty(d.titre.clone()))
            .unwrap_or_else(|| "Ensemble du texte".to_string());
        let target = AmendmentTarget::new(
            target_title,
            division.as_ref().and_then(|d| non_empty(d.kind.clone())),
        )
        .map_err(|e| e.to_string())?;

        let signataires = raw.signataires;
        let author = signataires
            .as_ref()
            .and_then(|s| s.auteur.as_ref())
            .map(read_author)
            .transpose()?
            .ok_or_else(|| format!("{uid} carries no author"))?;

        let cosignatories = signataires
            .as_ref()
            .and_then(|s| s.cosignataires.as_ref())
            .map(|c| actor_refs_in(c.acteur_ref.as_ref()))
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .filter_map(|(index, raw)| {
                let actor_uid = ActorUid::new(raw).ok()?;
                let rank = u16::try_from(index + 1).unwrap_or(u16::MAX);
                Some(Signatory::new(
                    actor_uid,
                    SignatoryRole::Cosignatory,
                    rank,
                    None,
                ))
            })
            .collect();

        let cycle = raw.cycle_de_vie;
        let deposited_on = cycle
            .as_ref()
            .and_then(|c| c.date_depot.as_deref())
            .and_then(parse_date);

        // La source publie le sort a deux endroits selon les versions du jeu de
        // donnees. On prend le premier renseigne, sans en synthetiser un.
        let fate_label = raw.sort_en_seance.clone().or_else(|| {
            cycle
                .as_ref()
                .and_then(|c| c.sort.clone())
        }).or_else(|| {
            cycle
                .as_ref()
                .and_then(|c| c.etat_des_traitements.as_ref())
                .and_then(|e| e.sort.as_ref())
                .and_then(|s| s.libelle.clone())
        });
        let fate = AmendmentFate::from_source(fate_label.as_deref());

        let state_label = non_empty(raw.etat).or_else(|| {
            cycle
                .as_ref()
                .and_then(|c| c.etat_des_traitements.as_ref())
                .and_then(|e| e.etat.as_ref().or(e.sous_etat.as_ref()))
                .and_then(|s| non_empty(s.libelle.clone()))
        });

        let summary = raw
            .corps
            .and_then(|c| c.contenu_auteur)
            .and_then(|c| non_empty(c.expose_sommaire))
            .map(|summary| normalize_source_text(&summary));

        let parent_uid = non_empty(raw.amendement_parent.or(raw.amendement_parent_ref))
            .map(AmendmentUid::new)
            .transpose()
            .map_err(|e| e.to_string())?;

        let amendment = Amendment::new(NewAmendment {
            uid,
            legislature,
            number,
            text_ref,
            examination_ref: non_empty(raw.examen_ref),
            target,
            author: Some(author),
            cosignatories,
            summary,
            fate,
            state_label,
            deposited_on,
            parent_uid,
        })
        .map_err(|e| e.to_string())?;

        Ok(Some(amendment))
    }
}

impl Default for AmendmentClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Auteur publie.
///
/// Un `acteurRef` fait un auteur nominatif. Sans lui, l'auteur est
/// institutionnel — Gouvernement, commission — et son libelle est conserve tel
/// quel plutot que rattache de force a une personne.
fn read_author(raw: &super::amendment_parsing::RawAuteur) -> Result<Author, String> {
    if let Some(actor_ref) = non_empty(raw.acteur_ref.clone()) {
        let actor_uid = ActorUid::new(actor_ref).map_err(|e| e.to_string())?;
        let published_group = non_empty(raw.groupe_politique_ref.clone())
            .map(GroupUid::new)
            .transpose()
            .map_err(|e| e.to_string())?;
        return Ok(Author::Deputy(Signatory::new(
            actor_uid,
            SignatoryRole::Author,
            0,
            published_group,
        )));
    }

    let label = non_empty(raw.libelle.clone())
        .or_else(|| non_empty(raw.type_auteur.clone()))
        .or_else(|| non_empty(raw.organe_ref.clone()))
        .ok_or_else(|| "author carries neither an actor ref nor a label".to_string())?;
    Ok(Author::Institutional { label })
}

/// Date publiee. La source ecrit tantot « 2025-10-14 », tantot un horodatage
/// complet: on ne retient que le jour.
fn parse_date(raw: &str) -> Option<NaiveDate> {
    let day = raw.get(..10)?;
    NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()
}

#[async_trait]
impl AmendmentSource for AmendmentClient {
    async fn fetch_amendments(
        &self,
        legislature: u16,
        batch_size: usize,
    ) -> Result<AmendmentFeed, SourceError> {
        let data = self.get_zip().await?;
        let archive_id = self.archive.archive_id();
        // Deux lots en vol au plus: le producteur attend le consommateur, ce qui
        // borne la memoire tenue quelle que soit la taille de l'archive.
        let (sender, receiver) = tokio::sync::mpsc::channel(2);

        tokio::task::spawn_blocking(move || {
            let outcome = Self::scan_archive(&data, legislature, batch_size, |batch| {
                sender
                    .blocking_send(Ok(AmendmentBatch::Items(batch)))
                    .is_ok()
            });
            let last = match outcome {
                Ok(scan) => Ok(AmendmentBatch::Done(scan)),
                Err(e) => Err(e),
            };
            // Le consommateur peut avoir abandonne: son depart n'est pas une
            // erreur d'ingestion.
            let _ = sender.blocking_send(last);
        });

        Ok(AmendmentFeed {
            archive_id,
            batches: AmendmentBatches::from_channel(receiver),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;
    use crate::domain::amendment::{FateCode, GroupOrigin};

    const DEPUTY_AMENDMENT: &str = r#"{
      "amendement": {
        "uid": "AMANR5L17PO838901BTC0633P0D1N000078",
        "legislature": "17",
        "texteLegislatifRef": "PRJLANR5L17B0324",
        "examenRef": "EXAMENANR5L17PO838901BTC0633P0D1",
        "identifiant": { "numero": "78", "numeroLong": "78" },
        "division": { "titre": "ARTICLE 3", "type": "ARTICLE" },
        "signataires": {
          "auteur": { "acteurRef": "PA1592", "groupePolitiqueRef": "PO845401", "typeAuteur": "Député" },
          "cosignataires": { "acteurRef": ["PA2000", "PA3000"] }
        },
        "corps": { "contenuAuteur": { "exposeSommaire": "Cet amendement vise à garantir …" } },
        "cycleDeVie": {
          "dateDepot": "2025-10-14",
          "etatDesTraitements": { "etat": { "libelle": "Discuté" } }
        },
        "sortEnSeance": "Adopté"
      }
    }"#;

    const SINGLE_COSIGNATORY: &str = r#"{
      "amendement": {
        "uid": "AM2",
        "legislature": "17",
        "texteLegislatifRef": "PRJLANR5L17B0324",
        "identifiant": { "numeroLong": "12" },
        "division": { "titre": "APRÈS L'ARTICLE 7" },
        "signataires": {
          "auteur": { "acteurRef": "PA1592" },
          "cosignataires": { "acteurRef": "PA4000" }
        },
        "cycleDeVie": { "dateDepot": "2025-10-15T09:30:00.000+02:00" },
        "sortEnSeance": "Rejeté"
      }
    }"#;

    const GOVERNMENT_AMENDMENT: &str = r#"{
      "amendement": {
        "uid": "AM3",
        "legislature": "17",
        "texteLegislatifRef": "PRJLANR5L17B0324",
        "identifiant": { "numeroLong": "900" },
        "division": { "titre": "ARTICLE 1" },
        "signataires": { "auteur": { "typeAuteur": "Gouvernement" } },
        "sortEnSeance": "Tombé"
      }
    }"#;

    const UNKNOWN_FATE: &str = r#"{
      "amendement": {
        "uid": "AM4",
        "legislature": "17",
        "identifiant": { "numeroLong": "45" },
        "division": { "titre": "ARTICLE 2" },
        "signataires": { "auteur": { "acteurRef": "PA1592" } },
        "sortEnSeance": "Réservé jusqu'au vote"
      }
    }"#;

    const OTHER_LEGISLATURE: &str = r#"{
      "amendement": {
        "uid": "AM5",
        "legislature": "16",
        "identifiant": { "numeroLong": "1" },
        "division": { "titre": "ARTICLE 1" },
        "signataires": { "auteur": { "acteurRef": "PA1592" } }
      }
    }"#;

    fn parse_one(raw: &str) -> Amendment {
        let wrapper: RawAmendmentWrapper = serde_json::from_str(raw).unwrap();
        AmendmentClient::to_domain(wrapper.amendement, 17)
            .unwrap()
            .unwrap()
    }

    #[test]
    fn parses_the_official_amendment_shape() {
        let amendment = parse_one(include_str!("../../../tests/fixtures/amendment_official_amanr5l17po59047btc1376p0d1n000005.json"));

        assert_eq!(amendment.uid().as_str(), "AMANR5L17PO59047BTC1376P0D1N000005");
        assert_eq!(amendment.number().as_str(), "AE5");
        assert_eq!(amendment.text_ref().unwrap().as_str(), "PNREANR5L17BTC1376");
        assert_eq!(amendment.target().title(), "Article unique");
        assert_eq!(amendment.target().kind(), Some("ARTICLE"));
        assert_eq!(amendment.deposited_on(), NaiveDate::from_ymd_opt(2025, 5, 28));
        assert_eq!(amendment.fate().label(), "Adopté");
        assert_eq!(amendment.state_label(), Some("Discuté"));
        assert!(amendment.summary().unwrap().contains("Danemark"));
    }

    #[test]
    fn a_deputy_amendment_is_read_whole() {
        let amendment = parse_one(DEPUTY_AMENDMENT);

        assert_eq!(amendment.number().as_str(), "78");
        assert_eq!(amendment.text_ref().unwrap().as_str(), "PRJLANR5L17B0324");
        assert_eq!(amendment.target().title(), "ARTICLE 3");
        assert_eq!(
            amendment.deposited_on(),
            NaiveDate::from_ymd_opt(2025, 10, 14)
        );
        assert_eq!(amendment.fate().code(), FateCode::Adopted);
        assert_eq!(amendment.fate().label(), "Adopté");
        assert_eq!(amendment.state_label(), Some("Discuté"));
        assert!(amendment.summary().unwrap().starts_with("Cet amendement"));

        let Author::Deputy(author) = amendment.author() else {
            panic!("expected a deputy author");
        };
        assert_eq!(author.actor_uid().as_str(), "PA1592");
        // Le groupe publie par la source est date par construction.
        assert_eq!(author.group_origin(), GroupOrigin::Published);
        assert_eq!(author.group_uid().unwrap().as_str(), "PO845401");

        let cosigners: Vec<&str> = amendment
            .cosignatories()
            .iter()
            .map(|s| s.actor_uid().as_str())
            .collect();
        assert_eq!(cosigners, vec!["PA2000", "PA3000"]);
    }

    #[test]
    fn a_single_cosignatory_is_not_lost_to_the_xml_collapse() {
        let amendment = parse_one(SINGLE_COSIGNATORY);
        assert_eq!(amendment.cosignatories().len(), 1);
        assert_eq!(amendment.cosignatories()[0].actor_uid().as_str(), "PA4000");
        // Horodatage complet: seul le jour est retenu.
        assert_eq!(
            amendment.deposited_on(),
            NaiveDate::from_ymd_opt(2025, 10, 15)
        );
    }

    #[test]
    fn a_government_amendment_is_institutional_not_attributed_to_a_person() {
        let amendment = parse_one(GOVERNMENT_AMENDMENT);
        assert!(matches!(
            amendment.author(),
            Author::Institutional { label } if label == "Gouvernement"
        ));
        assert!(amendment.signatory_uids().is_empty());
    }

    #[test]
    fn an_amendment_without_summary_carries_none() {
        let amendment = parse_one(GOVERNMENT_AMENDMENT);
        assert_eq!(amendment.summary(), None);
    }

    #[test]
    fn an_amendment_without_text_ref_still_enters() {
        let amendment = parse_one(UNKNOWN_FATE);
        assert!(amendment.text_ref().is_none());
    }

    #[test]
    fn an_amendment_from_another_legislature_is_skipped() {
        let wrapper: RawAmendmentWrapper = serde_json::from_str(OTHER_LEGISLATURE).unwrap();
        assert!(AmendmentClient::to_domain(wrapper.amendement, 17)
            .unwrap()
            .is_none());
    }

    fn zip_with(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut buffer = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
            for (name, content) in entries {
                writer.start_file(*name, options).unwrap();
                writer.write_all(content.as_bytes()).unwrap();
            }
            writer.finish().unwrap();
        }
        buffer.into_inner()
    }

    fn scan(data: &[u8], batch_size: usize) -> (Vec<Amendment>, ArchiveScan) {
        let mut collected = Vec::new();
        let scan = AmendmentClient::scan_archive(data, 17, batch_size, |batch| {
            collected.extend(batch);
            true
        })
        .unwrap();
        (collected, scan)
    }

    /// Le parseur ne doit rien devoir a l'arborescence: la source organise
    /// l'archive par texte, et ce classement peut changer sans preavis.
    #[test]
    fn a_flat_archive_and_a_nested_one_yield_the_same_amendments() {
        let flat = zip_with(&[
            ("AM1.json", DEPUTY_AMENDMENT),
            ("AM3.json", GOVERNMENT_AMENDMENT),
        ]);
        let nested = zip_with(&[
            ("Amendements/PRJLANR5L17B0324/AM1.json", DEPUTY_AMENDMENT),
            (
                "Amendements/PRJLANR5L17B0324/AM3.json",
                GOVERNMENT_AMENDMENT,
            ),
        ]);

        let (from_flat, flat_scan) = scan(&flat, 100);
        let (from_nested, nested_scan) = scan(&nested, 100);

        assert_eq!(from_flat, from_nested);
        assert_eq!(flat_scan.parsed, 2);
        assert_eq!(nested_scan.parsed, 2);
        // L'arborescence n'est qu'un diagnostic.
        assert!(flat_scan.top_level.is_empty());
        assert_eq!(nested_scan.top_level.get("Amendements"), Some(&2));
    }

    #[test]
    fn an_unreadable_entry_is_counted_not_fatal() {
        let data = zip_with(&[
            ("AM1.json", DEPUTY_AMENDMENT),
            ("broken.json", "{\"amendement\": {\"uid\""),
            ("AM3.json", GOVERNMENT_AMENDMENT),
            ("notes.txt", "ignored"),
        ]);

        let (amendments, scan) = scan(&data, 100);

        assert_eq!(amendments.len(), 2);
        assert_eq!(scan.json_entries, 3);
        assert_eq!(scan.parsed, 2);
        assert_eq!(scan.malformed, 1);
        assert_eq!(scan.unreadable(), 1);
    }

    #[test]
    fn a_wrongly_typed_sub_block_is_refused_without_malforming_the_file() {
        let wrong_signatories = r#"{
          "amendement": {
            "uid": "AM6",
            "legislature": "17",
            "identifiant": { "numeroLong": "6" },
            "signataires": "M. Dupont"
          }
        }"#;
        let data = zip_with(&[("AM6.json", wrong_signatories)]);

        let (amendments, scan) = scan(&data, 100);

        assert!(amendments.is_empty());
        assert_eq!(scan.malformed, 0);
        assert_eq!(scan.refused, 1);
        assert_eq!(
            scan.failures.get("refused: AM6 carries no author"),
            Some(&1)
        );
    }

    #[test]
    fn the_scan_reports_every_gap_it_walked_past() {
        let data = zip_with(&[
            ("AM1.json", DEPUTY_AMENDMENT),
            ("AM4.json", UNKNOWN_FATE),
            ("AM5.json", OTHER_LEGISLATURE),
        ]);

        let (_, scan) = scan(&data, 100);

        assert_eq!(scan.parsed, 2);
        assert_eq!(scan.other_legislature, 1);
        assert_eq!(scan.without_text_ref, 1);
        assert_eq!(scan.unknown_fates.get("Réservé jusqu'au vote"), Some(&1));
    }

    #[test]
    fn amendments_are_emitted_in_batches_of_the_requested_size() {
        let data = zip_with(&[
            ("AM1.json", DEPUTY_AMENDMENT),
            ("AM2.json", SINGLE_COSIGNATORY),
            ("AM3.json", GOVERNMENT_AMENDMENT),
        ]);

        let mut sizes = Vec::new();
        AmendmentClient::scan_archive(&data, 17, 2, |batch| {
            sizes.push(batch.len());
            true
        })
        .unwrap();

        assert_eq!(sizes, vec![2, 1]);
    }

    /// Un consommateur parti n'est pas une erreur d'ingestion: le parcours
    /// s'arrete, le bilan reste coherent.
    #[test]
    fn the_walk_stops_when_the_consumer_goes_away() {
        let data = zip_with(&[
            ("AM1.json", DEPUTY_AMENDMENT),
            ("AM2.json", SINGLE_COSIGNATORY),
            ("AM3.json", GOVERNMENT_AMENDMENT),
        ]);

        let mut batches = 0usize;
        let scan = AmendmentClient::scan_archive(&data, 17, 1, |_| {
            batches += 1;
            false
        })
        .unwrap();

        assert_eq!(batches, 1);
        assert_eq!(scan.parsed, 1);
    }

    /// Validation contre l'archive officielle. Ignore par defaut: l'archive
    /// n'est pas joignable depuis l'environnement de developpement web
    /// (SPEC-amendements §6).
    ///
    /// AMENDEMENTS_ZIP=/chemin/Amendements.json.zip cargo test -- --ignored
    #[test]
    #[ignore]
    fn parses_the_official_archive() {
        let path =
            std::env::var("AMENDEMENTS_ZIP").expect("AMENDEMENTS_ZIP must point to the archive");
        let data = std::fs::read(path).expect("archive must be readable");

        let mut parsed = 0usize;
        let scan = AmendmentClient::scan_archive(&data, 17, 1000, |batch| {
            parsed += batch.len();
            true
        })
        .unwrap();

        // Ce que ce test mesure, et qui reste inconnu tant qu'il n'a pas tourne:
        // H3 (volumetrie), H4 (noms de champs), H5 (valeurs de sort),
        // H10 (unicite du couple texte/numero). Voir SPEC-amendements §6.
        println!("json entries        : {}", scan.json_entries);
        println!("parsed              : {parsed}");
        println!("undecodable         : {}", scan.undecodable);
        println!("malformed           : {}", scan.malformed);
        println!("refused             : {}", scan.refused);
        println!("failures            : {:?}", scan.failures);
        println!("without text ref    : {}", scan.without_text_ref);
        println!("other legislature   : {}", scan.other_legislature);
        println!("unknown fates       : {:?}", scan.unknown_fates);
        println!("top level dirs      : {}", scan.top_level.len());

        assert_eq!(parsed, scan.parsed);
        // Un parcours qui ne rend rien signale un schema qui a bouge, pas une
        // archive vide.
        assert!(scan.parsed > 0, "the official archive parsed to nothing");
    }
}
