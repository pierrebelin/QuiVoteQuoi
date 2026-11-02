use chrono::NaiveDate;

use crate::application::ports::group_repository::{GroupRepository, RepositoryError};
use crate::application::use_cases::browse_groups::{merge_lineages, GroupSummary};
use crate::domain::group_profile::{
    MemberCountRange, ParticipationCounts, ParticipationRates, QualityCount,
};

#[derive(Debug, Clone)]
pub struct GetGroupDetailCommand {
    /// Identifiant ou sigle du groupe, ancien ou courant.
    pub group: String,
    pub today: NaiveDate,
}

#[derive(Debug, thiserror::Error)]
pub enum GetGroupDetailError {
    #[error("unknown parliamentary group: {0}")]
    UnknownGroup(String),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

/// Fiche d'un groupe: identite, effectif, participation.
///
/// Les taux sont ceux de la participation — voix exprimees, abstention,
/// non-participation — et jamais du sens du vote. Le detail « pour » et
/// « contre » n'apparait nulle part ici: cumule sur 8 000 scrutins dont 86 %
/// d'amendements, il ne voudrait rien dire et se lirait comme une position
/// (README.md §6, SPEC-PAGES-THEME-GROUPE RM-02).
#[derive(Debug, Clone)]
pub struct GroupProfileView {
    pub summary: GroupSummary,
    /// Deputes distincts ayant appartenu au groupe depuis sa constitution.
    pub total_member_count: i64,
    /// Effectif a la date de reference, par qualite publiee.
    pub qualities: Vec<QualityCount>,
    /// Bornes de l'effectif publie sur les scrutins. H4: aucun taux n'a de base
    /// fixe, et publier une seule valeur obligerait a en choisir une.
    pub published_member_range: Option<MemberCountRange>,
    pub line_count: i64,
    /// Lignes reconstituees depuis les positions nominales (SPEC-scrutins
    /// RM-03): elles portent leur mention de methode partout ou elles comptent.
    pub reconstructed_count: i64,
    /// Lignes ou aucun membre du groupe ne figure. H5 en denombre 8 834 sur la
    /// legislature; sans ce compte, elles disparaissent dans le denominateur.
    pub silent_line_count: i64,
    pub counts: ParticipationCounts,
    /// `None` quand la source ne publie aucune position pour ce groupe.
    pub rates: Option<ParticipationRates>,
}

pub struct GetGroupDetail<'a> {
    repository: &'a dyn GroupRepository,
}

impl<'a> GetGroupDetail<'a> {
    pub fn new(repository: &'a dyn GroupRepository) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        command: GetGroupDetailCommand,
    ) -> Result<GroupProfileView, GetGroupDetailError> {
        let token = command.group.trim();

        let records = self.repository.list_groups(command.today).await?;
        // Un groupe inconnu est refuse, jamais remplace par un groupe
        // approchant: servir la fiche d'un autre groupe sous l'adresse demandee
        // serait une fausse information (README.md §3.1).
        let summary = merge_lineages(records)
            .into_iter()
            .find(|group| group.designated_by(token))
            .ok_or_else(|| GetGroupDetailError::UnknownGroup(token.to_string()))?;

        let statistics = self
            .repository
            .statistics(&summary.uids, summary.reference_date)
            .await?;

        let rates = ParticipationRates::from_counts(&statistics.counts);
        let published_member_range = statistics
            .min_published_member_count
            .zip(statistics.max_published_member_count)
            .and_then(|(min, max)| MemberCountRange::new(min, max));

        Ok(GroupProfileView {
            summary,
            total_member_count: statistics.total_member_count,
            qualities: statistics.qualities,
            published_member_range,
            line_count: statistics.line_count,
            reconstructed_count: statistics.reconstructed_count,
            silent_line_count: statistics.silent_line_count,
            counts: statistics.counts,
            rates,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::use_cases::browse_groups::tests::{
        date, record, renamed_repository, today, InMemoryGroupRepository,
    };

    async fn profile(
        repository: &InMemoryGroupRepository,
        group: &str,
    ) -> Result<GroupProfileView, GetGroupDetailError> {
        GetGroupDetail::new(repository)
            .execute(GetGroupDetailCommand {
                group: group.to_string(),
                today: today(),
            })
            .await
    }

    fn counted_repository() -> InMemoryGroupRepository {
        let repository = InMemoryGroupRepository::default();
        *repository.records.lock().unwrap() = vec![record("PO845401", "RN", 123, 8_434)];
        {
            let mut statistics = repository.statistics.lock().unwrap();
            statistics.total_member_count = 131;
            statistics.min_published_member_count = Some(121);
            statistics.max_published_member_count = Some(125);
            statistics.line_count = 8_434;
            statistics.reconstructed_count = 12;
            statistics.silent_line_count = 40;
            statistics.counts = ParticipationCounts {
                votes_for: 500_000,
                votes_against: 300_000,
                abstentions: 100_000,
                not_voting: 60_000,
                voluntary_not_voting: 40_000,
            };
        }
        repository
    }

    #[tokio::test]
    async fn the_rates_decompose_participation_over_the_published_positions() {
        let view = profile(&counted_repository(), "RN").await.unwrap();
        let rates = view.rates.unwrap();

        assert_eq!(rates.base, 1_000_000);
        assert_eq!(rates.expressed_per_mille, 800);
        assert_eq!(rates.abstention_per_mille, 100);
        assert_eq!(rates.absence_per_mille, 100);
    }

    #[tokio::test]
    async fn the_headcount_range_keeps_both_bounds() {
        let view = profile(&counted_repository(), "RN").await.unwrap();
        let range = view.published_member_range.unwrap();

        assert_eq!((range.min(), range.max()), (121, 125));
        assert!(!range.is_stable());
        assert_eq!(view.total_member_count, 131);
    }

    #[tokio::test]
    async fn a_group_the_source_never_counts_has_no_rate() {
        let repository = InMemoryGroupRepository::default();
        *repository.records.lock().unwrap() = vec![record("PO845520", "AD", 0, 0)];

        let view = profile(&repository, "AD").await.unwrap();

        // Zero position publiee: afficher « 0 % d'abstention » laisserait croire
        // a une mesure, alors qu'il n'y a rien a mesurer.
        assert!(view.rates.is_none());
        assert!(view.published_member_range.is_none());
    }

    #[tokio::test]
    async fn a_renamed_group_is_asked_for_under_both_of_its_identifiers() {
        let repository = renamed_repository();

        for token in ["UDR", "UDDPLR", "PO847173", "PO872880"] {
            let view = profile(&repository, token).await.unwrap();
            assert_eq!(view.summary.abbrev, "UDDPLR", "jeton {token}");
        }
    }

    #[tokio::test]
    async fn the_statistics_of_a_renamed_group_cover_all_of_its_identifiers() {
        let repository = renamed_repository();

        profile(&repository, "UDR").await.unwrap();

        // Interroger le seul identifiant courant amputerait la fiche des
        // 3 053 scrutins d'avant le renommage.
        let uids = repository.last_uids.lock().unwrap().clone().unwrap();
        assert_eq!(uids.len(), 2);
        assert!(uids.contains(&"PO847173".to_string()));
        assert!(uids.contains(&"PO872880".to_string()));
    }

    #[tokio::test]
    async fn a_dissolved_group_is_counted_on_its_last_day() {
        let repository = InMemoryGroupRepository::default();
        let mut dissolved = record("PO845520", "AD", 15, 12);
        dissolved.end_date = Some(date(2024, 9, 11));
        dissolved.reference_date = date(2024, 9, 11);
        *repository.records.lock().unwrap() = vec![dissolved];

        let view = profile(&repository, "AD").await.unwrap();

        // Le compter a la date du jour donnerait zero membre, ce qui se lit
        // comme une donnee manquante et non comme une dissolution.
        assert_eq!(view.summary.reference_date, date(2024, 9, 11));
        assert_eq!(
            *repository.last_reference_date.lock().unwrap(),
            Some(date(2024, 9, 11))
        );
        assert_eq!(view.summary.member_count, 15);
    }

    #[tokio::test]
    async fn an_unknown_group_is_refused_rather_than_approximated() {
        let error = profile(&counted_repository(), "PS").await.unwrap_err();

        assert!(matches!(error, GetGroupDetailError::UnknownGroup(g) if g == "PS"));
    }
}
