//! Lecture de « En bref » sur la fiche d'un dossier (SPEC-en-bref CU-03,
//! RM-13, RM-14).

use std::collections::{BTreeSet, HashMap};

use chrono::NaiveDateTime;

use crate::application::ports::brief_generator::BriefGenerator;
use crate::application::ports::brief_line_repository::BriefLineRepository;
use crate::application::ports::dossier_repository::DossierRepository;
use crate::application::ports::text_version_repository::TextVersionRepository;
use crate::application::ports::RepositoryError;
use crate::domain::dossier::DossierUid;
use crate::domain::law_text::ArticleFingerprint;
use crate::domain::text_brief::{ArticleBriefState, StoredBriefLine, RULE_MODEL};
use crate::domain::text_version::{
    latest_available, newer_unavailable, newest_first, TextStatus, TextVersion,
};

#[derive(Debug, thiserror::Error)]
pub enum GetDossierBriefError {
    #[error("dossier not found: {0}")]
    NotFound(String),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

/// Un article de la version montrée, avec ce que « En bref » en dit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArticleBriefView {
    /// Numéro tel que l'en-tête l'écrit (`1er`, `19 bis`).
    pub label: String,
    pub division: Option<String>,
    pub state: ArticleBriefState,
}

/// « En bref » d'un dossier : une version, ses articles dans l'ordre du texte,
/// et la traçabilité des lignes montrées.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DossierBrief {
    /// Toutes les versions du dossier, de la plus récente à la plus ancienne.
    pub versions: Vec<TextVersion>,
    /// La version montrée : celle qui est demandée, à défaut la dernière
    /// disponible. `None` : le dossier n'en a aucune (RM-14).
    pub shown: Option<TextVersion>,
    /// `None` quand aucune version n'est montrée.
    pub text_status: Option<TextStatus>,
    /// La version montrée est la dernière disponible : ses articles sans ligne
    /// sont en attente, pas indisponibles (CU-03, étape 5).
    pub is_latest: bool,
    /// RM-14 : version plus récente que celle qui est montrée, dont le texte
    /// n'est pas publié par l'Assemblée.
    pub newer_unavailable_version: Option<TextVersion>,
    pub articles: Vec<ArticleBriefView>,
    /// Modèles distincts des lignes montrées, `rule` exclu (RM-16).
    pub models: Vec<String>,
    /// Dernière génération parmi les lignes que ces modèles ont produites.
    pub generated_at: Option<NaiveDateTime>,
}

pub struct GetDossierBrief<'a> {
    dossiers: &'a dyn DossierRepository,
    versions: &'a dyn TextVersionRepository,
    lines: &'a dyn BriefLineRepository,
    generator: &'a dyn BriefGenerator,
}

impl<'a> GetDossierBrief<'a> {
    pub fn new(
        dossiers: &'a dyn DossierRepository,
        versions: &'a dyn TextVersionRepository,
        lines: &'a dyn BriefLineRepository,
        generator: &'a dyn BriefGenerator,
    ) -> Self {
        Self {
            dossiers,
            versions,
            lines,
            generator,
        }
    }

    /// `version` : la référence du document à montrer ; sans elle, la dernière
    /// version disponible. `Ok(None)` : cette référence n'est pas une version
    /// du dossier.
    pub async fn execute(
        &self,
        dossier: &DossierUid,
        version: Option<&str>,
    ) -> Result<Option<DossierBrief>, GetDossierBriefError> {
        if self.dossiers.find_by_uid(dossier).await?.is_none() {
            return Err(GetDossierBriefError::NotFound(dossier.to_string()));
        }

        let versions = newest_first(&self.versions.versions_of(dossier).await?);
        let latest = latest_available(&versions);
        let shown = match version {
            None => latest,
            Some(reference) => {
                match versions
                    .iter()
                    .find(|known| known.document_uid().as_str() == reference)
                {
                    Some(known) => Some(known),
                    None => return Ok(None),
                }
            }
        };
        let Some(shown) = shown else {
            return Ok(Some(DossierBrief {
                versions,
                shown: None,
                text_status: None,
                is_latest: false,
                newer_unavailable_version: None,
                articles: Vec::new(),
                models: Vec::new(),
                generated_at: None,
            }));
        };

        let is_latest = latest.is_some_and(|latest| latest.document_uid() == shown.document_uid());
        let text_status = shown.text_status();
        let newer_unavailable_version = newer_unavailable(&versions, shown).cloned();
        let shown = shown.clone();

        let (articles, models, generated_at) = if text_status == TextStatus::Read {
            self.articles_of(&shown, is_latest).await?
        } else {
            (Vec::new(), Vec::new(), None)
        };

        Ok(Some(DossierBrief {
            versions,
            shown: Some(shown),
            text_status: Some(text_status),
            is_latest,
            newer_unavailable_version,
            articles,
            models,
            generated_at,
        }))
    }

    /// Les articles de la version lue, chacun avec la ligne de son empreinte,
    /// et les modèles et la date de génération des lignes montrées.
    async fn articles_of(
        &self,
        shown: &TextVersion,
        is_latest: bool,
    ) -> Result<(Vec<ArticleBriefView>, Vec<String>, Option<NaiveDateTime>), RepositoryError> {
        let articles = self
            .versions
            .articles_of(shown.document_uid())
            .await?
            .unwrap_or_default();
        let fingerprints: Vec<ArticleFingerprint> = articles
            .iter()
            .map(|article| article.fingerprint().clone())
            .collect();
        let stored = if fingerprints.is_empty() {
            Vec::new()
        } else {
            self.lines.lines_for(&fingerprints).await?
        };
        let by_fingerprint: HashMap<&str, &StoredBriefLine> = stored
            .iter()
            .map(|line| (line.fingerprint.as_str(), line))
            .collect();

        let prompt_version = self.generator.prompt_version();
        let mut models = BTreeSet::new();
        let mut generated_at: Option<NaiveDateTime> = None;
        let mut views = Vec::with_capacity(articles.len());
        for article in &articles {
            let line = by_fingerprint.get(article.fingerprint().as_str()).copied();
            let state = ArticleBriefState::of(line, is_latest, prompt_version);
            if let (ArticleBriefState::Ready(_), Some(line)) = (&state, line) {
                if line.model != RULE_MODEL {
                    models.insert(line.model.clone());
                    generated_at = generated_at.max(Some(line.generated_at));
                }
            }
            views.push(ArticleBriefView {
                label: article.label().to_string(),
                division: article.division().map(str::to_string),
                state,
            });
        }
        Ok((views, models.into_iter().collect(), generated_at))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::NaiveDate;

    use crate::application::ports::dossier_repository::{DossierCriteria, DossierPage};
    use crate::application::use_cases::brief_fakes::{
        FakeBriefGenerator, InMemoryBriefLineRepository, InMemoryTextVersionRepository,
    };
    use crate::domain::dossier::{
        DossierOutcome, LegislativeDossier, NewLegislativeDossier, Score,
    };
    use crate::domain::law_text::{ArticleFingerprint, NewVersionArticle, VersionArticle};
    use crate::domain::text_brief::BriefStatus;
    use crate::domain::text_version::{
        Chamber, DocumentUid, NewTextVersion, ReadingStatus, VersionKind,
    };

    const DOSSIER: &str = "DLR5L17N51670";
    const OLD_DEPOSIT: &str = "PIONANR5L17B1100";
    const ADOPTED: &str = "PIONANR5L17BTA0200";
    const SENATE_ADOPTED: &str = "PIONSNR5S479BTA0043";

    fn day(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn dossier_uid(raw: &str) -> DossierUid {
        DossierUid::new(raw.to_string()).unwrap()
    }

    fn sample_dossier(uid: &str) -> LegislativeDossier {
        LegislativeDossier::new(NewLegislativeDossier {
            uid: dossier_uid(uid),
            title: "Proposition de loi".into(),
            procedure: "PPL".into(),
            legislature: 17,
            url: None,
            summary: None,
            deposit_date: Some(day(2025, 5, 27)),
            last_activity_date: day(2026, 3, 1),
            last_activity_label: "Vote solennel".into(),
            acts: vec![],
            documents: vec![],
            score: Score::new(9, 10, 4, 85).unwrap(),
            current_stage: None,
            initiators: vec![],
            committee: None,
            outcome: DossierOutcome::NoRecordedConclusion,
        })
    }

    // Local: seul `find_by_uid` est lu par ce use case ; aucun double partagé
    // du dépôt des dossiers n'existe, chaque use case garde le sien.
    struct KnownDossiers(Vec<&'static str>);

    #[async_trait]
    impl DossierRepository for KnownDossiers {
        async fn save_all(
            &self,
            _dossiers: &[LegislativeDossier],
        ) -> Result<usize, RepositoryError> {
            unreachable!()
        }

        async fn load_states(
            &self,
        ) -> Result<HashMap<String, crate::domain::dossier::StoredDossierState>, RepositoryError>
        {
            unreachable!()
        }

        async fn find_recent(
            &self,
            _since: NaiveDate,
        ) -> Result<Vec<LegislativeDossier>, RepositoryError> {
            unreachable!()
        }

        async fn find_page(
            &self,
            _criteria: &DossierCriteria,
            _order: crate::application::ports::dossier_repository::DossierOrder,
            _limit: i64,
            _offset: i64,
        ) -> Result<DossierPage, RepositoryError> {
            unreachable!()
        }

        async fn find_by_uid(
            &self,
            uid: &DossierUid,
        ) -> Result<Option<LegislativeDossier>, RepositoryError> {
            Ok(self
                .0
                .contains(&uid.as_str())
                .then(|| sample_dossier(uid.as_str())))
        }

        async fn find_by_uids(
            &self,
            _uids: &[crate::domain::dossier::DossierUid],
        ) -> Result<Vec<LegislativeDossier>, RepositoryError> {
            unreachable!()
        }
    }

    struct Fixture {
        dossiers: KnownDossiers,
        versions: InMemoryTextVersionRepository,
        lines: InMemoryBriefLineRepository,
        generator: FakeBriefGenerator,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                dossiers: KnownDossiers(vec![DOSSIER]),
                versions: InMemoryTextVersionRepository::default(),
                lines: InMemoryBriefLineRepository::default(),
                generator: FakeBriefGenerator::default(),
            }
        }

        fn use_case(&self) -> GetDossierBrief<'_> {
            GetDossierBrief::new(&self.dossiers, &self.versions, &self.lines, &self.generator)
        }

        fn add_version(
            &self,
            uid: &str,
            kind: VersionKind,
            chamber: Chamber,
            on: NaiveDate,
            reading: ReadingStatus,
        ) {
            self.add_version_of(DOSSIER, uid, kind, chamber, on, reading);
        }

        fn add_version_of(
            &self,
            dossier: &str,
            uid: &str,
            kind: VersionKind,
            chamber: Chamber,
            on: NaiveDate,
            reading: ReadingStatus,
        ) {
            self.versions
                .versions
                .lock()
                .unwrap()
                .push(TextVersion::new(NewTextVersion {
                    document_uid: DocumentUid::new(uid.to_string()).unwrap(),
                    dossier_uid: dossier_uid(dossier),
                    kind,
                    chamber,
                    deposited_on: on,
                    title: "titre".to_string(),
                    reading,
                }));
        }

        /// Une version de l'Assemblée lue, avec ses articles.
        fn add_read_version(
            &self,
            uid: &str,
            kind: VersionKind,
            on: NaiveDate,
            articles: Vec<VersionArticle>,
        ) {
            self.add_version(uid, kind, Chamber::Assembly, on, ReadingStatus::Read);
            self.versions
                .articles
                .lock()
                .unwrap()
                .insert(uid.to_string(), articles);
        }

        fn add_line(&self, line: StoredBriefLine) {
            self.lines
                .lines
                .lock()
                .unwrap()
                .insert(line.fingerprint.as_str().to_string(), line);
        }
    }

    fn article(
        label: &str,
        division: Option<&str>,
        rank: u16,
        fingerprint: &str,
    ) -> VersionArticle {
        VersionArticle::new(NewVersionArticle {
            label: label.to_string(),
            division: division.map(str::to_string),
            rank,
            content: format!("Contenu de l'article {label}."),
            fingerprint: ArticleFingerprint::new(fingerprint.to_string()),
        })
    }

    fn stored(
        fingerprint: &str,
        status: BriefStatus,
        model: &str,
        prompt_version: &str,
        generated_at: NaiveDateTime,
    ) -> StoredBriefLine {
        StoredBriefLine {
            fingerprint: ArticleFingerprint::new(fingerprint.to_string()),
            status,
            line: (status == BriefStatus::Ready).then(|| format!("Ligne de {fingerprint}.")),
            rejection: (status == BriefStatus::Rejected).then(|| "too_long".to_string()),
            rejection_detail: None,
            rejected_line: None,
            model: model.to_string(),
            prompt_version: prompt_version.to_string(),
            generated_at,
            cited_sources: Vec::new(),
        }
    }

    fn at(year: i32, month: u32, date: u32, hour: u32) -> NaiveDateTime {
        day(year, month, date).and_hms_opt(hour, 0, 0).unwrap()
    }

    fn ready(fingerprint: &str) -> StoredBriefLine {
        stored(
            fingerprint,
            BriefStatus::Ready,
            "modele-essai",
            "prompt-essai-v1",
            at(2026, 11, 3, 8),
        )
    }

    fn uids(versions: &[TextVersion]) -> Vec<&str> {
        versions.iter().map(|v| v.document_uid().as_str()).collect()
    }

    fn states(brief: &DossierBrief) -> Vec<(&str, &ArticleBriefState)> {
        brief
            .articles
            .iter()
            .map(|a| (a.label.as_str(), &a.state))
            .collect()
    }

    /// Un dossier dont la dernière version est le texte adopté, lu, avec un
    /// article déjà résumé.
    fn fixture_with_deposit_and_adoption() -> Fixture {
        let fixture = Fixture::new();
        fixture.add_read_version(
            OLD_DEPOSIT,
            VersionKind::Deposited,
            day(2025, 5, 27),
            vec![article("1er", None, 1, "fp-a")],
        );
        fixture.add_read_version(
            ADOPTED,
            VersionKind::Adopted,
            day(2026, 3, 1),
            vec![
                article("1er", None, 1, "fp-a"),
                article("2", None, 2, "fp-b"),
            ],
        );
        fixture.add_line(ready("fp-a"));
        fixture
    }

    #[tokio::test]
    async fn should_serve_latest_available_version_when_no_version_is_asked() {
        let fixture = fixture_with_deposit_and_adoption();

        let brief = fixture
            .use_case()
            .execute(&dossier_uid(DOSSIER), None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            brief.shown.as_ref().unwrap().document_uid().as_str(),
            ADOPTED
        );
        assert!(brief.is_latest);
        assert_eq!(brief.text_status, Some(TextStatus::Read));
        assert_eq!(uids(&brief.versions), [ADOPTED, OLD_DEPOSIT]);
        assert_eq!(
            states(&brief),
            [
                (
                    "1er",
                    &ArticleBriefState::Ready("Ligne de fp-a.".to_string())
                ),
                ("2", &ArticleBriefState::Pending),
            ]
        );
    }

    #[tokio::test]
    async fn should_serve_requested_version_when_it_belongs_to_the_dossier() {
        let fixture = Fixture::new();
        fixture.add_read_version(
            OLD_DEPOSIT,
            VersionKind::Deposited,
            day(2025, 5, 27),
            vec![
                article("1er", None, 1, "fp-a"),
                article("2", None, 2, "fp-b"),
                article("3", None, 3, "fp-c"),
            ],
        );
        fixture.add_read_version(
            ADOPTED,
            VersionKind::Adopted,
            day(2026, 3, 1),
            vec![article("1er", None, 1, "fp-a")],
        );
        fixture.add_line(ready("fp-a"));
        fixture.add_line(stored(
            "fp-c",
            BriefStatus::Rejected,
            "modele-essai",
            "prompt-essai-v1",
            at(2026, 11, 3, 8),
        ));

        let brief = fixture
            .use_case()
            .execute(&dossier_uid(DOSSIER), Some(OLD_DEPOSIT))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            brief.shown.as_ref().unwrap().document_uid().as_str(),
            OLD_DEPOSIT
        );
        assert!(!brief.is_latest);
        assert_eq!(
            states(&brief),
            [
                (
                    "1er",
                    &ArticleBriefState::Ready("Ligne de fp-a.".to_string())
                ),
                ("2", &ArticleBriefState::Unavailable),
                ("3", &ArticleBriefState::Unavailable),
            ]
        );
    }

    #[tokio::test]
    async fn should_answer_none_when_requested_version_is_unknown() {
        let fixture = fixture_with_deposit_and_adoption();
        fixture.add_version_of(
            "DLR5L17N99999",
            "PIONANR5L17B2222",
            VersionKind::Deposited,
            Chamber::Assembly,
            day(2025, 1, 1),
            ReadingStatus::Read,
        );

        for reference in ["PIONANR5L17B9999", "PIONANR5L17B2222", ""] {
            let brief = fixture
                .use_case()
                .execute(&dossier_uid(DOSSIER), Some(reference))
                .await
                .unwrap();

            assert_eq!(brief, None, "reference {reference:?}");
        }
    }

    #[tokio::test]
    async fn should_list_every_article_in_text_order_when_some_have_no_line() {
        let fixture = Fixture::new();
        fixture.add_read_version(
            ADOPTED,
            VersionKind::Adopted,
            day(2026, 3, 1),
            vec![
                article("1er", Some("TITRE Ier"), 1, "fp-a"),
                article("2", Some("TITRE Ier"), 2, "fp-b"),
                article("3", Some("TITRE II > Chapitre Ier"), 3, "fp-c"),
                article("3 bis", None, 4, "fp-d"),
                article("4", None, 5, "fp-e"),
            ],
        );
        fixture.add_line(ready("fp-a"));
        fixture.add_line(stored(
            "fp-c",
            BriefStatus::Rejected,
            "modele-essai",
            "prompt-essai-v1",
            at(2026, 11, 3, 8),
        ));
        fixture.add_line(stored(
            "fp-d",
            BriefStatus::Rejected,
            "modele-essai",
            "prompt-ancien",
            at(2026, 11, 2, 8),
        ));

        let brief = fixture
            .use_case()
            .execute(&dossier_uid(DOSSIER), None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            states(&brief),
            [
                (
                    "1er",
                    &ArticleBriefState::Ready("Ligne de fp-a.".to_string())
                ),
                ("2", &ArticleBriefState::Pending),
                ("3", &ArticleBriefState::Unavailable),
                ("3 bis", &ArticleBriefState::Pending),
                ("4", &ArticleBriefState::Pending),
            ]
        );
        let divisions: Vec<Option<&str>> = brief
            .articles
            .iter()
            .map(|a| a.division.as_deref())
            .collect();
        assert_eq!(
            divisions,
            [
                Some("TITRE Ier"),
                Some("TITRE Ier"),
                Some("TITRE II > Chapitre Ier"),
                None,
                None
            ]
        );
    }

    #[tokio::test]
    async fn should_signal_newer_senate_version_when_one_exists() {
        let fixture = fixture_with_deposit_and_adoption();
        fixture.add_version(
            SENATE_ADOPTED,
            VersionKind::Adopted,
            Chamber::Senate,
            day(2026, 4, 1),
            ReadingStatus::Unread,
        );

        let brief = fixture
            .use_case()
            .execute(&dossier_uid(DOSSIER), None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            brief.shown.as_ref().unwrap().document_uid().as_str(),
            ADOPTED
        );
        assert_eq!(
            brief
                .newer_unavailable_version
                .as_ref()
                .map(|v| v.document_uid().as_str()),
            Some(SENATE_ADOPTED)
        );
        assert_eq!(
            uids(&brief.versions),
            [SENATE_ADOPTED, ADOPTED, OLD_DEPOSIT]
        );
    }

    #[tokio::test]
    async fn should_not_signal_a_newer_version_when_none_is_unavailable() {
        let fixture = fixture_with_deposit_and_adoption();

        let brief = fixture
            .use_case()
            .execute(&dossier_uid(DOSSIER), None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(brief.newer_unavailable_version, None);
    }

    #[tokio::test]
    async fn should_serve_no_shown_version_when_dossier_has_none_available() {
        let fixture = Fixture::new();
        fixture.add_version(
            SENATE_ADOPTED,
            VersionKind::Adopted,
            Chamber::Senate,
            day(2026, 4, 1),
            ReadingStatus::Unread,
        );

        let brief = fixture
            .use_case()
            .execute(&dossier_uid(DOSSIER), None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(brief.shown, None);
        assert_eq!(brief.text_status, None);
        assert!(!brief.is_latest);
        assert_eq!(uids(&brief.versions), [SENATE_ADOPTED]);
        assert!(brief.articles.is_empty());
        assert!(brief.models.is_empty());
        assert_eq!(brief.generated_at, None);
    }

    #[tokio::test]
    async fn should_serve_no_shown_version_when_dossier_has_no_version_at_all() {
        let fixture = Fixture::new();

        let brief = fixture
            .use_case()
            .execute(&dossier_uid(DOSSIER), None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(brief.shown, None);
        assert!(brief.versions.is_empty());
        assert!(brief.articles.is_empty());
    }

    #[tokio::test]
    async fn should_fail_not_found_when_dossier_is_unknown() {
        let fixture = fixture_with_deposit_and_adoption();

        let error = fixture
            .use_case()
            .execute(&dossier_uid("DLR5L17N00000"), None)
            .await
            .unwrap_err();

        assert!(
            matches!(error, GetDossierBriefError::NotFound(ref uid) if uid == "DLR5L17N00000"),
            "{error:?}"
        );
    }

    #[tokio::test]
    async fn should_state_text_status_when_requested_version_was_not_read() {
        let fixture = fixture_with_deposit_and_adoption();
        fixture.add_version(
            "PIONANR5L17B3000",
            VersionKind::Deposited,
            Chamber::Assembly,
            day(2025, 8, 1),
            ReadingStatus::Unread,
        );
        fixture.add_version(
            "PIONANR5L17B3100",
            VersionKind::Deposited,
            Chamber::Assembly,
            day(2025, 9, 1),
            ReadingStatus::Missing,
        );
        fixture.add_version(
            SENATE_ADOPTED,
            VersionKind::Adopted,
            Chamber::Senate,
            day(2025, 10, 1),
            ReadingStatus::Unread,
        );

        for (reference, status) in [
            ("PIONANR5L17B3000", TextStatus::Unread),
            ("PIONANR5L17B3100", TextStatus::Missing),
            (SENATE_ADOPTED, TextStatus::SenateOnly),
            (OLD_DEPOSIT, TextStatus::Read),
        ] {
            let brief = fixture
                .use_case()
                .execute(&dossier_uid(DOSSIER), Some(reference))
                .await
                .unwrap()
                .unwrap();

            assert_eq!(
                brief.shown.as_ref().unwrap().document_uid().as_str(),
                reference
            );
            assert_eq!(brief.text_status, Some(status), "{reference}");
            assert_eq!(brief.articles.is_empty(), status != TextStatus::Read);
        }
    }

    #[tokio::test]
    async fn should_serve_the_unread_latest_version_without_any_line() {
        let fixture = fixture_with_deposit_and_adoption();
        fixture.add_version(
            "PIONANR5L17BTA0300",
            VersionKind::Adopted,
            Chamber::Assembly,
            day(2026, 5, 1),
            ReadingStatus::Unread,
        );

        let brief = fixture
            .use_case()
            .execute(&dossier_uid(DOSSIER), None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            brief.shown.as_ref().unwrap().document_uid().as_str(),
            "PIONANR5L17BTA0300"
        );
        assert!(brief.is_latest);
        assert_eq!(brief.text_status, Some(TextStatus::Unread));
        assert!(brief.articles.is_empty());
    }

    #[tokio::test]
    async fn should_list_distinct_models_without_rule_when_lines_are_shown() {
        let fixture = Fixture::new();
        fixture.add_read_version(
            ADOPTED,
            VersionKind::Adopted,
            day(2026, 3, 1),
            vec![
                article("1er", None, 1, "fp-a"),
                article("2", None, 2, "fp-b"),
                article("3", None, 3, "fp-c"),
                article("4", None, 4, "fp-d"),
                article("5", None, 5, "fp-e"),
            ],
        );
        let model_line = |fingerprint: &str, model: &str, generated_at| {
            stored(
                fingerprint,
                BriefStatus::Ready,
                model,
                "prompt-essai-v1",
                generated_at,
            )
        };
        fixture.add_line(model_line("fp-a", "mistral-a", at(2026, 11, 1, 8)));
        fixture.add_line(model_line("fp-b", "mistral-a", at(2026, 11, 2, 8)));
        fixture.add_line(model_line("fp-c", "claude-b", at(2026, 11, 1, 9)));
        // Écrite par le code, plus tard que les autres : ni modèle ni date.
        fixture.add_line(model_line("fp-d", RULE_MODEL, at(2026, 11, 3, 8)));
        // Refusée : pas une ligne montrée.
        fixture.add_line(stored(
            "fp-e",
            BriefStatus::Rejected,
            "refuse-c",
            "prompt-essai-v1",
            at(2026, 11, 3, 9),
        ));
        // Hors de la version montrée.
        fixture.add_line(model_line("fp-z", "absent-d", at(2026, 11, 4, 8)));

        let brief = fixture
            .use_case()
            .execute(&dossier_uid(DOSSIER), None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(brief.models, ["claude-b", "mistral-a"]);
        assert_eq!(brief.generated_at, Some(at(2026, 11, 2, 8)));
    }

    #[tokio::test]
    async fn should_fail_when_the_repository_is_unreachable() {
        let fixture = fixture_with_deposit_and_adoption();
        *fixture.versions.unreachable.lock().unwrap() = true;

        let error = fixture
            .use_case()
            .execute(&dossier_uid(DOSSIER), None)
            .await
            .unwrap_err();

        assert!(
            matches!(error, GetDossierBriefError::Repository(_)),
            "{error:?}"
        );
    }
}
