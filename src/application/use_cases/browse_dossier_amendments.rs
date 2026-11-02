use crate::application::ports::actor_repository::ActorRepository;
use crate::application::ports::amendment_repository::{
    AmendmentGroupOption, AmendmentPageRequest, AmendmentRepository, AmendmentSummary,
    DossierAmendmentCoverage,
};
use crate::application::ports::amendment_citation_repository::{
    AmendmentCitationRepository, AmendmentVote, CitationRow,
};
use crate::application::ports::RepositoryError;
use crate::domain::actor::{ActorDirectory, ActorUid, GroupUid, TextTone};
use crate::domain::amendment::{AmendmentUid, FateBanner, FateCode, SummaryPreview};
use crate::domain::amendment_citation::{LinkMethod, UnlinkedCause};

/// Un amendement pret a afficher: la ligne stockee, plus ce que le referentiel
/// permet de nommer.
///
/// Les noms ne sont pas denormalises en base, ils se resolvent ici — comme les
/// positions nominales d'un scrutin. Un acteur absent du referentiel garde son
/// identifiant et rien n'est devine (ACTEURS RM-04).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmendmentView {
    pub summary: AmendmentSummary,
    pub author: AuthorView,
    pub preview: SummaryPreview,
    pub fate_banner: FateBanner,
    pub votes: Vec<AmendmentVote>,
}

/// Auteur tel que la carte l'affiche: nom, groupe, et la couleur publiee du
/// groupe avec le ton du texte qui s'y lit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorView {
    pub name: Option<String>,
    pub official_url: Option<String>,
    pub group_label: Option<String>,
    pub group_abbrev: Option<String>,
    pub group_color: Option<String>,
    pub text_tone: Option<TextTone>,
}

/// Amendement cite par l'objet d'un scrutin, pret a afficher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitedAmendmentView {
    pub rank: u16,
    pub cited_number: Option<String>,
    pub dossier_uid: String,
    pub number: String,
    pub target_title: String,
    pub author: AuthorView,
    pub cosignatory_count: i64,
    pub preview: SummaryPreview,
    pub summary: Option<String>,
    pub method: Option<LinkMethod>,
    pub cause: Option<UnlinkedCause>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DossierAmendments {
    pub items: Vec<AmendmentView>,
    pub total: i64,
    pub coverage: DossierAmendmentCoverage,
    pub groups: Vec<AmendmentGroupOption>,
}

/// CU-02 — Consulter les amendements d'un dossier.
pub struct BrowseDossierAmendments<'a> {
    amendments: &'a dyn AmendmentRepository,
    actors: &'a dyn ActorRepository,
    citations: &'a dyn AmendmentCitationRepository,
}

impl<'a> BrowseDossierAmendments<'a> {
    pub fn new(
        amendments: &'a dyn AmendmentRepository,
        actors: &'a dyn ActorRepository,
        citations: &'a dyn AmendmentCitationRepository,
    ) -> Self {
        Self {
            amendments,
            actors,
            citations,
        }
    }

    pub async fn execute(
        &self,
        dossier_uid: &str,
        page: &AmendmentPageRequest,
    ) -> Result<DossierAmendments, RepositoryError> {
        let page = self.amendments.by_dossier(dossier_uid, page).await?;
        let coverage = self.amendments.dossier_coverage(dossier_uid).await?;
        let groups = self.amendments.groups_by_dossier(dossier_uid).await?;

        // Le referentiel n'est charge que pour les auteurs de la page affichee.
        let uids = author_uids(page.items.iter());
        let directory = self.actors.load_directory_for(&uids).await?;

        // Une seule lecture pour toute la page, chronologique (RM-08).
        let uids: Vec<AmendmentUid> = page
            .items
            .iter()
            .filter_map(|summary| AmendmentUid::new(summary.uid.clone()).ok())
            .collect();
        let votes = self.citations.scrutins_of_amendments(&uids).await?;

        let items = page
            .items
            .into_iter()
            .map(|summary| {
                let mut view = view_of(summary, &directory);
                view.votes = votes
                    .iter()
                    .filter(|vote| vote.amendment.as_str() == view.summary.uid)
                    .cloned()
                    .collect();
                view
            })
            .collect();

        Ok(DossierAmendments {
            items,
            total: page.total,
            coverage,
            groups,
        })
    }
}

/// Acteurs a charger pour nommer les auteurs de ces amendements.
pub fn author_uids<'s>(summaries: impl Iterator<Item = &'s AmendmentSummary>) -> Vec<ActorUid> {
    summaries
        .filter_map(|summary| summary.author_actor_uid.as_ref())
        .filter_map(|raw| ActorUid::new(raw.clone()).ok())
        .collect()
}

fn author_of(summary: &AmendmentSummary, directory: &ActorDirectory) -> AuthorView {
    let actor = summary
        .author_actor_uid
        .as_ref()
        .and_then(|raw| ActorUid::new(raw.clone()).ok())
        .and_then(|uid| directory.actor(&uid).cloned());

    // Le groupe affiche est celui stocke — donc celui de la date de depot
    // (RM-02). Le referentiel ne sert qu'a lui donner son libelle, jamais a le
    // recalculer sur l'appartenance courante.
    let group = summary
        .author_group_uid
        .as_ref()
        .and_then(|raw| GroupUid::new(raw.clone()).ok())
        .and_then(|uid| directory.group(&uid));

    AuthorView {
        name: actor.as_ref().map(|a| a.full_name()),
        official_url: actor.as_ref().and_then(|a| a.official_url()),
        group_label: group.map(|g| g.label().to_string()),
        group_abbrev: group.map(|g| g.abbrev().to_string()),
        group_color: group.and_then(|g| g.color().map(str::to_string)),
        text_tone: group.and_then(|g| g.text_tone()),
    }
}

/// Amendement cite d'un scrutin; `None` quand la citation n'a pas de lien.
pub fn cited_view_of(row: CitationRow, directory: &ActorDirectory) -> Option<CitedAmendmentView> {
    let cited = row.amendment?;
    let summary = cited.summary;
    Some(CitedAmendmentView {
        rank: row.rank,
        cited_number: row.cited_number,
        dossier_uid: cited.dossier_uid.as_str().to_string(),
        author: author_of(&summary, directory),
        preview: SummaryPreview::of(summary.summary.as_deref()),
        number: summary.number,
        target_title: summary.target_title,
        cosignatory_count: summary.cosignatory_count,
        summary: None,
        method: Some(row.method),
        cause: row.cause,
    })
}

fn view_of(summary: AmendmentSummary, directory: &ActorDirectory) -> AmendmentView {
    AmendmentView {
        author: author_of(&summary, directory),
        preview: SummaryPreview::of(summary.summary.as_deref()),
        fate_banner: FateBanner::of(
            // Seul `unspecified` annonce un sort non publie; le libelle des
            // autres codes est affiche tel que publie.
            if summary.fate_code == FateCode::Unspecified.as_str() {
                FateCode::Unspecified
            } else {
                FateCode::Other
            },
            &summary.fate_label,
            summary.state_label.as_deref(),
        ),
        summary,
        votes: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::use_cases::read_fakes::{amendment_row, FakeActors, FakeAmendments};
    use crate::application::use_cases::citation_fakes::InMemoryCitationRepository;
    use crate::domain::amendment::{AmendmentUid, PREVIEW_LENGTH};
    use crate::domain::scrutin::{ScrutinUid, VoteTally};
    use chrono::NaiveDate;

    fn vote(amendment: &str, scrutin: &str, day: u32) -> AmendmentVote {
        AmendmentVote {
            amendment: AmendmentUid::new(amendment.into()).unwrap(),
            scrutin_uid: ScrutinUid::new(scrutin.into()).unwrap(),
            scrutin_number: day.to_string(),
            date: NaiveDate::from_ymd_opt(2025, 1, day).unwrap(),
            tally: VoteTally::default(),
        }
    }

    async fn page_with_votes(
        rows: Vec<AmendmentSummary>,
        votes: Vec<AmendmentVote>,
        request: &AmendmentPageRequest,
    ) -> DossierAmendments {
        let amendments = FakeAmendments { rows };
        let actors = FakeActors { group_color: None };
        let citations = InMemoryCitationRepository::default();
        *citations.votes.lock().unwrap() = votes;
        BrowseDossierAmendments::new(&amendments, &actors, &citations)
            .execute("D1", request)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn should_list_scrutins_in_chronological_order_when_amendment_was_put_to_the_vote() {
        let votes = vec![
            vote("AM1", "S1", 10),
            vote("AM1", "S2", 20),
            vote("AM9", "S3", 5),
        ];
        let page = page_with_votes(
            vec![amendment_row()],
            votes,
            &AmendmentPageRequest::default(),
        )
        .await;
        let uids: Vec<&str> = page.items[0]
            .votes
            .iter()
            .map(|v| v.scrutin_uid.as_str())
            .collect();
        assert_eq!(uids, vec!["S1", "S2"]);
    }

    #[tokio::test]
    async fn should_list_no_scrutin_when_amendment_was_never_linked() {
        let votes = vec![vote("AM9", "S3", 5)];
        let page = page_with_votes(
            vec![amendment_row()],
            votes,
            &AmendmentPageRequest::default(),
        )
        .await;
        assert!(page.items[0].votes.is_empty());
    }

    #[tokio::test]
    async fn should_open_page_holding_the_number_when_card_is_reached_from_a_scrutin() {
        let mut other = amendment_row();
        other.uid = "AM2".into();
        other.number = "13".into();
        let request = AmendmentPageRequest {
            number: Some("13".into()),
            ..AmendmentPageRequest::default()
        };
        let page = page_with_votes(vec![amendment_row(), other], vec![], &request).await;
        let numbers: Vec<&str> = page
            .items
            .iter()
            .map(|i| i.summary.number.as_str())
            .collect();
        assert_eq!(numbers, vec!["13"]);
    }

    async fn view_of_row(row: AmendmentSummary, group_color: Option<&str>) -> AmendmentView {
        let amendments = FakeAmendments { rows: vec![row] };
        let actors = FakeActors {
            group_color: group_color.map(str::to_string),
        };
        let citations = InMemoryCitationRepository::default();
        BrowseDossierAmendments::new(&amendments, &actors, &citations)
            .execute("D1", &AmendmentPageRequest::default())
            .await
            .unwrap()
            .items
            .remove(0)
    }

    #[tokio::test]
    async fn should_paint_author_in_group_color_when_source_publishes_one() {
        let view = view_of_row(amendment_row(), Some("#1A237E")).await;
        assert_eq!(view.author.group_color.as_deref(), Some("#1A237E"));
        assert_eq!(view.author.text_tone, Some(TextTone::Light));
        assert_eq!(view.author.group_abbrev.as_deref(), Some("A"));
    }

    #[tokio::test]
    async fn should_give_no_color_when_group_publishes_none() {
        let view = view_of_row(amendment_row(), None).await;
        assert_eq!(view.author.group_color, None);
        assert_eq!(view.author.text_tone, None);
        assert_eq!(view.author.group_label.as_deref(), Some("Groupe A"));
    }

    #[tokio::test]
    async fn should_give_summary_preview_when_amendment_has_a_summary() {
        let mut row = amendment_row();
        let long = "x".repeat(PREVIEW_LENGTH + 50);
        row.summary = Some(long.clone());
        let view = view_of_row(row, None).await;
        assert_eq!(view.preview, SummaryPreview::of(Some(&long)));
        assert!(matches!(
            view.preview,
            SummaryPreview::Text {
                truncated: true,
                ..
            }
        ));
        // L'expose entier reste servi (RM-03).
        assert_eq!(view.summary.summary.as_deref(), Some(long.as_str()));
    }

    #[tokio::test]
    async fn should_announce_unpublished_fate_when_source_gives_no_fate() {
        let mut row = amendment_row();
        row.fate_code = "unspecified".into();
        row.fate_label = String::new();
        row.state_label = Some("En traitement".into());
        let view = view_of_row(row, None).await;
        assert_eq!(
            view.fate_banner.label(),
            "Sort non publi\u{00e9} (\u{00e9}tat : En traitement)"
        );
    }
}
