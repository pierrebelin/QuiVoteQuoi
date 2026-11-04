use std::collections::{HashMap, HashSet};

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

use crate::application::ports::brief_generator::{
    BriefArticle, BriefCitation, BriefGenerator, BriefRequest, GeneratedLine, SummaryGeneratorError,
};
use crate::application::ports::brief_line_repository::BriefLineRepository;
use crate::application::ports::legal_text_source::LegalTextSource;
use crate::application::ports::text_version_repository::{TextReading, TextVersionRepository};
use crate::application::ports::text_version_source::TextVersionSource;
use crate::application::ports::RepositoryError;
use crate::domain::brief_line::BriefLine;
use crate::domain::cited_article::{CitedArticle, CitedText, CitedWording};
use crate::domain::dossier::DossierUid;
use crate::domain::law_text::{ArticleFingerprint, VersionArticle};
use crate::domain::text_brief::{
    brief_pass_order, plan_article, ArticleAction, BriefProgress, BriefStatus, StoredBriefLine,
    DELETED_ARTICLE_LINE, MAX_GENERATION_FAILURES, RULE_MODEL,
};
use crate::domain::text_version::{latest_available, DocumentUid, ReadingStatus, TextVersion};

/// Articles résumés en un appel au modèle (README §8.1, T4) : au-delà, la
/// version part en paquets successifs. Le projet de loi de finances compte 86
/// articles, plus leurs rédactions citées : un appel unique dépasserait ce
/// qu'une réponse rend sans être tronquée.
pub const BRIEF_ARTICLES_PER_CALL: usize = 40;

/// Rédactions lues chez la source légale par passe, tous dossiers confondus
/// (RM-08). Au-delà, les articles qui citent un article encore inconnu restent
/// en attente de la passe suivante.
pub const MAX_CITED_LOOKUPS_PER_PASS: usize = 200;

/// Motif d'une clé envoyée au modèle qui n'a reçu aucune ligne (RM-13).
const MISSING_LINE: &str = "missing_line";

/// Motif des articles envoyés quand le modèle échoue une troisième fois pour
/// leur version (CU-02, Erreurs).
const GENERATION_FAILED: &str = "generation_failed";

/// Ce qu'une passe « En bref » a fait.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextBriefRefreshReport {
    /// Versions publiées par la source et enregistrées.
    pub versions_seen: usize,
    /// Dossiers retenus par le plafond dont les textes ont été traités.
    pub dossiers_read: usize,
    /// Versions dont le texte a été lu et découpé en articles.
    pub texts_read: usize,
    /// Versions notées manquantes : texte introuvable ou sans article reconnu.
    pub texts_missing: usize,
    /// Articles de dernière version dont l'empreinte avait déjà une ligne
    /// publiable (RM-05).
    pub lines_reused: usize,
    /// Lignes publiables écrites : celles du modèle qui passent les contrôles
    /// et les lignes fixes des articles supprimés (RM-06).
    pub lines_written: usize,
    /// Refus écrits, avec leur motif (RM-13).
    pub lines_rejected: usize,
    /// Articles de dernière version sans ligne à la fin de la passe : une
    /// source manque (RM-08) ou l'appel au modèle a échoué.
    pub articles_pending: usize,
    /// Première défaillance rencontrée (RM-18), sans interrompre la passe
    /// quand elle vient de la source.
    pub anomaly: Option<String>,
}

impl TextBriefRefreshReport {
    fn note_anomaly(&mut self, message: String) {
        tracing::warn!("Text brief pass: {message}");
        self.anomaly.get_or_insert(message);
    }
}

/// CU-01 : enregistre les versions de chaque dossier, puis lit le texte des
/// versions disponibles pas encore lues des dossiers votés retenus par le
/// plafond (RM-15).
///
/// CU-02 : pour la dernière version de chaque dossier retenu, reprend les
/// lignes connues par empreinte, écrit la ligne fixe des articles supprimés,
/// envoie au modèle les seuls articles à résumer dont les sources sont
/// complètes, contrôle chaque ligne reçue et l'enregistre avec sa
/// traçabilité.
pub struct RefreshTextBriefs<'a> {
    source: &'a dyn TextVersionSource,
    versions: &'a dyn TextVersionRepository,
    legal: &'a dyn LegalTextSource,
    lines: &'a dyn BriefLineRepository,
    generator: &'a dyn BriefGenerator,
}

impl<'a> RefreshTextBriefs<'a> {
    pub fn new(
        source: &'a dyn TextVersionSource,
        versions: &'a dyn TextVersionRepository,
        legal: &'a dyn LegalTextSource,
        lines: &'a dyn BriefLineRepository,
        generator: &'a dyn BriefGenerator,
    ) -> Self {
        Self {
            source,
            versions,
            legal,
            lines,
            generator,
        }
    }

    /// Une passe ne fait jamais échouer l'appelant (RM-18) : une défaillance
    /// de la source, du dépôt ou du modèle devient l'anomalie du rapport. La
    /// source qui échoue laisse la passe continuer avec les versions déjà
    /// enregistrées et les autres textes ; le dépôt qui échoue l'arrête.
    pub async fn execute(&self, cap: usize, today: NaiveDate) -> TextBriefRefreshReport {
        let mut report = TextBriefRefreshReport::default();
        if let Err(error) = self.run(cap, today, &mut report).await {
            report.note_anomaly(error.to_string());
        }
        report
    }

    async fn run(
        &self,
        cap: usize,
        today: NaiveDate,
        report: &mut TextBriefRefreshReport,
    ) -> Result<(), RepositoryError> {
        match self.source.fetch_versions().await {
            Ok(published) => {
                self.versions.save_versions(&published).await?;
                report.versions_seen = published.len();
            }
            Err(error) => report.note_anomaly(format!("versions not fetched: {error}")),
        }

        let candidates = self.versions.voted_dossiers().await?;
        let versions = self.versions.versions_of_voted().await?;
        let progress = self.lines.progress().await?;
        let order = brief_pass_order(
            candidates,
            &versions,
            &progress,
            self.generator.prompt_version(),
            today,
            cap,
        );
        let mut lookups = Lookups::new();
        for dossier in &order {
            let versions = self.read_texts_of(dossier, report).await?;
            self.brief(dossier, &versions, today, &mut lookups, report)
                .await?;
            report.dossiers_read += 1;
        }
        Ok(())
    }

    /// Lit les versions disponibles pas encore lues d'un dossier (RM-14) et
    /// rend ses versions avec leur état de lecture à jour.
    async fn read_texts_of(
        &self,
        dossier: &DossierUid,
        report: &mut TextBriefRefreshReport,
    ) -> Result<Vec<TextVersion>, RepositoryError> {
        let mut versions = self.versions.versions_of(dossier).await?;
        for version in &mut versions {
            if version.is_available() && version.reading() == ReadingStatus::Unread {
                if let Some(status) = self.read_text_of(version, report).await? {
                    *version = version.with_reading(status);
                }
            }
        }
        Ok(versions)
    }

    /// L'état de lecture enregistré, ou `None` quand la source a échoué et que
    /// la version reste à lire à la passe suivante.
    async fn read_text_of(
        &self,
        version: &TextVersion,
        report: &mut TextBriefRefreshReport,
    ) -> Result<Option<ReadingStatus>, RepositoryError> {
        let reading = match self.source.version_articles(version).await {
            Ok(Some(articles)) if !articles.is_empty() => TextReading::Articles(articles),
            Ok(_) => TextReading::Missing,
            Err(error) => {
                report.note_anomaly(format!("{}: {error}", version.document_uid()));
                return Ok(None);
            }
        };
        self.versions
            .save_reading(version.document_uid(), &reading)
            .await?;
        Ok(Some(match reading {
            TextReading::Articles(_) => {
                report.texts_read += 1;
                ReadingStatus::Read
            }
            TextReading::Missing => {
                report.texts_missing += 1;
                ReadingStatus::Missing
            }
        }))
    }

    /// CU-02 pour la dernière version du dossier : classe ses articles, réunit
    /// les sources de ceux à résumer, les envoie par paquets, puis enregistre
    /// l'avancement du dossier.
    async fn brief(
        &self,
        dossier: &DossierUid,
        versions: &[TextVersion],
        today: NaiveDate,
        lookups: &mut Lookups,
        report: &mut TextBriefRefreshReport,
    ) -> Result<(), RepositoryError> {
        let Some(latest) = latest_available(versions) else {
            return Ok(());
        };
        // Texte jamais lu (téléchargement en échec) : la passe suivante le lit.
        let Some(articles) = self.versions.articles_of(latest.document_uid()).await? else {
            return Ok(());
        };
        let prompt_version = self.generator.prompt_version();

        let distinct = distinct_by_fingerprint(&articles);
        let fingerprints: Vec<ArticleFingerprint> = distinct
            .iter()
            .map(|article| article.fingerprint().clone())
            .collect();
        let known: HashMap<String, StoredBriefLine> = self
            .lines
            .lines_for(&fingerprints)
            .await?
            .into_iter()
            .map(|line| (line.fingerprint.as_str().to_string(), line))
            .collect();

        let mut rejected = 0;
        let mut fixed = Vec::new();
        let mut to_send = Vec::new();
        for article in distinct {
            let stored = known.get(article.fingerprint().as_str());
            match plan_article(article, stored, prompt_version) {
                ArticleAction::Reuse => report.lines_reused += 1,
                ArticleAction::Keep => rejected += 1,
                ArticleAction::WriteDeleted => fixed.push(self.stored_line(
                    article.fingerprint(),
                    RULE_MODEL,
                    Ok(DELETED_ARTICLE_LINE.to_string()),
                    Vec::new(),
                    today,
                )),
                ArticleAction::Send => to_send.push(article),
            }
        }
        if !fixed.is_empty() {
            self.lines.save_lines(&fixed).await?;
            report.lines_written += fixed.len();
        }

        let mut pending = 0;
        let mut sourced = Vec::new();
        for article in to_send {
            match self
                .sources_of(article, latest.deposited_on(), lookups)
                .await
            {
                Ok(Some(complete)) => sourced.push(complete),
                Ok(None) => pending += 1,
                Err(message) => {
                    report.note_anomaly(message);
                    pending += 1;
                }
            }
        }

        let tally = self
            .generate(latest.document_uid(), &sourced, today, report)
            .await?;
        pending += tally.pending;
        rejected += tally.rejected;

        report.articles_pending += pending;
        self.lines
            .save_progress(
                dossier,
                &BriefProgress {
                    document_uid: latest.document_uid().clone(),
                    prompt_version: prompt_version.to_string(),
                    pending_count: pending,
                    rejected_count: rejected,
                    attempted_on: today,
                },
            )
            .await
    }

    /// RM-07, RM-08 : les sources d'un article à résumer. `Ok(None)` : la passe
    /// a épuisé ses lectures, l'article attend. `Err` : une rédaction citée n'a
    /// pas pu être obtenue, l'article attend.
    async fn sources_of<'x>(
        &self,
        article: &'x VersionArticle,
        on: NaiveDate,
        lookups: &mut Lookups,
    ) -> Result<Option<Sourced<'x>>, String> {
        let cited = CitedArticle::cited_in(article.content());
        let unknown = cited
            .iter()
            .filter(|candidate| !lookups.is_known(candidate, on))
            .count();
        if unknown > lookups.left {
            return Ok(None);
        }

        let mut sourced = Sourced {
            article,
            request: BriefArticle {
                key: article.fingerprint().as_str().to_string(),
                content: article.content().to_string(),
                cited: Vec::new(),
            },
            sources: vec![article.content().to_string()],
            cited_sources: Vec::new(),
        };
        for candidate in &cited {
            let wording = lookups.wording(self.legal, candidate, on).await?;
            let label = format!(
                "{}, article {}",
                candidate.text().label(),
                candidate.cited_as()
            );
            let trace = format!(
                "{}, article {}",
                candidate.text().label(),
                candidate.number()
            );
            match &wording {
                CitedWording::InForce { text, .. } => {
                    sourced.sources.push(text.clone());
                    sourced
                        .cited_sources
                        .push(format!("{trace}, rédaction au {on}"));
                }
                CitedWording::Created => sourced
                    .cited_sources
                    .push(format!("{trace}, créé par le texte")),
                // Renvoi non reconnu : il ne bloque pas et n'est pas transmis.
                CitedWording::NotFound => continue,
            }
            sourced.request.cited.push(BriefCitation { label, wording });
        }
        Ok(Some(sourced))
    }

    /// RM-09, T4 : envoie les articles par paquets, contrôle et enregistre les
    /// lignes de chaque réponse dès qu'elle arrive. Une version compte au plus
    /// un échec par passe, quel que soit le nombre de paquets en échec ; le
    /// compteur ne revient à zéro que si tous les paquets ont réussi.
    async fn generate(
        &self,
        document_uid: &DocumentUid,
        sourced: &[Sourced<'_>],
        today: NaiveDate,
        report: &mut TextBriefRefreshReport,
    ) -> Result<Tally, RepositoryError> {
        let mut tally = Tally::default();
        if sourced.is_empty() {
            return Ok(tally);
        }
        let failures_before = self.lines.failures_of(document_uid).await?;
        let mut failed: Vec<&Sourced<'_>> = Vec::new();
        let mut last_error = String::new();
        let mut unavailable = false;

        for (index, packet) in sourced.chunks(BRIEF_ARTICLES_PER_CALL).enumerate() {
            let request = BriefRequest {
                articles: packet.iter().map(|s| s.request.clone()).collect(),
            };
            match self.generator.generate(&request).await {
                Ok(answered) => {
                    let judged = self.judge(packet, &answered, today);
                    self.lines.save_lines(&judged).await?;
                    for line in &judged {
                        match line.status {
                            BriefStatus::Ready => report.lines_written += 1,
                            BriefStatus::Rejected => {
                                report.lines_rejected += 1;
                                tally.rejected += 1;
                            }
                        }
                    }
                }
                // Pas de clé chez le fournisseur : ce n'est pas un échec du
                // modèle, rien ne se compte et rien n'est refusé.
                Err(error @ SummaryGeneratorError::Unavailable(_)) => {
                    report.note_anomaly(error.to_string());
                    tally.pending += sourced.len() - index * BRIEF_ARTICLES_PER_CALL;
                    unavailable = true;
                    break;
                }
                Err(error) => {
                    report.note_anomaly(error.to_string());
                    last_error = error.to_string();
                    failed.extend(packet);
                }
            }
        }

        if failed.is_empty() {
            if failures_before > 0 && !unavailable {
                self.lines.clear_failures(document_uid).await?;
            }
            return Ok(tally);
        }
        self.lines.record_failure(document_uid, &last_error).await?;
        if failures_before.saturating_add(1) >= MAX_GENERATION_FAILURES {
            let refused: Vec<StoredBriefLine> = failed
                .iter()
                .map(|s| {
                    self.stored_line(
                        s.article.fingerprint(),
                        self.generator.model(),
                        Err(Refusal::without_line(GENERATION_FAILED)),
                        s.cited_sources.clone(),
                        today,
                    )
                })
                .collect();
            self.lines.save_lines(&refused).await?;
            self.lines.clear_failures(document_uid).await?;
            report.lines_rejected += refused.len();
            tally.rejected += refused.len();
        } else {
            tally.pending += failed.len();
        }
        Ok(tally)
    }

    /// RM-10 à RM-13 : une ligne par article du paquet. Une ligne rendue sous
    /// une clé non envoyée est ignorée ; une clé rendue deux fois garde la
    /// première ; une clé sans ligne est refusée.
    fn judge(
        &self,
        packet: &[Sourced<'_>],
        answered: &[GeneratedLine],
        today: NaiveDate,
    ) -> Vec<StoredBriefLine> {
        let mut first: HashMap<&str, &str> = HashMap::new();
        for line in answered {
            first.entry(line.key.as_str()).or_insert(line.line.as_str());
        }
        packet
            .iter()
            .map(|sourced| {
                let outcome = match first.get(sourced.request.key.as_str()) {
                    None => Err(Refusal::without_line(MISSING_LINE)),
                    Some(raw) => {
                        let sources: Vec<&str> =
                            sourced.sources.iter().map(String::as_str).collect();
                        BriefLine::check(raw, &sources)
                            .map(|line| line.as_str().to_string())
                            .map_err(|error| Refusal {
                                code: error.as_str(),
                                detail: error.detail(),
                                line: Some((*raw).to_string()),
                            })
                    }
                };
                self.stored_line(
                    sourced.article.fingerprint(),
                    self.generator.model(),
                    outcome,
                    sourced.cited_sources.clone(),
                    today,
                )
            })
            .collect()
    }

    /// RM-16 : une ligne enregistrée avec le modèle, la version du prompt, la
    /// date de la passe et les articles cités transmis. `Err` : le refus.
    fn stored_line(
        &self,
        fingerprint: &ArticleFingerprint,
        model: &str,
        outcome: Result<String, Refusal>,
        cited_sources: Vec<String>,
        today: NaiveDate,
    ) -> StoredBriefLine {
        let (status, line, refusal) = match outcome {
            Ok(line) => (BriefStatus::Ready, Some(line), None),
            Err(refusal) => (BriefStatus::Rejected, None, Some(refusal)),
        };
        let (rejection, rejection_detail, rejected_line) = match refusal {
            Some(refusal) => (Some(refusal.code.to_string()), refusal.detail, refusal.line),
            None => (None, None, None),
        };
        StoredBriefLine {
            fingerprint: fingerprint.clone(),
            status,
            line,
            rejection,
            rejection_detail,
            rejected_line,
            model: model.to_string(),
            prompt_version: self.generator.prompt_version().to_string(),
            generated_at: NaiveDateTime::new(today, NaiveTime::MIN),
            cited_sources,
        }
    }
}

/// Pourquoi un article n'a pas de ligne publiable (RM-13).
struct Refusal {
    code: &'static str,
    detail: Option<String>,
    /// La ligne écartée, quand le modèle en a rendu une.
    line: Option<String>,
}

impl Refusal {
    fn without_line(code: &'static str) -> Self {
        Self {
            code,
            detail: None,
            line: None,
        }
    }
}

/// Un article à résumer avec ses sources complètes.
struct Sourced<'x> {
    article: &'x VersionArticle,
    request: BriefArticle,
    /// Textes contre lesquels la ligne est contrôlée : le contenu de l'article
    /// et les rédactions en vigueur transmises, jamais un article créé.
    sources: Vec<String>,
    cited_sources: Vec<String>,
}

/// Ce que la génération laisse de l'avancement d'un dossier.
#[derive(Default)]
struct Tally {
    /// Articles sans ligne à la fin de l'appel.
    pending: usize,
    /// Refus écrits pendant l'appel.
    rejected: usize,
}

/// Rédactions lues chez la source légale pendant la passe : une même citation
/// n'est lue qu'une fois, échec compris, et le nombre de lectures est borné
/// (`MAX_CITED_LOOKUPS_PER_PASS`).
struct Lookups {
    left: usize,
    done: HashMap<(CitedText, String, NaiveDate), Result<CitedWording, String>>,
}

impl Lookups {
    fn new() -> Self {
        Self {
            left: MAX_CITED_LOOKUPS_PER_PASS,
            done: HashMap::new(),
        }
    }

    fn key(cited: &CitedArticle, on: NaiveDate) -> (CitedText, String, NaiveDate) {
        (cited.text().clone(), cited.number().to_string(), on)
    }

    fn is_known(&self, cited: &CitedArticle, on: NaiveDate) -> bool {
        self.done.contains_key(&Self::key(cited, on))
    }

    async fn wording(
        &mut self,
        legal: &dyn LegalTextSource,
        cited: &CitedArticle,
        on: NaiveDate,
    ) -> Result<CitedWording, String> {
        let key = Self::key(cited, on);
        if let Some(known) = self.done.get(&key) {
            return known.clone();
        }
        self.left = self.left.saturating_sub(1);
        let result = legal
            .wording(cited, on)
            .await
            .map_err(|error| format!("{} {}: {error}", cited.text().label(), cited.number()));
        self.done.insert(key, result.clone());
        result
    }
}

/// Les articles de la version, le premier de chaque empreinte : deux articles
/// de même contenu ont la même ligne (RM-03, RM-05).
fn distinct_by_fingerprint(articles: &[VersionArticle]) -> Vec<&VersionArticle> {
    let mut seen = HashSet::new();
    articles
        .iter()
        .filter(|article| seen.insert(article.fingerprint().as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::brief_generator::BriefCitation;
    use crate::application::use_cases::brief_fakes::{
        FakeBriefGenerator, FakeLegalTextSource, FakeTextVersionSource,
        InMemoryBriefLineRepository, InMemoryTextVersionRepository,
    };
    use crate::domain::cited_article::{CitedText, CitedWording};
    use crate::domain::dossier::DossierUid;
    use crate::domain::law_text::{ArticleFingerprint, NewVersionArticle, VersionArticle};
    use crate::domain::text_brief::{
        BriefStatus, StoredBriefLine, DELETED_ARTICLE_LINE, RULE_MODEL,
    };
    use crate::domain::text_version::{
        Chamber, DocumentUid, NewTextVersion, ReadingStatus, TextVersion, VersionKind,
    };
    use chrono::NaiveDate;

    /// Version du prompt du générateur double.
    const PROMPT: &str = "prompt-essai-v1";

    fn day(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn today() -> NaiveDate {
        day(2026, 11, 3)
    }

    fn dossier(uid: &str) -> DossierUid {
        DossierUid::new(uid.to_string()).unwrap()
    }

    fn version(uid: &str, dossier_uid: &str, kind: VersionKind, chamber: Chamber) -> TextVersion {
        TextVersion::new(NewTextVersion {
            document_uid: DocumentUid::new(uid.to_string()).unwrap(),
            dossier_uid: dossier(dossier_uid),
            kind,
            chamber,
            deposited_on: day(2026, 1, 15),
            title: "titre".to_string(),
            reading: ReadingStatus::Unread,
        })
    }

    fn deposit(uid: &str, dossier_uid: &str) -> TextVersion {
        version(uid, dossier_uid, VersionKind::Deposited, Chamber::Assembly)
    }

    fn article(label: &str) -> VersionArticle {
        text_article(label, &format!("Contenu de l'article {label}."))
    }

    /// Article dont l'empreinte est `sha256:<label>`.
    fn text_article(label: &str, content: &str) -> VersionArticle {
        article_with_fingerprint(label, content, &format!("sha256:{label}"))
    }

    fn article_with_fingerprint(label: &str, content: &str, fingerprint: &str) -> VersionArticle {
        VersionArticle::new(NewVersionArticle {
            label: label.to_string(),
            division: None,
            rank: 1,
            content: content.to_string(),
            fingerprint: ArticleFingerprint::new(fingerprint.to_string()),
        })
    }

    /// Les doubles d'une passe, gardés d'une passe à l'autre comme le seraient
    /// les dépôts.
    struct Rig {
        source: FakeTextVersionSource,
        repository: InMemoryTextVersionRepository,
        legal: FakeLegalTextSource,
        lines: InMemoryBriefLineRepository,
        generator: FakeBriefGenerator,
    }

    impl Rig {
        async fn run(&self, cap: usize) -> TextBriefRefreshReport {
            RefreshTextBriefs::new(
                &self.source,
                &self.repository,
                &self.legal,
                &self.lines,
                &self.generator,
            )
            .execute(cap, today())
            .await
        }
    }

    /// Source et dépôt prêts : `votes` sont les dossiers votés et la date de
    /// leur dernier scrutin, `published` les versions que la source publie.
    fn setup(published: Vec<TextVersion>, votes: &[(&str, NaiveDate)]) -> Rig {
        let rig = Rig {
            source: FakeTextVersionSource::default(),
            repository: InMemoryTextVersionRepository::default(),
            legal: FakeLegalTextSource::default(),
            lines: InMemoryBriefLineRepository::default(),
            generator: FakeBriefGenerator::default(),
        };
        for version in &published {
            rig.source.texts.lock().unwrap().insert(
                version.document_uid().as_str().to_string(),
                vec![article("1er"), article("2")],
            );
        }
        *rig.source.versions.lock().unwrap() = published;
        *rig.repository.votes.lock().unwrap() =
            votes.iter().map(|(uid, on)| (dossier(uid), *on)).collect();
        rig
    }

    fn requests(source: &FakeTextVersionSource) -> Vec<String> {
        source.text_requests.lock().unwrap().clone()
    }

    /// Un dossier voté `D1` à une seule version, `PIONANR5L17B0001`, qui a ces
    /// articles.
    fn single(articles: Vec<VersionArticle>) -> Rig {
        let rig = setup(
            vec![deposit("PIONANR5L17B0001", "D1")],
            &[("D1", day(2026, 6, 1))],
        );
        rig.source
            .texts
            .lock()
            .unwrap()
            .insert("PIONANR5L17B0001".to_string(), articles);
        rig
    }

    /// `count` articles de contenu distinct, d'empreinte `sha256:<n>`.
    fn numbered(count: usize) -> Vec<VersionArticle> {
        (1..=count)
            .map(|n| text_article(&n.to_string(), &format!("Contenu {n}.")))
            .collect()
    }

    fn santé() -> CitedText {
        CitedText::Code("Code de la santé publique".to_string())
    }

    fn in_force(text: &str) -> CitedWording {
        CitedWording::InForce {
            text: text.to_string(),
            official_url: "https://www.legifrance.gouv.fr/article".to_string(),
        }
    }

    fn ready_line(fingerprint: &str, line: &str) -> StoredBriefLine {
        StoredBriefLine {
            fingerprint: ArticleFingerprint::new(fingerprint.to_string()),
            status: BriefStatus::Ready,
            line: Some(line.to_string()),
            rejection: None,
            rejection_detail: None,
            rejected_line: None,
            model: "autre-modele".to_string(),
            prompt_version: "ancien-prompt".to_string(),
            generated_at: day(2026, 10, 1).and_hms_opt(0, 0, 0).unwrap(),
            cited_sources: Vec::new(),
        }
    }

    fn keys(rig: &Rig) -> Vec<Vec<String>> {
        rig.generator.requested_keys()
    }

    #[tokio::test]
    async fn should_save_every_version_when_source_publishes_them() {
        let rig = setup(
            vec![
                deposit("PIONANR5L17B0001", "D1"),
                version(
                    "PIONSNR5S479B0440",
                    "D1",
                    VersionKind::Deposited,
                    Chamber::Senate,
                ),
                deposit("PIONANR5L17B0002", "D-NO-VOTE"),
            ],
            &[("D1", day(2026, 6, 1))],
        );

        let report = rig.run(5).await;

        assert_eq!(report.versions_seen, 3);
        assert_eq!(rig.repository.versions.lock().unwrap().len(), 3);
        assert_eq!(report.anomaly, None);
    }

    #[tokio::test]
    async fn should_read_unread_available_versions_when_dossier_is_voted() {
        let rig = setup(
            vec![
                deposit("PIONANR5L17B0001", "D1"),
                version(
                    "PIONANR5L17BTA0002",
                    "D1",
                    VersionKind::Adopted,
                    Chamber::Assembly,
                ),
            ],
            &[("D1", day(2026, 6, 1))],
        );

        let report = rig.run(5).await;

        assert_eq!(report.dossiers_read, 1);
        assert_eq!(report.texts_read, 2);
        assert_eq!(report.texts_missing, 0);
        for uid in ["PIONANR5L17B0001", "PIONANR5L17BTA0002"] {
            assert_eq!(rig.repository.reading_of(uid), Some(ReadingStatus::Read));
            let stored = rig.repository.articles.lock().unwrap();
            let labels: Vec<&str> = stored[uid].iter().map(VersionArticle::label).collect();
            assert_eq!(labels, vec!["1er", "2"]);
        }
    }

    #[tokio::test]
    async fn should_not_read_text_when_dossier_has_no_vote() {
        let rig = setup(vec![deposit("PIONANR5L17B0001", "D1")], &[]);

        let report = rig.run(5).await;

        assert_eq!(report.versions_seen, 1);
        assert_eq!(report.dossiers_read, 0);
        assert!(requests(&rig.source).is_empty());
        assert_eq!(
            rig.repository.reading_of("PIONANR5L17B0001"),
            Some(ReadingStatus::Unread)
        );
    }

    #[tokio::test]
    async fn should_not_read_senate_version_when_assembly_does_not_publish_it() {
        let rig = setup(
            vec![
                deposit("PIONANR5L17B0001", "D1"),
                version(
                    "PIONSNR5S479BTA0043",
                    "D1",
                    VersionKind::Adopted,
                    Chamber::Senate,
                ),
                version(
                    "PIONSNR5S479B0440",
                    "D-SENATE-ONLY",
                    VersionKind::Deposited,
                    Chamber::Senate,
                ),
            ],
            &[("D1", day(2026, 6, 1)), ("D-SENATE-ONLY", day(2026, 6, 2))],
        );

        let report = rig.run(5).await;

        assert_eq!(requests(&rig.source), vec!["PIONANR5L17B0001"]);
        assert_eq!(
            report.dossiers_read, 1,
            "a Senate-only dossier has nothing to read"
        );
        assert_eq!(
            rig.repository.reading_of("PIONSNR5S479BTA0043"),
            Some(ReadingStatus::Unread)
        );
    }

    #[tokio::test]
    async fn should_read_at_most_cap_dossiers_when_more_are_waiting() {
        let rig = setup(
            vec![
                deposit("PIONANR5L17B0001", "D-OLD"),
                deposit("PIONANR5L17B0002", "D-NEW"),
                deposit("PIONANR5L17B0003", "D-MID"),
            ],
            &[
                ("D-OLD", day(2025, 1, 1)),
                ("D-NEW", day(2026, 6, 1)),
                ("D-MID", day(2026, 1, 1)),
            ],
        );

        let report = rig.run(2).await;

        assert_eq!(report.dossiers_read, 2);
        assert_eq!(
            requests(&rig.source),
            vec!["PIONANR5L17B0002", "PIONANR5L17B0003"]
        );
        assert_eq!(
            rig.repository.reading_of("PIONANR5L17B0001"),
            Some(ReadingStatus::Unread)
        );

        let next = rig.run(2).await;

        assert_eq!(
            next.dossiers_read, 1,
            "the waiting dossier is read on the next pass"
        );
        assert_eq!(
            rig.repository.reading_of("PIONANR5L17B0001"),
            Some(ReadingStatus::Read)
        );
    }

    #[tokio::test]
    async fn should_not_read_again_when_version_was_already_read() {
        let rig = setup(
            vec![deposit("PIONANR5L17B0001", "D1")],
            &[("D1", day(2026, 6, 1))],
        );
        rig.run(5).await;

        let second = rig.run(5).await;

        assert_eq!(requests(&rig.source).len(), 1);
        assert_eq!(second.dossiers_read, 0);
        assert_eq!(second.texts_read, 0);
        assert_eq!(second.versions_seen, 1);
    }

    #[tokio::test]
    async fn should_record_missing_text_when_source_has_none() {
        let rig = setup(
            vec![deposit("PIONANR5L17B0001", "D1")],
            &[("D1", day(2026, 6, 1))],
        );
        rig.source.texts.lock().unwrap().clear();

        let report = rig.run(5).await;

        assert_eq!(report.texts_missing, 1);
        assert_eq!(report.texts_read, 0);
        assert_eq!(
            rig.repository.reading_of("PIONANR5L17B0001"),
            Some(ReadingStatus::Missing)
        );
        rig.run(5).await;
        assert_eq!(
            requests(&rig.source).len(),
            1,
            "a missing text is not read again"
        );
    }

    #[tokio::test]
    async fn should_record_missing_text_when_no_article_is_recognised() {
        let rig = setup(
            vec![deposit("PIONANR5L17B0001", "D1")],
            &[("D1", day(2026, 6, 1))],
        );
        rig.source
            .texts
            .lock()
            .unwrap()
            .insert("PIONANR5L17B0001".to_string(), Vec::new());

        let report = rig.run(5).await;

        assert_eq!(report.texts_missing, 1);
        assert_eq!(
            rig.repository.reading_of("PIONANR5L17B0001"),
            Some(ReadingStatus::Missing)
        );
        assert_eq!(rig.repository.articles.lock().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn should_report_anomaly_and_go_on_when_source_fails() {
        let rig = setup(
            vec![
                deposit("PIONANR5L17B0001", "D-NEW"),
                deposit("PIONANR5L17B0002", "D-OLD"),
            ],
            &[("D-NEW", day(2026, 6, 1)), ("D-OLD", day(2026, 1, 1))],
        );
        rig.source
            .failing_texts
            .lock()
            .unwrap()
            .push("PIONANR5L17B0001".to_string());

        let report = rig.run(5).await;

        assert!(report
            .anomaly
            .as_deref()
            .unwrap()
            .contains("PIONANR5L17B0001"));
        assert_eq!(report.texts_read, 1);
        assert_eq!(
            rig.repository.reading_of("PIONANR5L17B0001"),
            Some(ReadingStatus::Unread),
            "a text that failed to load stays to be read"
        );
        assert_eq!(
            rig.repository.reading_of("PIONANR5L17B0002"),
            Some(ReadingStatus::Read)
        );
    }

    #[tokio::test]
    async fn should_read_stored_versions_when_publication_of_versions_fails() {
        let rig = setup(Vec::new(), &[("D1", day(2026, 6, 1))]);
        rig.repository
            .versions
            .lock()
            .unwrap()
            .push(deposit("PIONANR5L17B0001", "D1"));
        rig.source
            .texts
            .lock()
            .unwrap()
            .insert("PIONANR5L17B0001".to_string(), vec![article("1er")]);
        *rig.source.versions_unavailable.lock().unwrap() = true;

        let report = rig.run(5).await;

        assert!(report.anomaly.is_some());
        assert_eq!(report.versions_seen, 0);
        assert_eq!(report.texts_read, 1);
    }

    #[tokio::test]
    async fn should_report_anomaly_and_not_fail_when_repository_fails() {
        let rig = setup(
            vec![deposit("PIONANR5L17B0001", "D1")],
            &[("D1", day(2026, 6, 1))],
        );
        *rig.repository.unreachable.lock().unwrap() = true;

        let report = rig.run(5).await;

        assert!(report
            .anomaly
            .as_deref()
            .unwrap()
            .contains("base injoignable"));
        assert_eq!(report.texts_read, 0);
        assert!(requests(&rig.source).is_empty());
    }

    #[tokio::test]
    async fn should_keep_reading_status_when_version_is_saved_again() {
        let rig = setup(
            vec![deposit("PIONANR5L17B0001", "D1")],
            &[("D1", day(2026, 6, 1))],
        );
        rig.run(5).await;
        assert_eq!(
            rig.repository.reading_of("PIONANR5L17B0001"),
            Some(ReadingStatus::Read)
        );

        // La source republie la version telle qu'elle est : non lue.
        let republished = deposit("PIONANR5L17B0001", "D1");
        assert_eq!(republished.reading(), ReadingStatus::Unread);
        rig.run(5).await;

        assert_eq!(
            rig.repository.reading_of("PIONANR5L17B0001"),
            Some(ReadingStatus::Read)
        );
        assert_eq!(rig.repository.versions.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn should_send_only_new_or_changed_articles_when_a_new_version_arrives() {
        let rig = single(vec![
            text_article("a", "Contenu A."),
            text_article("b", "Contenu B."),
        ]);
        rig.generator.answer("sha256:a", "Fixe une règle A.");
        rig.generator.answer("sha256:b", "Fixe une règle B.");
        rig.run(5).await;
        rig.generator.requests.lock().unwrap().clear();

        rig.source.versions.lock().unwrap().push(version(
            "PIONANR5L17BTA0002",
            "D1",
            VersionKind::Adopted,
            Chamber::Assembly,
        ));
        rig.source.texts.lock().unwrap().insert(
            "PIONANR5L17BTA0002".to_string(),
            vec![
                text_article("a", "Contenu A."),
                text_article("b2", "Contenu B modifié."),
                text_article("c", "Contenu C."),
            ],
        );
        rig.generator
            .answer("sha256:b2", "Fixe une règle B modifiée.");
        rig.generator.answer("sha256:c", "Fixe une règle C.");

        let report = rig.run(5).await;

        assert_eq!(keys(&rig), vec![vec!["sha256:b2", "sha256:c"]]);
        assert_eq!(report.lines_reused, 1);
        assert_eq!(report.lines_written, 2);
    }

    #[tokio::test]
    async fn should_make_no_call_when_every_article_already_has_a_line() {
        let rig = single(vec![
            text_article("a", "Contenu A."),
            text_article("b", "Contenu B."),
        ]);
        for fingerprint in ["sha256:a", "sha256:b"] {
            rig.lines.lines.lock().unwrap().insert(
                fingerprint.to_string(),
                ready_line(fingerprint, "Fixe une règle."),
            );
        }

        let report = rig.run(5).await;

        assert!(keys(&rig).is_empty());
        assert_eq!(report.lines_reused, 2);
        assert_eq!(report.lines_written, 0);
        assert_eq!(report.articles_pending, 0);
        assert_eq!(
            rig.lines.line("sha256:a").unwrap().model,
            "autre-modele",
            "a line already known is not rewritten"
        );
    }

    #[tokio::test]
    async fn should_write_fixed_line_without_call_when_article_is_deleted() {
        let rig = single(vec![text_article("7", "(Supprimé)")]);

        let report = rig.run(5).await;

        assert!(keys(&rig).is_empty());
        let stored = rig.lines.line("sha256:7").unwrap();
        assert_eq!(stored.status, BriefStatus::Ready);
        assert_eq!(stored.line.as_deref(), Some(DELETED_ARTICLE_LINE));
        assert_eq!(stored.model, RULE_MODEL);
        assert_eq!(stored.prompt_version, PROMPT);
        assert_eq!(report.lines_written, 1);
    }

    #[tokio::test]
    async fn should_save_line_with_model_prompt_and_cited_sources_when_it_passes_checks() {
        let rig = single(vec![text_article(
            "1er",
            "L'article L. 1110-5 du code de la santé publique est complété par un alinéa ainsi rédigé.",
        )]);
        rig.legal.wordings.lock().unwrap().insert(
            (santé(), "L1110-5".to_string()),
            in_force("Toute personne a le droit d'être informée."),
        );
        rig.generator
            .answer("sha256:1er", "Ajoute une règle au droit à l'information.");

        let report = rig.run(5).await;

        let request = rig.generator.requests.lock().unwrap()[0].clone();
        assert_eq!(request.articles[0].key, "sha256:1er");
        assert_eq!(
            request.articles[0].cited,
            vec![BriefCitation {
                label: "code de la santé publique, article L. 1110-5".to_string(),
                wording: in_force("Toute personne a le droit d'être informée."),
            }]
        );
        let stored = rig.lines.line("sha256:1er").unwrap();
        assert_eq!(stored.status, BriefStatus::Ready);
        assert_eq!(
            stored.line.as_deref(),
            Some("Ajoute une règle au droit à l'information.")
        );
        assert_eq!(stored.model, "modele-essai");
        assert_eq!(stored.prompt_version, PROMPT);
        assert_eq!(
            stored.cited_sources,
            vec!["code de la santé publique, article L1110-5, rédaction au 2026-01-15"]
        );
        assert_eq!(stored.generated_at, today().and_hms_opt(0, 0, 0).unwrap());
        assert_eq!(report.lines_written, 1);
        assert_eq!(
            rig.legal.requests.lock().unwrap()[0].2,
            day(2026, 1, 15),
            "the wording is the one in force on the date of the version"
        );
    }

    #[tokio::test]
    async fn should_save_refusal_with_its_code_when_line_fails_checks() {
        let rig = single(vec![text_article("1er", "Contenu A.")]);
        rig.generator
            .answer("sha256:1er", "Vise à réduire le délai de recours.");

        let report = rig.run(5).await;

        let stored = rig.lines.line("sha256:1er").unwrap();
        assert_eq!(stored.status, BriefStatus::Rejected);
        assert_eq!(stored.rejection.as_deref(), Some("forbidden_wording"));
        assert_eq!(stored.line, None);
        assert_eq!(stored.model, "modele-essai");
        assert_eq!(report.lines_rejected, 1);
        assert_eq!(report.lines_written, 0);
    }

    #[tokio::test]
    async fn should_keep_the_refused_line_and_what_was_refused_for_the_diagnosis() {
        let rig = single(vec![text_article("1er", "Contenu A.")]);
        rig.generator
            .answer("sha256:1er", "Vise à réduire le délai de recours.");

        rig.run(5).await;

        let stored = rig.lines.line("sha256:1er").unwrap();
        assert_eq!(stored.rejection_detail.as_deref(), Some("vise à"));
        assert_eq!(
            stored.rejected_line.as_deref(),
            Some("Vise à réduire le délai de recours.")
        );
    }

    #[tokio::test]
    async fn should_refuse_article_when_answer_has_no_line_for_it() {
        let rig = single(vec![
            text_article("a", "Contenu A."),
            text_article("b", "Contenu B."),
        ]);
        rig.generator.answer("sha256:a", "Fixe une règle A.");

        let report = rig.run(5).await;

        assert_eq!(
            rig.lines.line("sha256:a").unwrap().status,
            BriefStatus::Ready
        );
        let missing = rig.lines.line("sha256:b").unwrap();
        assert_eq!(missing.status, BriefStatus::Rejected);
        assert_eq!(missing.rejection.as_deref(), Some("missing_line"));
        assert_eq!(missing.rejected_line, None);
        assert_eq!(report.lines_rejected, 1);
    }

    #[tokio::test]
    async fn should_leave_article_waiting_when_a_cited_wording_cannot_be_obtained() {
        let rig = single(vec![
            text_article(
                "a",
                "L'article L. 1110-5 du code de la santé publique est abrogé.",
            ),
            text_article("b", "Contenu B."),
        ]);
        rig.legal
            .failing
            .lock()
            .unwrap()
            .push((santé(), "L1110-5".to_string()));
        rig.generator.answer("sha256:a", "Abroge une règle.");
        rig.generator.answer("sha256:b", "Fixe une règle B.");

        let report = rig.run(5).await;

        assert_eq!(keys(&rig), vec![vec!["sha256:b"]]);
        assert!(rig.lines.line("sha256:a").is_none());
        assert_eq!(report.articles_pending, 1);
        assert_eq!(rig.lines.progress_of("D1").unwrap().pending_count, 1);
        assert!(report.anomaly.as_deref().unwrap().contains("legifrance"));
    }

    #[tokio::test]
    async fn should_send_article_when_cited_article_is_created_by_the_text() {
        let rig = single(vec![text_article(
            "a",
            "Le code de la santé publique est ainsi modifié : après l'article L. 1111-12, il est inséré un article L. 1111-12-1 ainsi rédigé : « Le patient est informé. »",
        )]);
        rig.legal.wordings.lock().unwrap().insert(
            (santé(), "L1111-12".to_string()),
            in_force("Le consentement est recueilli."),
        );
        rig.generator
            .answer("sha256:a", "Crée une règle sur l'information du patient.");

        let report = rig.run(5).await;

        let request = rig.generator.requests.lock().unwrap()[0].clone();
        let wordings: Vec<CitedWording> = request.articles[0]
            .cited
            .iter()
            .map(|citation| citation.wording.clone())
            .collect();
        assert_eq!(
            wordings,
            vec![
                in_force("Le consentement est recueilli."),
                CitedWording::Created
            ]
        );
        assert_eq!(report.articles_pending, 0);
        assert_eq!(
            rig.lines.line("sha256:a").unwrap().cited_sources,
            vec![
                "code de la santé publique, article L1111-12, rédaction au 2026-01-15",
                "code de la santé publique, article L1111-12-1, créé par le texte",
            ]
        );
    }

    #[tokio::test]
    async fn should_keep_articles_to_summarise_when_generator_fails() {
        let rig = single(vec![
            text_article("a", "Contenu A."),
            text_article("b", "Contenu B."),
        ]);
        *rig.generator.failing.lock().unwrap() = true;

        let report = rig.run(5).await;

        assert!(rig.lines.lines.lock().unwrap().is_empty());
        assert_eq!(report.articles_pending, 2);
        assert_eq!(report.lines_rejected, 0);
        assert!(report.anomaly.is_some());
        assert_eq!(rig.lines.failure_count("PIONANR5L17B0001"), 1);

        rig.run(5).await;

        assert_eq!(
            keys(&rig).len(),
            2,
            "the articles are sent again on the next pass"
        );
    }

    #[tokio::test]
    async fn should_refuse_sent_articles_when_generator_fails_a_third_time() {
        let rig = single(vec![
            text_article("a", "Contenu A."),
            text_article("b", "Contenu B."),
        ]);
        rig.lines.seed_failures("PIONANR5L17B0001", 2);
        *rig.generator.failing.lock().unwrap() = true;

        let report = rig.run(5).await;

        for fingerprint in ["sha256:a", "sha256:b"] {
            let stored = rig.lines.line(fingerprint).unwrap();
            assert_eq!(stored.status, BriefStatus::Rejected);
            assert_eq!(stored.rejection.as_deref(), Some("generation_failed"));
        }
        assert_eq!(report.lines_rejected, 2);
        assert_eq!(report.articles_pending, 0);
        assert_eq!(rig.lines.failure_count("PIONANR5L17B0001"), 0);
    }

    #[tokio::test]
    async fn should_split_call_in_packets_of_40_when_version_has_more_articles() {
        let rig = single(numbered(90));
        *rig.generator.line_for_every_key.lock().unwrap() = Some("Fixe une règle.".to_string());

        let report = rig.run(5).await;

        let sizes: Vec<usize> = keys(&rig).iter().map(Vec::len).collect();
        assert_eq!(sizes, vec![40, 40, 10]);
        assert_eq!(report.lines_written, 90);
        assert_eq!(report.articles_pending, 0);
    }

    #[tokio::test]
    async fn should_put_dossier_attempted_today_after_others_when_it_produced_nothing() {
        let rig = setup(
            vec![
                deposit("PIONANR5L17B0001", "D-OLD"),
                deposit("PIONANR5L17B0002", "D-NEW"),
            ],
            &[("D-OLD", day(2026, 1, 1)), ("D-NEW", day(2026, 6, 1))],
        );
        rig.source.texts.lock().unwrap().insert(
            "PIONANR5L17B0002".to_string(),
            vec![text_article(
                "a",
                "L'article L. 1110-5 du code de la santé publique est abrogé.",
            )],
        );
        rig.legal
            .failing
            .lock()
            .unwrap()
            .push((santé(), "L1110-5".to_string()));

        rig.run(1).await;
        assert!(rig.lines.progress_of("D-NEW").is_some());
        assert!(rig.lines.progress_of("D-OLD").is_none());

        rig.run(1).await;

        assert!(
            rig.lines.progress_of("D-OLD").is_some(),
            "the dossier that produced nothing today gives way to the others"
        );
    }

    #[tokio::test]
    async fn should_ignore_line_when_its_key_was_not_sent() {
        let rig = single(vec![text_article("a", "Contenu A.")]);
        rig.generator.answer("sha256:a", "Fixe une règle A.");
        rig.generator
            .answer("sha256:inconnue", "Fixe une règle étrangère.");

        let report = rig.run(5).await;

        assert!(rig.lines.line("sha256:inconnue").is_none());
        assert_eq!(rig.lines.lines.lock().unwrap().len(), 1);
        assert_eq!(report.lines_written, 1);
    }

    #[tokio::test]
    async fn should_keep_first_line_when_a_key_is_answered_twice() {
        let rig = single(vec![text_article("a", "Contenu A.")]);
        rig.generator.answer("sha256:a", "Fixe la règle voulue.");
        rig.generator.answer("sha256:a", "Fixe une autre règle.");

        rig.run(5).await;

        assert_eq!(
            rig.lines.line("sha256:a").unwrap().line.as_deref(),
            Some("Fixe la règle voulue.")
        );
    }

    #[tokio::test]
    async fn should_count_one_failure_per_pass_when_several_packets_fail() {
        let rig = single(numbered(90));
        *rig.generator.failing.lock().unwrap() = true;

        let report = rig.run(5).await;

        assert_eq!(keys(&rig).len(), 3, "every packet is tried");
        assert_eq!(rig.lines.failure_count("PIONANR5L17B0001"), 1);
        assert_eq!(report.articles_pending, 90);
    }

    #[tokio::test]
    async fn should_keep_failure_count_when_one_packet_of_the_pass_fails() {
        let rig = single(numbered(90));
        *rig.generator.line_for_every_key.lock().unwrap() = Some("Fixe une règle.".to_string());
        rig.generator.failing_calls.lock().unwrap().push(1);
        rig.lines.seed_failures("PIONANR5L17B0001", 1);

        let report = rig.run(5).await;

        assert_eq!(
            rig.lines.failure_count("PIONANR5L17B0001"),
            2,
            "the count is neither reset by the packets that succeeded nor counted twice"
        );
        assert_eq!(report.lines_written, 50);
        assert_eq!(report.articles_pending, 40);
        assert_eq!(rig.lines.lines.lock().unwrap().len(), 50);
    }

    #[tokio::test]
    async fn should_leave_articles_waiting_when_cited_lookups_exceed_the_pass_cap() {
        let articles = (1..=201)
            .map(|n| {
                text_article(
                    &n.to_string(),
                    &format!("L'article {n} de la loi n° 2024-322 est abrogé."),
                )
            })
            .collect();
        let rig = single(articles);
        *rig.generator.line_for_every_key.lock().unwrap() = Some("Abroge une règle.".to_string());

        let report = rig.run(5).await;

        assert_eq!(rig.legal.requests.lock().unwrap().len(), 200);
        let sent: usize = keys(&rig).iter().map(Vec::len).sum();
        assert_eq!(sent, 200);
        assert!(rig.lines.line("sha256:201").is_none());
        assert_eq!(report.articles_pending, 1);
    }

    #[tokio::test]
    async fn should_not_count_a_failure_when_generator_is_unavailable() {
        let rig = single(vec![text_article("a", "Contenu A.")]);
        *rig.generator.unavailable.lock().unwrap() = true;

        let report = rig.run(5).await;

        assert_eq!(rig.lines.failure_count("PIONANR5L17B0001"), 0);
        assert!(rig.lines.lines.lock().unwrap().is_empty());
        assert_eq!(report.articles_pending, 1);
        assert!(report.anomaly.is_some());
    }

    #[tokio::test]
    async fn should_clear_failures_when_every_packet_of_the_pass_succeeds() {
        let rig = single(vec![text_article("a", "Contenu A.")]);
        rig.lines.seed_failures("PIONANR5L17B0001", 2);
        rig.generator.answer("sha256:a", "Fixe une règle A.");

        rig.run(5).await;

        assert_eq!(rig.lines.failure_count("PIONANR5L17B0001"), 0);
    }

    #[tokio::test]
    async fn should_send_once_when_two_articles_share_a_fingerprint() {
        let rig = single(vec![
            article_with_fingerprint("4", "Gage financier.", "sha256:gage"),
            article_with_fingerprint("9", "Gage financier.", "sha256:gage"),
        ]);
        rig.generator
            .answer("sha256:gage", "Prévoit un gage financier.");

        let report = rig.run(5).await;

        assert_eq!(keys(&rig), vec![vec!["sha256:gage"]]);
        assert_eq!(report.lines_written, 1);
    }

    #[tokio::test]
    async fn should_ask_legifrance_once_when_several_articles_cite_the_same_article() {
        let rig = single(vec![
            text_article(
                "a",
                "L'article L. 1110-5 du code de la santé publique est abrogé.",
            ),
            text_article(
                "b",
                "L'article L. 1110-5 du code de la santé publique est complété.",
            ),
        ]);
        rig.legal.wordings.lock().unwrap().insert(
            (santé(), "L1110-5".to_string()),
            in_force("Toute personne a le droit d'être informée."),
        );
        *rig.generator.line_for_every_key.lock().unwrap() = Some("Modifie une règle.".to_string());

        rig.run(5).await;

        assert_eq!(rig.legal.requests.lock().unwrap().len(), 1);
        let request = rig.generator.requests.lock().unwrap()[0].clone();
        assert!(request.articles.iter().all(|a| a.cited.len() == 1));
    }

    #[tokio::test]
    async fn should_record_progress_of_the_dossier_after_its_pass() {
        let rig = single(vec![
            text_article("a", "Contenu A."),
            text_article("b", "Contenu B."),
        ]);
        rig.generator.answer("sha256:a", "Fixe une règle A.");

        rig.run(5).await;

        let progress = rig.lines.progress_of("D1").unwrap();
        assert_eq!(progress.document_uid.as_str(), "PIONANR5L17B0001");
        assert_eq!(progress.prompt_version, PROMPT);
        assert_eq!(progress.pending_count, 0);
        assert_eq!(progress.rejected_count, 1);
        assert_eq!(progress.attempted_on, today());
    }

    #[tokio::test]
    async fn should_send_refused_article_again_when_prompt_version_changed() {
        let mut rig = single(vec![text_article("a", "Contenu A.")]);
        rig.run(5).await;
        assert_eq!(
            rig.lines.line("sha256:a").unwrap().status,
            BriefStatus::Rejected
        );

        rig.run(5).await;
        assert_eq!(
            keys(&rig).len(),
            1,
            "a refusal is not sent again under the same prompt"
        );

        rig.generator.prompt_version = "prompt-essai-v2".to_string();
        rig.generator.answer("sha256:a", "Fixe une règle A.");
        let report = rig.run(5).await;

        assert_eq!(keys(&rig).len(), 2);
        let stored = rig.lines.line("sha256:a").unwrap();
        assert_eq!(stored.status, BriefStatus::Ready);
        assert_eq!(stored.prompt_version, "prompt-essai-v2");
        assert_eq!(report.lines_written, 1);
    }

    #[tokio::test]
    async fn should_report_anomaly_and_make_no_call_when_line_repository_fails() {
        let rig = single(vec![text_article("a", "Contenu A.")]);
        *rig.lines.unreachable.lock().unwrap() = true;

        let report = rig.run(5).await;

        assert!(report
            .anomaly
            .as_deref()
            .unwrap()
            .contains("base injoignable"));
        assert!(keys(&rig).is_empty());
    }
}
