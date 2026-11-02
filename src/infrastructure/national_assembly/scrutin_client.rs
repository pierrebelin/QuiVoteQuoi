use std::io::{Cursor, Read};

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::application::ports::scrutin_source::{ScrutinSource, SourceError};
use crate::domain::actor::{ActorUid, GroupUid};
use crate::domain::dossier::DossierUid;
use crate::domain::scrutin::{
    BallotType, DossierReference, GroupTally, NewScrutin, NominalVote, NonVotingCause, Outcome,
    Scrutin, ScrutinUid, TallyOrigin, VoteCorrection, VotePosition, VoteSynthesis, VoteTally,
    MISSING_GROUP_SENTINEL,
};

use super::archive_fetcher::ArchiveFetcher;
use super::scrutin_parsing::{
    count, is_true, non_empty, optional_count, votants_in, RawDecompte, RawGroupe, RawMiseAuPoint,
    RawScrutin, RawScrutinWrapper, RawVotant, RawVotantBlock,
};

/// Archive complete des scrutins de la legislature. RM-01: il n'existe pas de
/// sous-ensemble a demander, on prend tout.
const SCRUTINS_URL: &str = "https://data.assemblee-nationale.fr/static/openData/repository/17/loi/scrutins/Scrutins.json.zip";

pub struct ScrutinClient {
    archive: ArchiveFetcher,
}

impl ScrutinClient {
    pub fn new() -> Self {
        Self {
            archive: ArchiveFetcher::new(SCRUTINS_URL, "scrutins"),
        }
    }

    async fn get_zip(&self) -> Result<bytes::Bytes, SourceError> {
        self.archive.fetch().await
    }

    pub(crate) fn parse_archive(
        data: &[u8],
        legislature: u16,
    ) -> Result<Vec<Scrutin>, SourceError> {
        let cursor = Cursor::new(data);
        let mut archive =
            zip::ZipArchive::new(cursor).map_err(|e| SourceError::Parse(e.to_string()))?;

        let mut scrutins = Vec::new();
        let mut unreadable = 0usize;
        let mut other_legislature = 0usize;
        let mut buffer = String::new();

        for i in 0..archive.len() {
            let Ok(mut file) = archive.by_index(i) else {
                continue;
            };
            if !file.name().ends_with(".json") {
                continue;
            }

            buffer.clear();
            if file.read_to_string(&mut buffer).is_err() {
                unreadable += 1;
                continue;
            }

            let wrapper: RawScrutinWrapper = match serde_json::from_str(&buffer) {
                Ok(w) => w,
                Err(e) => {
                    unreadable += 1;
                    tracing::warn!("Skipping unreadable scrutin file {}: {e}", file.name());
                    continue;
                }
            };

            match Self::to_domain(wrapper.scrutin, legislature) {
                Ok(Some(scrutin)) => scrutins.push(scrutin),
                Ok(None) => other_legislature += 1,
                Err(e) => {
                    unreadable += 1;
                    tracing::warn!("Skipping scrutin: {e}");
                }
            }
        }

        // RM-01: tout ecart entre le publie et l'ingere est une lacune, pas un
        // detail d'implementation. Il doit se voir dans les journaux.
        if unreadable > 0 {
            tracing::warn!("{unreadable} scrutins unreadable at the source");
        }
        if other_legislature > 0 {
            tracing::info!("{other_legislature} scrutins from another legislature ignored");
        }

        scrutins.sort_by(|a, b| {
            b.date()
                .cmp(&a.date())
                .then_with(|| b.uid().as_str().cmp(a.uid().as_str()))
        });

        tracing::info!(
            "Parsed {} scrutins (legislature {legislature})",
            scrutins.len()
        );
        Ok(scrutins)
    }

    fn to_domain(mut raw: RawScrutin, legislature: u16) -> Result<Option<Scrutin>, String> {
        let raw_legislature = raw
            .legislature
            .as_deref()
            .and_then(|l| l.trim().parse::<u16>().ok());
        if raw_legislature.is_some_and(|l| l != legislature) {
            return Ok(None);
        }

        let uid = ScrutinUid::new(raw.uid.clone()).map_err(|e| e.to_string())?;
        let corrections = Self::read_corrections(&raw.uid, raw.mise_au_point.take());
        let date = raw
            .date_scrutin
            .as_deref()
            .and_then(|d| NaiveDate::parse_from_str(d.trim(), "%Y-%m-%d").ok())
            .ok_or_else(|| format!("{uid}: unusable date"))?;

        let raw_type = raw
            .type_vote
            .ok_or_else(|| format!("{uid}: no ballot type"))?;
        let ballot_type = BallotType::new(
            raw_type.code_type_vote.unwrap_or_default(),
            raw_type.libelle_type_vote.unwrap_or_default(),
            non_empty(raw_type.type_majorite),
        )
        .map_err(|e| e.to_string())?;

        let raw_sort = raw.sort.ok_or_else(|| format!("{uid}: no outcome"))?;
        let outcome = Outcome::new(
            raw_sort.code.unwrap_or_default(),
            raw_sort.libelle.unwrap_or_default(),
        )
        .map_err(|e| e.to_string())?;

        let objet = raw.objet;
        let subject = objet
            .as_ref()
            .and_then(|o| o.libelle.clone())
            .unwrap_or_default();

        let dossier = match objet.and_then(|o| o.dossier_legislatif) {
            Some(d) => match (non_empty(d.dossier_ref), d.libelle) {
                (Some(uid), label) => Some(DossierReference {
                    uid: DossierUid::new(uid).map_err(|e| e.to_string())?,
                    label: label.unwrap_or_default(),
                }),
                // RM-10: pas de dossier, le scrutin passe quand meme.
                (None, _) => None,
            },
            None => None,
        };

        let raw_synthese = raw
            .synthese_vote
            .ok_or_else(|| format!("{uid}: no synthesis"))?;
        let synthesis = VoteSynthesis {
            voters: count(raw_synthese.nombre_votants.as_ref()),
            expressed: count(raw_synthese.suffrages_exprimes.as_ref()),
            required: count(raw_synthese.nbr_suffrages_requis.as_ref()),
            announcement: raw_synthese.annonce.unwrap_or_default(),
            tally: tally_from(raw_synthese.decompte.as_ref()),
        };

        let raw_groups = raw
            .ventilation_votes
            .and_then(|v| v.organe)
            .and_then(|o| o.groupes)
            .and_then(|g| g.groupe)
            .map(|g| g.into_vec())
            .unwrap_or_default();

        let mut group_tallies = Vec::with_capacity(raw_groups.len());
        let mut nominal_votes = Vec::new();
        for group in raw_groups {
            let (tally, votes) = Self::read_group(group)?;
            group_tallies.push(tally);
            nominal_votes.extend(votes);
        }

        Scrutin::new(NewScrutin {
            uid,
            number: raw.numero,
            legislature: raw_legislature.unwrap_or(legislature),
            date,
            session_ref: non_empty(raw.session_ref),
            sitting_ref: non_empty(raw.seance_ref),
            place: non_empty(raw.lieu_vote),
            ballot_type,
            outcome,
            requester: raw
                .demandeur
                .and_then(|d| non_empty(d.texte))
                // Le XML colle plusieurs demandeurs sur une ligne separee par
                // des retours chariot: on les rend lisibles sans les fusionner.
                .map(|t| t.replace('\r', " \u{2014} ")),
            subject,
            synthesis,
            group_tallies,
            nominal_votes,
            corrections,
            dossier,
        })
        .map(Some)
        .map_err(|e| e.to_string())
    }

    fn read_group(raw: RawGroupe) -> Result<(GroupTally, Vec<NominalVote>), String> {
        let is_sentinel = raw.organe_ref == MISSING_GROUP_SENTINEL;
        let group_uid = GroupUid::new(raw.organe_ref.clone()).map_err(|e| e.to_string())?;
        let vote = raw.vote;

        let tally = GroupTally {
            group_uid: group_uid.clone(),
            member_count: optional_count(raw.nombre_membres_groupe.as_ref()),
            majority_position: vote
                .as_ref()
                .and_then(|v| v.position_majoritaire.as_deref())
                .and_then(|p| VotePosition::from_source(p).ok()),
            tally: tally_from(vote.as_ref().and_then(|v| v.decompte_voix.as_ref())),
            origin: TallyOrigin::Published,
        };

        // RM-04: le votant est range sous la ligne de groupe du scrutin. Sous la
        // sentinelle, la source ne dit rien: aucun groupe n'est pose, la
        // reconstruction s'en chargera (RM-03).
        let attributed = if is_sentinel { None } else { Some(group_uid) };

        let mut votes = Vec::new();
        if let Some(nominal) = vote.and_then(|v| v.decompte_nominatif) {
            let buckets = [
                (nominal.pours, VotePosition::For),
                (nominal.contres, VotePosition::Against),
                (nominal.abstentions, VotePosition::Abstention),
                (nominal.non_votants, VotePosition::NotVoting),
            ];
            for (block, position) in buckets {
                for votant in block_votants(block) {
                    votes.push(NominalVote {
                        actor_uid: ActorUid::new(votant.acteur_ref.clone())
                            .map_err(|e| e.to_string())?,
                        group_uid: attributed.clone(),
                        position,
                        cause: votant
                            .cause_position_vote
                            .clone()
                            .and_then(|c| NonVotingCause::new(c).ok()),
                        by_delegation: is_true(votant.par_delegation.as_ref()),
                        seat: optional_count(votant.num_place.as_ref()),
                    });
                }
            }
        }

        Ok((tally, votes))
    }

    /// RM-05: mises au point et dysfonctionnements, lus sans toucher aux
    /// decomptes. Un acteur ne peut porter qu'une declaration par scrutin;
    /// la premiere lue fait foi et le doublon est signale.
    fn read_corrections(
        scrutin_uid: &str,
        mise_au_point: Option<RawMiseAuPoint>,
    ) -> Vec<VoteCorrection> {
        let Some(mise_au_point) = mise_au_point else {
            return Vec::new();
        };

        let mut corrections: Vec<VoteCorrection> = Vec::new();
        let mut push = |votants: Vec<RawVotant>, position: VotePosition, malfunction: bool| {
            for votant in votants {
                let Ok(actor_uid) = ActorUid::new(votant.acteur_ref.clone()) else {
                    continue;
                };
                if corrections.iter().any(|c| c.actor_uid == actor_uid) {
                    tracing::warn!(
                        "Scrutin {scrutin_uid}: several corrections for actor {actor_uid}, keeping the first"
                    );
                    continue;
                }
                corrections.push(VoteCorrection {
                    actor_uid,
                    claimed_position: position,
                    malfunction,
                });
            }
        };

        push(
            votants_in(mise_au_point.pours.as_ref()),
            VotePosition::For,
            false,
        );
        push(
            votants_in(mise_au_point.contres.as_ref()),
            VotePosition::Against,
            false,
        );
        push(
            votants_in(mise_au_point.abstentions.as_ref()),
            VotePosition::Abstention,
            false,
        );
        push(
            votants_in(mise_au_point.non_votants.as_ref()),
            VotePosition::NotVoting,
            false,
        );
        push(
            votants_in(mise_au_point.non_votants_volontaires.as_ref()),
            VotePosition::NotVoting,
            false,
        );

        if let Some(dys) = mise_au_point.dysfonctionnement.as_ref() {
            push(votants_in(dys.pour.as_ref()), VotePosition::For, true);
            push(votants_in(dys.contre.as_ref()), VotePosition::Against, true);
            push(
                votants_in(dys.abstentions.as_ref()),
                VotePosition::Abstention,
                true,
            );
            push(
                votants_in(dys.non_votants.as_ref()),
                VotePosition::NotVoting,
                true,
            );
            push(
                votants_in(dys.non_votants_volontaires.as_ref()),
                VotePosition::NotVoting,
                true,
            );
        }

        corrections
    }
}

impl Default for ScrutinClient {
    fn default() -> Self {
        Self::new()
    }
}

fn tally_from(raw: Option<&RawDecompte>) -> VoteTally {
    match raw {
        Some(d) => VoteTally {
            votes_for: count(d.pour.as_ref()),
            votes_against: count(d.contre.as_ref()),
            abstentions: count(d.abstentions.as_ref()),
            not_voting: count(d.non_votants.as_ref()),
            voluntary_not_voting: count(d.non_votants_volontaires.as_ref()),
        },
        None => VoteTally::default(),
    }
}

fn block_votants(block: Option<RawVotantBlock>) -> Vec<RawVotant> {
    block
        .and_then(|b| b.votant)
        .map(|v| v.into_vec())
        .unwrap_or_default()
}

#[async_trait]
impl ScrutinSource for ScrutinClient {
    async fn fetch_scrutins(&self, legislature: u16) -> Result<Vec<Scrutin>, SourceError> {
        let zip_data = self.get_zip().await?;

        tokio::task::spawn_blocking(move || Self::parse_archive(&zip_data, legislature))
            .await
            .map_err(|e| SourceError::Parse(e.to_string()))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> RawScrutin {
        serde_json::from_str(SAMPLE).unwrap()
    }

    const SAMPLE: &str = r#"{
        "uid": "VTANR5L17V1191",
        "numero": "1191",
        "organeRef": "PO838901",
        "legislature": "17",
        "sessionRef": "SCR5A2025O1",
        "seanceRef": "RUANR5L17S2025IDS29344",
        "dateScrutin": "2025-03-27",
        "typeVote": {
            "codeTypeVote": "SPO",
            "libelleTypeVote": "scrutin public ordinaire",
            "typeMajorite": "Majorité absolue des suffrages exprimés"
        },
        "sort": { "code": "adopté", "libelle": "l'Assemblée nationale a adopté" },
        "titre": "l'amendement n° 1",
        "demandeur": { "texte": "Président du groupe \"A\"\rPrésidente du groupe \"B\"" },
        "objet": {
            "libelle": "l'amendement n° 1 du Gouvernement",
            "dossierLegislatif": { "libelle": "Narcotrafic", "dossierRef": "DLR5L17N50579" }
        },
        "syntheseVote": {
            "nombreVotants": "81",
            "suffragesExprimes": "80",
            "nbrSuffragesRequis": "41",
            "annonce": "l'Assemblée nationale a adopté",
            "decompte": { "nonVotants": "1", "pour": "72", "contre": "8", "abstentions": "1", "nonVotantsVolontaires": "0" }
        },
        "ventilationVotes": {
            "organe": {
                "organeRef": "PO838901",
                "groupes": {
                    "groupe": [
                        {
                            "organeRef": "PO845401",
                            "nombreMembresGroupe": "123",
                            "vote": {
                                "positionMajoritaire": "pour",
                                "decompteVoix": { "pour": "2", "contre": "0", "abstentions": "0", "nonVotants": "1", "nonVotantsVolontaires": "4" },
                                "decompteNominatif": {
                                    "pours": { "votant": [ {"acteurRef": "PA1", "mandatRef": "PM1", "parDelegation": "true", "numPlace": "12"}, {"acteurRef": "PA2", "mandatRef": "PM2", "parDelegation": "false", "numPlace": "13"} ] },
                                    "contres": null,
                                    "abstentions": null,
                                    "nonVotants": { "votant": {"acteurRef": "PA3", "mandatRef": "PM3", "parDelegation": "false", "numPlace": "1", "causePositionVote": "PAN"} }
                                }
                            }
                        },
                        {
                            "organeRef": "PO0",
                            "nombreMembresGroupe": "122",
                            "vote": {
                                "positionMajoritaire": "contre",
                                "decompteVoix": { "pour": "0", "contre": "1", "abstentions": "0", "nonVotants": "0", "nonVotantsVolontaires": "0" },
                                "decompteNominatif": {
                                    "pours": null, "abstentions": null, "nonVotants": null,
                                    "contres": { "votant": {"acteurRef": "PA9", "mandatRef": "PM9", "parDelegation": "false", "numPlace": "300"} }
                                }
                            }
                        }
                    ]
                }
            }
        },
        "miseAuPoint": {
            "nonVotants": [null, null],
            "pours": {"votant": {"acteurRef": "PA5", "mandatRef": "PM5", "parDelegation": "false", "numPlace": "9"}},
            "abstentions": [null, {"votant": {"acteurRef": "PA6", "mandatRef": "PM6", "parDelegation": "false", "numPlace": "10"}}],
            "nonVotantsVolontaires": [null, null],
            "contres": null,
            "dysfonctionnement": { "nonVotants": null, "pour": null, "contre": {"votant": {"acteurRef": "PA7", "mandatRef": "PM7", "parDelegation": "false", "numPlace": "11"}}, "abstentions": null, "nonVotantsVolontaires": null }
        },
        "lieuVote": "Hémicycle"
    }"#;

    #[test]
    fn reads_identity_synthesis_and_dossier() {
        let s = ScrutinClient::to_domain(sample(), 17).unwrap().unwrap();

        assert_eq!(s.uid().as_str(), "VTANR5L17V1191");
        assert_eq!(s.number(), "1191");
        assert_eq!(s.legislature(), 17);
        assert_eq!(s.date(), NaiveDate::from_ymd_opt(2025, 3, 27).unwrap());
        assert_eq!(s.place(), Some("H\u{00e9}micycle"));
        assert_eq!(s.ballot_type().code(), "SPO");
        assert!(s.outcome().is_adopted());
        assert_eq!(s.synthesis().voters, 81);
        assert_eq!(s.synthesis().tally.votes_for, 72);
        assert_eq!(s.dossier().unwrap().uid.as_str(), "DLR5L17N50579");
        assert_eq!(
            s.official_url(),
            "https://www.assemblee-nationale.fr/dyn/17/scrutins/1191"
        );
    }

    #[test]
    fn keeps_the_published_group_tally_untouched() {
        let s = ScrutinClient::to_domain(sample(), 17).unwrap().unwrap();
        let published = &s.group_tallies()[0];

        assert_eq!(published.group_uid.as_str(), "PO845401");
        assert_eq!(published.member_count, Some(123));
        assert_eq!(published.majority_position, Some(VotePosition::For));
        assert_eq!(published.tally.votes_for, 2);
        assert_eq!(published.tally.not_voting, 1);
        // Publie par groupe, absent de la synthese officielle.
        assert_eq!(published.tally.voluntary_not_voting, 4);
        assert_eq!(published.origin, TallyOrigin::Published);
    }

    #[test]
    fn attributes_each_vote_to_its_published_group_line() {
        let s = ScrutinClient::to_domain(sample(), 17).unwrap().unwrap();
        let votes = s.nominal_votes();

        assert_eq!(votes.len(), 4);
        let pa1 = votes
            .iter()
            .find(|v| v.actor_uid.as_str() == "PA1")
            .unwrap();
        assert_eq!(pa1.group_uid.as_ref().unwrap().as_str(), "PO845401");
        assert_eq!(pa1.position, VotePosition::For);
        assert!(pa1.by_delegation);
        assert_eq!(pa1.seat, Some(12));

        let pa3 = votes
            .iter()
            .find(|v| v.actor_uid.as_str() == "PA3")
            .unwrap();
        assert_eq!(pa3.position, VotePosition::NotVoting);
        assert_eq!(pa3.cause.as_ref().unwrap().as_str(), "PAN");
    }

    #[test]
    fn leaves_a_vote_under_the_sentinel_without_a_group() {
        let s = ScrutinClient::to_domain(sample(), 17).unwrap().unwrap();

        let pa9 = s
            .nominal_votes()
            .iter()
            .find(|v| v.actor_uid.as_str() == "PA9")
            .unwrap();
        assert!(pa9.group_uid.is_none());
        assert!(s.has_sentinel_tallies());
        assert_eq!(
            s.actors_under_sentinel()
                .iter()
                .map(|u| u.as_str().to_string())
                .collect::<Vec<_>>(),
            vec!["PA9".to_string()]
        );
    }

    #[test]
    fn reads_corrections_including_malfunctions_without_touching_the_counts() {
        let s = ScrutinClient::to_domain(sample(), 17).unwrap().unwrap();

        let corrections = s.corrections();
        assert_eq!(corrections.len(), 3);

        let pour = corrections
            .iter()
            .find(|c| c.actor_uid.as_str() == "PA5")
            .unwrap();
        assert_eq!(pour.claimed_position, VotePosition::For);
        assert!(!pour.malfunction);

        let malfunction = corrections
            .iter()
            .find(|c| c.actor_uid.as_str() == "PA7")
            .unwrap();
        assert_eq!(malfunction.claimed_position, VotePosition::Against);
        assert!(malfunction.malfunction);

        // RM-05: les decomptes publies sont intacts.
        assert_eq!(s.synthesis().tally.votes_for, 72);
        assert_eq!(s.group_tallies()[0].tally.votes_for, 2);
    }

    #[test]
    fn joins_several_requesters_on_one_readable_line() {
        let s = ScrutinClient::to_domain(sample(), 17).unwrap().unwrap();
        assert_eq!(
            s.requester(),
            Some("Pr\u{00e9}sident du groupe \"A\" \u{2014} Pr\u{00e9}sidente du groupe \"B\"")
        );
    }

    #[test]
    fn ignores_a_scrutin_of_another_legislature() {
        assert!(ScrutinClient::to_domain(sample(), 16).unwrap().is_none());
    }

    /// Verifie le parseur contre l'archive officielle.
    ///
    /// Ignore par defaut: telecharger l'archive rendrait la suite dependante du
    /// reseau. Pour l'executer:
    ///   SCRUTINS_ZIP=/chemin/Scrutins.json.zip cargo test -- --ignored
    #[test]
    #[ignore]
    fn parses_the_official_archive() {
        let path = std::env::var("SCRUTINS_ZIP").expect("SCRUTINS_ZIP must point to the archive");
        let data = std::fs::read(path).expect("archive must be readable");

        let scrutins = ScrutinClient::parse_archive(&data, 17).unwrap();

        // RM-01: tout ce que la source publie entre.
        assert_eq!(scrutins.len(), 8434);

        // 69 % des scrutins n'ont pas de dossier et restent la (H6).
        let without_dossier = scrutins.iter().filter(|s| s.dossier().is_none()).count();
        assert_eq!(without_dossier, 5826);

        // H5: la sentinelle occupe 146 lignes sur 14 scrutins.
        let sentinel_scrutins = scrutins.iter().filter(|s| s.has_sentinel_tallies()).count();
        assert_eq!(sentinel_scrutins, 14);

        // H1: le decompte nominatif est present partout.
        assert!(scrutins.iter().all(|s| !s.nominal_votes().is_empty()));

        // H9: mises au point et dysfonctionnements, sans toucher aux decomptes.
        let with_corrections = scrutins
            .iter()
            .filter(|s| !s.corrections().is_empty())
            .count();
        assert_eq!(with_corrections, 1544);
        let declarations: usize = scrutins.iter().map(|s| s.corrections().len()).sum();
        assert_eq!(declarations, 3206);
        let malfunctions: usize = scrutins
            .iter()
            .map(|s| s.corrections().iter().filter(|c| c.malfunction).count())
            .sum();
        assert_eq!(malfunctions, 163);
    }
}
