//! Vote sur l'ensemble d'un texte.
//!
//! Un texte porte des dizaines de scrutins — amendements, articles, motions de
//! procedure. Un seul tranche le texte entier: celui dont l'objet commence par
//! « l'ensemble ». Agreger tous les scrutins d'un texte donnerait un chiffre
//! sans signification: un groupe qui vote contre quarante amendements puis pour
//! le texte en ressortirait « majoritairement contre » (README.md §6).
//!
//! Le perimetre de cette page est donc restreint, et la restriction est
//! affichee: les autres scrutins restent tous consultables sur `/scrutins`
//! (README.md §2).

use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;

use super::actor::GroupUid;
use super::scrutin::{Outcome, VotePosition, VoteTally};

/// Debuts d'objet designant un vote sur l'ensemble d'un texte.
///
/// Les deux formes d'apostrophe sont presentes dans la source: sur les 222
/// votes sur l'ensemble, 212 objets portent l'apostrophe droite et 10 la
/// typographique. N'en retenir qu'une en laisserait dix hors de la page.
pub const FINAL_VOTE_MARKERS: [&str; 2] = ["l'ensemble ", "l\u{2019}ensemble "];

/// Vrai quand l'objet du scrutin porte sur l'ensemble d'un texte.
///
/// Regle deterministe, rejouable a l'identique: aucun modele n'intervient.
pub fn is_final_vote_subject(subject: &str) -> bool {
    let lowered = subject.trim_start().to_lowercase();
    FINAL_VOTE_MARKERS
        .iter()
        .any(|marker| lowered.starts_with(marker))
}

/// Mention de lecture d'un vote, reprise telle quelle a la source.
///
/// L'extraction du texte debattu retire les mentions de stade de la cle: sans
/// elles, deux lectures du meme texte donnent deux lignes identiques a l'ecran.
/// La mention est reprise entre parentheses, jamais reformulee, et tue quand le
/// libelle du texte la porte deja — la commission mixte paritaire, par exemple,
/// n'est pas retiree du libelle.
pub fn reading_of(subject: &str, text_label: &str) -> Option<String> {
    let trimmed = subject.trim().trim_end_matches('.').trim_end();
    let inner = trimmed.strip_suffix(')')?;
    let open = inner.rfind('(')?;
    let reading = inner[open + 1..].trim();

    if reading.is_empty() || text_label.to_lowercase().contains(&reading.to_lowercase()) {
        return None;
    }
    Some(reading.to_string())
}

/// Part des votants d'un groupe sur un vote.
///
/// Denominateur: pour + contre + abstention. Les non-votants en sont exclus —
/// ils ne se prononcent pas — et restent affiches en valeur brute a cote.
///
/// Les trois parts sont arrondies a la plus forte moyenne pour totaliser 100:
/// trois arrondis independants afficheraient « 33 % / 33 % / 33 % » sur un
/// groupe qui a pourtant vote a l'unanimite des votants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VoterShare {
    pub voters: u16,
    pub for_percent: u8,
    pub against_percent: u8,
    pub abstention_percent: u8,
    /// Positions arrivant en tete, lues sur les comptes bruts et non sur les
    /// parts arrondies — 8 pour / 6 contre / 8 abstentions donne 37 / 27 / 36
    /// une fois arrondi, alors que l'egalite est bien reelle.
    ///
    /// Un seul element dans le cas courant. Plusieurs a egalite: 20 des 2 547
    /// positions de groupe sur un vote sur l'ensemble sont dans ce cas (mesure
    /// du 06/08/2026), et les nommer vaut mieux que d'en designer une au
    /// hasard. Distinct de la position majoritaire publiee par la source, qui
    /// n'est jamais recalculee (RM-02) et que l'affichage montre a part quand
    /// les deux different.
    pub tied: Vec<VotePosition>,
}

impl VoterShare {
    /// `None` quand aucun membre du groupe ne s'est prononce: il n'y a pas de
    /// part a calculer, et afficher 0 % laisserait croire a un vote contre.
    pub fn from_tally(tally: &VoteTally) -> Option<Self> {
        let counts = [
            u32::from(tally.votes_for),
            u32::from(tally.votes_against),
            u32::from(tally.abstentions),
        ];
        let voters: u32 = counts.iter().sum();
        if voters == 0 {
            return None;
        }

        let mut percents = [0u32; 3];
        let mut remainders = [(0u32, 0usize); 3];
        for (index, count) in counts.iter().enumerate() {
            percents[index] = count * 100 / voters;
            remainders[index] = (count * 100 % voters, index);
        }

        // Plus forte moyenne: le reste des unites va aux plus grands restes.
        // A egalite, l'ordre pour / contre / abstention tranche, ce qui garde
        // le calcul rejouable a l'identique.
        let mut left = 100 - percents.iter().sum::<u32>();
        remainders.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        for (_, index) in remainders {
            if left == 0 {
                break;
            }
            percents[index] += 1;
            left -= 1;
        }

        Some(Self {
            voters: voters as u16,
            for_percent: percents[0] as u8,
            against_percent: percents[1] as u8,
            abstention_percent: percents[2] as u8,
            tied: tied_positions(&counts),
        })
    }

    /// Position majoritaire chez les votants. `None` a egalite: annoncer une
    /// majorite qui n'existe pas serait une invention.
    pub fn leading(&self) -> Option<VotePosition> {
        match self.tied.as_slice() {
            [only] => Some(*only),
            _ => None,
        }
    }

    /// Part de la position majoritaire chez les votants.
    pub fn leading_percent(&self) -> Option<u8> {
        match self.leading()? {
            VotePosition::For => Some(self.for_percent),
            VotePosition::Against => Some(self.against_percent),
            VotePosition::Abstention => Some(self.abstention_percent),
            VotePosition::NotVoting => None,
        }
    }
}

const RANKED_POSITIONS: [VotePosition; 3] = [
    VotePosition::For,
    VotePosition::Against,
    VotePosition::Abstention,
];

/// Positions arrivant en tete sur les comptes bruts.
fn tied_positions(counts: &[u32; 3]) -> Vec<VotePosition> {
    let Some(highest) = counts.iter().max().copied() else {
        return Vec::new();
    };
    counts
        .iter()
        .enumerate()
        .filter(|(_, count)| **count == highest)
        .map(|(index, _)| RANKED_POSITIONS[index])
        .collect()
}

/// Groupe parlementaire tel qu'il est nomme par l'Assemblee.
///
/// README.md §3.1: aucune traduction vers un parti. Le libelle affiche est
/// celui du groupe, et rien d'autre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GroupIdentity {
    pub uid: GroupUid,
    pub abbrev: String,
    pub label: String,
    pub color: Option<String>,
}

/// Ce qu'un groupe a fait sur un vote donne.
#[derive(Debug, Clone, Serialize)]
pub struct GroupStance {
    pub group: GroupIdentity,
    /// Effectif publie du groupe au moment du scrutin (RM-08).
    pub member_count: Option<u16>,
    /// Position majoritaire publiee par la source, jamais recalculee (RM-02).
    pub published_majority: Option<VotePosition>,
    pub tally: VoteTally,
    pub share: Option<VoterShare>,
}

impl GroupStance {
    pub fn new(
        group: GroupIdentity,
        member_count: Option<u16>,
        published_majority: Option<VotePosition>,
        tally: VoteTally,
    ) -> Self {
        let share = VoterShare::from_tally(&tally);
        Self {
            group,
            member_count,
            published_majority,
            tally,
            share,
        }
    }
}

/// Un vote sur l'ensemble d'un texte, avec la position des groupes demandes.
#[derive(Debug, Clone, Serialize)]
pub struct FinalVote {
    pub scrutin_uid: String,
    pub number: String,
    pub date: NaiveDate,
    pub ballot_type_label: String,
    pub outcome: Outcome,
    /// Cle du texte debattu: adresse de la fiche vers laquelle le titre pointe.
    pub text_key: String,
    pub text_label: String,
    pub reading: Option<String>,
    /// Dossier legislatif quand la source en rattache un. Absent sur 150 des
    /// 222 votes sur l'ensemble: le titre vient donc du texte, pas du dossier.
    pub dossier_uid: Option<String>,
    pub dossier_label: Option<String>,
    /// Version exacte du texte dont une source officielle établit le lien avec
    /// ce scrutin. `None` signifie que ce lien n'a pas encore été vérifié : le
    /// titre du scrutin ne permet jamais de le déduire.
    pub official_text: Option<OfficialTextVersion>,
    /// Decompte officiel de l'ensemble de l'Assemblee, publie tel quel.
    pub synthesis: VoteTally,
    /// Positions des groupes demandes, dans l'ordre de la demande. Un groupe
    /// absent de la liste n'avait pas de ligne dans ce scrutin.
    pub stances: Vec<GroupStance>,
}

/// Version officielle explicitement rattachée à un vote sur l'ensemble.
///
/// La référence qui établit le lien est distincte du document lui-même : une
/// page de séance, d'analyse de scrutin ou de dossier peut en être la preuve.
/// Cette séparation évite de confondre une version simplement présente dans un
/// dossier avec celle qui a effectivement été soumise au vote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OfficialTextVersion {
    pub document_uid: String,
    pub document_title: String,
    pub version_label: String,
    pub document_published_on: Option<NaiveDate>,
    pub official_url: String,
    pub mapping_source_url: String,
    pub source_producer: String,
    pub source_license: String,
    /// Empreinte de la source de rattachement, quand elle a pu être calculée.
    pub source_metadata_fingerprint: Option<String>,
    pub source_retrieved_at: DateTime<Utc>,
}

impl FinalVote {
    /// Le texte est-il passe. Lu sur l'issue publiee du scrutin, qui porte sur
    /// cette lecture-la et non sur le sort final de la loi.
    pub fn adopted(&self) -> bool {
        self.outcome.code() == Outcome::ADOPTED
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tally(votes_for: u16, votes_against: u16, abstentions: u16, not_voting: u16) -> VoteTally {
        VoteTally {
            votes_for,
            votes_against,
            abstentions,
            not_voting,
            voluntary_not_voting: 0,
        }
    }

    #[test]
    fn a_subject_opening_on_the_whole_text_is_a_final_vote() {
        assert!(is_final_vote_subject(
            "l'ensemble de la proposition de loi relative au droit à l'aide à mourir (première lecture)."
        ));
    }

    #[test]
    fn the_typographic_apostrophe_is_recognised_too() {
        assert!(is_final_vote_subject(
            "l\u{2019}ensemble de la proposition de loi visant à réduire l\u{2019}impact environnemental de l\u{2019}industrie textile."
        ));
    }

    #[test]
    fn an_amendment_is_not_a_final_vote() {
        assert!(!is_final_vote_subject(
            "l'amendement n° 12 de M. Tanguy à l'article 3 du projet de loi de finances pour 2026."
        ));
        assert!(!is_final_vote_subject(
            "la motion de censure déposée en application de l'article 49, alinéa 2, de la Constitution."
        ));
    }

    #[test]
    fn the_reading_distinguishes_two_votes_on_the_same_text() {
        let label = "proposition de loi relative au droit à l'aide à mourir";
        assert_eq!(
            reading_of(
                "l'ensemble de la proposition de loi relative au droit à l'aide à mourir (première lecture).",
                label
            ),
            Some("première lecture".to_string())
        );
    }

    #[test]
    fn a_mention_already_carried_by_the_label_is_not_repeated() {
        let label = "proposition de loi pour une montagne vivante et souveraine (texte de la commission mixte paritaire)";
        assert_eq!(
            reading_of(
                "l'ensemble de la proposition de loi pour une montagne vivante et souveraine (texte de la commission mixte paritaire).",
                label
            ),
            None
        );
    }

    #[test]
    fn a_subject_without_parenthesis_carries_no_reading() {
        assert_eq!(
            reading_of("l'ensemble du projet de loi X.", "projet de loi X"),
            None
        );
    }

    #[test]
    fn non_voters_are_out_of_the_denominator() {
        // 30 votants sur 40 membres: la part se lit sur ceux qui se prononcent.
        let share = VoterShare::from_tally(&tally(15, 9, 6, 10)).unwrap();
        assert_eq!(share.voters, 30);
        assert_eq!(share.for_percent, 50);
        assert_eq!(share.against_percent, 30);
        assert_eq!(share.abstention_percent, 20);
    }

    #[test]
    fn a_unanimous_group_shows_a_hundred_percent() {
        let share = VoterShare::from_tally(&tally(3, 0, 0, 1)).unwrap();
        assert_eq!(
            (
                share.for_percent,
                share.against_percent,
                share.abstention_percent
            ),
            (100, 0, 0)
        );
    }

    #[test]
    fn the_three_shares_always_total_a_hundred() {
        // 1/3 chacun: trois arrondis independants donneraient 99.
        let share = VoterShare::from_tally(&tally(1, 1, 1, 0)).unwrap();
        let total = share.for_percent as u16
            + share.against_percent as u16
            + share.abstention_percent as u16;
        assert_eq!(total, 100);
        assert_eq!(share.for_percent, 34);
    }

    #[test]
    fn a_group_without_a_single_voter_has_no_share() {
        assert!(VoterShare::from_tally(&tally(0, 0, 0, 12)).is_none());
    }

    #[test]
    fn the_leading_position_is_the_most_numerous_one() {
        // Cas reel du scrutin 8431: la source publie « contre » comme position
        // majoritaire du groupe alors que 14 votants sur 16 s'abstiennent.
        // Coller le pourcentage sur la position publiee afficherait
        // « contre 13 % », qui ne veut rien dire.
        let share = VoterShare::from_tally(&tally(0, 2, 14, 1)).unwrap();
        assert_eq!(share.leading(), Some(VotePosition::Abstention));
        assert_eq!(share.leading_percent(), Some(87));
    }

    #[test]
    fn a_tie_on_raw_counts_survives_rounding() {
        // Cas reel du scrutin 5729 pour LIOT: 8 pour, 6 contre, 8 abstentions.
        // Les parts arrondies donnent 37 / 27 / 36, l'egalite ne se lit que sur
        // les comptes bruts.
        let share = VoterShare::from_tally(&tally(8, 6, 8, 0)).unwrap();
        assert_ne!(share.for_percent, share.abstention_percent);
        assert_eq!(share.leading(), None);
        assert_eq!(
            share.tied,
            vec![VotePosition::For, VotePosition::Abstention]
        );
    }

    #[test]
    fn a_tie_yields_no_leading_position() {
        let share = VoterShare::from_tally(&tally(10, 10, 3, 0)).unwrap();
        assert_eq!(share.leading(), None);
        assert_eq!(share.leading_percent(), None);
        assert_eq!(share.tied, vec![VotePosition::For, VotePosition::Against]);
    }

    #[test]
    fn the_leading_position_is_read_on_raw_counts() {
        // Une voix d'ecart sur 99 votants: le comptage brut tranche, la part
        // arrondie ne sert qu'a l'affichage.
        let share = VoterShare::from_tally(&tally(50, 49, 0, 0)).unwrap();
        assert_eq!(share.leading(), Some(VotePosition::For));
    }
}
