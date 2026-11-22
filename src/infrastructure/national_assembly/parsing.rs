use chrono::NaiveDate;
use serde::Deserialize;

use crate::domain::dossier::{DossierOutcome, DossierUid, LawPublication, LegislativeStage};

#[derive(Deserialize)]
pub struct RawDossierWrapper {
    #[serde(rename = "dossierParlementaire")]
    pub parliamentary_dossier: RawDossier,
}

#[derive(Deserialize)]
pub struct RawDossier {
    pub uid: String,
    pub legislature: Option<String>,
    #[serde(rename = "titreDossier")]
    pub dossier_title: RawTitle,
    #[serde(rename = "procedureParlementaire")]
    pub parliamentary_procedure: RawProcedure,
    #[serde(rename = "actesLegislatifs")]
    pub legislative_acts: Option<RawActsContainer>,
    #[serde(rename = "initiateur")]
    pub initiator: Option<RawInitiator>,
    #[serde(rename = "fusionDossier")]
    pub fusion: Option<RawFusion>,
}

#[derive(Deserialize)]
pub struct RawFusion {
    pub cause: Option<String>,
    #[serde(rename = "dossierAbsorbantRef")]
    pub absorbing_dossier_ref: Option<String>,
}

#[derive(Deserialize)]
pub struct RawTitle {
    pub titre: String,
    #[serde(rename = "titreChemin")]
    pub titre_chemin: Option<String>,
}

#[derive(Deserialize)]
pub struct RawProcedure {
    pub libelle: String,
}

#[derive(Deserialize)]
pub struct RawActsContainer {
    #[serde(rename = "acteLegislatif")]
    pub legislative_act: SingleOrVec<RawAct>,
}

#[derive(Deserialize)]
pub struct RawAct {
    #[serde(rename = "codeActe")]
    pub code: Option<String>,
    #[serde(rename = "organeRef")]
    pub organe_ref: Option<String>,
    #[serde(rename = "dateActe")]
    pub act_date: Option<String>,
    #[serde(rename = "libelleActe")]
    pub act_label: Option<RawActLabel>,
    #[serde(rename = "texteAssocie")]
    pub texte_associe: Option<String>,
    #[serde(rename = "actesLegislatifs")]
    pub legislative_acts: Option<RawActsContainer>,
    #[serde(rename = "statutConclusion")]
    pub conclusion: Option<RawConclusion>,
    #[serde(rename = "infoJO")]
    pub jo_info: Option<RawJoInfo>,
    #[serde(rename = "codeLoi")]
    pub law_code: Option<String>,
    #[serde(rename = "voteRefs")]
    pub vote_refs: Option<RawVoteRefs>,
}

/// Scrutins publics cites par un acte: `{"voteRef": "VTANR5L17V9759"}`, ou
/// une liste quand la source en cite plusieurs.
#[derive(Deserialize)]
pub struct RawVoteRefs {
    #[serde(rename = "voteRef")]
    pub vote_ref: Option<SingleOrVec<String>>,
}

#[derive(Deserialize)]
pub struct RawConclusion {
    pub fam_code: Option<String>,
    pub libelle: Option<String>,
}

#[derive(Deserialize)]
pub struct RawJoInfo {
    #[serde(rename = "dateJO")]
    pub jo_date: Option<String>,
    #[serde(rename = "urlLegifrance")]
    pub legifrance_url: Option<String>,
}

#[derive(Deserialize)]
pub struct RawActLabel {
    #[serde(rename = "libelleCourt")]
    pub short_label: Option<String>,
}

#[derive(Deserialize)]
pub struct RawInitiator {
    pub acteurs: Option<RawActeurs>,
}

#[derive(Deserialize)]
pub struct RawActeurs {
    pub acteur: SingleOrVec<RawActeur>,
}

#[derive(Deserialize)]
pub struct RawActeur {
    #[serde(rename = "acteurRef")]
    pub acteur_ref: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum SingleOrVec<T> {
    Vec(Vec<T>),
    Single(Box<T>),
}

#[derive(Deserialize)]
pub struct RawDocumentWrapper {
    pub document: RawDocument,
}

#[derive(Deserialize)]
pub struct RawDocument {
    pub uid: String,
    #[serde(rename = "denominationStructurelle")]
    pub denomination: Option<String>,
    pub provenance: Option<String>,
    pub titres: Option<RawDocumentTitles>,
    #[serde(rename = "cycleDeVie")]
    pub cycle_de_vie: Option<RawCycleDeVie>,
}

#[derive(Deserialize)]
pub struct RawDocumentTitles {
    #[serde(rename = "titrePrincipal")]
    pub titre_principal: Option<String>,
    #[serde(rename = "titrePrincipalCourt")]
    pub titre_principal_court: Option<String>,
}

#[derive(Deserialize)]
pub struct RawCycleDeVie {
    pub chrono: Option<RawChrono>,
}

#[derive(Deserialize)]
pub struct RawChrono {
    #[serde(rename = "dateDepot")]
    pub date_depot: Option<String>,
}

pub struct ActInfo {
    pub date: String,
    pub label: String,
    pub code: Option<String>,
    /// Scrutin public a l'origine de la decision, cite par la source.
    pub decision_vote: Option<String>,
    /// Libelle du `statutConclusion`, tel quel (« modifiée », « Accord »).
    pub conclusion_label: Option<String>,
    /// Famille du `statutConclusion` (`fam_code`), telle quelle.
    pub conclusion_code: Option<String>,
}

/// Libelle et famille du `statutConclusion` d'un acte, tels que la source les
/// ecrit. Une valeur blanche est une absence.
fn conclusion_of(act: &RawAct) -> (Option<String>, Option<String>) {
    let Some(conclusion) = act.conclusion.as_ref() else {
        return (None, None);
    };
    let present = |value: &Option<String>| {
        value
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
    };
    (present(&conclusion.libelle), present(&conclusion.fam_code))
}

/// Scrutin cite par un acte de decision de seance (`*-DEC` portant un
/// `statutConclusion`): celui qui a tranche, d'apres la source elle-meme.
///
/// Les votes de motion (`*-MOTION-VOTE`) citent aussi un scrutin mais ne
/// tranchent pas le texte: ils sont laisses de cote. Quand la source cite
/// plusieurs scrutins pour une meme decision (un cas en legislature 17, dont
/// l'un appartient a un autre texte), le plus recent — numero le plus eleve —
/// est retenu: c'est le dernier vote avant la decision.
fn decision_vote_of(act: &RawAct) -> Option<String> {
    let code = act.code.as_deref()?;
    if !code.ends_with("-DEC") || act.conclusion.is_none() {
        return None;
    }
    let refs: Vec<&String> = match act.vote_refs.as_ref()?.vote_ref.as_ref()? {
        SingleOrVec::Vec(refs) => refs.iter().collect(),
        SingleOrVec::Single(one) => vec![one.as_ref()],
    };
    refs.into_iter()
        .filter(|uid| !uid.trim().is_empty())
        .max_by_key(|uid| scrutin_number(uid))
        .cloned()
}

/// Numero d'ordre d'un scrutin, lu en fin d'identifiant (`...V9759`).
fn scrutin_number(uid: &str) -> u64 {
    let digits: String = uid
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    digits.parse().unwrap_or(0)
}

pub fn find_latest_act(acts: &Option<RawActsContainer>) -> Option<ActInfo> {
    let container = acts.as_ref()?;
    let mut latest: Option<ActInfo> = None;

    fn walk(act: &RawAct, latest: &mut Option<ActInfo>) {
        if let Some(ref date_str) = act.act_date {
            let date_short = &date_str[..10.min(date_str.len())];
            let label = act
                .act_label
                .as_ref()
                .and_then(|l| l.short_label.as_deref())
                .unwrap_or("?")
                .to_string();

            let dominated = latest
                .as_ref()
                .map(|l| date_short > l.date.as_str())
                .unwrap_or(true);

            if dominated {
                let (conclusion_label, conclusion_code) = conclusion_of(act);
                *latest = Some(ActInfo {
                    date: date_short.to_string(),
                    label,
                    code: act.code.clone(),
                    decision_vote: decision_vote_of(act),
                    conclusion_label,
                    conclusion_code,
                });
            }
        }
        if let Some(ref children) = act.legislative_acts {
            walk_container(children, latest);
        }
    }

    fn walk_container(container: &RawActsContainer, latest: &mut Option<ActInfo>) {
        match &container.legislative_act {
            SingleOrVec::Vec(v) => {
                for a in v {
                    walk(a, latest);
                }
            }
            SingleOrVec::Single(a) => walk(a, latest),
        }
    }

    walk_container(container, &mut latest);
    latest
}

pub fn collect_all_acts(acts: &Option<RawActsContainer>) -> Vec<ActInfo> {
    let mut result = Vec::new();

    fn walk(act: &RawAct, result: &mut Vec<ActInfo>) {
        if let Some(ref date_str) = act.act_date {
            let date_short = &date_str[..10.min(date_str.len())];
            let label = act
                .act_label
                .as_ref()
                .and_then(|l| l.short_label.as_deref())
                .unwrap_or("?")
                .to_string();
            let (conclusion_label, conclusion_code) = conclusion_of(act);
            result.push(ActInfo {
                date: date_short.to_string(),
                label,
                code: act.code.clone(),
                decision_vote: decision_vote_of(act),
                conclusion_label,
                conclusion_code,
            });
        }
        if let Some(ref children) = act.legislative_acts {
            walk_container(children, result);
        }
    }

    fn walk_container(container: &RawActsContainer, result: &mut Vec<ActInfo>) {
        match &container.legislative_act {
            SingleOrVec::Vec(v) => {
                for a in v {
                    walk(a, result);
                }
            }
            SingleOrVec::Single(a) => walk(a, result),
        }
    }

    if let Some(container) = acts.as_ref() {
        walk_container(container, &mut result);
    }

    result.sort_by(|a, b| a.date.cmp(&b.date));
    result
}

/// Date de depot du texte: le plus ancien acte de depot du dossier.
///
/// Les codes varient selon la chambre saisie en premier (`AN1-DEPOT`,
/// `SN1-DEPOT`, `ANLUNI-DEPOT`...), d'ou le suffixe plutot qu'une liste fermee.
pub fn find_deposit_date(acts: &Option<RawActsContainer>) -> Option<String> {
    let mut earliest: Option<String> = None;

    fn walk(act: &RawAct, earliest: &mut Option<String>) {
        let is_deposit = act
            .code
            .as_deref()
            .map(|c| c.ends_with("-DEPOT"))
            .unwrap_or(false);

        if is_deposit {
            if let Some(ref date_str) = act.act_date {
                let date_short = date_str[..10.min(date_str.len())].to_string();
                if earliest.as_ref().map_or(true, |e| date_short < *e) {
                    *earliest = Some(date_short);
                }
            }
        }
        if let Some(ref children) = act.legislative_acts {
            walk_container(children, earliest);
        }
    }

    fn walk_container(container: &RawActsContainer, earliest: &mut Option<String>) {
        match &container.legislative_act {
            SingleOrVec::Vec(v) => {
                for a in v {
                    walk(a, earliest);
                }
            }
            SingleOrVec::Single(a) => walk(a, earliest),
        }
    }

    walk_container(acts.as_ref()?, &mut earliest);
    earliest
}

pub fn extract_document_refs(acts: &Option<RawActsContainer>) -> Vec<String> {
    let mut refs = Vec::new();

    fn walk(act: &RawAct, refs: &mut Vec<String>) {
        if let Some(ref doc_ref) = act.texte_associe {
            if !refs.contains(doc_ref) {
                refs.push(doc_ref.clone());
            }
        }
        if let Some(ref children) = act.legislative_acts {
            walk_container(children, refs);
        }
    }

    fn walk_container(container: &RawActsContainer, refs: &mut Vec<String>) {
        match &container.legislative_act {
            SingleOrVec::Vec(v) => {
                for a in v {
                    walk(a, refs);
                }
            }
            SingleOrVec::Single(a) => walk(a, refs),
        }
    }

    if let Some(container) = acts.as_ref() {
        walk_container(container, &mut refs);
    }

    refs
}

pub fn find_current_stage(acts: &Option<RawActsContainer>) -> Option<LegislativeStage> {
    let container = acts.as_ref()?;
    let mut best: Option<LegislativeStage> = None;

    fn check_top_level(act: &RawAct, best: &mut Option<LegislativeStage>) {
        if let Some(ref code) = act.code {
            if let Some(stage) = LegislativeStage::from_code(code) {
                if best.map_or(true, |b| stage > b) {
                    *best = Some(stage);
                }
            }
        }
    }

    match &container.legislative_act {
        SingleOrVec::Vec(v) => {
            for a in v {
                check_top_level(a, &mut best);
            }
        }
        SingleOrVec::Single(a) => check_top_level(a, &mut best),
    }

    best
}

/// Familles de sorts de seance valant rejet du texte.
///
/// `TSORTF07` = « rejete », `TSORTF24` = rejet en application de l'article 49
/// alinea 3. Les autres familles `TSORTF*` sont des adoptions ou des
/// modifications.
const REJECTION_FAM_CODES: [&str; 2] = ["TSORTF07", "TSORTF24"];

fn act_date(act: &RawAct) -> Option<NaiveDate> {
    let raw = act.act_date.as_deref()?;
    NaiveDate::parse_from_str(&raw[..10.min(raw.len())], "%Y-%m-%d").ok()
}

fn for_each_act(acts: &Option<RawActsContainer>, visit: &mut impl FnMut(&RawAct)) {
    fn walk(act: &RawAct, visit: &mut impl FnMut(&RawAct)) {
        visit(act);
        if let Some(ref children) = act.legislative_acts {
            walk_container(children, visit);
        }
    }

    fn walk_container(container: &RawActsContainer, visit: &mut impl FnMut(&RawAct)) {
        match &container.legislative_act {
            SingleOrVec::Vec(v) => {
                for a in v {
                    walk(a, visit);
                }
            }
            SingleOrVec::Single(a) => walk(a, visit),
        }
    }

    if let Some(container) = acts.as_ref() {
        walk_container(container, visit);
    }
}

/// Sort du dossier, lu dans les actes — jamais deduit d'une absence.
///
/// La source ne porte pas de champ de statut: seuls quelques actes concluent
/// un dossier. Tout le reste reste `NoRecordedConclusion`, y compris un
/// dossier sans acte depuis des annees (README.md §6).
///
/// Precedence: promulgation, puis retrait, puis fusion. Un dossier a la fois
/// retire et absorbe existe (2 cas en legislature 17): le retrait est un acte
/// parlementaire date, la fusion un simple rattachement documentaire.
pub fn find_outcome(acts: &Option<RawActsContainer>, fusion: Option<&RawFusion>) -> DossierOutcome {
    let mut promulgation: Option<(NaiveDate, LawPublication)> = None;
    let mut withdrawal: Option<NaiveDate> = None;
    let mut last_decision: Option<(NaiveDate, String, String)> = None;

    for_each_act(acts, &mut |act| {
        let code = act.code.as_deref().unwrap_or("");

        if code == "PROM-PUB" {
            if let Some(date) = act_date(act) {
                let publication = LawPublication {
                    law_code: act.law_code.clone(),
                    jo_date: act.jo_info.as_ref().and_then(|jo| {
                        let raw = jo.jo_date.as_deref()?;
                        NaiveDate::parse_from_str(&raw[..10.min(raw.len())], "%Y-%m-%d").ok()
                    }),
                    legifrance_url: act
                        .jo_info
                        .as_ref()
                        .and_then(|jo| jo.legifrance_url.clone()),
                };
                if promulgation.as_ref().map_or(true, |(d, _)| date > *d) {
                    promulgation = Some((date, publication));
                }
            }
        }

        if code.ends_with("-RTRINI") {
            if let Some(date) = act_date(act) {
                if withdrawal.map_or(true, |d| date > d) {
                    withdrawal = Some(date);
                }
            }
        }

        // Seul le dernier sort connu compte: un rejet en premiere lecture suivi
        // d'une adoption ne doit pas laisser le dossier marque rejete.
        if let (Some(conclusion), Some(date)) = (act.conclusion.as_ref(), act_date(act)) {
            let (Some(fam_code), Some(label)) = (
                conclusion.fam_code.as_deref(),
                conclusion.libelle.as_deref(),
            ) else {
                return;
            };
            if !fam_code.starts_with("TSORTF") {
                return;
            }
            if last_decision.as_ref().map_or(true, |(d, _, _)| date >= *d) {
                last_decision = Some((date, fam_code.to_string(), label.to_string()));
            }
        }
    });

    if let Some((date, publication)) = promulgation {
        return DossierOutcome::Promulgated { date, publication };
    }
    if let Some(date) = withdrawal {
        return DossierOutcome::Withdrawn { date };
    }
    if let Some(fusion) = fusion {
        if let Some(uid) = fusion
            .absorbing_dossier_ref
            .clone()
            .and_then(|r| DossierUid::new(r).ok())
        {
            return DossierOutcome::MergedInto {
                dossier_uid: uid,
                cause: fusion.cause.clone(),
            };
        }
    }
    if let Some((date, fam_code, label)) = last_decision {
        if REJECTION_FAM_CODES.contains(&fam_code.as_str()) {
            return DossierOutcome::Rejected { date, label };
        }
    }

    DossierOutcome::NoRecordedConclusion
}

pub fn find_committee_organe_ref(acts: &Option<RawActsContainer>) -> Option<String> {
    let container = acts.as_ref()?;
    let mut result: Option<String> = None;

    fn walk(act: &RawAct, result: &mut Option<String>) {
        if result.is_some() {
            return;
        }
        if let Some(ref code) = act.code {
            if code.ends_with("-COM-FOND-SAISIE") {
                if let Some(ref organe) = act.organe_ref {
                    *result = Some(organe.clone());
                    return;
                }
            }
        }
        if let Some(ref children) = act.legislative_acts {
            walk_container(children, result);
        }
    }

    fn walk_container(container: &RawActsContainer, result: &mut Option<String>) {
        match &container.legislative_act {
            SingleOrVec::Vec(v) => {
                for a in v {
                    walk(a, result);
                    if result.is_some() {
                        return;
                    }
                }
            }
            SingleOrVec::Single(a) => walk(a, result),
        }
    }

    walk_container(container, &mut result);
    result
}

pub fn extract_initiator_refs(initiator: &Option<RawInitiator>) -> Vec<String> {
    let Some(init) = initiator.as_ref() else {
        return vec![];
    };
    let Some(acteurs) = init.acteurs.as_ref() else {
        return vec![];
    };
    match &acteurs.acteur {
        SingleOrVec::Vec(v) => v.iter().map(|a| a.acteur_ref.clone()).collect(),
        SingleOrVec::Single(a) => vec![a.acteur_ref.clone()],
    }
}

#[cfg(test)]
mod outcome_tests {
    use super::*;

    /// Les fragments sont recopies du dump officiel
    /// `Dossiers_Legislatifs.json.zip` (legislature 17), structure comprise.
    fn dossier(json: &str) -> RawDossier {
        let wrapper: RawDossierWrapper =
            serde_json::from_str(json).expect("fragment is valid dossier JSON");
        wrapper.parliamentary_dossier
    }

    fn wrap(acts: &str, fusion: &str) -> String {
        format!(
            r#"{{"dossierParlementaire": {{
                "uid": "DLR5L17N1",
                "legislature": "17",
                "titreDossier": {{"titre": "Loi test"}},
                "procedureParlementaire": {{"libelle": "Projet de loi ordinaire"}},
                "actesLegislatifs": {{"acteLegislatif": {acts}}},
                "fusionDossier": {fusion}
            }}}}"#
        )
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn a_promulgation_carries_the_law_and_its_journal_officiel_reference() {
        let raw = dossier(&wrap(
            r#"{
                "codeActe": "PROM",
                "dateActe": null,
                "actesLegislatifs": {"acteLegislatif": {
                    "codeActe": "PROM-PUB",
                    "dateActe": "2026-04-21T00:00:00.000+02:00",
                    "codeLoi": "2026-300",
                    "infoJO": {
                        "dateJO": "2026-04-22+02:00",
                        "urlLegifrance": "http://www.legifrance.gouv.fr/x"
                    }
                }}
            }"#,
            "null",
        ));

        let outcome = find_outcome(&raw.legislative_acts, raw.fusion.as_ref());

        let DossierOutcome::Promulgated {
            date: d,
            publication,
        } = outcome
        else {
            panic!("expected a promulgation, got {outcome:?}");
        };
        assert_eq!(d, date(2026, 4, 21));
        assert_eq!(publication.law_code.as_deref(), Some("2026-300"));
        assert_eq!(publication.jo_date, Some(date(2026, 4, 22)));
        assert_eq!(
            publication.legifrance_url.as_deref(),
            Some("http://www.legifrance.gouv.fr/x")
        );
    }

    #[test]
    fn a_withdrawn_initiative_is_read_from_its_act() {
        let raw = dossier(&wrap(
            r#"{
                "codeActe": "ANLUNI",
                "dateActe": null,
                "actesLegislatifs": {"acteLegislatif": {
                    "codeActe": "ANLUNI-RTRINI",
                    "dateActe": "2026-08-04T00:00:00.000+02:00"
                }}
            }"#,
            "null",
        ));

        assert_eq!(
            find_outcome(&raw.legislative_acts, raw.fusion.as_ref()),
            DossierOutcome::Withdrawn {
                date: date(2026, 8, 4)
            }
        );
    }

    #[test]
    fn an_absorbed_dossier_points_to_the_one_that_absorbed_it() {
        let raw = dossier(&wrap(
            r#"{"codeActe": "AN1-DEPOT", "dateActe": "2025-03-12T00:00:00.000+01:00"}"#,
            r#"{"cause": "Dossier absorbé", "dossierAbsorbantRef": "DLR5L17N54344"}"#,
        ));

        let outcome = find_outcome(&raw.legislative_acts, raw.fusion.as_ref());

        let DossierOutcome::MergedInto { dossier_uid, cause } = outcome else {
            panic!("expected a merge, got {outcome:?}");
        };
        assert_eq!(dossier_uid.as_str(), "DLR5L17N54344");
        assert_eq!(cause.as_deref(), Some("Dossier absorb\u{00e9}"));
    }

    /// Un retrait est un acte parlementaire date, la fusion un rattachement
    /// documentaire: 2 dossiers de la legislature 17 portent les deux.
    #[test]
    fn a_withdrawal_wins_over_a_merge() {
        let raw = dossier(&wrap(
            r#"{
                "codeActe": "AN1-RTRINI",
                "dateActe": "2026-02-10T00:00:00.000+01:00"
            }"#,
            r#"{"cause": "Dossier absorbé", "dossierAbsorbantRef": "DLR5L17N54344"}"#,
        ));

        assert_eq!(
            find_outcome(&raw.legislative_acts, raw.fusion.as_ref()),
            DossierOutcome::Withdrawn {
                date: date(2026, 2, 10)
            }
        );
    }

    #[test]
    fn a_rejection_keeps_the_wording_of_the_source() {
        let raw = dossier(&wrap(
            r#"{
                "codeActe": "AN1-DEBATS-DEC",
                "dateActe": "2025-11-04T00:00:00.000+01:00",
                "statutConclusion": {"fam_code": "TSORTF07", "libelle": "rejetée"}
            }"#,
            "null",
        ));

        assert_eq!(
            find_outcome(&raw.legislative_acts, raw.fusion.as_ref()),
            DossierOutcome::Rejected {
                date: date(2025, 11, 4),
                label: "rejet\u{00e9}e".into()
            }
        );
    }

    /// Fragment recopie du PLF 2027 (DLR5L17N54629): la decision de premiere
    /// lecture cite le scrutin sur la premiere partie, pas un vote sur l'ensemble.
    #[test]
    fn a_decision_carries_the_vote_the_source_cites() {
        let raw = dossier(&wrap(
            r#"{
                "codeActe": "AN1-DEBATS-DEC",
                "dateActe": "2026-11-04T00:00:00.000+01:00",
                "statutConclusion": {"fam_code": "TSORTF07", "libelle": "rejeté"},
                "voteRefs": {"voteRef": "VTANR5L17V9759"}
            }"#,
            "null",
        ));

        let acts = collect_all_acts(&raw.legislative_acts);

        assert_eq!(acts[0].decision_vote.as_deref(), Some("VTANR5L17V9759"));
    }

    #[test]
    fn a_motion_vote_is_not_a_decision_vote() {
        let raw = dossier(&wrap(
            r#"{
                "codeActe": "AN21-MOTION-VOTE",
                "dateActe": "2025-01-16T00:00:00.000+01:00",
                "voteRefs": {"voteRef": "VTANR5L17V519"}
            }"#,
            "null",
        ));

        let acts = collect_all_acts(&raw.legislative_acts);

        assert_eq!(acts[0].decision_vote, None);
    }

    /// DLR5L17N54951: la source cite deux scrutins, dont une motion d'un autre
    /// texte votee dix jours plus tot. Le plus recent est le vote decisif.
    #[test]
    fn a_decision_citing_several_votes_keeps_the_latest() {
        let raw = dossier(&wrap(
            r#"{
                "codeActe": "AN1-DEBATS-DEC",
                "dateActe": "2026-10-23T00:00:00.000+01:00",
                "statutConclusion": {"fam_code": "TSORTF07", "libelle": "rejeté"},
                "voteRefs": {"voteRef": ["VTANR5L17V9097", "VTANR5L17V8707"]}
            }"#,
            "null",
        ));

        let acts = collect_all_acts(&raw.legislative_acts);

        assert_eq!(acts[0].decision_vote.as_deref(), Some("VTANR5L17V9097"));
    }

    /// Un rejet en premiere lecture suivi d'une adoption ne doit pas laisser le
    /// dossier marque rejete: seul le dernier sort connu compte.
    #[test]
    fn a_rejection_followed_by_an_adoption_is_no_longer_a_rejection() {
        let raw = dossier(&wrap(
            r#"[
                {
                    "codeActe": "AN1-DEBATS-DEC",
                    "dateActe": "2025-11-04T00:00:00.000+01:00",
                    "statutConclusion": {"fam_code": "TSORTF07", "libelle": "rejetée"}
                },
                {
                    "codeActe": "AN2-DEBATS-DEC",
                    "dateActe": "2026-03-17T00:00:00.000+01:00",
                    "statutConclusion": {"fam_code": "TSORTF01", "libelle": "adoptée"}
                }
            ]"#,
            "null",
        ));

        assert_eq!(
            find_outcome(&raw.legislative_acts, raw.fusion.as_ref()),
            DossierOutcome::NoRecordedConclusion
        );
    }

    /// 2 788 des 3 035 dossiers de la legislature 17 sont dans ce cas: la
    /// source ne conclut rien, on ne conclut rien non plus.
    #[test]
    fn a_dossier_deposited_and_never_examined_has_no_recorded_conclusion() {
        let raw = dossier(&wrap(
            r#"{"codeActe": "AN1-DEPOT", "dateActe": "2024-11-05T00:00:00.000+01:00"}"#,
            "null",
        ));

        assert_eq!(
            find_outcome(&raw.legislative_acts, raw.fusion.as_ref()),
            DossierOutcome::NoRecordedConclusion
        );
    }

    #[test]
    fn a_dossier_without_any_act_has_no_recorded_conclusion() {
        assert_eq!(
            find_outcome(&None, None),
            DossierOutcome::NoRecordedConclusion
        );
    }
}

/// Etat « ou en est la loi » calcule de bout en bout depuis la source
/// (SPEC-navette §5). Les fragments viennent du jeu AN du 21/11/2026, reduits
/// aux actes utiles : depots, decisions `*-DEC`, publication.
#[cfg(test)]
mod law_progress_tests {
    use super::*;
    use crate::domain::dossier::{Chamber, LegislativeAct};
    use crate::domain::law_progress::{
        Agreement, DecisionSense, LawProgress, LawState, MilestoneStatus, MilestoneStep,
    };
    use MilestoneStatus::{Current, Done, Upcoming};

    const DLR5L17N51037: &str = r#"[
        {"codeActe": "AN1", "dateActe": null, "actesLegislatifs": {"acteLegislatif": [
            {"codeActe": "AN1-DEPOT", "dateActe": "2024-12-03T00:00:00.000+01:00"},
            {"codeActe": "AN1-DEBATS", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
                {"codeActe": "AN1-DEBATS-DEC", "dateActe": "2026-07-07T00:00:00.000+02:00", "statutConclusion": {"fam_code": "TSORTF01", "libelle": "adoptée"}}
            }}
        ]}},
        {"codeActe": "SN1", "dateActe": null, "actesLegislatifs": {"acteLegislatif": [
            {"codeActe": "SN1-DEPOT", "dateActe": "2026-07-07T00:00:00.000+02:00"},
            {"codeActe": "SN1-DEBATS", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
                {"codeActe": "SN1-DEBATS-DEC", "dateActe": "2026-11-20T00:00:00.000+01:00", "statutConclusion": {"fam_code": "TSORTF05", "libelle": "modifiée"}}
            }}
        ]}}
    ]"#;

    const DLR5L17N51962: &str = r#"[
        {"codeActe": "AN1", "dateActe": null, "actesLegislatifs": {"acteLegislatif": [
            {"codeActe": "AN1-DEPOT", "dateActe": "2025-04-17T00:00:00.000+02:00"},
            {"codeActe": "AN1-DEBATS", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
                {"codeActe": "AN1-DEBATS-DEC", "dateActe": "2026-01-27T00:00:00.000+01:00", "statutConclusion": {"fam_code": "TSORTF01", "libelle": "adoptée"}}
            }}
        ]}},
        {"codeActe": "SN1", "dateActe": null, "actesLegislatifs": {"acteLegislatif": [
            {"codeActe": "SN1-DEPOT", "dateActe": "2026-01-28T00:00:00.000+01:00"},
            {"codeActe": "SN1-DEBATS", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
                {"codeActe": "SN1-DEBATS-DEC", "dateActe": "2026-06-09T00:00:00.000+02:00", "statutConclusion": {"fam_code": "TSORTF05", "libelle": "modifiée"}}
            }}
        ]}},
        {"codeActe": "AN2", "dateActe": null, "actesLegislatifs": {"acteLegislatif": [
            {"codeActe": "AN2-DEPOT", "dateActe": "2026-06-10T00:00:00.000+02:00"},
            {"codeActe": "AN2-DEBATS", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
                {"codeActe": "AN2-DEBATS-DEC", "dateActe": "2026-11-19T00:00:00.000+01:00", "statutConclusion": {"fam_code": "TSORTF03", "libelle": "adoptée sans modification"}}
            }}
        ]}}
    ]"#;

    const DLR5L17N51095: &str = r#"[
        {"codeActe": "SN1", "dateActe": null, "actesLegislatifs": {"acteLegislatif": [
            {"codeActe": "SN1-DEPOT", "dateActe": "2024-12-10T00:00:00.000+01:00"},
            {"codeActe": "SN1-DEBATS", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
                {"codeActe": "SN1-DEBATS-DEC", "dateActe": "2025-03-05T00:00:00.000+01:00", "statutConclusion": {"fam_code": "TSORTF01", "libelle": "adoptée"}}
            }}
        ]}},
        {"codeActe": "AN1", "dateActe": null, "actesLegislatifs": {"acteLegislatif": [
            {"codeActe": "AN1-DEPOT", "dateActe": "2025-03-06T00:00:00.000+01:00"},
            {"codeActe": "AN1-DEBATS", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
                {"codeActe": "AN1-DEBATS-DEC", "dateActe": "2026-11-19T00:00:00.000+01:00", "statutConclusion": {"fam_code": "TSORTF03", "libelle": "adoptée sans modification"}}
            }}
        ]}}
    ]"#;

    const DLR5L17N54096: &str = r#"[
        {"codeActe": "SN1", "dateActe": null, "actesLegislatifs": {"acteLegislatif": [
            {"codeActe": "SN1-DEPOT", "dateActe": "2026-04-09T00:00:00.000+02:00"},
            {"codeActe": "SN1-DEBATS", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
                {"codeActe": "SN1-DEBATS-DEC", "dateActe": "2026-06-15T00:00:00.000+02:00", "statutConclusion": {"fam_code": "TSORTF01", "libelle": "adoptée"}}
            }}
        ]}},
        {"codeActe": "AN1", "dateActe": null, "actesLegislatifs": {"acteLegislatif": [
            {"codeActe": "AN1-DEPOT", "dateActe": "2026-06-15T00:00:00.000+02:00"},
            {"codeActe": "AN1-DEBATS", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
                {"codeActe": "AN1-DEBATS-DEC", "dateActe": "2026-10-12T00:00:00.000+02:00", "statutConclusion": {"fam_code": "TSORTF03", "libelle": "adoptée sans modification"}}
            }}
        ]}},
        {"codeActe": "PROM", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
            {"codeActe": "PROM-PUB", "dateActe": "2026-10-26T00:00:00.000+01:00", "codeLoi": "2026-992", "infoJO": {"dateJO": "2026-10-27+01:00", "urlLegifrance": "http://www.legifrance.gouv.fr/WAspad/UnTexteDeJorf?numjo=SPOX2616967L"}}
        }}
    ]"#;

    const DLR5L17N53886: &str = r#"[
        {"codeActe": "SN1", "dateActe": null, "actesLegislatifs": {"acteLegislatif": [
            {"codeActe": "SN1-DEPOT", "dateActe": "2026-02-27T00:00:00.000+01:00"},
            {"codeActe": "SN1-DEBATS", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
                {"codeActe": "SN1-DEBATS-DEC", "dateActe": "2026-05-28T00:00:00.000+02:00", "statutConclusion": {"fam_code": "TSORTF01", "libelle": "adoptée"}}
            }}
        ]}},
        {"codeActe": "AN1", "dateActe": null, "actesLegislatifs": {"acteLegislatif": [
            {"codeActe": "AN1-DEPOT", "dateActe": "2026-05-28T00:00:00.000+02:00"},
            {"codeActe": "AN1-DEBATS", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
                {"codeActe": "AN1-DEBATS-DEC", "dateActe": "2026-10-12T00:00:00.000+02:00", "statutConclusion": {"fam_code": "TSORTF03", "libelle": "adoptée sans modification"}}
            }}
        ]}},
        {"codeActe": "PROM", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
            {"codeActe": "PROM-PUB", "dateActe": "2026-10-21T00:00:00.000+02:00", "codeLoi": "2026-975", "infoJO": {"dateJO": "2026-10-22+02:00", "urlLegifrance": "http://www.legifrance.gouv.fr/WAspad/UnTexteDeJorf?numjo=SFHX2614236L"}}
        }}
    ]"#;

    const DLR5L17N51330: &str = r#"[
        {"codeActe": "AN1", "dateActe": null, "actesLegislatifs": {"acteLegislatif": [
            {"codeActe": "AN1-DEPOT", "dateActe": "2025-01-21T00:00:00.000+01:00"},
            {"codeActe": "AN1-DEBATS", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
                {"codeActe": "AN1-DEBATS-DEC", "dateActe": "2026-03-26T00:00:00.000+01:00", "statutConclusion": {"fam_code": "TSORTF01", "libelle": "adoptée"}}
            }}
        ]}},
        {"codeActe": "SN1", "dateActe": null, "actesLegislatifs": {"acteLegislatif": [
            {"codeActe": "SN1-DEPOT", "dateActe": "2026-03-26T00:00:00.000+01:00"},
            {"codeActe": "SN1-DEBATS", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
                {"codeActe": "SN1-DEBATS-DEC", "dateActe": "2026-06-16T00:00:00.000+02:00", "statutConclusion": {"fam_code": "TSORTF05", "libelle": "modifiée"}}
            }}
        ]}},
        {"codeActe": "CMP", "dateActe": null, "actesLegislatifs": {"acteLegislatif": [
            {"codeActe": "CMP-DEPOT", "dateActe": "2026-06-17T00:00:00.000+02:00"},
            {"codeActe": "CMP-DEBATS-AN", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
                {"codeActe": "CMP-DEBATS-AN-DEC", "dateActe": "2026-11-12T00:00:00.000+01:00", "statutConclusion": {"fam_code": "TSORTF18", "libelle": "adoptée, dans les conditions prévues à l'article 45, alinéa 3, de la Constitution"}}
            }},
            {"codeActe": "CMP-DEBATS-SN", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
                {"codeActe": "CMP-DEBATS-SN-DEC", "dateActe": "2026-11-19T00:00:00.000+01:00", "statutConclusion": {"fam_code": "TSORTF18", "libelle": "adoptée, dans les conditions prévues à l'article 45, alinéa 3, de la Constitution"}}
            }},
            {"codeActe": "CMP-DEC", "dateActe": "2026-11-04T00:00:00.000+01:00", "statutConclusion": {"fam_code": "TCCMP01", "libelle": "Accord"}}
        ]}}
    ]"#;

    const DLR5L17N53159: &str = r#"[
        {"codeActe": "SN1", "dateActe": null, "actesLegislatifs": {"acteLegislatif": [
            {"codeActe": "SN1-DEPOT", "dateActe": "2025-11-14T00:00:00.000+01:00"},
            {"codeActe": "SN1-DEBATS", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
                {"codeActe": "SN1-DEBATS-DEC", "dateActe": "2026-01-14T00:00:00.000+01:00", "statutConclusion": {"fam_code": "TSORTF01", "libelle": "adoptée"}}
            }}
        ]}},
        {"codeActe": "AN1", "dateActe": null, "actesLegislatifs": {"acteLegislatif": [
            {"codeActe": "AN1-DEPOT", "dateActe": "2026-01-14T00:00:00.000+01:00"},
            {"codeActe": "AN1-DEBATS", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
                {"codeActe": "AN1-DEBATS-DEC", "dateActe": "2026-06-03T00:00:00.000+02:00", "statutConclusion": {"fam_code": "TSORTF05", "libelle": "modifiée"}}
            }}
        ]}},
        {"codeActe": "CMP", "dateActe": null, "actesLegislatifs": {"acteLegislatif": [
            {"codeActe": "CMP-DEPOT", "dateActe": "2026-06-03T00:00:00.000+02:00"},
            {"codeActe": "CMP-DEBATS-SN", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
                {"codeActe": "CMP-DEBATS-SN-DEC", "dateActe": "2026-11-19T00:00:00.000+01:00", "statutConclusion": {"fam_code": "TSORTF18", "libelle": "adoptée, dans les conditions prévues à l'article 45, alinéa 3, de la Constitution"}}
            }},
            {"codeActe": "CMP-DEC", "dateActe": "2026-11-18T00:00:00.000+01:00", "statutConclusion": {"fam_code": "TCCMP01", "libelle": "Accord"}}
        ]}}
    ]"#;

    /// Fin de vie, sans ses actes `CC` et `PROM` : la lecture definitive telle
    /// qu'elle se lisait avant la promulgation.
    const DLR5L17N51670: &str = r#"[
        {"codeActe": "AN1", "dateActe": null, "actesLegislatifs": {"acteLegislatif": [
            {"codeActe": "AN1-DEPOT", "dateActe": "2025-03-11T00:00:00.000+01:00"},
            {"codeActe": "AN1-DEBATS", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
                {"codeActe": "AN1-DEBATS-DEC", "dateActe": "2025-05-27T00:00:00.000+02:00", "statutConclusion": {"fam_code": "TSORTF01", "libelle": "adoptée"}}
            }}
        ]}},
        {"codeActe": "SN1", "dateActe": null, "actesLegislatifs": {"acteLegislatif": [
            {"codeActe": "SN1-DEPOT", "dateActe": "2025-05-27T00:00:00.000+02:00"},
            {"codeActe": "SN1-DEBATS", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
                {"codeActe": "SN1-DEBATS-DEC", "dateActe": "2026-01-28T00:00:00.000+01:00", "statutConclusion": {"fam_code": "TSORTF07", "libelle": "rejetée"}}
            }}
        ]}},
        {"codeActe": "AN2", "dateActe": null, "actesLegislatifs": {"acteLegislatif": [
            {"codeActe": "AN2-DEPOT", "dateActe": "2026-01-28T00:00:00.000+01:00"},
            {"codeActe": "AN2-DEBATS", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
                {"codeActe": "AN2-DEBATS-DEC", "dateActe": "2026-02-25T00:00:00.000+01:00", "statutConclusion": {"fam_code": "TSORTF01", "libelle": "adoptée"}}
            }}
        ]}},
        {"codeActe": "SN2", "dateActe": null, "actesLegislatifs": {"acteLegislatif": [
            {"codeActe": "SN2-DEPOT", "dateActe": "2026-02-25T00:00:00.000+01:00"},
            {"codeActe": "SN2-DEBATS", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
                {"codeActe": "SN2-DEBATS-DEC", "dateActe": "2026-05-12T00:00:00.000+02:00", "statutConclusion": {"fam_code": "TSORTF07", "libelle": "rejetée"}}
            }}
        ]}},
        {"codeActe": "CMP", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
            {"codeActe": "CMP-DEC", "dateActe": "2026-06-03T00:00:00.000+02:00", "statutConclusion": {"fam_code": "TCCMP02", "libelle": "Désaccord"}}
        }},
        {"codeActe": "ANNLEC", "dateActe": null, "actesLegislatifs": {"acteLegislatif": [
            {"codeActe": "ANNLEC-DEPOT", "dateActe": "2026-05-12T00:00:00.000+02:00"},
            {"codeActe": "ANNLEC-DEBATS", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
                {"codeActe": "ANNLEC-DEBATS-DEC", "dateActe": "2026-06-30T00:00:00.000+02:00", "statutConclusion": {"fam_code": "TSORTF01", "libelle": "adoptée"}}
            }}
        ]}},
        {"codeActe": "SNNLEC", "dateActe": null, "actesLegislatifs": {"acteLegislatif": [
            {"codeActe": "SNNLEC-DEPOT", "dateActe": "2026-06-30T00:00:00.000+02:00"},
            {"codeActe": "SNNLEC-DEBATS", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
                {"codeActe": "SNNLEC-DEBATS-DEC", "dateActe": "2026-07-07T00:00:00.000+02:00", "statutConclusion": {"fam_code": "TSORTF07", "libelle": "rejetée"}}
            }}
        ]}},
        {"codeActe": "ANLDEF", "dateActe": null, "actesLegislatifs": {"acteLegislatif": [
            {"codeActe": "ANLDEF-DEPOT", "dateActe": "2026-07-09T00:00:00.000+02:00"},
            {"codeActe": "ANLDEF-DEBATS", "dateActe": null, "actesLegislatifs": {"acteLegislatif":
                {"codeActe": "ANLDEF-DEBATS-DEC", "dateActe": "2026-07-15T00:00:00.000+02:00", "statutConclusion": {"fam_code": "TSORTF01", "libelle": "adoptée"}}
            }}
        ]}}
    ]"#;

    fn acts_and_outcome(acts_json: &str) -> (Vec<LegislativeAct>, DossierOutcome) {
        let json = format!(
            r#"{{"dossierParlementaire": {{
                "uid": "DLR5L17N1",
                "legislature": "17",
                "titreDossier": {{"titre": "Loi test"}},
                "procedureParlementaire": {{"libelle": "Proposition de loi ordinaire"}},
                "actesLegislatifs": {{"acteLegislatif": {acts_json}}}
            }}}}"#
        );
        let wrapper: RawDossierWrapper =
            serde_json::from_str(&json).expect("fragment is valid dossier JSON");
        let raw = wrapper.parliamentary_dossier;
        let acts = collect_all_acts(&raw.legislative_acts)
            .into_iter()
            .map(|a| LegislativeAct {
                date: NaiveDate::parse_from_str(&a.date, "%Y-%m-%d").unwrap(),
                label: a.label,
                code: a.code,
                decision_vote: None,
                conclusion_label: a.conclusion_label,
                conclusion_code: a.conclusion_code,
            })
            .collect();
        let outcome = find_outcome(&raw.legislative_acts, raw.fusion.as_ref());
        (acts, outcome)
    }

    fn progress_of(acts_json: &str) -> LawProgress {
        let (acts, outcome) = acts_and_outcome(acts_json);
        LawProgress::from_acts(&acts, &outcome)
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn statuses(progress: &LawProgress) -> Vec<MilestoneStatus> {
        progress.milestones().iter().map(|m| m.status).collect()
    }

    #[test]
    fn every_nested_decision_keeps_the_wording_and_family_of_its_conclusion() {
        let (acts, _) = acts_and_outcome(DLR5L17N51037);

        let decisions: Vec<_> = acts
            .iter()
            .filter(|a| a.code.as_deref().is_some_and(|c| c.ends_with("-DEC")))
            .map(|a| {
                (
                    a.code.as_deref().unwrap(),
                    a.conclusion_label.as_deref(),
                    a.conclusion_code.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            decisions,
            vec![
                ("AN1-DEBATS-DEC", Some("adoptée"), Some("TSORTF01")),
                ("SN1-DEBATS-DEC", Some("modifiée"), Some("TSORTF05")),
            ]
        );
        let deposit = acts
            .iter()
            .find(|a| a.code.as_deref() == Some("AN1-DEPOT"))
            .unwrap();
        assert_eq!(deposit.conclusion_label, None);
    }

    #[test]
    fn self_defence_bill_modified_by_the_senate_is_in_shuttle() {
        let progress = progress_of(DLR5L17N51037);

        let Some(LawState::InShuttle { last_decision }) = progress.state() else {
            panic!("expected in shuttle, got {:?}", progress.state());
        };
        assert_eq!(last_decision.chamber, Chamber::Senat);
        assert_eq!(last_decision.sense, DecisionSense::Modified);
        assert_eq!(last_decision.date, date(2026, 11, 20));
        assert_eq!(statuses(&progress), vec![Done, Current, Upcoming, Upcoming]);
        let assembly = &progress.milestones()[0];
        assert_eq!(
            assembly.step,
            MilestoneStep::Chamber(Chamber::AssembleeNationale)
        );
        assert_eq!(assembly.sense, Some(DecisionSense::Adopted));
        assert_eq!(assembly.date, Some(date(2026, 7, 7)));
    }

    #[test]
    fn zero_unemployment_bill_adopted_unchanged_in_second_reading_is_definitive() {
        let progress = progress_of(DLR5L17N51962);

        assert_eq!(
            progress.state(),
            Some(&LawState::DefinitivelyAdopted {
                date: date(2026, 11, 19),
                agreement: Agreement::SameText
            })
        );
        assert_eq!(statuses(&progress), vec![Done, Done, Done, Current]);
        let senate = &progress.milestones()[1];
        assert_eq!(senate.sense, Some(DecisionSense::Modified));
        assert_eq!(senate.date, Some(date(2026, 6, 9)));
    }

    #[test]
    fn overseas_cost_of_living_bill_adopted_unchanged_by_the_assembly_is_definitive() {
        let progress = progress_of(DLR5L17N51095);

        assert_eq!(
            progress.state(),
            Some(&LawState::DefinitivelyAdopted {
                date: date(2026, 11, 19),
                agreement: Agreement::SameText
            })
        );
        assert_eq!(
            progress.milestones()[0].step,
            MilestoneStep::Chamber(Chamber::Senat)
        );
        assert_eq!(progress.milestones()[0].date, Some(date(2025, 3, 5)));
        assert_eq!(statuses(&progress), vec![Done, Done, Done, Current]);
    }

    #[test]
    fn creps_vichy_bill_became_law() {
        let progress = progress_of(DLR5L17N54096);

        let Some(LawState::BecameLaw {
            date: on,
            publication,
        }) = progress.state()
        else {
            panic!("expected became law, got {:?}", progress.state());
        };
        assert_eq!(*on, date(2026, 10, 26));
        assert_eq!(publication.law_code.as_deref(), Some("2026-992"));
        assert_eq!(publication.jo_date, Some(date(2026, 10, 27)));
        assert_eq!(statuses(&progress), vec![Done, Done, Done, Done]);
        let accord = &progress.milestones()[2];
        assert_eq!(accord.agreement, Some(Agreement::SameText));
        assert_eq!(accord.date, Some(date(2026, 10, 12)));
    }

    #[test]
    fn medico_social_bill_became_law() {
        let progress = progress_of(DLR5L17N53886);

        let Some(LawState::BecameLaw {
            date: on,
            publication,
        }) = progress.state()
        else {
            panic!("expected became law, got {:?}", progress.state());
        };
        assert_eq!(*on, date(2026, 10, 21));
        assert_eq!(publication.law_code.as_deref(), Some("2026-975"));
        assert_eq!(publication.jo_date, Some(date(2026, 10, 22)));
        assert_eq!(progress.milestones()[2].date, Some(date(2026, 10, 12)));
        assert_eq!(progress.milestones()[3].date, Some(date(2026, 10, 21)));
    }

    #[test]
    fn neighbouring_rights_bill_adopted_by_both_chambers_after_joint_committee_is_definitive() {
        let progress = progress_of(DLR5L17N51330);

        assert_eq!(
            progress.state(),
            Some(&LawState::DefinitivelyAdopted {
                date: date(2026, 11, 19),
                agreement: Agreement::JointCommittee
            })
        );
        assert_eq!(statuses(&progress), vec![Done, Done, Done, Current]);
    }

    #[test]
    fn joint_committee_text_read_by_the_senate_only_is_still_in_shuttle() {
        let progress = progress_of(DLR5L17N53159);

        let Some(LawState::InShuttle { last_decision }) = progress.state() else {
            panic!("expected in shuttle, got {:?}", progress.state());
        };
        assert_eq!(last_decision.code, "CMP-DEBATS-SN-DEC");
        assert_eq!(statuses(&progress), vec![Done, Done, Current, Upcoming]);
        assert_eq!(
            progress.milestones()[2].agreement,
            Some(Agreement::JointCommittee)
        );
    }

    #[test]
    fn end_of_life_bill_adopted_in_final_reading_is_definitive() {
        let progress = progress_of(DLR5L17N51670);

        assert_eq!(
            progress.state(),
            Some(&LawState::DefinitivelyAdopted {
                date: date(2026, 7, 15),
                agreement: Agreement::FinalReading
            })
        );
        assert_eq!(statuses(&progress), vec![Done, Done, Done, Current]);
    }
}
