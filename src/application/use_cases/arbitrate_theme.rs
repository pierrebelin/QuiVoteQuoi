use chrono::NaiveDate;

use crate::application::ports::theme_repository::{RepositoryError, ThemeRepository};
use crate::domain::theme::{FamilyCode, SubjectRef, ThemeAssignment, ThemeError, MAX_FAMILIES};

#[derive(Debug, thiserror::Error)]
pub enum ArbitrationError {
    #[error("unknown subject: {kind}/{id}")]
    UnknownSubject { kind: String, id: String },
    #[error("at most {MAX_FAMILIES} families, {0} requested")]
    TooManyFamilies(usize),
    #[error("dropping every family requires a motive")]
    MotiveRequired,
    #[error(transparent)]
    Domain(#[from] ThemeError),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

/// Ce que le mainteneur soumet depuis l'ecran d'arbitrage.
#[derive(Debug, Clone)]
pub struct ArbitrationCommand {
    pub subject_kind: String,
    pub subject_id: String,
    pub families: Vec<FamilyCode>,
    pub author: String,
    pub motive: Option<String>,
}

/// CU-03 — Arbitrer une proposition.
///
/// L'arbitrage ne modifie rien en place: il clot les rattachements courants et
/// en ouvre de nouveaux a la date du jour (RM-07). Une liste vide ecarte tout,
/// et demande un motif.
pub struct ArbitrateTheme<'a> {
    repository: &'a dyn ThemeRepository,
}

impl<'a> ArbitrateTheme<'a> {
    pub fn new(repository: &'a dyn ThemeRepository) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        command: ArbitrationCommand,
        on: NaiveDate,
    ) -> Result<Vec<ThemeAssignment>, ArbitrationError> {
        let subject = SubjectRef::parse(&command.subject_kind, command.subject_id.clone())
            .ok_or_else(|| ArbitrationError::UnknownSubject {
                kind: command.subject_kind.clone(),
                id: command.subject_id.clone(),
            })?;

        let mut families = command.families;
        families.dedup_by(|a, b| a == b);
        let mut unique: Vec<FamilyCode> = Vec::with_capacity(families.len());
        for family in families {
            if !unique.contains(&family) {
                unique.push(family);
            }
        }
        if unique.len() > MAX_FAMILIES {
            return Err(ArbitrationError::TooManyFamilies(unique.len()));
        }
        if unique.is_empty() && command.motive.as_deref().unwrap_or("").trim().is_empty() {
            return Err(ArbitrationError::MotiveRequired);
        }

        let opened = unique
            .into_iter()
            .map(|family| {
                ThemeAssignment::open(
                    subject.clone(),
                    family,
                    on,
                    command.author.clone(),
                    command.motive.clone(),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        self.repository
            .replace_assignments(&subject, on, &opened)
            .await?;
        Ok(opened)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::use_cases::theme_fakes::InMemoryThemeRepository;
    use crate::domain::theme::TextKey;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 8, 3).unwrap()
    }

    fn later() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 15).unwrap()
    }

    fn command(families: Vec<FamilyCode>) -> ArbitrationCommand {
        ArbitrationCommand {
            subject_kind: "text".into(),
            subject_id: "projet de loi de finances pour 2026".into(),
            families,
            author: "mainteneur".into(),
            motive: Some("motif".into()),
        }
    }

    fn subject() -> SubjectRef {
        SubjectRef::Text(TextKey::from_raw("projet de loi de finances pour 2026"))
    }

    async fn seed_proposal(repository: &InMemoryThemeRepository, family: FamilyCode) {
        let assignment =
            ThemeAssignment::open(subject(), family, today(), "modèle".into(), None).unwrap();
        repository
            .replace_assignments(&subject(), today(), &[assignment])
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn arbitration_replaces_the_proposal_and_keeps_it_readable() {
        let repository = InMemoryThemeRepository::default();
        seed_proposal(&repository, FamilyCode::Numerique).await;

        ArbitrateTheme::new(&repository)
            .execute(command(vec![FamilyCode::PouvoirAchat]), later())
            .await
            .unwrap();

        assert_eq!(
            repository.current_families(&subject()),
            vec![FamilyCode::PouvoirAchat]
        );
        assert_eq!(repository.closed_count(&subject()), 1);
        let history = repository.assignment_history(&subject()).await.unwrap();
        let closed = history.iter().find(|a| !a.is_current()).unwrap();
        assert_eq!(closed.family(), FamilyCode::Numerique);
        assert_eq!(closed.closed_on(), Some(later()));
    }

    #[tokio::test]
    async fn arbitration_marks_the_assignment_as_human() {
        let repository = InMemoryThemeRepository::default();

        let opened = ArbitrateTheme::new(&repository)
            .execute(command(vec![FamilyCode::Logement]), today())
            .await
            .unwrap();

        assert_eq!(opened[0].author(), "mainteneur");
    }

    #[tokio::test]
    async fn four_families_are_refused() {
        let repository = InMemoryThemeRepository::default();

        let result = ArbitrateTheme::new(&repository)
            .execute(
                command(vec![
                    FamilyCode::Logement,
                    FamilyCode::Sante,
                    FamilyCode::Numerique,
                    FamilyCode::TravailEmploi,
                ]),
                today(),
            )
            .await;

        assert!(matches!(result, Err(ArbitrationError::TooManyFamilies(4))));
        assert!(repository.current_families(&subject()).is_empty());
    }

    #[tokio::test]
    async fn a_repeated_family_counts_once() {
        let repository = InMemoryThemeRepository::default();

        ArbitrateTheme::new(&repository)
            .execute(
                command(vec![
                    FamilyCode::Logement,
                    FamilyCode::Logement,
                    FamilyCode::Sante,
                ]),
                today(),
            )
            .await
            .unwrap();

        assert_eq!(repository.current_families(&subject()).len(), 2);
    }

    #[tokio::test]
    async fn dropping_every_family_requires_a_motive() {
        let repository = InMemoryThemeRepository::default();
        seed_proposal(&repository, FamilyCode::Numerique).await;
        let mut cmd = command(vec![]);
        cmd.motive = None;

        let result = ArbitrateTheme::new(&repository).execute(cmd, later()).await;

        assert!(matches!(result, Err(ArbitrationError::MotiveRequired)));
        assert_eq!(
            repository.current_families(&subject()),
            vec![FamilyCode::Numerique]
        );
    }

    #[tokio::test]
    async fn dropping_every_family_with_a_motive_leaves_the_text_unassigned() {
        let repository = InMemoryThemeRepository::default();
        seed_proposal(&repository, FamilyCode::Numerique).await;

        ArbitrateTheme::new(&repository)
            .execute(command(vec![]), later())
            .await
            .unwrap();

        assert!(repository.current_families(&subject()).is_empty());
        assert_eq!(repository.closed_count(&subject()), 1);
    }

    #[tokio::test]
    async fn an_unknown_subject_kind_is_refused() {
        let repository = InMemoryThemeRepository::default();
        let mut cmd = command(vec![FamilyCode::Logement]);
        cmd.subject_kind = "scrutin".into();

        let result = ArbitrateTheme::new(&repository).execute(cmd, today()).await;

        assert!(matches!(
            result,
            Err(ArbitrationError::UnknownSubject { .. })
        ));
    }
}
