//! Citation d'un amendement par un scrutin.
//!
//! Voir `todo/sujet-amendements/SPEC-sujet-amendements.md`. L'objet d'un scrutin
//! cite des numeros d'amendement; chacun se resout en un amendement unique du
//! dossier publie par la source, ou en une cause d'absence de lien (RM-01 a RM-04,
//! RM-11).

use chrono::NaiveDate;

use super::amendment::{
    amendment_numbers_in_subject, cited_author_in_subject, AmendmentNumber, AmendmentTarget,
    AmendmentUid, CitedAuthor,
};
use super::article::ArticleNumber;
use super::dossier::DossierUid;
use super::scrutin::ScrutinUid;
use super::theme::lowercase;

/// Dossier dans lequel chercher les amendements cites (RM-04).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchDossier {
    Published(DossierUid),
    Missing,
}

impl SearchDossier {
    pub fn for_scrutin(published: Option<&DossierUid>) -> Self {
        match published {
            Some(uid) => Self::Published(uid.clone()),
            None => Self::Missing,
        }
    }
}

/// Auteur publie d'un amendement, tel que compare a l'objet du scrutin (RM-11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CandidateAuthor {
    /// `name` vaut `None` quand le repertoire ne connait pas l'acteur.
    Deputy { name: Option<String> },
    Institutional { label: String },
}

impl CandidateAuthor {
    /// Vrai seulement quand l'objet et la source designent deux auteurs differents.
    ///
    /// Un rapporteur peut porter en seance l'amendement de sa commission: un
    /// depute nomme ne contredit donc pas une commission. Un auteur inconnu ne
    /// contredit rien.
    fn contradicts(&self, cited: &CitedAuthor) -> bool {
        match (cited, self) {
            (CitedAuthor::NotStated, _) => false,
            (CitedAuthor::Government, Self::Deputy { .. }) => true,
            (CitedAuthor::Government, Self::Institutional { label }) => !is_government(label),
            (CitedAuthor::Deputy(_), Self::Institutional { label }) => is_government(label),
            (CitedAuthor::Deputy(_), Self::Deputy { name: None }) => false,
            (CitedAuthor::Deputy(cited), Self::Deputy { name: Some(name) }) => {
                !name_contains(name, cited)
            }
        }
    }
}

fn is_government(label: &str) -> bool {
    lowercase(label).contains("gouvernement")
}

/// Chaque mot du nom cite figure dans le nom publie, particules exceptees, sans
/// tenir compte de la casse, des accents ni des tirets.
fn name_contains(published: &str, cited: &str) -> bool {
    let published = comparable_words(published);
    comparable_words(cited)
        .iter()
        .all(|word| published.contains(word))
}

fn comparable_words(name: &str) -> Vec<String> {
    let folded: String = lowercase(name).chars().map(without_accent).collect();
    folded
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .filter(|word| !matches!(*word, "de" | "du" | "des" | "d" | "la" | "le" | "les" | "l"))
        .map(str::to_string)
        .collect()
}

fn without_accent(c: char) -> char {
    match c {
        '\u{e0}' | '\u{e2}' | '\u{e4}' => 'a',
        '\u{e7}' => 'c',
        '\u{e8}' | '\u{e9}' | '\u{ea}' | '\u{eb}' => 'e',
        '\u{ee}' | '\u{ef}' => 'i',
        '\u{f4}' | '\u{f6}' => 'o',
        '\u{f9}' | '\u{fb}' | '\u{fc}' => 'u',
        '\u{ff}' => 'y',
        other => other,
    }
}

/// Amendement d'un dossier, tel que propose au rapprochement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateAmendment {
    uid: AmendmentUid,
    dossier: DossierUid,
    number: AmendmentNumber,
    article: Option<ArticleNumber>,
    deposited_on: Option<NaiveDate>,
    author: CandidateAuthor,
}

impl CandidateAmendment {
    pub fn new(
        uid: AmendmentUid,
        dossier: DossierUid,
        number: AmendmentNumber,
        target: &AmendmentTarget,
        deposited_on: Option<NaiveDate>,
        author: CandidateAuthor,
    ) -> Self {
        Self {
            uid,
            dossier,
            number,
            article: target.article(),
            deposited_on,
            author,
        }
    }
}

/// Maniere dont un lien a ete etabli (RM-06).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkMethod {
    pub by_cited_article: bool,
}

impl LinkMethod {
    /// Libelles affiches: ce qui a servi a etablir le lien (RM-06).
    pub fn labels(&self) -> Vec<&'static str> {
        let mut labels = vec!["Num\u{e9}ro d'amendement cit\u{e9}"];
        if self.by_cited_article {
            labels.push("Article cit\u{e9}");
        }
        labels
    }
}

/// Cause d'absence de lien pour un numero cite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnlinkedCause {
    NoNumber,
    NoDossier,
    Unresolved,
    Ambiguous,
    AuthorMismatch,
}

impl UnlinkedCause {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NoNumber => "no_number",
            Self::NoDossier => "no_dossier",
            Self::Unresolved => "unresolved",
            Self::Ambiguous => "ambiguous",
            Self::AuthorMismatch => "author_mismatch",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "no_number" => Some(Self::NoNumber),
            "no_dossier" => Some(Self::NoDossier),
            "unresolved" => Some(Self::Unresolved),
            "ambiguous" => Some(Self::Ambiguous),
            "author_mismatch" => Some(Self::AuthorMismatch),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::NoNumber => "L'objet nomme un amendement sans en citer le num\u{e9}ro",
            Self::NoDossier => "La source ne rattache aucun dossier \u{e0} ce scrutin",
            Self::Unresolved => {
                "Aucun amendement du dossier ne correspond au num\u{e9}ro cit\u{e9}"
            }
            Self::Ambiguous => {
                "Plusieurs amendements du dossier correspondent au num\u{e9}ro cit\u{e9}"
            }
            Self::AuthorMismatch => {
                "L'amendement du dossier portant ce num\u{e9}ro a un autre auteur que celui nomm\u{e9} par l'objet"
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CitationResult {
    Linked {
        amendment: AmendmentUid,
        method: LinkMethod,
    },
    Unlinked(UnlinkedCause),
}

/// Resolution d'un numero cite (ou de l'absence de numero).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitationOutcome {
    pub rank: u16,
    pub cited_number: Option<AmendmentNumber>,
    pub result: CitationResult,
}

/// Citations d'amendements contenues dans l'objet d'un scrutin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrutinCitations {
    scrutin: ScrutinUid,
    date: NaiveDate,
    search: SearchDossier,
    article: Option<ArticleNumber>,
    numbers: Vec<AmendmentNumber>,
    author: CitedAuthor,
}

impl ScrutinCitations {
    /// `None` quand l'objet ne nomme aucun amendement.
    pub fn from_subject(
        scrutin: ScrutinUid,
        date: NaiveDate,
        subject: &str,
        search: SearchDossier,
    ) -> Option<Self> {
        if !subject.to_lowercase().contains("amendement") {
            return None;
        }
        Some(Self {
            scrutin,
            date,
            search,
            article: ArticleNumber::in_text(subject),
            numbers: amendment_numbers_in_subject(subject),
            author: cited_author_in_subject(subject),
        })
    }

    pub fn scrutin(&self) -> &ScrutinUid {
        &self.scrutin
    }

    pub fn search(&self) -> &SearchDossier {
        &self.search
    }

    /// Une issue par numero cite, dans l'ordre de l'objet (RM-01 a RM-04).
    pub fn resolve(&self, candidates: &[CandidateAmendment]) -> Vec<CitationOutcome> {
        if self.numbers.is_empty() {
            return vec![CitationOutcome {
                rank: 0,
                cited_number: None,
                result: CitationResult::Unlinked(UnlinkedCause::NoNumber),
            }];
        }
        let mut eligible: Vec<&CandidateAmendment> = Vec::new();
        if let SearchDossier::Published(dossier) = &self.search {
            for candidate in candidates {
                let in_time = candidate.deposited_on.map_or(true, |d| d <= self.date);
                if &candidate.dossier == dossier
                    && in_time
                    && !eligible.iter().any(|kept| kept.uid == candidate.uid)
                {
                    eligible.push(candidate);
                }
            }
        }
        self.numbers
            .iter()
            .enumerate()
            .map(|(index, cited)| CitationOutcome {
                rank: u16::try_from(index).unwrap_or(u16::MAX),
                cited_number: Some(cited.clone()),
                result: self.resolve_one(cited, &eligible),
            })
            .collect()
    }

    fn resolve_one(
        &self,
        cited: &AmendmentNumber,
        eligible: &[&CandidateAmendment],
    ) -> CitationResult {
        if self.search == SearchDossier::Missing {
            return CitationResult::Unlinked(UnlinkedCause::NoDossier);
        }
        let sharing: Vec<&CandidateAmendment> = eligible
            .iter()
            .copied()
            .filter(|c| c.number.answers(cited))
            .collect();
        let linked = |candidate: &CandidateAmendment, by_cited_article| {
            if candidate.author.contradicts(&self.author) {
                return CitationResult::Unlinked(UnlinkedCause::AuthorMismatch);
            }
            CitationResult::Linked {
                amendment: candidate.uid.clone(),
                method: LinkMethod { by_cited_article },
            }
        };
        match sharing.as_slice() {
            [] => CitationResult::Unlinked(UnlinkedCause::Unresolved),
            [only] => match (&self.article, &only.article) {
                (Some(cited_article), Some(article)) if cited_article != article => {
                    CitationResult::Unlinked(UnlinkedCause::Unresolved)
                }
                _ => linked(only, false),
            },
            several => {
                let Some(cited_article) = &self.article else {
                    return CitationResult::Unlinked(UnlinkedCause::Ambiguous);
                };
                let mut same_article = several
                    .iter()
                    .copied()
                    .filter(|c| c.article.as_ref() == Some(cited_article));
                match (same_article.next(), same_article.next()) {
                    (Some(one), None) => linked(one, true),
                    _ => CitationResult::Unlinked(UnlinkedCause::Ambiguous),
                }
            }
        }
    }
}

/// Nombre de citations d'une meme issue, tel que relu du stockage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CitationCount {
    pub cause: Option<UnlinkedCause>,
    pub by_cited_article: bool,
    pub count: i64,
}

/// Bilan d'une passe de rapprochement.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LinkingReport {
    pub linked: i64,
    pub linked_by_cited_article: i64,
    pub no_number: i64,
    pub no_dossier: i64,
    pub unresolved: i64,
    pub ambiguous: i64,
    pub author_mismatch: i64,
}

impl LinkingReport {
    pub fn from_counts(counts: &[CitationCount]) -> Self {
        let mut report = Self::default();
        for entry in counts {
            match entry.cause {
                None => {
                    report.linked += entry.count;
                    if entry.by_cited_article {
                        report.linked_by_cited_article += entry.count;
                    }
                }
                Some(UnlinkedCause::NoNumber) => report.no_number += entry.count,
                Some(UnlinkedCause::NoDossier) => report.no_dossier += entry.count,
                Some(UnlinkedCause::Unresolved) => report.unresolved += entry.count,
                Some(UnlinkedCause::Ambiguous) => report.ambiguous += entry.count,
                Some(UnlinkedCause::AuthorMismatch) => report.author_mismatch += entry.count,
            }
        }
        report
    }

    pub fn from_outcomes(outcomes: &[CitationOutcome]) -> Self {
        let counts: Vec<CitationCount> = outcomes
            .iter()
            .map(|o| match &o.result {
                CitationResult::Linked { method, .. } => CitationCount {
                    cause: None,
                    by_cited_article: method.by_cited_article,
                    count: 1,
                },
                CitationResult::Unlinked(cause) => CitationCount {
                    cause: Some(*cause),
                    by_cited_article: false,
                    count: 1,
                },
            })
            .collect();
        Self::from_counts(&counts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2025, 10, d).unwrap()
    }

    fn dossier(raw: &str) -> DossierUid {
        DossierUid::new(raw.to_string()).unwrap()
    }

    fn number(raw: &str) -> AmendmentNumber {
        AmendmentNumber::new(raw.to_string()).unwrap()
    }

    fn uid(raw: &str) -> AmendmentUid {
        AmendmentUid::new(raw.to_string()).unwrap()
    }

    fn candidate(
        id: &str,
        dossier_uid: &str,
        raw_number: &str,
        title: &str,
        deposited_on: Option<NaiveDate>,
    ) -> CandidateAmendment {
        let target = AmendmentTarget::new(title.to_string(), None).unwrap();
        CandidateAmendment::new(
            uid(id),
            dossier(dossier_uid),
            number(raw_number),
            &target,
            deposited_on,
            CandidateAuthor::Deputy { name: None },
        )
    }

    fn authored(id: &str, raw_number: &str, title: &str, author: CandidateAuthor) -> CandidateAmendment {
        let target = AmendmentTarget::new(title.to_string(), None).unwrap();
        CandidateAmendment::new(
            uid(id),
            dossier("DLR5L17N1"),
            number(raw_number),
            &target,
            Some(day(10)),
            author,
        )
    }

    fn deputy(name: &str) -> CandidateAuthor {
        CandidateAuthor::Deputy {
            name: Some(name.to_string()),
        }
    }

    fn government() -> CandidateAuthor {
        CandidateAuthor::Institutional {
            label: "Gouvernement".to_string(),
        }
    }

    fn published() -> SearchDossier {
        SearchDossier::Published(dossier("DLR5L17N1"))
    }

    fn citations(subject: &str, search: SearchDossier) -> ScrutinCitations {
        ScrutinCitations::from_subject(
            ScrutinUid::new("VTANR5L17V489".to_string()).unwrap(),
            day(20),
            subject,
            search,
        )
        .unwrap()
    }

    fn linked(id: &str, by_cited_article: bool) -> CitationResult {
        CitationResult::Linked {
            amendment: uid(id),
            method: LinkMethod { by_cited_article },
        }
    }

    fn outcome(rank: u16, cited: Option<&str>, result: CitationResult) -> CitationOutcome {
        CitationOutcome {
            rank,
            cited_number: cited.map(number),
            result,
        }
    }

    #[test]
    fn should_link_cited_number_when_a_single_candidate_shares_its_reference_number() {
        let cited = citations("l'amendement n° 12 de M. Martin du projet de loi", published());
        let candidates = [
            candidate("A12", "DLR5L17N1", "12", "Article 3", Some(day(10))),
            candidate("A13", "DLR5L17N1", "13", "Article 3", Some(day(10))),
        ];
        assert_eq!(
            cited.resolve(&candidates),
            vec![outcome(0, Some("12"), linked("A12", false))]
        );
    }

    #[test]
    fn should_link_each_cited_number_in_citation_order_when_subject_cites_several() {
        let cited = citations("les amendements n° 45 et 12 de M. Martin", published());
        let candidates = [
            candidate("A12", "DLR5L17N1", "12", "Article 3", Some(day(10))),
            candidate("A45", "DLR5L17N1", "45", "Article 3", Some(day(10))),
        ];
        assert_eq!(
            cited.resolve(&candidates),
            vec![
                outcome(0, Some("45"), linked("A45", false)),
                outcome(1, Some("12"), linked("A12", false)),
            ]
        );
    }

    #[test]
    fn should_ignore_candidate_when_it_was_deposited_after_the_scrutin() {
        let cited = citations("l'amendement n° 12 de M. Martin", published());
        let candidates = [
            candidate("LATE", "DLR5L17N1", "12", "Article 3", Some(day(21))),
            candidate("A12", "DLR5L17N1", "12", "Article 3", Some(day(20))),
        ];
        assert_eq!(
            cited.resolve(&candidates),
            vec![outcome(0, Some("12"), linked("A12", false))]
        );
    }

    #[test]
    fn should_keep_candidate_when_its_deposit_date_is_unknown() {
        let cited = citations("l'amendement n° 12 de M. Martin", published());
        let candidates = [candidate("A12", "DLR5L17N1", "12", "Article 3", None)];
        assert_eq!(
            cited.resolve(&candidates),
            vec![outcome(0, Some("12"), linked("A12", false))]
        );
    }

    #[test]
    fn should_link_by_cited_article_when_several_candidates_share_the_number() {
        let cited = citations(
            "l'amendement n° 243 de M. Martin après l'article 15 du projet de loi de finances",
            published(),
        );
        let candidates = [
            candidate("II-243", "DLR5L17N1", "II-243", "Article 52", Some(day(10))),
            candidate("I-243", "DLR5L17N1", "I-243", "Après l'article 15", Some(day(10))),
        ];
        assert_eq!(
            cited.resolve(&candidates),
            vec![outcome(0, Some("243"), linked("I-243", true))]
        );
    }

    #[test]
    fn should_mark_ambiguous_when_cited_article_does_not_separate_candidates() {
        let cited = citations("l'amendement n° 12 de M. Martin à l'article 3", published());
        let candidates = [
            candidate("A12a", "DLR5L17N1", "12", "Article 3", Some(day(10))),
            candidate("A12b", "DLR5L17N1", "12", "Article 3", Some(day(11))),
        ];
        assert_eq!(
            cited.resolve(&candidates),
            vec![outcome(
                0,
                Some("12"),
                CitationResult::Unlinked(UnlinkedCause::Ambiguous)
            )]
        );
    }

    #[test]
    fn should_mark_ambiguous_when_subject_cites_no_article_and_candidates_are_several() {
        let cited = citations("l'amendement n° 12 de M. Martin", published());
        let candidates = [
            candidate("A12a", "DLR5L17N1", "12", "Article 3", Some(day(10))),
            candidate("A12b", "DLR5L17N1", "12", "Article 4", Some(day(10))),
        ];
        assert_eq!(
            cited.resolve(&candidates),
            vec![outcome(
                0,
                Some("12"),
                CitationResult::Unlinked(UnlinkedCause::Ambiguous)
            )]
        );
    }

    #[test]
    fn should_mark_unresolved_when_single_candidate_targets_another_article() {
        let cited = citations("l'amendement n° 12 de M. Martin à l'article 15", published());
        let candidates = [candidate("A12", "DLR5L17N1", "12", "Article 52", Some(day(10)))];
        assert_eq!(
            cited.resolve(&candidates),
            vec![outcome(
                0,
                Some("12"),
                CitationResult::Unlinked(UnlinkedCause::Unresolved)
            )]
        );
    }

    #[test]
    fn should_mark_unresolved_when_no_candidate_shares_the_number() {
        let cited = citations("l'amendement n° 12 de M. Martin", published());
        let candidates = [
            candidate("A99", "DLR5L17N1", "99", "Article 3", Some(day(10))),
            candidate("OTHER", "DLR5L17N2", "12", "Article 3", Some(day(10))),
        ];
        assert_eq!(
            cited.resolve(&candidates),
            vec![outcome(
                0,
                Some("12"),
                CitationResult::Unlinked(UnlinkedCause::Unresolved)
            )]
        );
    }

    #[test]
    fn should_mark_author_mismatch_when_subject_names_government_and_candidate_is_a_deputy() {
        let cited = citations(
            "l'amendement n° 1 du Gouvernement à l'article liminaire du projet de loi de \
             finances pour 2027 (seconde délibération) (première lecture).",
            published(),
        );
        let candidates = [authored("I-1", "I-1", "Article 3", deputy("Laurent Wauquiez"))];
        assert_eq!(
            cited.resolve(&candidates),
            vec![outcome(
                0,
                Some("1"),
                CitationResult::Unlinked(UnlinkedCause::AuthorMismatch)
            )]
        );
    }

    #[test]
    fn should_link_government_amendment_when_its_published_author_is_the_government() {
        let cited = citations(
            "l'amendement n° 3990 du Gouvernement après l'article 27 du projet de loi de \
             finances pour 2027 (première lecture).",
            published(),
        );
        let candidates = [authored("I-3990", "I-3990", "Article 27", government())];
        assert_eq!(
            cited.resolve(&candidates),
            vec![outcome(0, Some("3990"), linked("I-3990", false))]
        );
    }

    #[test]
    fn should_mark_author_mismatch_when_subject_names_government_and_candidate_is_a_commission() {
        let cited = citations("l'amendement n° 1 du Gouvernement à l'article 3", published());
        let commission = CandidateAuthor::Institutional {
            label: "Commission".to_string(),
        };
        let candidates = [authored("A1", "1", "Article 3", commission)];
        assert_eq!(
            cited.resolve(&candidates)[0].result,
            CitationResult::Unlinked(UnlinkedCause::AuthorMismatch)
        );
    }

    #[test]
    fn should_mark_author_mismatch_when_subject_names_another_deputy() {
        let cited = citations("l'amendement n° 12 de M. Martin à l'article 3", published());
        let candidates = [authored("A12", "12", "Article 3", deputy("Laurent Wauquiez"))];
        assert_eq!(
            cited.resolve(&candidates)[0].result,
            CitationResult::Unlinked(UnlinkedCause::AuthorMismatch)
        );
    }

    #[test]
    fn should_mark_author_mismatch_when_subject_names_a_deputy_and_candidate_is_the_government() {
        let cited = citations("l'amendement n° 12 de M. Martin à l'article 3", published());
        let candidates = [authored("A12", "12", "Article 3", government())];
        assert_eq!(
            cited.resolve(&candidates)[0].result,
            CitationResult::Unlinked(UnlinkedCause::AuthorMismatch)
        );
    }

    #[test]
    fn should_link_when_named_deputy_matches_despite_accents_case_hyphens_and_particles() {
        let pairs = [
            ("de Mme Thiebault Martinez", "Céline Thiébault-Martinez"),
            ("de M. Le Coq", "Aurélien Le Coq"),
            ("de M. Courson", "Charles de Courson"),
            ("de Mme d'Intorni", "Christelle D'Intorni"),
            ("de Mme K/Bidi", "Émeline K/Bidi"),
            ("de M. Philippe Vigier", "Philippe Vigier"),
            ("de M.Renault", "Marc Renault"),
        ];
        for (clause, name) in pairs {
            let cited = citations(&format!("l'amendement n° 12 {clause} à l'article 3"), published());
            let candidates = [authored("A12", "12", "Article 3", deputy(name))];
            assert_eq!(
                cited.resolve(&candidates)[0].result,
                linked("A12", false),
                "{clause} / {name}"
            );
        }
    }

    #[test]
    fn should_link_when_named_deputy_cannot_be_compared() {
        let cited = citations("l'amendement n° 12 de M. Martin à l'article 3", published());
        let unknown = CandidateAuthor::Deputy { name: None };
        let commission = CandidateAuthor::Institutional {
            label: "Commission des finances".to_string(),
        };
        for author in [unknown, commission] {
            let candidates = [authored("A12", "12", "Article 3", author)];
            assert_eq!(cited.resolve(&candidates)[0].result, linked("A12", false));
        }
    }

    #[test]
    fn should_link_when_subject_states_no_comparable_author() {
        let subjects = [
            "l'amendement n° 5 de la commission des lois à l'article 3",
            "l'amendement n° 5 après l'article 3",
            "l'amendement n° 5 de M. le rapporteur à l'article 3",
        ];
        for subject in subjects {
            let cited = citations(subject, published());
            let candidates = [authored("A5", "5", "Article 3", deputy("Laurent Wauquiez"))];
            assert_eq!(cited.resolve(&candidates)[0].result, linked("A5", false), "{subject}");
        }
    }

    #[test]
    fn should_mark_author_mismatch_when_article_separates_candidates_but_author_differs() {
        let cited = citations("l'amendement n° 243 du Gouvernement après l'article 15", published());
        let candidates = [
            authored("II-243", "II-243", "Article 52", government()),
            authored("I-243", "I-243", "Après l'article 15", deputy("Laurent Wauquiez")),
        ];
        assert_eq!(
            cited.resolve(&candidates)[0].result,
            CitationResult::Unlinked(UnlinkedCause::AuthorMismatch)
        );
    }

    #[test]
    fn should_link_single_amendment_when_join_lists_it_twice() {
        let cited = citations("l'amendement n° 12 de M. Martin à l'article 3", published());
        let candidates = [
            candidate("A12", "DLR5L17N1", "12", "Article 3", Some(day(10))),
            candidate("A12", "DLR5L17N1", "12", "Article 3", Some(day(10))),
        ];
        assert_eq!(
            cited.resolve(&candidates),
            vec![outcome(0, Some("12"), linked("A12", false))]
        );
    }

    #[test]
    fn should_mark_no_dossier_when_source_attaches_no_dossier() {
        let cited = citations(
            "les amendements n° 12 et 45 de M. Martin",
            SearchDossier::for_scrutin(None),
        );
        let no_dossier = CitationResult::Unlinked(UnlinkedCause::NoDossier);
        assert_eq!(
            cited.resolve(&[]),
            vec![
                outcome(0, Some("12"), no_dossier.clone()),
                outcome(1, Some("45"), no_dossier),
            ]
        );
    }

    #[test]
    fn should_mark_no_number_when_subject_names_an_amendment_without_number() {
        let cited = citations("l'amendement de M. Martin à l'article 3", published());
        assert_eq!(
            cited.resolve(&[]),
            vec![outcome(
                0,
                None,
                CitationResult::Unlinked(UnlinkedCause::NoNumber)
            )]
        );
    }

    #[test]
    fn should_give_no_citation_when_subject_names_no_amendment() {
        assert_eq!(
            ScrutinCitations::from_subject(
                ScrutinUid::new("VTANR5L17V490".to_string()).unwrap(),
                day(20),
                "l'article 12 du projet de loi de finances",
                published(),
            ),
            None
        );
    }

    #[test]
    fn should_name_cited_number_and_article_method_when_link_used_them() {
        assert_eq!(
            LinkMethod { by_cited_article: false }.labels(),
            vec!["Numéro d'amendement cité"]
        );
        assert_eq!(
            LinkMethod { by_cited_article: true }.labels(),
            vec!["Numéro d'amendement cité", "Article cité"]
        );
    }

    #[test]
    fn should_name_each_cause_when_number_is_not_linked() {
        let expected = [
            (UnlinkedCause::NoNumber, "no_number", "L'objet nomme un amendement sans en citer le numéro"),
            (UnlinkedCause::NoDossier, "no_dossier", "La source ne rattache aucun dossier à ce scrutin"),
            (UnlinkedCause::Unresolved, "unresolved", "Aucun amendement du dossier ne correspond au numéro cité"),
            (UnlinkedCause::Ambiguous, "ambiguous", "Plusieurs amendements du dossier correspondent au numéro cité"),
            (UnlinkedCause::AuthorMismatch, "author_mismatch", "L'amendement du dossier portant ce numéro a un autre auteur que celui nommé par l'objet"),
        ];
        for (cause, code, label) in expected {
            assert_eq!(cause.as_str(), code);
            assert_eq!(UnlinkedCause::parse(code), Some(cause));
            assert_eq!(cause.label(), label);
        }
        assert_eq!(UnlinkedCause::parse("inconnu"), None);
    }

    #[test]
    fn should_count_links_by_method_and_absences_by_cause_when_outcomes_are_summed() {
        let outcomes = [
            outcome(0, Some("1"), linked("A1", false)),
            outcome(1, Some("2"), linked("A2", true)),
            outcome(2, Some("3"), linked("A3", true)),
            outcome(0, None, CitationResult::Unlinked(UnlinkedCause::NoNumber)),
            outcome(0, Some("4"), CitationResult::Unlinked(UnlinkedCause::NoDossier)),
            outcome(1, Some("5"), CitationResult::Unlinked(UnlinkedCause::NoDossier)),
            outcome(0, Some("6"), CitationResult::Unlinked(UnlinkedCause::Unresolved)),
            outcome(0, Some("7"), CitationResult::Unlinked(UnlinkedCause::Ambiguous)),
            outcome(0, Some("8"), CitationResult::Unlinked(UnlinkedCause::AuthorMismatch)),
        ];
        let expected = LinkingReport {
            linked: 3,
            linked_by_cited_article: 2,
            no_number: 1,
            no_dossier: 2,
            unresolved: 1,
            ambiguous: 1,
            author_mismatch: 1,
        };
        assert_eq!(LinkingReport::from_outcomes(&outcomes), expected);
        assert_eq!(
            LinkingReport::from_counts(&[
                CitationCount { cause: None, by_cited_article: false, count: 1 },
                CitationCount { cause: None, by_cited_article: true, count: 2 },
                CitationCount { cause: Some(UnlinkedCause::NoNumber), by_cited_article: false, count: 1 },
                CitationCount { cause: Some(UnlinkedCause::NoDossier), by_cited_article: false, count: 2 },
                CitationCount { cause: Some(UnlinkedCause::Unresolved), by_cited_article: false, count: 1 },
                CitationCount { cause: Some(UnlinkedCause::Ambiguous), by_cited_article: false, count: 1 },
                CitationCount { cause: Some(UnlinkedCause::AuthorMismatch), by_cited_article: false, count: 1 },
            ]),
            expected
        );
    }
}
