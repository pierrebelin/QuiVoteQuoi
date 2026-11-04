//! Essai réel de « En bref » sur un seul dossier, lancé à la main.
//!
//! Exécute la seule passe `RefreshTextBriefs` sur le dossier
//! `BRIEF_TRIAL_DOSSIER`, contre la base `DATABASE_URL`, puis affiche le
//! rapport et les lignes de la dernière version lue. Ni thématisation, ni
//! synthèse par groupe, ni téléchargement de l'archive de l'Assemblée : la
//! liste des versions n'est pas relue, seuls les textes encore non lus du
//! dossier le sont.
//!
//! Aucun `.env` n'est chargé : tout vient de l'environnement de la commande.
//! Le modèle du fournisseur `LLM_PROVIDER` n'est branché que si
//! `BRIEF_TRIAL_LIVE=1` et sa clé sont posés ; sinon la passe tourne sans
//! modèle et n'envoie rien.
//! Légifrance n'est branché que si ses identifiants sont posés.
//!
//! ```text
//! DATABASE_URL=postgresql://... BRIEF_TRIAL_DOSSIER=DLR5L17N54629 \
//!   cargo test --test text_brief_trial -- --ignored --nocapture
//! ```

use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use chrono::Utc;
use quivotequoi::application::ports::brief_generator::{
    BriefGenerator, BriefRequest, GeneratedLine, SummaryGeneratorError,
};
use quivotequoi::application::ports::legal_text_source::LegalTextSource;
use quivotequoi::application::ports::text_version_repository::{
    RepositoryError, TextReading, TextVersionRepository,
};
use quivotequoi::application::ports::text_version_source::{SourceError, TextVersionSource};
use quivotequoi::application::use_cases::refresh_text_briefs::RefreshTextBriefs;
use quivotequoi::domain::cited_article::CitedArticle;
use quivotequoi::domain::dossier::DossierUid;
use quivotequoi::domain::law_text::VersionArticle;
use quivotequoi::domain::text_version::{
    latest_available, BriefCandidate, DocumentUid, TextVersion,
};
use quivotequoi::infrastructure::config;
use quivotequoi::infrastructure::legifrance::client::LegifranceClient;
use quivotequoi::infrastructure::legifrance::unavailable::UnavailableLegalTextSource;
use quivotequoi::infrastructure::llm::anthropic_brief_generator::AnthropicBriefGenerator;
use quivotequoi::infrastructure::llm::mistral_brief_generator::MistralBriefGenerator;
use quivotequoi::infrastructure::llm::openai_brief_generator::OpenAiBriefGenerator;
use quivotequoi::infrastructure::llm::provider::LlmProvider;
use quivotequoi::infrastructure::llm::unavailable_brief_generator::UnavailableBriefGenerator;
use quivotequoi::infrastructure::national_assembly::text_version_client::TextVersionClient;
use quivotequoi::infrastructure::persistence::pg_brief_line_repository::PgBriefLineRepository;
use quivotequoi::infrastructure::persistence::pg_text_version_repository::PgTextVersionRepository;

/// Les versions déjà enregistrées, sans relire l'archive ; les textes non lus
/// sont lus chez l'Assemblée.
struct RecordedVersionsOnly(TextVersionClient);

#[async_trait]
impl TextVersionSource for RecordedVersionsOnly {
    async fn fetch_versions(&self) -> Result<Vec<TextVersion>, SourceError> {
        Ok(Vec::new())
    }

    async fn version_articles(
        &self,
        version: &TextVersion,
    ) -> Result<Option<Vec<VersionArticle>>, SourceError> {
        self.0.version_articles(version).await
    }
}

/// Le dépôt Postgres, réduit au seul dossier essayé parmi les dossiers votés.
struct OneDossier {
    inner: PgTextVersionRepository,
    dossier: String,
}

#[async_trait]
impl TextVersionRepository for OneDossier {
    async fn save_versions(&self, versions: &[TextVersion]) -> Result<(), RepositoryError> {
        self.inner.save_versions(versions).await
    }

    async fn versions_of(&self, dossier: &DossierUid) -> Result<Vec<TextVersion>, RepositoryError> {
        self.inner.versions_of(dossier).await
    }

    async fn voted_dossiers(&self) -> Result<Vec<BriefCandidate>, RepositoryError> {
        let mut candidates = self.inner.voted_dossiers().await?;
        candidates.retain(|candidate| candidate.dossier_uid.as_str() == self.dossier);
        Ok(candidates)
    }

    async fn versions_of_voted(&self) -> Result<Vec<TextVersion>, RepositoryError> {
        let mut versions = self.inner.versions_of_voted().await?;
        versions.retain(|version| version.dossier_uid().as_str() == self.dossier);
        Ok(versions)
    }

    async fn articles_of(
        &self,
        document_uid: &DocumentUid,
    ) -> Result<Option<Vec<VersionArticle>>, RepositoryError> {
        self.inner.articles_of(document_uid).await
    }

    async fn save_reading(
        &self,
        document_uid: &DocumentUid,
        reading: &TextReading,
    ) -> Result<(), RepositoryError> {
        self.inner.save_reading(document_uid, reading).await
    }
}

/// Le générateur, avec le nombre d'appels qu'il a reçus : une deuxième passe
/// sur le même texte doit en compter zéro.
struct CountedCalls {
    inner: Box<dyn BriefGenerator>,
    calls: AtomicUsize,
}

#[async_trait]
impl BriefGenerator for CountedCalls {
    async fn generate(
        &self,
        request: &BriefRequest,
    ) -> Result<Vec<GeneratedLine>, SummaryGeneratorError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        println!("appel au modèle : {} articles", request.articles.len());
        self.inner.generate(request).await
    }

    fn model(&self) -> &str {
        self.inner.model()
    }

    fn prompt_version(&self) -> &str {
        self.inner.prompt_version()
    }
}

/// Article, statut, ligne ou refus, modèle, prompt, sources citées.
type BriefRow = (String, String, String, String, String, serde_json::Value);

#[tokio::test]
#[ignore = "essai réel : base, dossier et clé du modèle à poser à la main"]
async fn brief_trial_on_one_dossier() {
    let dossier = std::env::var("BRIEF_TRIAL_DOSSIER").expect("BRIEF_TRIAL_DOSSIER required");
    let db = config::try_connect_database()
        .await
        .expect("DATABASE_URL required");

    let live = std::env::var("BRIEF_TRIAL_LIVE").as_deref() == Ok("1");
    let provider = LlmProvider::from_env().expect("LLM_PROVIDER readable");
    let chosen: Option<Box<dyn BriefGenerator>> = match provider {
        _ if !live => None,
        LlmProvider::Anthropic => AnthropicBriefGenerator::from_env().map(|g| Box::new(g) as _),
        LlmProvider::OpenAi => OpenAiBriefGenerator::from_env().map(|g| Box::new(g) as _),
        LlmProvider::Mistral => MistralBriefGenerator::from_env().map(|g| Box::new(g) as _),
    };
    assert!(
        !live || chosen.is_some(),
        "BRIEF_TRIAL_LIVE=1 sans clé pour {}",
        provider.label()
    );
    let generator = CountedCalls {
        inner: chosen.unwrap_or_else(|| Box::new(UnavailableBriefGenerator)),
        calls: AtomicUsize::new(0),
    };
    let legal: Box<dyn LegalTextSource> = match LegifranceClient::from_env() {
        Some(client) => Box::new(client),
        None => Box::new(UnavailableLegalTextSource),
    };
    println!(
        "dossier {dossier} — modèle {} (prompt {}) — Légifrance {}",
        generator.model(),
        generator.prompt_version(),
        if LegifranceClient::from_env().is_some() {
            "branché"
        } else {
            "absent"
        }
    );

    let source = RecordedVersionsOnly(TextVersionClient::new());
    let versions = OneDossier {
        inner: PgTextVersionRepository::new(db.clone()),
        dossier: dossier.clone(),
    };

    // Sans Légifrance, seuls les articles sans renvoi reconnu partent au
    // modèle (RM-08) : leur nombre et leur taille chiffrent l'appel.
    let recorded = versions
        .versions_of(&DossierUid::new(dossier.clone()).expect("dossier uid"))
        .await
        .expect("versions readable");
    if let Some(latest) = latest_available(&recorded) {
        let articles = versions
            .articles_of(latest.document_uid())
            .await
            .expect("articles readable")
            .unwrap_or_default();
        let uncited: Vec<&VersionArticle> = articles
            .iter()
            .filter(|article| CitedArticle::cited_in(article.content()).is_empty())
            .collect();
        println!(
            "{} : {} articles, {} sans renvoi reconnu ({} caractères)",
            latest.document_uid().as_str(),
            articles.len(),
            uncited.len(),
            uncited
                .iter()
                .map(|a| a.content().chars().count())
                .sum::<usize>()
        );
    }
    let lines = PgBriefLineRepository::new(db.clone());
    let report = RefreshTextBriefs::new(&source, &versions, legal.as_ref(), &lines, &generator)
        .execute(1, Utc::now().date_naive())
        .await;
    println!("{report:#?}");
    println!(
        "appels au modèle : {}",
        generator.calls.load(Ordering::SeqCst)
    );

    let rows: Vec<BriefRow> = sqlx::query_as(
        "SELECT a.label, COALESCE(b.status, 'pending'), \
                    COALESCE(b.line, b.rejection \
                        || COALESCE(' (' || b.rejection_detail || ')', '') \
                        || COALESCE(' : « ' || b.rejected_line || ' »', ''), ''), \
                    COALESCE(b.model, ''), COALESCE(b.prompt_version, ''), \
                    COALESCE(b.cited_sources, '[]'::jsonb) \
             FROM dossier_brief_progress p \
             JOIN text_version_articles a ON a.document_uid = p.document_uid \
             LEFT JOIN brief_lines b ON b.fingerprint = a.fingerprint \
             WHERE p.dossier_uid = $1 \
             ORDER BY a.rank",
    )
    .bind(&dossier)
    .fetch_all(&db)
    .await
    .expect("brief lines readable");
    for (label, status, text, model, prompt, cited) in rows {
        println!("art. {label} [{status}] {text} — {model} {prompt} {cited}");
    }
}
