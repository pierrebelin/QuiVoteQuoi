use crate::application::ports::actor_repository::ActorRepository;
use crate::application::ports::amendment_repository::{
    AmendmentGroupOption, AmendmentPageRequest, AmendmentRepository, AmendmentSummary,
    DossierAmendmentCoverage,
};
use crate::application::ports::RepositoryError;
use crate::domain::actor::{ActorDirectory, ActorUid, GroupUid, TextTone};
use crate::domain::amendment::{FateBanner, FateCode, SummaryPreview};

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
}

impl<'a> BrowseDossierAmendments<'a> {
    pub fn new(amendments: &'a dyn AmendmentRepository, actors: &'a dyn ActorRepository) -> Self {
        Self { amendments, actors }
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
        let uids: Vec<ActorUid> = page
            .items
            .iter()
            .filter_map(|item| item.author_actor_uid.as_ref())
            .filter_map(|raw| ActorUid::new(raw.clone()).ok())
            .collect();
        let directory = self.actors.load_directory_for(&uids).await?;

        let items = page
            .items
            .into_iter()
            .map(|summary| view_of(summary, &directory))
            .collect();

        Ok(DossierAmendments {
            items,
            total: page.total,
            coverage,
            groups,
        })
    }
}

fn view_of(summary: AmendmentSummary, directory: &ActorDirectory) -> AmendmentView {
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

    AmendmentView {
        author: AuthorView {
            name: actor.as_ref().map(|a| a.full_name()),
            official_url: actor.as_ref().and_then(|a| a.official_url()),
            group_label: group.map(|g| g.label().to_string()),
            group_abbrev: group.map(|g| g.abbrev().to_string()),
            group_color: group.and_then(|g| g.color().map(str::to_string)),
            text_tone: group.and_then(|g| g.text_tone()),
        },
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::use_cases::read_fakes::{amendment_row, FakeActors, FakeAmendments};
    use crate::domain::amendment::PREVIEW_LENGTH;

    async fn view_of_row(row: AmendmentSummary, group_color: Option<&str>) -> AmendmentView {
        let amendments = FakeAmendments { rows: vec![row] };
        let actors = FakeActors {
            group_color: group_color.map(str::to_string),
        };
        BrowseDossierAmendments::new(&amendments, &actors)
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
