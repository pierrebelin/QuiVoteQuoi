//! Lignes « En bref » enregistrées et choix de ce qu'une passe en fait
//! (SPEC-en-bref RM-05, RM-06, RM-13, RM-15, RM-17).

use std::collections::HashMap;

use chrono::{NaiveDate, NaiveDateTime};

use super::dossier::DossierUid;
use super::law_text::{is_deleted, ArticleFingerprint, VersionArticle};
use super::text_version::{
    latest_available, BriefCandidate, Chamber, DocumentUid, ReadingStatus, TextVersion,
};

/// RM-06 : la ligne que le code écrit, sans appel au modèle, pour un article
/// supprimé.
pub const DELETED_ARTICLE_LINE: &str = "Article supprimé à ce stade de la navette.";

/// « Modèle » d'une ligne écrite par le code : aucun modèle n'a été appelé.
pub const RULE_MODEL: &str = "rule";

/// CU-02, Erreurs : au troisième échec consécutif du modèle pour une même
/// version, les articles envoyés sont refusés.
pub const MAX_GENERATION_FAILURES: u8 = 3;

/// État d'une ligne enregistrée : publiable, ou refusée avec son motif (RM-13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BriefStatus {
    Ready,
    Rejected,
}

impl BriefStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            BriefStatus::Ready => "ready",
            BriefStatus::Rejected => "rejected",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "ready" => Some(BriefStatus::Ready),
            "rejected" => Some(BriefStatus::Rejected),
            _ => None,
        }
    }
}

/// Ligne « En bref » telle qu'elle est enregistrée, avec sa traçabilité
/// (RM-16). Elle appartient à l'empreinte de son article, pas à un dossier
/// (RM-05).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredBriefLine {
    pub fingerprint: ArticleFingerprint,
    pub status: BriefStatus,
    /// Le texte de la ligne, quand elle est publiable.
    pub line: Option<String>,
    /// Code du motif de refus, quand elle est refusée.
    pub rejection: Option<String>,
    /// Ce que le contrôle a refusé, quand il le précise (RM-13).
    pub rejection_detail: Option<String>,
    /// La ligne écartée, quand le modèle en a rendu une (RM-13).
    pub rejected_line: Option<String>,
    /// Modèle qui l'a produite, ou `rule` quand c'est le code (RM-06).
    pub model: String,
    pub prompt_version: String,
    pub generated_at: NaiveDateTime,
    /// Articles cités transmis au modèle avec leur date de rédaction.
    pub cited_sources: Vec<String>,
}

/// Ce que la passe fait d'un article de la dernière version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArticleAction {
    /// Son empreinte a une ligne publiable : reprise (RM-05).
    Reuse,
    /// Son empreinte est refusée sous le prompt courant : gardée refusée
    /// (RM-13).
    Keep,
    /// Article supprimé : le code écrit la ligne fixe (RM-06).
    WriteDeleted,
    /// À résumer par le modèle.
    Send,
}

/// RM-05, RM-06, RM-13, RM-17 : classe un article d'après la ligne que son
/// empreinte porte déjà. Une ligne publiable est reprise quel que soit le
/// prompt qui l'a produite ; un refus n'est gardé que sous le prompt courant.
pub fn plan_article(
    article: &VersionArticle,
    stored: Option<&StoredBriefLine>,
    prompt_version: &str,
) -> ArticleAction {
    match stored {
        Some(line) if line.status == BriefStatus::Ready => ArticleAction::Reuse,
        _ if is_deleted(article.content()) => ArticleAction::WriteDeleted,
        Some(line) if line.prompt_version == prompt_version => ArticleAction::Keep,
        _ => ArticleAction::Send,
    }
}

/// Ce que « En bref » montre d'un article (RM-13, CU-03, étape 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArticleBriefState {
    /// La ligne publiable de son empreinte.
    Ready(String),
    /// Ni résumé ni refusé : « résumé en attente ».
    Pending,
    /// Refusé, ou sans ligne dans une version précédente : « résumé
    /// indisponible ».
    Unavailable,
}

impl ArticleBriefState {
    /// CU-03, étape 5 : une ligne publiable est montrée dans toute version,
    /// quel que soit le prompt qui l'a produite. Sans elle, un article de la
    /// dernière version est refusé sous le prompt courant (RM-13), ou en
    /// attente : sans ligne, ou refusé sous un autre prompt, dont la passe
    /// retentera sa chance (RM-17). Dans une version précédente, la passe ne
    /// reviendra pas : l'article est indisponible.
    pub fn of(stored: Option<&StoredBriefLine>, is_latest: bool, prompt_version: &str) -> Self {
        match stored {
            Some(StoredBriefLine {
                status: BriefStatus::Ready,
                line: Some(text),
                ..
            }) => Self::Ready(text.clone()),
            _ if !is_latest => Self::Unavailable,
            Some(line)
                if line.status == BriefStatus::Rejected
                    && line.prompt_version == prompt_version =>
            {
                Self::Unavailable
            }
            _ => Self::Pending,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ready(_) => "ready",
            Self::Pending => "pending",
            Self::Unavailable => "unavailable",
        }
    }
}

/// Avancement de la passe sur la dernière version d'un dossier, enregistré
/// après chaque dossier traité (RM-08, RM-15).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BriefProgress {
    /// La version dont la passe a classé les articles.
    pub document_uid: DocumentUid,
    pub prompt_version: String,
    /// Articles sans ligne : source manquante ou appel en échec.
    pub pending_count: usize,
    /// Articles refusés sous `prompt_version`.
    pub rejected_count: usize,
    pub attempted_on: NaiveDate,
}

/// RM-15 : les dossiers que la passe traite, au vote le plus récent d'abord
/// (la référence départage), dans la limite du plafond.
///
/// Un dossier est retenu si une version publiée par l'Assemblée n'est pas lue ;
/// ou, quand il a une version disponible, s'il n'a pas d'avancement, si sa
/// dernière version n'est pas celle de l'avancement, s'il lui reste des
/// articles en attente, ou s'il a des refus donnés sous une autre version du
/// prompt. Un dossier tenté aujourd'hui qui a encore des articles en attente
/// passe après les autres : sans cela, les dossiers les plus récemment votés
/// occuperaient le plafond à chaque passe tant que Légifrance ne répond pas.
pub fn brief_pass_order(
    candidates: Vec<BriefCandidate>,
    versions: &[TextVersion],
    progress: &[(DossierUid, BriefProgress)],
    prompt_version: &str,
    today: NaiveDate,
    cap: usize,
) -> Vec<DossierUid> {
    let mut versions_of: HashMap<&DossierUid, Vec<TextVersion>> = HashMap::new();
    for version in versions {
        versions_of
            .entry(version.dossier_uid())
            .or_default()
            .push(version.clone());
    }
    let progress_of: HashMap<&DossierUid, &BriefProgress> =
        progress.iter().map(|(dossier, p)| (dossier, p)).collect();

    let mut waiting: Vec<(bool, BriefCandidate)> = candidates
        .into_iter()
        .filter_map(|candidate| {
            let own = versions_of
                .get(&candidate.dossier_uid)
                .map_or(&[][..], Vec::as_slice);
            let advance = progress_of.get(&candidate.dossier_uid).copied();
            needs_a_pass(own, advance, prompt_version)
                .then(|| (is_stuck_today(advance, today), candidate))
        })
        .collect();
    waiting.sort_by(|(stuck_a, a), (stuck_b, b)| {
        stuck_a
            .cmp(stuck_b)
            .then_with(|| b.last_vote_on.cmp(&a.last_vote_on))
            .then_with(|| a.dossier_uid.as_str().cmp(b.dossier_uid.as_str()))
    });
    waiting
        .into_iter()
        .take(cap)
        .map(|(_, candidate)| candidate.dossier_uid)
        .collect()
}

/// Le dossier a-t-il quelque chose à lire ou à résumer ?
fn needs_a_pass(
    versions: &[TextVersion],
    progress: Option<&BriefProgress>,
    prompt_version: &str,
) -> bool {
    let has_unread_text = versions
        .iter()
        .any(|v| v.chamber() == Chamber::Assembly && v.reading() == ReadingStatus::Unread);
    if has_unread_text {
        return true;
    }
    let Some(latest) = latest_available(versions) else {
        return false;
    };
    match progress {
        None => true,
        Some(done) => {
            latest.document_uid() != &done.document_uid
                || done.pending_count > 0
                || (done.rejected_count > 0 && done.prompt_version != prompt_version)
        }
    }
}

/// Tenté aujourd'hui avec encore des articles en attente : la source n'a pas
/// répondu, la reprendre avant les autres ne servirait à rien.
fn is_stuck_today(progress: Option<&BriefProgress>, today: NaiveDate) -> bool {
    progress.is_some_and(|done| done.attempted_on == today && done.pending_count > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::law_text::NewVersionArticle;
    use crate::domain::text_version::{Chamber, NewTextVersion, ReadingStatus, VersionKind};

    const PROMPT: &str = "text-brief-v2";
    const OLD_PROMPT: &str = "text-brief-v1";

    fn day(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn article(content: &str) -> VersionArticle {
        VersionArticle::new(NewVersionArticle {
            label: "1er".to_string(),
            division: None,
            rank: 1,
            content: content.to_string(),
            fingerprint: ArticleFingerprint::new("sha256:abc".to_string()),
        })
    }

    fn stored(status: BriefStatus, prompt_version: &str) -> StoredBriefLine {
        StoredBriefLine {
            fingerprint: ArticleFingerprint::new("sha256:abc".to_string()),
            status,
            line: (status == BriefStatus::Ready)
                .then(|| "Fixe un délai de trois mois.".to_string()),
            rejection: (status == BriefStatus::Rejected).then(|| "too_long".to_string()),
            rejection_detail: None,
            rejected_line: None,
            model: "modele".to_string(),
            prompt_version: prompt_version.to_string(),
            generated_at: day(2026, 11, 3).and_hms_opt(0, 0, 0).unwrap(),
            cited_sources: Vec::new(),
        }
    }

    #[test]
    fn should_reuse_line_when_fingerprint_already_has_a_ready_line() {
        let line = stored(BriefStatus::Ready, PROMPT);

        let action = plan_article(&article("Contenu."), Some(&line), PROMPT);

        assert_eq!(action, ArticleAction::Reuse);
    }

    #[test]
    fn should_reuse_line_when_ready_line_came_from_an_older_prompt() {
        let line = stored(BriefStatus::Ready, OLD_PROMPT);

        let action = plan_article(&article("Contenu."), Some(&line), PROMPT);

        assert_eq!(action, ArticleAction::Reuse);
    }

    #[test]
    fn should_keep_refusal_when_rejected_under_the_current_prompt() {
        let line = stored(BriefStatus::Rejected, PROMPT);

        let action = plan_article(&article("Contenu."), Some(&line), PROMPT);

        assert_eq!(action, ArticleAction::Keep);
    }

    #[test]
    fn should_send_again_when_rejected_under_an_older_prompt() {
        let line = stored(BriefStatus::Rejected, OLD_PROMPT);

        let action = plan_article(&article("Contenu."), Some(&line), PROMPT);

        assert_eq!(action, ArticleAction::Send);
    }

    #[test]
    fn should_write_fixed_line_when_article_is_deleted() {
        let action = plan_article(&article("(Supprimé)"), None, PROMPT);

        assert_eq!(action, ArticleAction::WriteDeleted);
        assert_eq!(
            DELETED_ARTICLE_LINE,
            "Article supprimé à ce stade de la navette."
        );
    }

    #[test]
    fn should_send_when_fingerprint_is_unknown() {
        let action = plan_article(&article("Le délai est de trois mois."), None, PROMPT);

        assert_eq!(action, ArticleAction::Send);
    }

    #[test]
    fn should_reuse_fixed_line_when_deleted_article_already_has_one() {
        let line = stored(BriefStatus::Ready, PROMPT);

        let action = plan_article(&article("(Supprimé)"), Some(&line), PROMPT);

        assert_eq!(action, ArticleAction::Reuse);
    }

    #[test]
    fn should_show_line_when_fingerprint_has_a_ready_line() {
        let line = stored(BriefStatus::Ready, PROMPT);

        for is_latest in [true, false] {
            let state = ArticleBriefState::of(Some(&line), is_latest, PROMPT);

            assert_eq!(
                state,
                ArticleBriefState::Ready("Fixe un délai de trois mois.".to_string())
            );
        }
    }

    #[test]
    fn should_show_line_when_ready_line_came_from_an_older_prompt() {
        let line = stored(BriefStatus::Ready, OLD_PROMPT);

        let state = ArticleBriefState::of(Some(&line), true, PROMPT);

        assert_eq!(
            state,
            ArticleBriefState::Ready("Fixe un délai de trois mois.".to_string())
        );
    }

    #[test]
    fn should_show_pending_when_latest_version_article_has_no_line() {
        let state = ArticleBriefState::of(None, true, PROMPT);

        assert_eq!(state, ArticleBriefState::Pending);
    }

    #[test]
    fn should_show_unavailable_when_older_version_article_has_no_line() {
        let state = ArticleBriefState::of(None, false, PROMPT);

        assert_eq!(state, ArticleBriefState::Unavailable);
    }

    #[test]
    fn should_show_unavailable_when_line_was_rejected() {
        let line = stored(BriefStatus::Rejected, PROMPT);

        assert_eq!(
            ArticleBriefState::of(Some(&line), true, PROMPT),
            ArticleBriefState::Unavailable
        );
        assert_eq!(
            ArticleBriefState::of(Some(&line), false, PROMPT),
            ArticleBriefState::Unavailable
        );
    }

    #[test]
    fn should_show_pending_when_latest_article_was_rejected_under_an_older_prompt() {
        let line = stored(BriefStatus::Rejected, OLD_PROMPT);

        assert_eq!(
            ArticleBriefState::of(Some(&line), true, PROMPT),
            ArticleBriefState::Pending
        );
        assert_eq!(
            ArticleBriefState::of(Some(&line), false, PROMPT),
            ArticleBriefState::Unavailable
        );
    }

    #[test]
    fn should_show_no_line_when_a_ready_line_carries_no_text() {
        let mut line = stored(BriefStatus::Ready, PROMPT);
        line.line = None;

        assert_eq!(
            ArticleBriefState::of(Some(&line), true, PROMPT),
            ArticleBriefState::Pending
        );
        assert_eq!(
            ArticleBriefState::of(Some(&line), false, PROMPT),
            ArticleBriefState::Unavailable
        );
    }

    #[test]
    fn should_name_article_states_when_written_for_the_api() {
        assert_eq!(ArticleBriefState::Ready("x".to_string()).as_str(), "ready");
        assert_eq!(ArticleBriefState::Pending.as_str(), "pending");
        assert_eq!(ArticleBriefState::Unavailable.as_str(), "unavailable");
    }

    #[test]
    fn should_round_trip_status_when_parsed_back() {
        for status in [BriefStatus::Ready, BriefStatus::Rejected] {
            assert_eq!(BriefStatus::parse(status.as_str()), Some(status));
        }
        assert_eq!(BriefStatus::parse("pending"), None);
    }

    fn dossier(uid: &str) -> DossierUid {
        DossierUid::new(uid.to_string()).unwrap()
    }

    fn candidate(uid: &str, last_vote_on: NaiveDate) -> BriefCandidate {
        BriefCandidate {
            dossier_uid: dossier(uid),
            last_vote_on,
        }
    }

    fn version(
        uid: &str,
        dossier_uid: &str,
        chamber: Chamber,
        on: NaiveDate,
        reading: ReadingStatus,
    ) -> TextVersion {
        TextVersion::new(NewTextVersion {
            document_uid: DocumentUid::new(uid.to_string()).unwrap(),
            dossier_uid: dossier(dossier_uid),
            kind: VersionKind::Deposited,
            chamber,
            deposited_on: on,
            title: "titre".to_string(),
            reading,
        })
    }

    /// Version lue, publiée par l'Assemblée.
    fn read_version(uid: &str, dossier_uid: &str, on: NaiveDate) -> TextVersion {
        version(uid, dossier_uid, Chamber::Assembly, on, ReadingStatus::Read)
    }

    fn progress_of(
        dossier_uid: &str,
        document_uid: &str,
        prompt_version: &str,
        pending_count: usize,
        rejected_count: usize,
        attempted_on: NaiveDate,
    ) -> (DossierUid, BriefProgress) {
        (
            dossier(dossier_uid),
            BriefProgress {
                document_uid: DocumentUid::new(document_uid.to_string()).unwrap(),
                prompt_version: prompt_version.to_string(),
                pending_count,
                rejected_count,
                attempted_on,
            },
        )
    }

    fn uids(order: &[DossierUid]) -> Vec<&str> {
        order.iter().map(DossierUid::as_str).collect()
    }

    fn order(
        candidates: Vec<BriefCandidate>,
        versions: &[TextVersion],
        progress: &[(DossierUid, BriefProgress)],
        cap: usize,
    ) -> Vec<DossierUid> {
        brief_pass_order(
            candidates,
            versions,
            progress,
            PROMPT,
            day(2026, 11, 3),
            cap,
        )
    }

    #[test]
    fn should_select_dossier_when_its_latest_version_is_not_the_one_already_briefed() {
        let versions = [
            read_version("PIONANR5L17B0001", "D1", day(2026, 1, 1)),
            read_version("PIONANR5L17BTA0002", "D1", day(2026, 3, 1)),
        ];
        let progress = [progress_of(
            "D1",
            "PIONANR5L17B0001",
            PROMPT,
            0,
            0,
            day(2026, 2, 1),
        )];

        let selected = order(
            vec![candidate("D1", day(2026, 6, 1))],
            &versions,
            &progress,
            5,
        );

        assert_eq!(uids(&selected), vec!["D1"]);
    }

    #[test]
    fn should_skip_dossier_when_its_latest_version_is_briefed_without_pending_article() {
        let versions = [read_version("PIONANR5L17B0001", "D1", day(2026, 1, 1))];
        // Des refus sous le prompt courant ne justifient pas un nouveau passage.
        let progress = [progress_of(
            "D1",
            "PIONANR5L17B0001",
            PROMPT,
            0,
            2,
            day(2026, 2, 1),
        )];

        let selected = order(
            vec![candidate("D1", day(2026, 6, 1))],
            &versions,
            &progress,
            5,
        );

        assert!(selected.is_empty());
    }

    #[test]
    fn should_select_dossier_with_refusals_when_prompt_version_changed() {
        let versions = [read_version("PIONANR5L17B0001", "D1", day(2026, 1, 1))];
        let progress = [progress_of(
            "D1",
            "PIONANR5L17B0001",
            OLD_PROMPT,
            0,
            2,
            day(2026, 2, 1),
        )];

        let selected = order(
            vec![candidate("D1", day(2026, 6, 1))],
            &versions,
            &progress,
            5,
        );

        assert_eq!(uids(&selected), vec!["D1"]);
    }

    #[test]
    fn should_leave_dossier_without_refusals_out_when_prompt_version_changed() {
        let versions = [read_version("PIONANR5L17B0001", "D1", day(2026, 1, 1))];
        let progress = [progress_of(
            "D1",
            "PIONANR5L17B0001",
            OLD_PROMPT,
            0,
            0,
            day(2026, 2, 1),
        )];

        let selected = order(
            vec![candidate("D1", day(2026, 6, 1))],
            &versions,
            &progress,
            5,
        );

        assert!(selected.is_empty());
    }

    #[test]
    fn should_put_dossier_attempted_today_last_when_it_still_has_pending_articles() {
        let versions = [
            read_version("PIONANR5L17B0001", "D-NEW", day(2026, 1, 1)),
            read_version("PIONANR5L17B0002", "D-OLD", day(2026, 1, 1)),
            read_version("PIONANR5L17B0003", "D-MID", day(2026, 1, 1)),
        ];
        let progress = [
            progress_of("D-NEW", "PIONANR5L17B0001", PROMPT, 4, 0, day(2026, 11, 3)),
            progress_of("D-OLD", "PIONANR5L17B0002", PROMPT, 4, 0, day(2026, 11, 2)),
            progress_of("D-MID", "PIONANR5L17B0003", PROMPT, 4, 0, day(2026, 11, 2)),
        ];
        let candidates = vec![
            candidate("D-NEW", day(2026, 6, 1)),
            candidate("D-OLD", day(2025, 1, 1)),
            candidate("D-MID", day(2026, 1, 1)),
        ];

        let selected = order(candidates, &versions, &progress, 5);

        assert_eq!(uids(&selected), vec!["D-MID", "D-OLD", "D-NEW"]);
    }

    #[test]
    fn should_select_dossier_when_a_version_is_still_unread() {
        let versions = [
            read_version("PIONANR5L17B0001", "D1", day(2026, 1, 1)),
            version(
                "PIONANR5L17BTA0002",
                "D1",
                Chamber::Assembly,
                day(2026, 3, 1),
                ReadingStatus::Unread,
            ),
        ];
        let progress = [progress_of(
            "D1",
            "PIONANR5L17BTA0002",
            PROMPT,
            0,
            0,
            day(2026, 2, 1),
        )];

        let selected = order(
            vec![candidate("D1", day(2026, 6, 1))],
            &versions,
            &progress,
            5,
        );

        assert_eq!(uids(&selected), vec!["D1"]);
    }

    #[test]
    fn should_select_dossier_when_it_has_no_progress_yet() {
        let versions = [read_version("PIONANR5L17B0001", "D1", day(2026, 1, 1))];

        let selected = order(vec![candidate("D1", day(2026, 6, 1))], &versions, &[], 5);

        assert_eq!(uids(&selected), vec!["D1"]);
    }

    #[test]
    fn should_select_dossier_when_articles_are_still_pending_from_an_earlier_day() {
        let versions = [read_version("PIONANR5L17B0001", "D1", day(2026, 1, 1))];
        let progress = [progress_of(
            "D1",
            "PIONANR5L17B0001",
            PROMPT,
            3,
            0,
            day(2026, 11, 2),
        )];

        let selected = order(
            vec![candidate("D1", day(2026, 6, 1))],
            &versions,
            &progress,
            5,
        );

        assert_eq!(uids(&selected), vec!["D1"]);
    }

    #[test]
    fn should_leave_dossier_out_when_assembly_publishes_none_of_its_versions() {
        let versions = [version(
            "PIONSNR5S479B0440",
            "D-SENATE-ONLY",
            Chamber::Senate,
            day(2026, 1, 1),
            ReadingStatus::Unread,
        )];

        let selected = order(
            vec![candidate("D-SENATE-ONLY", day(2026, 6, 1))],
            &versions,
            &[],
            5,
        );

        assert!(selected.is_empty());
    }

    #[test]
    fn should_order_pass_by_most_recent_vote_and_keep_cap_when_candidates_exceed_it() {
        let versions = [
            read_version("PIONANR5L17B0001", "D-OLD", day(2026, 1, 1)),
            read_version("PIONANR5L17B0002", "D-NEW", day(2026, 1, 1)),
            read_version("PIONANR5L17B0003", "D-MID", day(2026, 1, 1)),
        ];
        let candidates = vec![
            candidate("D-OLD", day(2025, 1, 1)),
            candidate("D-NEW", day(2026, 6, 1)),
            candidate("D-MID", day(2026, 1, 1)),
        ];

        let selected = order(candidates, &versions, &[], 2);

        assert_eq!(uids(&selected), vec!["D-NEW", "D-MID"]);
    }

    #[test]
    fn should_break_a_tie_on_vote_date_by_dossier_reference() {
        let versions = [
            read_version("PIONANR5L17B0001", "D-B", day(2026, 1, 1)),
            read_version("PIONANR5L17B0002", "D-A", day(2026, 1, 1)),
        ];
        let candidates = vec![
            candidate("D-B", day(2026, 6, 1)),
            candidate("D-A", day(2026, 6, 1)),
        ];

        let selected = order(candidates, &versions, &[], 5);

        assert_eq!(uids(&selected), vec!["D-A", "D-B"]);
    }

    #[test]
    fn should_keep_no_dossier_when_cap_is_zero() {
        let versions = [read_version("PIONANR5L17B0001", "D1", day(2026, 1, 1))];

        let selected = order(vec![candidate("D1", day(2026, 6, 1))], &versions, &[], 0);

        assert!(selected.is_empty());
    }
}
