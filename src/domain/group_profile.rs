//! Fiche d'un groupe parlementaire: identite, effectif, participation.
//!
//! Ce module produit des taux, ce que le reste du site evite soigneusement. La
//! precaution n'etait pas une pudeur: README.md §6 interdit « l'agregat qui se
//! lit comme un classement », et SPEC-PAGES-THEME-GROUPE RM-01 refuse tout taux
//! sur une page filtree par groupe. Trois garde-fous rendent la fiche tenable,
//! et ils sont dans le code plutot que dans une note de bas de page:
//!
//! 1. **Aucun cumul du sens du vote.** Les « pour » et les « contre » sont
//!    additionnes ensemble sous un seul libelle — les voix exprimees — et jamais
//!    separement. Un groupe qui vote contre quarante amendements puis pour le
//!    texte ne ressort d'ici ni « pour » ni « contre ».
//! 2. **Un denominateur nomme.** Le taux porte les positions publiees qui l'ont
//!    produit; sans elles, 12 % sur 25 lignes et 12 % sur 8 000 se lisent pareil.
//! 3. **Aucune comparabilite implicite.** Les groupes ne siegent pas sur les
//!    memes scrutins ni sur les memes periodes (H3), et leur effectif varie en
//!    cours de legislature (H4). La fiche porte donc sa fenetre et son effectif
//!    a cote de ses taux, et le site ne classe jamais deux groupes l'un contre
//!    l'autre.

use chrono::NaiveDate;
use serde::Serialize;

/// Positions publiees par l'Assemblee pour un groupe, cumulees sur les scrutins
/// ou elle publie une ligne pour lui.
///
/// Les cinq comptes sont ceux de la source, additionnes tels quels (RM-08).
/// `votes_for` et `votes_against` ne sortent jamais d'ici separement: ils
/// n'existent que dans `expressed()`.
///
/// `voluntary_not_voting` reste conserve mais n'entre dans aucun taux: dans le
/// fichier de l'Assemblee, ce compte de groupe recopie les abstentions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ParticipationCounts {
    pub votes_for: u64,
    pub votes_against: u64,
    pub abstentions: u64,
    pub not_voting: u64,
    pub voluntary_not_voting: u64,
}

impl ParticipationCounts {
    /// Voix exprimees: pour et contre confondus. Le sens du vote ne sort pas de
    /// cette addition, et c'est deliberé.
    pub fn expressed(&self) -> u64 {
        self.votes_for + self.votes_against
    }

    /// Denominateur de tous les taux de la fiche: chaque position que la source
    /// publie pour le groupe. Ce n'est pas son effectif — la source ne fait pas
    /// toujours coincider les deux, et supposer l'egalite serait produire un
    /// chiffre qu'elle ne publie pas.
    pub fn published_positions(&self) -> u64 {
        self.expressed() + self.abstentions + self.not_voting
    }

    pub fn add(&mut self, other: &Self) {
        self.votes_for += other.votes_for;
        self.votes_against += other.votes_against;
        self.abstentions += other.abstentions;
        self.not_voting += other.not_voting;
        self.voluntary_not_voting += other.voluntary_not_voting;
    }
}

/// Repartition de la participation, en pour mille.
///
/// Le pour mille plutot que le pourcentage entier: sur 100 000 positions, une
/// abstention a 0,4 % s'afficherait « 0 % », et un chiffre publie arrondi a zero
/// se lit comme une absence de donnee.
///
/// Les trois parts sont arrondies a la plus forte moyenne pour totaliser 1 000:
/// trois arrondis independants laisseraient un reste visible a l'ecran.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ParticipationRates {
    /// Positions publiees ayant servi de denominateur. Affichee avec les taux:
    /// un taux sans sa base n'est pas verifiable (README.md §9).
    pub base: u64,
    /// Part des voix exprimees, pour et contre confondus.
    pub expressed_per_mille: u16,
    pub abstention_per_mille: u16,
    /// Part des non-votants.
    pub absence_per_mille: u16,
}

impl ParticipationRates {
    /// `None` quand la source ne publie aucune position pour ce groupe: il n'y
    /// a rien a rapporter, et afficher 0 % laisserait croire a une abstention
    /// nulle mesuree.
    pub fn from_counts(counts: &ParticipationCounts) -> Option<Self> {
        let base = counts.published_positions();
        if base == 0 {
            return None;
        }

        let parts = [counts.expressed(), counts.abstentions, counts.not_voting];
        let mut shares = [0u64; 3];
        let mut remainders = [(0u64, 0usize); 3];
        for (index, part) in parts.iter().enumerate() {
            shares[index] = part * 1_000 / base;
            remainders[index] = (part * 1_000 % base, index);
        }

        // Plus forte moyenne: les unites restantes vont aux plus grands restes.
        // A egalite, l'ordre exprime / abstention / non-participation tranche,
        // ce qui garde le calcul rejouable a l'identique.
        let mut left = 1_000 - shares.iter().sum::<u64>();
        remainders.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        for (_, index) in remainders {
            if left == 0 {
                break;
            }
            shares[index] += 1;
            left -= 1;
        }

        Some(Self {
            base,
            expressed_per_mille: shares[0] as u16,
            abstention_per_mille: shares[1] as u16,
            absence_per_mille: shares[2] as u16,
        })
    }
}

/// Effectif publie par la source sur les scrutins du groupe.
///
/// H4: l'effectif bouge en cours de legislature — RN de 121 a 125, EPR de 91 a
/// 95. Publier une seule valeur obligerait a choisir laquelle, et ce choix
/// serait un chiffre invente. Les deux bornes sont donc gardees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct MemberCountRange {
    min: u16,
    max: u16,
}

impl MemberCountRange {
    /// `None` quand la source ne publie pas d'effectif, ou quand les bornes
    /// sont incoherentes: mieux vaut ne rien afficher qu'un intervalle inverse.
    pub fn new(min: u16, max: u16) -> Option<Self> {
        (min <= max).then_some(Self { min, max })
    }

    pub fn min(&self) -> u16 {
        self.min
    }

    pub fn max(&self) -> u16 {
        self.max
    }

    pub fn is_stable(&self) -> bool {
        self.min == self.max
    }
}

/// Fenetre pendant laquelle la source publie des lignes de vote pour le groupe.
///
/// Distincte des dates de creation et de dissolution du referentiel: un groupe
/// peut exister sans qu'aucun scrutin ne le nomme. H3 en fait un fait
/// structurant — UDR et UDDPLR partitionnent exactement la legislature — et
/// c'est cette fenetre, pas les dates du referentiel, qui explique pourquoi
/// deux taux ne se comparent pas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct VotingWindow {
    first: NaiveDate,
    last: NaiveDate,
}

impl VotingWindow {
    pub fn new(first: Option<NaiveDate>, last: Option<NaiveDate>) -> Option<Self> {
        match (first, last) {
            (Some(first), Some(last)) if first <= last => Some(Self { first, last }),
            _ => None,
        }
    }

    pub fn first(&self) -> NaiveDate {
        self.first
    }

    pub fn last(&self) -> NaiveDate {
        self.last
    }

    /// Plus petite fenetre qui couvre les deux: un groupe renomme est vu sur
    /// l'ensemble de ses periodes, de la premiere a la derniere.
    pub fn span(self, other: Self) -> Self {
        Self {
            first: self.first.min(other.first),
            last: self.last.max(other.last),
        }
    }
}

/// Nombre de deputes portant une qualite d'appartenance donnee.
///
/// La qualite est le libelle de la source — « Membre », « Membre apparente »,
/// « President » — conserve tel quel (SPEC-acteurs RM-02).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QualityCount {
    pub quality: String,
    pub members: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counts(
        votes_for: u64,
        votes_against: u64,
        abstentions: u64,
        not_voting: u64,
        voluntary_not_voting: u64,
    ) -> ParticipationCounts {
        ParticipationCounts {
            votes_for,
            votes_against,
            abstentions,
            not_voting,
            voluntary_not_voting,
        }
    }

    #[test]
    fn the_denominator_holds_every_published_position() {
        let counts = counts(10, 5, 3, 2, 0);
        assert_eq!(counts.expressed(), 15);
        assert_eq!(counts.published_positions(), 20);
    }

    #[test]
    fn the_voluntary_count_repeats_the_abstentions_and_adds_no_position() {
        // V8280, EPR: 64 pour, 18 contre, 9 abstentions pour 91 membres, et
        // 9 « non-votants volontaires » qui sont ces memes 9 abstentionnistes.
        let counts = counts(64, 18, 9, 0, 9);
        assert_eq!(counts.published_positions(), 91);

        let rates = ParticipationRates::from_counts(&counts).unwrap();
        assert_eq!(rates.base, 91);
        assert_eq!(rates.absence_per_mille, 0);
    }

    #[test]
    fn the_three_shares_always_total_a_thousand() {
        // Un tiers chacun: trois arrondis independants donneraient 999.
        let rates = ParticipationRates::from_counts(&counts(1, 0, 1, 1, 0)).unwrap();
        let total =
            rates.expressed_per_mille + rates.abstention_per_mille + rates.absence_per_mille;
        assert_eq!(total, 1_000);
    }

    #[test]
    fn a_group_without_a_single_published_position_has_no_rate() {
        assert!(ParticipationRates::from_counts(&ParticipationCounts::default()).is_none());
    }

    #[test]
    fn a_rare_abstention_survives_the_rounding() {
        // 4 abstentions sur 1 000 positions: en pourcentage entier, le chiffre
        // s'afficherait « 0 % » et se lirait comme une absence de mesure.
        let rates = ParticipationRates::from_counts(&counts(996, 0, 4, 0, 0)).unwrap();
        assert_eq!(rates.abstention_per_mille, 4);
        assert_eq!(rates.base, 1_000);
    }

    #[test]
    fn the_direction_of_the_vote_never_leaves_the_expressed_share() {
        // Deux groupes opposes sur tout, memes taux: la fiche ne dit rien du
        // sens du vote, et c'est ce que README.md §6 exige d'elle.
        let one = ParticipationRates::from_counts(&counts(80, 10, 6, 4, 0)).unwrap();
        let other = ParticipationRates::from_counts(&counts(10, 80, 6, 4, 0)).unwrap();
        assert_eq!(one, other);
    }

    #[test]
    fn counts_add_up_across_the_identifiers_of_a_renamed_group() {
        let mut total = counts(10, 5, 3, 2, 1);
        total.add(&counts(1, 2, 3, 4, 5));
        assert_eq!(total, counts(11, 7, 6, 6, 6));
    }

    #[test]
    fn a_stable_headcount_is_told_apart_from_a_moving_one() {
        assert!(MemberCountRange::new(66, 66).unwrap().is_stable());
        assert!(!MemberCountRange::new(121, 125).unwrap().is_stable());
    }

    #[test]
    fn a_reversed_headcount_range_is_refused() {
        assert!(MemberCountRange::new(125, 121).is_none());
    }

    #[test]
    fn a_headcount_range_exposes_its_two_bounds() {
        let range = MemberCountRange::new(121, 125).unwrap();
        assert_eq!((range.min(), range.max()), (121, 125));
    }

    #[test]
    fn a_group_that_never_appears_in_a_scrutin_has_no_window() {
        let date = NaiveDate::from_ymd_opt(2025, 3, 27).unwrap();
        assert!(VotingWindow::new(None, None).is_none());
        assert!(VotingWindow::new(Some(date), None).is_none());
        assert_eq!(
            VotingWindow::new(Some(date), Some(date)),
            Some(VotingWindow {
                first: date,
                last: date
            })
        );
    }

    #[test]
    fn a_window_missing_one_of_its_dates_is_refused() {
        let date = NaiveDate::from_ymd_opt(2025, 3, 27).unwrap();
        assert!(VotingWindow::new(None, Some(date)).is_none());
    }

    #[test]
    fn a_window_whose_first_date_follows_its_last_is_refused() {
        let earlier = NaiveDate::from_ymd_opt(2025, 3, 27).unwrap();
        let later = NaiveDate::from_ymd_opt(2025, 4, 2).unwrap();
        assert!(VotingWindow::new(Some(later), Some(earlier)).is_none());
    }

    #[test]
    fn a_window_exposes_its_first_and_last_dates() {
        let first = NaiveDate::from_ymd_opt(2025, 3, 27).unwrap();
        let last = NaiveDate::from_ymd_opt(2025, 4, 2).unwrap();
        let window = VotingWindow::new(Some(first), Some(last)).unwrap();
        assert_eq!((window.first(), window.last()), (first, last));
    }

    fn window(first: (i32, u32, u32), last: (i32, u32, u32)) -> VotingWindow {
        let date = |(y, m, d): (i32, u32, u32)| NaiveDate::from_ymd_opt(y, m, d).unwrap();
        VotingWindow {
            first: date(first),
            last: date(last),
        }
    }

    #[test]
    fn the_span_of_two_disjoint_windows_runs_from_the_first_start_to_the_last_end() {
        let former = window((2024, 10, 8), (2025, 7, 10));
        let current = window((2025, 10, 15), (2026, 7, 21));

        let expected = window((2024, 10, 8), (2026, 7, 21));
        assert_eq!(former.span(current), expected);
        assert_eq!(current.span(former), expected);
    }

    #[test]
    fn the_span_of_nested_windows_is_the_outer_one() {
        let outer = window((2024, 10, 8), (2026, 7, 21));
        let inner = window((2025, 1, 27), (2025, 7, 10));

        assert_eq!(outer.span(inner), outer);
        assert_eq!(inner.span(outer), outer);
    }

    #[test]
    fn the_span_of_overlapping_windows_keeps_the_earliest_start_and_the_latest_end() {
        let early = window((2024, 10, 8), (2025, 7, 10));
        let late = window((2025, 1, 27), (2026, 7, 21));

        assert_eq!(early.span(late), window((2024, 10, 8), (2026, 7, 21)));
    }

    #[test]
    fn a_window_spanned_with_itself_is_unchanged() {
        let only = window((2025, 1, 27), (2025, 7, 10));

        assert_eq!(only.span(only), only);
    }
}
