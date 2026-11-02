use chrono::NaiveDate;

use crate::application::ports::group_repository::{GroupRecord, GroupRepository, RepositoryError};
use crate::domain::group_lineage::{designates, group_by_lineage, LineageGroup};
use crate::domain::group_profile::VotingWindow;

#[derive(Debug, Clone)]
pub struct BrowseGroupsCommand {
    /// Date a laquelle un groupe actif est compte. Fournie par l'appelant
    /// plutot que lue ici: le use case reste rejouable a l'identique.
    pub today: NaiveDate,
}

#[derive(Debug, thiserror::Error)]
pub enum BrowseGroupsError {
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

/// Un groupe de la liste, lignee rapprochee.
///
/// Aucun taux ici. Treize taux d'abstention alignes en colonnes forment un
/// tableau qui se lit comme un classement, ce que README.md §6 interdit —
/// alors que les memes chiffres sur la fiche d'un seul groupe restent une
/// description. Les taux vivent donc dans `get_group_detail`.
#[derive(Debug, Clone)]
pub struct GroupSummary {
    /// Identifiant d'affichage: le canonique de la lignee quand il y en a une.
    pub uid: String,
    /// Tous les identifiants sous lesquels la base porte ce groupe.
    pub uids: Vec<String>,
    pub abbrev: String,
    /// Sigles anterieurs. Affiches: rapprocher deux identifiants est une
    /// decision editoriale, elle se montre au lieu de se deviner
    /// (`domain::group_lineage`).
    pub former_abbrevs: Vec<String>,
    pub label: String,
    pub color: Option<String>,
    pub legislature: i16,
    /// Dates publiees par le referentiel, distinctes de la fenetre de vote.
    pub created_on: Option<NaiveDate>,
    pub dissolved_on: Option<NaiveDate>,
    /// Date a laquelle `member_count` est compte.
    pub reference_date: NaiveDate,
    pub member_count: i64,
    /// Scrutins ou la source publie une ligne pour ce groupe.
    pub scrutin_count: i64,
    /// Premier et dernier scrutin ou le groupe apparait. `None` quand la source
    /// ne le nomme sur aucun scrutin.
    pub window: Option<VotingWindow>,
}

impl GroupSummary {
    pub fn is_dissolved(&self) -> bool {
        self.dissolved_on.is_some()
    }

    /// Vrai quand le jeton designe ce groupe, par identifiant ou par sigle,
    /// ancien ou courant. Le sigle rend l'adresse partageable, l'identifiant la
    /// rend stable (README.md §8.1).
    pub fn designated_by(&self, token: &str) -> bool {
        designates(&self.uid, &self.abbrev, token)
    }
}

#[derive(Debug, Clone)]
pub struct GroupListView {
    pub groups: Vec<GroupSummary>,
}

/// Liste des groupes parlementaires de la legislature.
pub struct BrowseGroups<'a> {
    repository: &'a dyn GroupRepository,
}

impl<'a> BrowseGroups<'a> {
    pub fn new(repository: &'a dyn GroupRepository) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        command: BrowseGroupsCommand,
    ) -> Result<GroupListView, BrowseGroupsError> {
        let records = self.repository.list_groups(command.today).await?;
        Ok(GroupListView {
            groups: merge_lineages(records),
        })
    }
}

/// Rassemble en une seule entree les identifiants successifs d'un groupe
/// renomme.
///
/// Sans ce repli, UDR et UDDPLR apparaissent comme deux groupes, le premier
/// dissous et le second sans passe, alors que la source decrit un seul groupe
/// renomme en cours de legislature.
///
/// Les comptes de scrutins s'additionnent — les deux periodes sont disjointes
/// (H3) — mais **pas les effectifs**: a la date de reference du canonique, les
/// appartenances sous l'ancien identifiant sont toutes closes. Les sommer
/// compterait deux fois un depute reste dans le groupe.
pub fn merge_lineages(records: Vec<GroupRecord>) -> Vec<GroupSummary> {
    let mut groups: Vec<GroupSummary> = group_by_lineage(records, |record| record.uid.as_str())
        .iter()
        .map(summarize)
        .collect();

    // Groupes actifs d'abord, par effectif decroissant — l'ordre dont
    // l'Assemblee elle-meme se sert. Les groupes dissous suivent, du plus
    // recemment actif au plus ancien: ils restent visibles, jamais retires
    // (README.md §2).
    groups.sort_by(|a, b| {
        a.is_dissolved()
            .cmp(&b.is_dissolved())
            .then_with(|| b.member_count.cmp(&a.member_count))
            .then_with(|| b.window.map(|w| w.last()).cmp(&a.window.map(|w| w.last())))
            .then_with(|| a.abbrev.cmp(&b.abbrev))
    });
    groups
}

fn summarize(group: &LineageGroup<GroupRecord>) -> GroupSummary {
    let records = group.members();
    // L'effectif, la dissolution et la date de reference sont ceux de la ligne
    // representante, jamais une somme.
    let current = group.representative();
    let shown = group.displayed(&current.abbrev, &current.label);

    GroupSummary {
        uid: group.key().to_string(),
        uids: records.iter().map(|record| record.uid.clone()).collect(),
        abbrev: shown.abbrev.to_string(),
        former_abbrevs: shown
            .former_abbrevs
            .iter()
            .map(|abbrev| abbrev.to_string())
            .collect(),
        label: shown.label.to_string(),
        color: records.iter().find_map(|record| record.color.clone()),
        legislature: current.legislature,
        // La creation du groupe est celle de son premier identifiant; sa
        // dissolution, celle du dernier.
        created_on: records.iter().filter_map(|record| record.start_date).min(),
        dissolved_on: current.end_date,
        reference_date: current.reference_date,
        member_count: current.member_count,
        scrutin_count: records.iter().map(|record| record.scrutin_count).sum(),
        window: records
            .iter()
            .filter_map(|record| {
                VotingWindow::new(record.first_scrutin_date, record.last_scrutin_date)
            })
            .reduce(VotingWindow::span),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;

    use super::*;
    use crate::application::ports::group_repository::GroupStatisticsRecord;

    pub(crate) fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    pub(crate) fn today() -> NaiveDate {
        date(2026, 8, 7)
    }

    #[derive(Default)]
    pub(crate) struct InMemoryGroupRepository {
        pub(crate) records: Mutex<Vec<GroupRecord>>,
        pub(crate) statistics: Mutex<GroupStatisticsRecord>,
        pub(crate) last_uids: Mutex<Option<Vec<String>>>,
        pub(crate) last_reference_date: Mutex<Option<NaiveDate>>,
    }

    #[async_trait]
    impl GroupRepository for InMemoryGroupRepository {
        async fn list_groups(
            &self,
            _today: NaiveDate,
        ) -> Result<Vec<GroupRecord>, RepositoryError> {
            Ok(self.records.lock().unwrap().clone())
        }

        async fn statistics(
            &self,
            group_uids: &[String],
            reference_date: NaiveDate,
        ) -> Result<GroupStatisticsRecord, RepositoryError> {
            *self.last_uids.lock().unwrap() = Some(group_uids.to_vec());
            *self.last_reference_date.lock().unwrap() = Some(reference_date);
            Ok(self.statistics.lock().unwrap().clone())
        }
    }

    pub(crate) fn record(uid: &str, abbrev: &str, members: i64, scrutins: i64) -> GroupRecord {
        GroupRecord {
            uid: uid.to_string(),
            legislature: 17,
            label: format!("Groupe {abbrev}"),
            abbrev: abbrev.to_string(),
            color: Some("#3367A7".to_string()),
            start_date: Some(date(2024, 7, 18)),
            end_date: None,
            reference_date: today(),
            member_count: members,
            scrutin_count: scrutins,
            first_scrutin_date: Some(date(2024, 10, 8)),
            last_scrutin_date: Some(date(2026, 7, 21)),
        }
    }

    /// Le cas UDR devenu UDDPLR: deux identifiants, deux periodes disjointes
    /// qui partitionnent exactement la legislature (H3).
    pub(crate) fn renamed_repository() -> InMemoryGroupRepository {
        let mut former = record("PO847173", "UDR", 16, 3_053);
        former.end_date = Some(date(2025, 7, 10));
        former.reference_date = date(2025, 7, 10);
        former.first_scrutin_date = Some(date(2024, 10, 8));
        former.last_scrutin_date = Some(date(2025, 7, 10));

        let mut current = record("PO872880", "UDDPLR", 16, 5_381);
        current.start_date = Some(date(2025, 9, 8));
        current.first_scrutin_date = Some(date(2025, 9, 8));

        let repository = InMemoryGroupRepository::default();
        *repository.records.lock().unwrap() =
            vec![record("PO845401", "RN", 123, 8_434), former, current];
        repository
    }

    async fn view(repository: &InMemoryGroupRepository) -> GroupListView {
        BrowseGroups::new(repository)
            .execute(BrowseGroupsCommand { today: today() })
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn a_renamed_group_is_listed_once_under_its_current_name() {
        let view = view(&renamed_repository()).await;

        let abbrevs: Vec<&str> = view.groups.iter().map(|g| g.abbrev.as_str()).collect();
        assert_eq!(abbrevs, vec!["RN", "UDDPLR"]);
        assert_eq!(view.groups[1].former_abbrevs, vec!["UDR"]);
    }

    #[tokio::test]
    async fn a_renamed_group_keeps_the_coverage_of_both_periods() {
        let view = view(&renamed_repository()).await;
        let group = &view.groups[1];

        // 3 053 + 5 381 = 8 434, la legislature entiere.
        assert_eq!(group.scrutin_count, 8_434);
        let window = group.window.unwrap();
        assert_eq!(window.first(), date(2024, 10, 8));
        assert_eq!(window.last(), date(2026, 7, 21));
        assert_eq!(group.created_on, Some(date(2024, 7, 18)));
    }

    #[tokio::test]
    async fn the_headcount_of_a_renamed_group_is_not_the_sum_of_its_periods() {
        let view = view(&renamed_repository()).await;

        // 16 sous chaque identifiant, mais un seul groupe de 16 deputes: les
        // appartenances sous l'ancien identifiant sont closes.
        assert_eq!(view.groups[1].member_count, 16);
        assert!(!view.groups[1].is_dissolved());
    }

    #[tokio::test]
    async fn a_renamed_group_merges_whatever_the_order_of_the_rows() {
        let repository = renamed_repository();
        repository.records.lock().unwrap().reverse();

        let view = view(&repository).await;
        let merged = view.groups.iter().find(|g| g.abbrev == "UDDPLR").unwrap();

        assert_eq!(merged.uid, "PO872880");
        assert_eq!(merged.member_count, 16);
        assert_eq!(merged.scrutin_count, 8_434);
    }

    #[tokio::test]
    async fn a_group_without_a_single_vote_line_is_still_listed() {
        let repository = InMemoryGroupRepository::default();
        let mut silent = record("PO845520", "AD", 0, 0);
        silent.end_date = Some(date(2024, 9, 11));
        silent.first_scrutin_date = None;
        silent.last_scrutin_date = None;
        *repository.records.lock().unwrap() = vec![record("PO845401", "RN", 123, 8_434), silent];

        let view = view(&repository).await;

        // Couverture nulle affichee comme telle, jamais retiree de la liste.
        let dissolved = &view.groups[1];
        assert_eq!(dissolved.abbrev, "AD");
        assert_eq!(dissolved.scrutin_count, 0);
        assert!(dissolved.window.is_none());
        assert!(dissolved.is_dissolved());
    }

    #[tokio::test]
    async fn active_groups_come_before_dissolved_ones() {
        let repository = InMemoryGroupRepository::default();
        let mut dissolved = record("PO845520", "AD", 500, 12);
        dissolved.end_date = Some(date(2024, 9, 11));
        *repository.records.lock().unwrap() = vec![dissolved, record("PO845401", "RN", 123, 8_434)];

        let view = view(&repository).await;

        // L'effectif ne fait pas remonter un groupe dissous devant un actif.
        assert_eq!(view.groups[0].abbrev, "RN");
        assert_eq!(view.groups[1].abbrev, "AD");
    }

    #[tokio::test]
    async fn active_groups_are_ordered_by_headcount() {
        let repository = InMemoryGroupRepository::default();
        *repository.records.lock().unwrap() = vec![
            record("PO1", "SOC", 66, 8_434),
            record("PO2", "RN", 123, 8_434),
            record("PO3", "LFI", 71, 8_434),
        ];

        let view = view(&repository).await;

        let abbrevs: Vec<&str> = view.groups.iter().map(|g| g.abbrev.as_str()).collect();
        assert_eq!(abbrevs, vec!["RN", "LFI", "SOC"]);
    }

    #[test]
    fn a_group_answers_to_its_identifier_and_to_both_of_its_abbrevs() {
        let summary = merge_lineages(vec![record("PO872880", "UDDPLR", 16, 5_381)])
            .pop()
            .unwrap();

        assert!(summary.designated_by("PO872880"));
        assert!(summary.designated_by("UDDPLR"));
        assert!(summary.designated_by("uddplr"));
        assert!(summary.designated_by("UDR"));
        assert!(!summary.designated_by("RN"));
    }
}
