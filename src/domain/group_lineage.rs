//! Groupes renommes en cours de legislature.
//!
//! Le referentiel de l'Assemblee ouvre un nouvel identifiant quand un groupe
//! change de nom, sans jamais relier l'ancien au nouveau. Les deux apparaissent
//! donc cote a cote dans le selecteur, chacun muet sur la periode de l'autre:
//! choisir l'ancien nom affiche « aucune ligne » sur tous les votes recents.
//!
//! Rapprocher les deux identifiants est une decision editoriale, pas une
//! deduction: elle est ecrite ici en toutes lettres, verifiable ligne a ligne,
//! plutot que devinee a l'execution sur une ressemblance de sigle ou de
//! couleur (README.md §8).
//!
//! La lignee vaut pour l'affichage — un groupe, une colonne, sur toute la
//! legislature. Elle ne modifie rien en base: les ventilations restent portees
//! par l'identifiant que la source leur donne.

/// Un meme groupe parlementaire sous ses identifiants successifs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupLineage {
    /// Identifiant retenu pour la lignee: celui de la periode en cours.
    pub canonical_uid: &'static str,
    /// Tous les identifiants de la lignee, le canonique compris.
    pub uids: &'static [&'static str],
    /// Sigle affiche: le plus recent, celui que la source emploie aujourd'hui.
    pub abbrev: &'static str,
    pub label: &'static str,
    /// Sigles anterieurs. Une adresse partagee avec l'ancien sigle continue de
    /// designer le groupe (README.md §8.1).
    pub former_abbrevs: &'static [&'static str],
}

impl GroupLineage {
    pub fn contains_uid(&self, uid: &str) -> bool {
        self.uids.contains(&uid)
    }

    /// Vrai quand le jeton designe ce groupe, par identifiant ou par sigle,
    /// ancien ou courant.
    pub fn matches(&self, token: &str) -> bool {
        self.contains_uid(token)
            || self.abbrev.eq_ignore_ascii_case(token)
            || self
                .former_abbrevs
                .iter()
                .any(|abbrev| abbrev.eq_ignore_ascii_case(token))
    }
}

/// Renommages constates dans le referentiel de la 17e legislature.
///
/// UDR (PO847173) porte 108 votes sur l'ensemble, du 27/01/2025 au 10/07/2025;
/// UDDPLR (PO872880) en porte 114, a partir du 15/10/2025. Les deux periodes
/// sont disjointes, la couleur publiee est la meme (#3367A7), et 108 + 114
/// fait les 222 votes sur l'ensemble de la legislature (mesure du 06/08/2026).
pub const GROUP_LINEAGES: &[GroupLineage] = &[GroupLineage {
    canonical_uid: "PO872880",
    uids: &["PO872880", "PO847173"],
    abbrev: "UDDPLR",
    label: "Union des droites pour la République (anciennement UDR)",
    former_abbrevs: &["UDR"],
}];

/// Lignee portant cet identifiant, s'il en existe une.
pub fn lineage_of_uid(uid: &str) -> Option<&'static GroupLineage> {
    GROUP_LINEAGES
        .iter()
        .find(|lineage| lineage.contains_uid(uid))
}

/// Identifiants sous lesquels la base porte le groupe de cet identifiant: tous
/// ceux de sa lignee, sinon lui seul.
pub fn uids_of(uid: &str) -> Vec<&str> {
    match lineage_of_uid(uid) {
        Some(lineage) => lineage.uids.to_vec(),
        None => vec![uid],
    }
}

/// Vrai quand le jeton designe le groupe de cet identifiant et de ce sigle: par
/// identifiant ou par sigle (casse ignoree) et, sous une lignee, par n'importe
/// lequel de ses identifiants et de ses sigles, anciens compris. Le sigle rend
/// l'adresse partageable, l'identifiant la rend stable (README.md §8.1).
pub fn designates(uid: &str, abbrev: &str, token: &str) -> bool {
    uid == token
        || abbrev.eq_ignore_ascii_case(token)
        || lineage_of_uid(uid).is_some_and(|lineage| lineage.matches(token))
}

/// Vrai quand les deux identifiants sont ceux d'un meme groupe: le meme, ou
/// deux identifiants successifs d'une meme lignee.
pub fn same_group(selected_uid: &str, other_uid: &str) -> bool {
    selected_uid == other_uid
        || lineage_of_uid(selected_uid).is_some_and(|lineage| lineage.contains_uid(other_uid))
}

/// Identite sous laquelle un groupe s'affiche.
///
/// Sous une lignee, c'est celle de la lignee et non celle que la ligne porte:
/// sinon la meme colonne changerait de sigle au milieu de la liste, au vote ou
/// le groupe a ete renomme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayedGroup<'a> {
    pub uid: &'a str,
    pub abbrev: &'a str,
    pub label: &'a str,
    /// Vide hors lignee.
    pub former_abbrevs: &'a [&'a str],
}

impl<'a> DisplayedGroup<'a> {
    fn under(
        lineage: Option<&'static GroupLineage>,
        uid: &'a str,
        abbrev: &'a str,
        label: &'a str,
    ) -> Self {
        match lineage {
            Some(lineage) => Self {
                uid: lineage.canonical_uid,
                abbrev: lineage.abbrev,
                label: lineage.label,
                former_abbrevs: lineage.former_abbrevs,
            },
            None => Self {
                uid,
                abbrev,
                label,
                former_abbrevs: &[],
            },
        }
    }
}

/// Identite affichee du groupe qu'une ligne decrit: celle de sa lignee quand
/// elle en a une, sinon celle de la ligne elle-meme.
pub fn displayed_group<'a>(uid: &'a str, abbrev: &'a str, label: &'a str) -> DisplayedGroup<'a> {
    DisplayedGroup::under(lineage_of_uid(uid), uid, abbrev, label)
}

/// Lignes d'un meme groupe, rassemblees sous un identifiant unique.
///
/// Un groupe renomme arrive de la source en plusieurs lignes, une par
/// identifiant. Le regroupement est la regle; ce que chaque ecran en fait —
/// additionner des comptes, choisir une couleur — reste a lui.
#[derive(Debug)]
pub struct LineageGroup<T> {
    key: String,
    lineage: Option<&'static GroupLineage>,
    members: Vec<T>,
    /// Rang du membre dont les attributs courants valent pour le groupe.
    representative: usize,
    /// Faux tant que le membre canonique n'a pas ete vu.
    canonical_seen: bool,
}

impl<T> LineageGroup<T> {
    /// Identifiant du groupe: le canonique de la lignee, sinon celui du membre.
    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn lineage(&self) -> Option<&'static GroupLineage> {
        self.lineage
    }

    /// Les membres, dans l'ordre ou ils ont ete rencontres.
    pub fn members(&self) -> &[T] {
        &self.members
    }

    /// Membre dont les attributs courants — effectif, dissolution, date de
    /// reference — valent pour le groupe entier: le canonique quand il est
    /// present, sinon le dernier rencontre. Hors lignee, le premier membre.
    ///
    /// Les attributs des autres membres sont ceux d'une periode close: les
    /// reporter sur le groupe ferait compter deux fois ce qui n'a pas change.
    pub fn representative(&self) -> &T {
        &self.members[self.representative]
    }

    /// Identite affichee du groupe: celle de la lignee, sinon le sigle et le
    /// libelle fournis par l'appelant — ceux de la ligne qui le represente.
    pub fn displayed<'a>(&'a self, abbrev: &'a str, label: &'a str) -> DisplayedGroup<'a> {
        DisplayedGroup::under(self.lineage, &self.key, abbrev, label)
    }

    fn seed(
        key: String,
        lineage: Option<&'static GroupLineage>,
        first: T,
        is_canonical: bool,
    ) -> Self {
        Self {
            key,
            lineage,
            members: vec![first],
            representative: 0,
            canonical_seen: lineage.is_none() || is_canonical,
        }
    }

    fn absorb(&mut self, member: T, is_canonical: bool) {
        self.members.push(member);
        if is_canonical || !self.canonical_seen {
            self.representative = self.members.len() - 1;
        }
        self.canonical_seen |= is_canonical;
    }
}

/// Rassemble les elements qui decrivent un meme groupe: ceux d'une meme lignee
/// d'un cote, chacun des autres seul.
///
/// Les groupes sortent dans l'ordre de premiere apparition de leur cle, quel que
/// soit l'ordre des identifiants d'une lignee en entree.
pub fn group_by_lineage<T>(items: Vec<T>, uid_of: impl Fn(&T) -> &str) -> Vec<LineageGroup<T>> {
    let mut groups: Vec<LineageGroup<T>> = Vec::with_capacity(items.len());

    for item in items {
        let uid = uid_of(&item);
        let lineage = lineage_of_uid(uid);
        let key = lineage.map_or(uid, |lineage| lineage.canonical_uid);
        let is_canonical = lineage.is_some_and(|lineage| lineage.canonical_uid == uid);

        match groups.iter_mut().find(|group| group.key == key) {
            Some(group) => group.absorb(item, is_canonical),
            None => {
                let group = LineageGroup::seed(key.to_string(), lineage, item, is_canonical);
                groups.push(group);
            }
        }
    }

    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_identifiers_of_a_renamed_group_lead_to_the_same_lineage() {
        let before = lineage_of_uid("PO847173").unwrap();
        let after = lineage_of_uid("PO872880").unwrap();

        assert_eq!(before, after);
        assert_eq!(before.canonical_uid, "PO872880");
    }

    #[test]
    fn a_group_outside_any_lineage_is_left_alone() {
        assert!(lineage_of_uid("PO800000").is_none());
    }

    #[test]
    fn the_former_abbrev_still_designates_the_group() {
        let lineage = lineage_of_uid("PO872880").unwrap();

        assert!(lineage.matches("UDR"));
        assert!(lineage.matches("udr"));
        assert!(lineage.matches("UDDPLR"));
        assert!(lineage.matches("PO847173"));
        assert!(!lineage.matches("RN"));
    }

    /// Le canonique doit appartenir a sa propre lignee, sinon la selection
    /// renverrait un identifiant qu'aucune ventilation ne porte.
    #[test]
    fn every_canonical_uid_belongs_to_its_lineage() {
        for lineage in GROUP_LINEAGES {
            assert!(
                lineage.contains_uid(lineage.canonical_uid),
                "{} absent de sa lignee",
                lineage.canonical_uid
            );
        }
    }

    const FORMER: &str = "PO847173";
    const CURRENT: &str = "PO872880";

    /// Une ligne de source: un identifiant et de quoi la reconnaitre.
    #[derive(Debug, PartialEq, Eq)]
    struct Row {
        uid: &'static str,
        tag: &'static str,
    }

    fn row(uid: &'static str, tag: &'static str) -> Row {
        Row { uid, tag }
    }

    fn grouped(rows: Vec<Row>) -> Vec<LineageGroup<Row>> {
        group_by_lineage(rows, |row| row.uid)
    }

    fn tags(group: &LineageGroup<Row>) -> Vec<&'static str> {
        group.members().iter().map(|row| row.tag).collect()
    }

    #[test]
    fn two_identifiers_of_a_lineage_form_one_group_whatever_their_order() {
        for rows in [
            vec![row(FORMER, "former"), row(CURRENT, "current")],
            vec![row(CURRENT, "current"), row(FORMER, "former")],
        ] {
            let groups = grouped(rows);

            assert_eq!(groups.len(), 1);
            assert_eq!(groups[0].key(), CURRENT);
            assert_eq!(groups[0].members().len(), 2);
            assert_eq!(groups[0].lineage(), lineage_of_uid(CURRENT));
        }
    }

    #[test]
    fn a_group_outside_any_lineage_stands_alone_under_its_own_identifier() {
        let groups = grouped(vec![row("PO800000", "alone")]);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].key(), "PO800000");
        assert!(groups[0].lineage().is_none());
        assert_eq!(tags(&groups[0]), vec!["alone"]);
        assert_eq!(groups[0].representative().tag, "alone");
    }

    #[test]
    fn the_canonical_member_represents_the_group_when_it_is_present() {
        let canonical_last = grouped(vec![row(FORMER, "former"), row(CURRENT, "current")]);
        let canonical_first = grouped(vec![row(CURRENT, "current"), row(FORMER, "former")]);

        assert_eq!(canonical_last[0].representative().tag, "current");
        assert_eq!(canonical_first[0].representative().tag, "current");
    }

    #[test]
    fn the_last_member_represents_the_group_when_the_canonical_is_absent() {
        let groups = grouped(vec![row(FORMER, "first"), row(FORMER, "second")]);

        assert_eq!(groups[0].representative().tag, "second");
    }

    #[test]
    fn a_lone_former_member_represents_its_group() {
        let groups = grouped(vec![row(FORMER, "former")]);

        assert_eq!(groups[0].key(), CURRENT);
        assert_eq!(groups[0].representative().tag, "former");
    }

    #[test]
    fn groups_come_out_in_the_order_their_key_first_appeared() {
        let groups = grouped(vec![
            row("PO800000", "a"),
            row(FORMER, "former"),
            row("PO900000", "b"),
            row(CURRENT, "current"),
        ]);

        let keys: Vec<&str> = groups.iter().map(|group| group.key()).collect();
        assert_eq!(keys, vec!["PO800000", CURRENT, "PO900000"]);
        assert_eq!(tags(&groups[1]), vec!["former", "current"]);
    }

    #[test]
    fn a_lineage_group_is_shown_under_the_identity_of_its_lineage() {
        let groups = grouped(vec![row(FORMER, "former")]);

        let shown = groups[0].displayed("UDR", "Groupe UDR");

        assert_eq!(shown.uid, CURRENT);
        assert_eq!(shown.abbrev, "UDDPLR");
        assert_eq!(shown.label, lineage_of_uid(CURRENT).unwrap().label);
        assert_eq!(shown.former_abbrevs, ["UDR"]);
    }

    #[test]
    fn a_group_outside_any_lineage_is_shown_under_its_own_identity() {
        let groups = grouped(vec![row("PO800000", "alone")]);

        let shown = groups[0].displayed("RN", "Rassemblement national");

        assert_eq!(shown.uid, "PO800000");
        assert_eq!(shown.abbrev, "RN");
        assert_eq!(shown.label, "Rassemblement national");
        assert!(shown.former_abbrevs.is_empty());
    }

    #[test]
    fn a_row_is_shown_under_the_identity_of_its_lineage_whichever_identifier_it_carries() {
        let from_former = displayed_group(FORMER, "UDR", "Groupe UDR");
        let from_current = displayed_group(CURRENT, "UDDPLR", "Groupe UDDPLR");

        assert_eq!(from_former, from_current);
        assert_eq!(from_former.uid, CURRENT);
        assert_eq!(from_former.abbrev, "UDDPLR");
    }

    #[test]
    fn a_row_outside_any_lineage_is_shown_as_it_is() {
        let shown = displayed_group("PO845401", "RN", "Rassemblement national");

        assert_eq!(shown.uid, "PO845401");
        assert_eq!(shown.abbrev, "RN");
        assert_eq!(shown.label, "Rassemblement national");
        assert!(shown.former_abbrevs.is_empty());
    }

    #[test]
    fn a_group_is_designated_by_its_identifier() {
        assert!(designates("PO845401", "RN", "PO845401"));
        assert!(!designates("PO845401", "RN", "PO845402"));
    }

    #[test]
    fn a_group_is_designated_by_its_abbrev_whatever_the_case() {
        assert!(designates("PO845401", "RN", "RN"));
        assert!(designates("PO845401", "RN", "rn"));
        assert!(!designates("PO845401", "RN", "SOC"));
    }

    #[test]
    fn a_renamed_group_is_designated_by_every_identifier_and_abbrev_of_its_lineage() {
        for token in [CURRENT, FORMER, "UDDPLR", "uddplr", "UDR", "udr"] {
            assert!(designates(CURRENT, "UDDPLR", token), "jeton {token}");
        }
        assert!(!designates(CURRENT, "UDDPLR", "RN"));
    }

    #[test]
    fn a_former_abbrev_designates_nothing_outside_its_lineage() {
        assert!(!designates("PO845401", "RN", "UDR"));
    }

    #[test]
    fn two_identifiers_of_a_lineage_are_the_same_group() {
        assert!(same_group(CURRENT, FORMER));
        assert!(same_group(FORMER, CURRENT));
    }

    #[test]
    fn an_identifier_is_the_same_group_as_itself() {
        assert!(same_group("PO845401", "PO845401"));
        assert!(same_group(CURRENT, CURRENT));
    }

    #[test]
    fn distinct_groups_are_not_the_same_group() {
        assert!(!same_group("PO845401", "PO845402"));
        assert!(!same_group("PO845401", CURRENT));
        assert!(!same_group(CURRENT, "PO845401"));
    }

    #[test]
    fn a_renamed_group_answers_to_all_the_identifiers_of_its_lineage() {
        assert_eq!(uids_of(FORMER), vec![CURRENT, FORMER]);
        assert_eq!(uids_of(CURRENT), vec![CURRENT, FORMER]);
    }

    #[test]
    fn a_group_outside_any_lineage_answers_to_its_own_identifier_only() {
        assert_eq!(uids_of("PO845401"), vec!["PO845401"]);
    }
}
