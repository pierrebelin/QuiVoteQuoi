use crate::application::ports::final_vote_repository::{
    FinalVoteFilter, FinalVoteRecord, FinalVoteRepository, GroupOption, GroupTallyRecord,
    RepositoryError,
};
use crate::application::ports::theme_repository::AssignedFamily;
use crate::domain::actor::{ActorError, GroupUid};
use crate::domain::final_vote::{reading_of, FinalVote, GroupIdentity, GroupStance};
use crate::domain::group_lineage::{
    designates, displayed_group, group_by_lineage, same_group, uids_of,
};
use crate::domain::scrutin::{Outcome, ScrutinError, VotePosition};
use crate::domain::theme::FamilyCode;

/// Quatre groupes au plus. La comparaison reste une lecture cote a cote, ou
/// chaque groupe garde ses chiffres bruts; au-dela, les colonnes se resserrent
/// au point qu'il ne reste que les pourcentages, et la page se lit comme un
/// classement — interdit par README.md §6.
pub const MAX_COMPARED_GROUPS: usize = 4;
pub const DEFAULT_PAGE_SIZE: i64 = 20;
pub const MAX_PAGE_SIZE: i64 = 100;

#[derive(Debug, Clone, Default)]
pub struct BrowseFinalVotesCommand {
    pub family: Option<String>,
    /// Groupes demandes, par identifiant ou par sigle, dans l'ordre d'affichage.
    pub groups: Vec<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, thiserror::Error)]
pub enum BrowseFinalVotesError {
    #[error("at most {MAX_COMPARED_GROUPS} groups can be compared, got {0}")]
    TooManyGroups(usize),
    #[error("unknown parliamentary group: {0}")]
    UnknownGroup(String),
    #[error("unknown theme family: {0}")]
    UnknownFamily(String),
    #[error(transparent)]
    Scrutin(#[from] ScrutinError),
    #[error(transparent)]
    Actor(#[from] ActorError),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

/// Un vote et ses rattachements thematiques.
///
/// Les familles restent hors de l'agregat: elles viennent du texte debattu, pas
/// du vote, et le domaine du vote n'a pas a en dependre.
#[derive(Debug, Clone)]
pub struct FinalVoteEntry {
    pub vote: FinalVote,
    pub families: Vec<AssignedFamily>,
}

#[derive(Debug, Clone)]
pub struct FinalVoteView {
    pub items: Vec<FinalVoteEntry>,
    /// Votes correspondant au filtre.
    pub total: i64,
    /// Votes sur l'ensemble, filtre thematique exclu. Rend visible ce que le
    /// filtre laisse de cote (README.md §2).
    pub total_unfiltered: i64,
    /// Votes sur l'ensemble deja rattaches a une famille. Un filtre par theme
    /// ne peut rien trouver au-dela: l'avancement de la thematisation est
    /// affiche plutot que subi.
    pub total_with_family: i64,
    /// Referentiel complet, pour le selecteur de groupes.
    pub groups: Vec<GroupOption>,
    /// Groupes retenus, dans l'ordre demande.
    pub selected: Vec<GroupOption>,
}

/// CU-07 — Consulter les votes sur l'ensemble d'un texte, groupe par groupe.
pub struct BrowseFinalVotes<'a> {
    repository: &'a dyn FinalVoteRepository,
}

impl<'a> BrowseFinalVotes<'a> {
    pub fn new(repository: &'a dyn FinalVoteRepository) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        command: BrowseFinalVotesCommand,
    ) -> Result<FinalVoteView, BrowseFinalVotesError> {
        if command.groups.len() > MAX_COMPARED_GROUPS {
            return Err(BrowseFinalVotesError::TooManyGroups(command.groups.len()));
        }

        let family = match command.family.as_deref().filter(|f| !f.is_empty()) {
            Some(raw) => Some(
                FamilyCode::parse(raw)
                    .map_err(|_| BrowseFinalVotesError::UnknownFamily(raw.to_string()))?,
            ),
            None => None,
        };

        let groups = merge_renamed_groups(self.repository.groups().await?);
        let selected = resolve_groups(&groups, &command.groups)?;

        let filter = FinalVoteFilter {
            family,
            // Un groupe renomme repond a plusieurs identifiants: les demander
            // tous evite que ses votes d'avant le changement de nom
            // disparaissent de la comparaison (README.md §2).
            group_uids: selected
                .iter()
                .flat_map(|group| uids_of(&group.uid))
                .map(str::to_string)
                .collect(),
            limit: command
                .limit
                .unwrap_or(DEFAULT_PAGE_SIZE)
                .clamp(1, MAX_PAGE_SIZE),
            offset: command.offset.unwrap_or(0).max(0),
        };

        let page = self.repository.list_final_votes(&filter).await?;
        let totals = self.repository.totals().await?;

        let items = page
            .items
            .into_iter()
            .map(|record| build_entry(record, &selected))
            .collect::<Result<_, _>>()?;

        Ok(FinalVoteView {
            items,
            total: page.total,
            total_unfiltered: totals.total,
            total_with_family: totals.with_family,
            groups,
            selected,
        })
    }
}

/// Rassemble en une seule entree les identifiants successifs d'un groupe
/// renomme.
///
/// Sans ce repli, le selecteur propose deux fois le meme groupe et chacune des
/// deux entrees est vide sur la periode de l'autre: le visiteur en conclut que
/// le groupe n'a pas vote. La couverture affichee est la somme des periodes,
/// pas celle de la derniere.
fn merge_renamed_groups(groups: Vec<GroupOption>) -> Vec<GroupOption> {
    let mut merged: Vec<GroupOption> = group_by_lineage(groups, |group| group.uid.as_str())
        .iter()
        .map(|group| {
            let options = group.members();
            let own = group.representative();
            let shown = group.displayed(&own.abbrev, &own.label);

            GroupOption {
                uid: shown.uid.to_string(),
                abbrev: shown.abbrev.to_string(),
                label: shown.label.to_string(),
                color: options.iter().find_map(|option| option.color.clone()),
                final_vote_count: options.iter().map(|option| option.final_vote_count).sum(),
            }
        })
        .collect();

    // Le depot trie par couverture decroissante; la fusion additionne des
    // comptes et defait ce tri.
    merged.sort_by(|a, b| {
        b.final_vote_count
            .cmp(&a.final_vote_count)
            .then_with(|| a.abbrev.cmp(&b.abbrev))
    });
    merged
}

/// Un groupe est designe par son identifiant ou par son sigle. Le sigle rend
/// l'adresse partageable (`?groupes=RN,SOC`), l'identifiant la rend stable
/// (README.md §8.1). Un groupe renomme repond aussi a son ancien sigle.
fn resolve_groups(
    known: &[GroupOption],
    requested: &[String],
) -> Result<Vec<GroupOption>, BrowseFinalVotesError> {
    let mut resolved: Vec<GroupOption> = Vec::with_capacity(requested.len());

    for token in requested {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }

        let group = known
            .iter()
            .find(|group| designates(&group.uid, &group.abbrev, token))
            .cloned()
            .ok_or_else(|| BrowseFinalVotesError::UnknownGroup(token.to_string()))?;

        // Deux jetons pour un meme groupe — `?groupes=UDR,UDDPLR` — donneraient
        // deux colonnes identiques. Le doublon est absorbe, pas refuse: l'adresse
        // reste valide.
        if !resolved.iter().any(|kept| kept.uid == group.uid) {
            resolved.push(group);
        }
    }

    Ok(resolved)
}

fn build_entry(
    record: FinalVoteRecord,
    selected: &[GroupOption],
) -> Result<FinalVoteEntry, BrowseFinalVotesError> {
    // L'ordre des positions suit l'ordre demande, pas celui de la base: la
    // colonne de gauche doit rester le groupe choisi en premier.
    let stances = selected
        .iter()
        .filter_map(|group| {
            record
                .tallies
                .iter()
                .find(|tally| same_group(&group.uid, &tally.group_uid))
                .map(build_stance)
        })
        .collect::<Result<_, _>>()?;

    let outcome = Outcome::new(record.outcome_code, record.outcome_label)?;

    let vote = FinalVote {
        reading: reading_of(&record.subject, &record.text_label),
        scrutin_uid: record.scrutin_uid,
        number: record.number,
        date: record.date,
        ballot_type_label: record.ballot_type_label,
        outcome,
        text_key: record.text_key,
        text_label: record.text_label,
        dossier_uid: record.dossier_uid,
        dossier_label: record.dossier_label,
        official_text: record.official_text,
        synthesis: record.synthesis,
        stances,
    };

    Ok(FinalVoteEntry {
        vote,
        families: record.families,
    })
}

/// Position d'un groupe sur un vote.
///
/// Sous une lignee, l'identite affichee est celle de la lignee et non celle que
/// la ventilation porte (`displayed_group`).
fn build_stance(record: &GroupTallyRecord) -> Result<GroupStance, ActorError> {
    let shown = displayed_group(&record.group_uid, &record.abbrev, &record.label);
    let identity = GroupIdentity {
        uid: GroupUid::new(shown.uid.to_string())?,
        abbrev: shown.abbrev.to_string(),
        label: shown.label.to_string(),
        color: record.color.clone(),
    };

    Ok(GroupStance::new(
        identity,
        record.member_count,
        record
            .majority_position
            .as_deref()
            .and_then(VotePosition::parse),
        record.tally.clone(),
    ))
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::NaiveDate;

    use super::*;
    use crate::application::ports::final_vote_repository::{FinalVotePage, FinalVoteTotals};
    use crate::domain::scrutin::VoteTally;

    #[derive(Default)]
    struct InMemoryFinalVoteRepository {
        records: Mutex<Vec<FinalVoteRecord>>,
        groups: Mutex<Vec<GroupOption>>,
        last_filter: Mutex<Option<FinalVoteFilter>>,
    }

    #[async_trait]
    impl FinalVoteRepository for InMemoryFinalVoteRepository {
        async fn list_final_votes(
            &self,
            filter: &FinalVoteFilter,
        ) -> Result<FinalVotePage, RepositoryError> {
            *self.last_filter.lock().unwrap() = Some(filter.clone());
            let records = self.records.lock().unwrap();
            let items: Vec<FinalVoteRecord> = records
                .iter()
                .filter(|record| match filter.family {
                    Some(family) => record.families.iter().any(|f| f.family == family),
                    None => true,
                })
                .cloned()
                .collect();
            let total = items.len() as i64;
            Ok(FinalVotePage {
                items: items
                    .into_iter()
                    .skip(filter.offset as usize)
                    .take(filter.limit as usize)
                    .collect(),
                total,
            })
        }

        async fn groups(&self) -> Result<Vec<GroupOption>, RepositoryError> {
            Ok(self.groups.lock().unwrap().clone())
        }

        async fn totals(&self) -> Result<FinalVoteTotals, RepositoryError> {
            let records = self.records.lock().unwrap();
            Ok(FinalVoteTotals {
                total: records.len() as i64,
                with_family: records.iter().filter(|r| !r.families.is_empty()).count() as i64,
            })
        }
    }

    fn group(uid: &str, abbrev: &str) -> GroupOption {
        GroupOption {
            uid: uid.to_string(),
            abbrev: abbrev.to_string(),
            label: format!("Groupe {abbrev}"),
            color: None,
            final_vote_count: 1,
        }
    }

    fn tally_record(
        uid: &str,
        abbrev: &str,
        votes_for: u16,
        votes_against: u16,
    ) -> GroupTallyRecord {
        GroupTallyRecord {
            group_uid: uid.to_string(),
            abbrev: abbrev.to_string(),
            label: format!("Groupe {abbrev}"),
            color: None,
            member_count: Some(60),
            majority_position: Some(
                if votes_for >= votes_against {
                    "for"
                } else {
                    "against"
                }
                .to_string(),
            ),
            tally: VoteTally {
                votes_for,
                votes_against,
                abstentions: 0,
                not_voting: 0,
                voluntary_not_voting: 0,
            },
        }
    }

    fn record(uid: &str, tallies: Vec<GroupTallyRecord>) -> FinalVoteRecord {
        FinalVoteRecord {
            scrutin_uid: uid.to_string(),
            number: "42".to_string(),
            date: NaiveDate::from_ymd_opt(2026, 5, 27).unwrap(),
            subject: "l'ensemble de la proposition de loi sur le logement (première lecture)."
                .to_string(),
            ballot_type_label: "scrutin public solennel".to_string(),
            outcome_code: "adopté".to_string(),
            outcome_label: "Adopté".to_string(),
            text_key: "proposition de loi sur le logement".to_string(),
            text_label: "proposition de loi sur le logement".to_string(),
            dossier_uid: None,
            dossier_label: None,
            official_text: None,
            synthesis: VoteTally {
                votes_for: 300,
                votes_against: 200,
                abstentions: 10,
                not_voting: 2,
                voluntary_not_voting: 0,
            },
            families: Vec::new(),
            tallies,
        }
    }

    fn repository() -> InMemoryFinalVoteRepository {
        let repository = InMemoryFinalVoteRepository::default();
        *repository.groups.lock().unwrap() = vec![group("PO1", "RN"), group("PO2", "SOC")];
        *repository.records.lock().unwrap() = vec![record(
            "S1",
            vec![
                tally_record("PO1", "RN", 2, 84),
                tally_record("PO2", "SOC", 60, 0),
            ],
        )];
        repository
    }

    #[tokio::test]
    async fn a_record_without_outcome_code_is_reported_not_panicked_on() {
        let repository = repository();
        repository.records.lock().unwrap()[0].outcome_code = String::new();

        let error = BrowseFinalVotes::new(&repository)
            .execute(BrowseFinalVotesCommand::default())
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            BrowseFinalVotesError::Scrutin(ScrutinError::EmptyOutcomeCode)
        ));
    }

    #[tokio::test]
    async fn two_groups_are_returned_in_the_requested_order() {
        let repository = repository();

        let view = BrowseFinalVotes::new(&repository)
            .execute(BrowseFinalVotesCommand {
                groups: vec!["SOC".to_string(), "RN".to_string()],
                ..Default::default()
            })
            .await
            .unwrap();

        let stances = &view.items[0].vote.stances;
        assert_eq!(stances.len(), 2);
        assert_eq!(stances[0].group().abbrev, "SOC");
        assert_eq!(stances[1].group().abbrev, "RN");
    }

    #[tokio::test]
    async fn a_group_is_found_by_uid_as_well_as_by_abbrev() {
        let repository = repository();

        let view = BrowseFinalVotes::new(&repository)
            .execute(BrowseFinalVotesCommand {
                groups: vec!["PO1".to_string()],
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(view.selected[0].abbrev, "RN");
    }

    #[tokio::test]
    async fn the_share_is_computed_on_voters() {
        let repository = repository();

        let view = BrowseFinalVotes::new(&repository)
            .execute(BrowseFinalVotesCommand {
                groups: vec!["RN".to_string()],
                ..Default::default()
            })
            .await
            .unwrap();

        let share = view.items[0].vote.stances[0].share().cloned().unwrap();
        assert_eq!(share.voters(), 86);
        assert_eq!(share.against_percent(), 98);
        assert_eq!(share.for_percent(), 2);
    }

    #[tokio::test]
    async fn a_group_absent_from_a_scrutin_yields_no_stance() {
        let repository = repository();
        *repository.groups.lock().unwrap() =
            vec![group("PO1", "RN"), group("PO2", "SOC"), group("PO3", "UDR")];

        let view = BrowseFinalVotes::new(&repository)
            .execute(BrowseFinalVotesCommand {
                groups: vec!["UDR".to_string()],
                ..Default::default()
            })
            .await
            .unwrap();

        // Le groupe est bien selectionne, mais le scrutin ne porte aucune ligne
        // pour lui: rien n'est invente, l'absence reste visible a l'affichage.
        assert_eq!(view.selected.len(), 1);
        assert!(view.items[0].vote.stances.is_empty());
    }

    #[tokio::test]
    async fn an_unknown_group_is_refused_rather_than_ignored() {
        let repository = repository();

        let error = BrowseFinalVotes::new(&repository)
            .execute(BrowseFinalVotesCommand {
                groups: vec!["PS".to_string()],
                ..Default::default()
            })
            .await
            .unwrap_err();

        assert!(matches!(error, BrowseFinalVotesError::UnknownGroup(g) if g == "PS"));
    }

    #[tokio::test]
    async fn comparing_more_groups_than_allowed_is_refused() {
        let repository = repository();
        let requested: Vec<String> = (0..MAX_COMPARED_GROUPS + 1)
            .map(|i| format!("PO{i}"))
            .collect();

        let error = BrowseFinalVotes::new(&repository)
            .execute(BrowseFinalVotesCommand {
                groups: requested,
                ..Default::default()
            })
            .await
            .unwrap_err();

        // Le refus tombe avant la resolution des sigles: la limite porte sur la
        // demande, pas sur ce qu'elle designe.
        assert!(matches!(
            error,
            BrowseFinalVotesError::TooManyGroups(n) if n == MAX_COMPARED_GROUPS + 1
        ));
    }

    /// Depot portant un groupe renomme: l'ancien identifiant sur un vote, le
    /// nouveau sur l'autre — la situation exacte de UDR devenu UDDPLR.
    fn renamed_group_repository() -> InMemoryFinalVoteRepository {
        let repository = InMemoryFinalVoteRepository::default();
        *repository.groups.lock().unwrap() = vec![
            group("PO1", "RN"),
            group("PO872880", "UDDPLR"),
            group("PO847173", "UDR"),
        ];
        *repository.records.lock().unwrap() = vec![
            record("S1", vec![tally_record("PO872880", "UDDPLR", 16, 0)]),
            record("S2", vec![tally_record("PO847173", "UDR", 0, 16)]),
        ];
        repository
    }

    #[tokio::test]
    async fn a_renamed_group_is_offered_once_with_the_coverage_of_both_periods() {
        let repository = renamed_group_repository();

        let view = BrowseFinalVotes::new(&repository)
            .execute(BrowseFinalVotesCommand::default())
            .await
            .unwrap();

        let abbrevs: Vec<&str> = view.groups.iter().map(|g| g.abbrev.as_str()).collect();
        assert_eq!(abbrevs, vec!["UDDPLR", "RN"]);
        assert_eq!(view.groups[0].uid, "PO872880");
        assert_eq!(view.groups[0].final_vote_count, 2);
    }

    #[tokio::test]
    async fn a_renamed_group_keeps_its_votes_from_before_the_rename() {
        let repository = renamed_group_repository();

        let view = BrowseFinalVotes::new(&repository)
            .execute(BrowseFinalVotesCommand {
                groups: vec!["UDDPLR".to_string()],
                ..Default::default()
            })
            .await
            .unwrap();

        // Un vote par periode, et la colonne garde le meme sigle sur les deux:
        // le renommage ne doit pas se lire comme deux groupes distincts.
        let stances: Vec<&str> = view
            .items
            .iter()
            .flat_map(|entry| entry.vote.stances.iter())
            .map(|stance| stance.group().abbrev.as_str())
            .collect();
        assert_eq!(stances, vec!["UDDPLR", "UDDPLR"]);
    }

    #[tokio::test]
    async fn the_two_names_of_a_renamed_group_yield_one_column() {
        let repository = renamed_group_repository();

        let view = BrowseFinalVotes::new(&repository)
            .execute(BrowseFinalVotesCommand {
                groups: vec!["UDR".to_string(), "UDDPLR".to_string()],
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(view.selected.len(), 1);
    }

    #[tokio::test]
    async fn the_former_abbrev_still_resolves_to_the_group() {
        let repository = renamed_group_repository();

        let view = BrowseFinalVotes::new(&repository)
            .execute(BrowseFinalVotesCommand {
                groups: vec!["UDR".to_string()],
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(view.selected.len(), 1);
        assert_eq!(view.selected[0].uid, "PO872880");
    }

    #[tokio::test]
    async fn an_unknown_family_is_refused() {
        let repository = repository();

        let error = BrowseFinalVotes::new(&repository)
            .execute(BrowseFinalVotesCommand {
                family: Some("agriculture".to_string()),
                ..Default::default()
            })
            .await
            .unwrap_err();

        assert!(matches!(error, BrowseFinalVotesError::UnknownFamily(f) if f == "agriculture"));
    }

    #[tokio::test]
    async fn the_page_size_stays_within_bounds() {
        let repository = repository();

        BrowseFinalVotes::new(&repository)
            .execute(BrowseFinalVotesCommand {
                limit: Some(5_000),
                offset: Some(-3),
                ..Default::default()
            })
            .await
            .unwrap();

        let filter = repository.last_filter.lock().unwrap().clone().unwrap();
        assert_eq!(filter.limit, MAX_PAGE_SIZE);
        assert_eq!(filter.offset, 0);
    }

    #[tokio::test]
    async fn the_unfiltered_total_stays_visible_under_a_theme_filter() {
        let repository = repository();
        // Un vote rattache au logement, un autre sans famille: le filtre en
        // retient un, le total complet reste affichable.
        let mut themed = record("S2", Vec::new());
        themed.families = vec![AssignedFamily {
            family: FamilyCode::Logement,
            opened_on: NaiveDate::from_ymd_opt(2026, 8, 1).unwrap(),
            motive: None,
        }];
        repository.records.lock().unwrap().push(themed);

        let view = BrowseFinalVotes::new(&repository)
            .execute(BrowseFinalVotesCommand {
                family: Some("logement".to_string()),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(view.total, 1);
        assert_eq!(view.total_unfiltered, 2);
        assert_eq!(view.total_with_family, 1);
    }

    #[tokio::test]
    async fn an_official_text_is_exposed_only_when_explicitly_referenced() {
        let repository = repository();
        let retrieved_at = chrono::DateTime::parse_from_rfc3339("2026-08-16T08:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        repository.records.lock().unwrap()[0].official_text =
            Some(crate::domain::final_vote::OfficialTextVersion {
                document_uid: "PIONANR5L17B1560".to_string(),
                document_title: "Proposition de loi n° 1560".to_string(),
                version_label: "Texte adopté en première lecture".to_string(),
                document_published_on: Some(NaiveDate::from_ymd_opt(2026, 5, 27).unwrap()),
                official_url: "https://www.assemblee-nationale.fr/dyn/docs/PIONANR5L17B1560.raw"
                    .to_string(),
                mapping_source_url: "https://www.assemblee-nationale.fr/dyn/17/scrutins/42"
                    .to_string(),
                source_producer: "Assemblée nationale".to_string(),
                source_license: "Licence Ouverte / Open Licence".to_string(),
                source_metadata_fingerprint: Some("sha256:test".to_string()),
                source_retrieved_at: retrieved_at,
            });

        let view = BrowseFinalVotes::new(&repository)
            .execute(BrowseFinalVotesCommand::default())
            .await
            .unwrap();

        let text = view.items[0].vote.official_text.as_ref().unwrap();
        assert_eq!(text.document_uid, "PIONANR5L17B1560");
        assert_eq!(text.version_label, "Texte adopté en première lecture");
    }
}
