use std::collections::HashMap;

use chrono::NaiveDate;
use serde::Serialize;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ActorError {
    #[error("actor uid must not be empty")]
    EmptyActorUid,
    #[error("group uid must not be empty")]
    EmptyGroupUid,
    #[error("actor must have at least a first or last name")]
    EmptyActorName,
    #[error("group label must not be empty")]
    EmptyGroupLabel,
    #[error("group abbreviation must not be empty")]
    EmptyGroupAbbrev,
    #[error("membership quality must not be empty")]
    EmptyMembershipQuality,
    #[error("membership ends on {end} but starts on {start}")]
    ReversedMembershipPeriod { start: NaiveDate, end: NaiveDate },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct ActorUid(String);

impl ActorUid {
    pub fn new(raw: String) -> Result<Self, ActorError> {
        if raw.trim().is_empty() {
            return Err(ActorError::EmptyActorUid);
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ActorUid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct GroupUid(String);

impl GroupUid {
    pub fn new(raw: String) -> Result<Self, ActorError> {
        if raw.trim().is_empty() {
            return Err(ActorError::EmptyGroupUid);
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for GroupUid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Qualite d'un acteur, deduite des mandats qu'il detient.
///
/// Sert a l'affichage (CU-03) et decide si une page officielle de depute existe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ActorRole {
    Deputy,
    Minister,
    Senator,
    Other,
}

impl ActorRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Deputy => "deputy",
            Self::Minister => "minister",
            Self::Senator => "senator",
            Self::Other => "other",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "deputy" => Some(Self::Deputy),
            "minister" => Some(Self::Minister),
            "senator" => Some(Self::Senator),
            "other" => Some(Self::Other),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Deputy => "D\u{00e9}put\u{00e9}",
            Self::Minister => "Ministre",
            Self::Senator => "S\u{00e9}nateur",
            Self::Other => "Autre",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Actor {
    uid: ActorUid,
    civility: Option<String>,
    first_name: String,
    last_name: String,
    role: ActorRole,
}

impl Actor {
    pub fn new(
        uid: ActorUid,
        civility: Option<String>,
        first_name: String,
        last_name: String,
        role: ActorRole,
    ) -> Result<Self, ActorError> {
        if first_name.trim().is_empty() && last_name.trim().is_empty() {
            return Err(ActorError::EmptyActorName);
        }
        Ok(Self {
            uid,
            civility,
            first_name,
            last_name,
            role,
        })
    }

    pub fn uid(&self) -> &ActorUid {
        &self.uid
    }

    pub fn civility(&self) -> Option<&str> {
        self.civility.as_deref()
    }

    pub fn first_name(&self) -> &str {
        &self.first_name
    }

    pub fn last_name(&self) -> &str {
        &self.last_name
    }

    pub fn role(&self) -> ActorRole {
        self.role
    }

    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
            .trim()
            .to_string()
    }

    /// Page officielle de l'acteur sur le site de l'Assemblee.
    ///
    /// Emise uniquement pour les deputes: la page n'existe pas pour les autres
    /// acteurs et un lien mort serait une information fausse (RM-04).
    pub fn official_url(&self) -> Option<String> {
        match self.role {
            ActorRole::Deputy => Some(format!(
                "https://www.assemblee-nationale.fr/dyn/deputes/{}",
                self.uid
            )),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ParliamentaryGroup {
    uid: GroupUid,
    legislature: u16,
    label: String,
    abbrev: String,
    color: Option<String>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
}

impl ParliamentaryGroup {
    pub fn new(
        uid: GroupUid,
        legislature: u16,
        label: String,
        abbrev: String,
        color: Option<String>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<Self, ActorError> {
        if label.trim().is_empty() {
            return Err(ActorError::EmptyGroupLabel);
        }
        if abbrev.trim().is_empty() {
            return Err(ActorError::EmptyGroupAbbrev);
        }
        Ok(Self {
            uid,
            legislature,
            label,
            abbrev,
            color,
            start_date,
            end_date,
        })
    }

    pub fn uid(&self) -> &GroupUid {
        &self.uid
    }

    pub fn legislature(&self) -> u16 {
        self.legislature
    }

    /// Libelle officiel du groupe, jamais traduit en parti politique (RM-06).
    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn abbrev(&self) -> &str {
        &self.abbrev
    }

    pub fn color(&self) -> Option<&str> {
        self.color.as_deref()
    }

    pub fn start_date(&self) -> Option<NaiveDate> {
        self.start_date
    }

    pub fn end_date(&self) -> Option<NaiveDate> {
        self.end_date
    }

    pub fn is_dissolved(&self) -> bool {
        self.end_date.is_some()
    }
}

/// Qualite de l'appartenance, conservee telle qu'elle est publiee.
///
/// RM-02: aucune qualite n'est ecartee du rattachement. Le newtype garde le
/// libelle de la source plutot qu'une enumeration fermee, pour qu'une qualite
/// inconnue soit conservee au lieu d'etre silencieusement perdue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MembershipQuality(String);

impl MembershipQuality {
    pub fn new(raw: String) -> Result<Self, ActorError> {
        if raw.trim().is_empty() {
            return Err(ActorError::EmptyMembershipQuality);
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for MembershipQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Periode de validite d'une appartenance. Fin absente = appartenance active.
/// La date de fin est inclusive: elle est le dernier jour d'appartenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct MembershipPeriod {
    start: NaiveDate,
    end: Option<NaiveDate>,
}

impl MembershipPeriod {
    pub fn new(start: NaiveDate, end: Option<NaiveDate>) -> Result<Self, ActorError> {
        if let Some(end) = end {
            if end < start {
                return Err(ActorError::ReversedMembershipPeriod { start, end });
            }
        }
        Ok(Self { start, end })
    }

    pub fn start(&self) -> NaiveDate {
        self.start
    }

    pub fn end(&self) -> Option<NaiveDate> {
        self.end
    }

    pub fn contains(&self, date: NaiveDate) -> bool {
        date >= self.start && self.end.map_or(true, |end| date <= end)
    }

    pub fn is_open(&self) -> bool {
        self.end.is_none()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupMembership {
    source_uid: String,
    actor_uid: ActorUid,
    group_uid: GroupUid,
    legislature: u16,
    period: MembershipPeriod,
    quality: MembershipQuality,
}

impl GroupMembership {
    pub fn new(
        source_uid: String,
        actor_uid: ActorUid,
        group_uid: GroupUid,
        legislature: u16,
        period: MembershipPeriod,
        quality: MembershipQuality,
    ) -> Self {
        Self {
            source_uid,
            actor_uid,
            group_uid,
            legislature,
            period,
            quality,
        }
    }

    pub fn source_uid(&self) -> &str {
        &self.source_uid
    }

    pub fn actor_uid(&self) -> &ActorUid {
        &self.actor_uid
    }

    pub fn group_uid(&self) -> &GroupUid {
        &self.group_uid
    }

    pub fn legislature(&self) -> u16 {
        self.legislature
    }

    pub fn period(&self) -> &MembershipPeriod {
        &self.period
    }

    pub fn quality(&self) -> &MembershipQuality {
        &self.quality
    }

    pub fn is_active_on(&self, date: NaiveDate) -> bool {
        self.period.contains(date)
    }
}

/// Resultat de la recherche de l'appartenance valide a une date donnee.
#[derive(Debug, Clone, Copy)]
pub enum MembershipAtDate<'a> {
    /// Une seule appartenance de groupe est valide a cette date.
    Found(&'a GroupMembership),
    /// Aucune appartenance ouverte a cette date.
    None,
    /// Plusieurs groupes distincts revendiquent l'acteur a cette date.
    /// Donnee incoherente: rien n'est affiche, l'anomalie est signalee (RM-04).
    Ambiguous,
}

/// Appartenance valide a la date de l'acte (RM-01), toutes qualites confondues (RM-02).
///
/// Quand plusieurs appartenances au *meme* groupe se recouvrent — le cas des
/// presidents de groupe, membres et presidents en meme temps — la plus
/// recemment ouverte est retenue: le groupe est identique, seule la qualite
/// affichee change.
pub fn membership_at<'a>(
    memberships: &'a [GroupMembership],
    date: NaiveDate,
) -> MembershipAtDate<'a> {
    let mut active: Vec<&GroupMembership> = memberships
        .iter()
        .filter(|m| m.is_active_on(date))
        .collect();

    if active.is_empty() {
        return MembershipAtDate::None;
    }

    let first_group = active[0].group_uid();
    if active.iter().any(|m| m.group_uid() != first_group) {
        return MembershipAtDate::Ambiguous;
    }

    active.sort_by(|a, b| {
        b.period()
            .start()
            .cmp(&a.period().start())
            .then_with(|| a.source_uid().cmp(b.source_uid()))
    });
    MembershipAtDate::Found(active[0])
}

/// Instantane complet du referentiel, tel que produit par la source officielle.
#[derive(Debug, Clone)]
pub struct ActorRegistry {
    pub actors: Vec<Actor>,
    pub groups: Vec<ParliamentaryGroup>,
    pub memberships: Vec<GroupMembership>,
}

/// Vue de lecture du referentiel, indexee pour resoudre les rattachements.
pub struct ActorDirectory {
    actors: HashMap<ActorUid, Actor>,
    groups: HashMap<GroupUid, ParliamentaryGroup>,
    memberships: HashMap<ActorUid, Vec<GroupMembership>>,
}

/// Rattachement d'un acteur a la date d'un acte.
pub struct ActorAtDate<'a> {
    pub actor: &'a Actor,
    pub group: Option<&'a ParliamentaryGroup>,
    pub quality: Option<&'a MembershipQuality>,
    /// Vrai quand la source donne plusieurs groupes concurrents a cette date.
    pub ambiguous: bool,
}

impl ActorDirectory {
    pub fn new(
        actors: Vec<Actor>,
        groups: Vec<ParliamentaryGroup>,
        memberships: Vec<GroupMembership>,
    ) -> Self {
        let mut by_actor: HashMap<ActorUid, Vec<GroupMembership>> = HashMap::new();
        for membership in memberships {
            by_actor
                .entry(membership.actor_uid().clone())
                .or_default()
                .push(membership);
        }

        Self {
            actors: actors.into_iter().map(|a| (a.uid().clone(), a)).collect(),
            groups: groups.into_iter().map(|g| (g.uid().clone(), g)).collect(),
            memberships: by_actor,
        }
    }

    pub fn actor(&self, uid: &ActorUid) -> Option<&Actor> {
        self.actors.get(uid)
    }

    pub fn group(&self, uid: &GroupUid) -> Option<&ParliamentaryGroup> {
        self.groups.get(uid)
    }

    pub fn is_empty(&self) -> bool {
        self.actors.is_empty()
    }

    /// Acteur et groupe qu'il avait a la date de l'acte.
    ///
    /// Retourne None quand l'acteur est absent du referentiel: son nom brut est
    /// conserve par l'appelant, aucun groupe n'est devine (RM-04).
    pub fn resolve_at(&self, uid: &ActorUid, date: NaiveDate) -> Option<ActorAtDate<'_>> {
        let actor = self.actors.get(uid)?;
        let memberships = self.memberships.get(uid).map(Vec::as_slice).unwrap_or(&[]);

        let (group, quality, ambiguous) = match membership_at(memberships, date) {
            MembershipAtDate::Found(m) => {
                (self.groups.get(m.group_uid()), Some(m.quality()), false)
            }
            MembershipAtDate::None => (None, None, false),
            MembershipAtDate::Ambiguous => (None, None, true),
        };

        Some(ActorAtDate {
            actor,
            group,
            quality,
            ambiguous,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn membership(
        source: &str,
        actor: &str,
        group: &str,
        start: NaiveDate,
        end: Option<NaiveDate>,
        quality: &str,
    ) -> GroupMembership {
        GroupMembership::new(
            source.into(),
            ActorUid::new(actor.into()).unwrap(),
            GroupUid::new(group.into()).unwrap(),
            17,
            MembershipPeriod::new(start, end).unwrap(),
            MembershipQuality::new(quality.into()).unwrap(),
        )
    }

    fn deputy(uid: &str, first: &str, last: &str) -> Actor {
        Actor::new(
            ActorUid::new(uid.into()).unwrap(),
            Some("Mme".into()),
            first.into(),
            last.into(),
            ActorRole::Deputy,
        )
        .unwrap()
    }

    fn group(uid: &str, abbrev: &str, label: &str) -> ParliamentaryGroup {
        ParliamentaryGroup::new(
            GroupUid::new(uid.into()).unwrap(),
            17,
            label.into(),
            abbrev.into(),
            None,
            Some(date(2024, 7, 18)),
            None,
        )
        .unwrap()
    }

    #[test]
    fn actor_role_round_trips_through_its_string() {
        for role in [
            ActorRole::Deputy,
            ActorRole::Minister,
            ActorRole::Senator,
            ActorRole::Other,
        ] {
            assert_eq!(ActorRole::parse(role.as_str()), Some(role));
        }
    }

    #[test]
    fn actor_uid_rejects_empty() {
        assert_eq!(
            ActorUid::new("".into()).unwrap_err(),
            ActorError::EmptyActorUid
        );
    }

    #[test]
    fn actor_uid_rejects_blank() {
        assert_eq!(
            ActorUid::new(" \t".into()).unwrap_err(),
            ActorError::EmptyActorUid
        );
    }

    #[test]
    fn group_uid_rejects_empty() {
        assert_eq!(
            GroupUid::new("".into()).unwrap_err(),
            ActorError::EmptyGroupUid
        );
    }

    #[test]
    fn group_uid_rejects_blank() {
        assert_eq!(
            GroupUid::new(" \t".into()).unwrap_err(),
            ActorError::EmptyGroupUid
        );
    }

    #[test]
    fn actor_rejects_fully_empty_name() {
        let result = Actor::new(
            ActorUid::new("PA1".into()).unwrap(),
            None,
            "  ".into(),
            "".into(),
            ActorRole::Deputy,
        );
        assert_eq!(result.unwrap_err(), ActorError::EmptyActorName);
    }

    #[test]
    fn actor_exposes_official_page_for_deputies_only() {
        let deputy = deputy("PA720916", "Fiona", "Lazaar");
        assert_eq!(
            deputy.official_url().as_deref(),
            Some("https://www.assemblee-nationale.fr/dyn/deputes/PA720916")
        );

        let minister = Actor::new(
            ActorUid::new("PA2".into()).unwrap(),
            None,
            "Jean".into(),
            "Dupont".into(),
            ActorRole::Minister,
        )
        .unwrap();
        assert_eq!(minister.official_url(), None);
    }

    #[test]
    fn group_rejects_empty_label_or_abbrev() {
        assert_eq!(
            ParliamentaryGroup::new(
                GroupUid::new("PO1".into()).unwrap(),
                17,
                "".into(),
                "GRP".into(),
                None,
                None,
                None,
            )
            .unwrap_err(),
            ActorError::EmptyGroupLabel
        );
        assert_eq!(
            ParliamentaryGroup::new(
                GroupUid::new("PO1".into()).unwrap(),
                17,
                "Groupe".into(),
                " ".into(),
                None,
                None,
                None,
            )
            .unwrap_err(),
            ActorError::EmptyGroupAbbrev
        );
    }

    #[test]
    fn membership_quality_rejects_empty() {
        assert_eq!(
            MembershipQuality::new("  ".into()).unwrap_err(),
            ActorError::EmptyMembershipQuality
        );
    }

    #[test]
    fn membership_quality_keeps_unknown_source_label() {
        let quality = MembershipQuality::new("Qualit\u{00e9} in\u{00e9}dite".into()).unwrap();
        assert_eq!(quality.as_str(), "Qualit\u{00e9} in\u{00e9}dite");
    }

    #[test]
    fn period_rejects_end_before_start() {
        let result = MembershipPeriod::new(date(2024, 7, 18), Some(date(2024, 7, 17)));
        assert_eq!(
            result.unwrap_err(),
            ActorError::ReversedMembershipPeriod {
                start: date(2024, 7, 18),
                end: date(2024, 7, 17),
            }
        );
    }

    #[test]
    fn period_end_is_inclusive() {
        let period = MembershipPeriod::new(date(2024, 7, 18), Some(date(2024, 9, 11))).unwrap();
        assert!(period.contains(date(2024, 7, 18)));
        assert!(period.contains(date(2024, 9, 11)));
        assert!(!period.contains(date(2024, 9, 12)));
        assert!(!period.contains(date(2024, 7, 17)));
    }

    #[test]
    fn open_period_contains_any_later_date() {
        let period = MembershipPeriod::new(date(2024, 7, 18), None).unwrap();
        assert!(period.is_open());
        assert!(period.contains(date(2030, 1, 1)));
    }

    #[test]
    fn membership_at_returns_group_valid_on_that_date_not_the_current_one() {
        let memberships = vec![
            membership(
                "PM1",
                "PA1",
                "PO_A",
                date(2024, 7, 19),
                Some(date(2025, 3, 31)),
                "Membre",
            ),
            membership("PM2", "PA1", "PO_B", date(2025, 4, 1), None, "Membre"),
        ];

        match membership_at(&memberships, date(2024, 9, 12)) {
            MembershipAtDate::Found(m) => assert_eq!(m.group_uid().as_str(), "PO_A"),
            other => panic!("expected PO_A, got {other:?}"),
        }
        match membership_at(&memberships, date(2025, 6, 1)) {
            MembershipAtDate::Found(m) => assert_eq!(m.group_uid().as_str(), "PO_B"),
            other => panic!("expected PO_B, got {other:?}"),
        }
    }

    #[test]
    fn membership_at_keeps_president_quality_in_their_own_group() {
        let memberships = vec![
            membership("PM1", "PA1", "PO_A", date(2024, 7, 19), None, "Membre"),
            membership(
                "PM2",
                "PA1",
                "PO_A",
                date(2025, 3, 1),
                None,
                "Pr\u{00e9}sident",
            ),
        ];

        match membership_at(&memberships, date(2025, 6, 1)) {
            MembershipAtDate::Found(m) => {
                assert_eq!(m.group_uid().as_str(), "PO_A");
                assert_eq!(m.quality().as_str(), "Pr\u{00e9}sident");
            }
            other => panic!("expected PO_A, got {other:?}"),
        }
    }

    #[test]
    fn membership_at_counts_affiliated_members() {
        let memberships = vec![membership(
            "PM1",
            "PA1",
            "PO_A",
            date(2024, 7, 19),
            None,
            "Membre apparent\u{00e9}",
        )];

        assert!(matches!(
            membership_at(&memberships, date(2025, 1, 1)),
            MembershipAtDate::Found(_)
        ));
    }

    #[test]
    fn membership_at_treats_unregistered_as_a_group() {
        let memberships = vec![membership(
            "PM1",
            "PA1",
            "PO840056",
            date(2024, 7, 1),
            Some(date(2024, 7, 18)),
            "D\u{00e9}put\u{00e9} non-inscrit",
        )];

        match membership_at(&memberships, date(2024, 7, 5)) {
            MembershipAtDate::Found(m) => assert_eq!(m.group_uid().as_str(), "PO840056"),
            other => panic!("expected the non-inscrit group, got {other:?}"),
        }
    }

    #[test]
    fn membership_at_returns_none_outside_every_period() {
        let memberships = vec![membership(
            "PM1",
            "PA1",
            "PO_A",
            date(2024, 7, 19),
            Some(date(2024, 12, 31)),
            "Membre",
        )];

        assert!(matches!(
            membership_at(&memberships, date(2025, 1, 1)),
            MembershipAtDate::None
        ));
    }

    #[test]
    fn membership_at_reports_two_distinct_groups_as_ambiguous() {
        let memberships = vec![
            membership("PM1", "PA1", "PO_A", date(2024, 7, 19), None, "Membre"),
            membership("PM2", "PA1", "PO_B", date(2024, 7, 19), None, "Membre"),
        ];

        assert!(matches!(
            membership_at(&memberships, date(2025, 1, 1)),
            MembershipAtDate::Ambiguous
        ));
    }

    #[test]
    fn directory_resolves_actor_and_group_at_date() {
        let directory = ActorDirectory::new(
            vec![deputy("PA1", "Fiona", "Lazaar")],
            vec![group("PO_A", "GRP", "Groupe A")],
            vec![membership(
                "PM1",
                "PA1",
                "PO_A",
                date(2024, 7, 19),
                None,
                "Membre",
            )],
        );

        let uid = ActorUid::new("PA1".into()).unwrap();
        let resolved = directory.resolve_at(&uid, date(2024, 9, 12)).unwrap();
        assert_eq!(resolved.actor.full_name(), "Fiona Lazaar");
        assert_eq!(resolved.group.unwrap().abbrev(), "GRP");
        assert_eq!(resolved.quality.unwrap().as_str(), "Membre");
        assert!(!resolved.ambiguous);
    }

    #[test]
    fn directory_returns_none_for_unknown_actor() {
        let directory = ActorDirectory::new(vec![], vec![], vec![]);
        let uid = ActorUid::new("PA_UNKNOWN".into()).unwrap();
        assert!(directory.resolve_at(&uid, date(2024, 9, 12)).is_none());
    }

    #[test]
    fn directory_yields_actor_without_group_when_no_membership_covers_the_date() {
        let directory = ActorDirectory::new(
            vec![deputy("PA1", "Jean", "Dupont")],
            vec![group("PO_A", "GRP", "Groupe A")],
            vec![membership(
                "PM1",
                "PA1",
                "PO_A",
                date(2025, 1, 1),
                None,
                "Membre",
            )],
        );

        let uid = ActorUid::new("PA1".into()).unwrap();
        let resolved = directory.resolve_at(&uid, date(2024, 9, 12)).unwrap();
        assert!(resolved.group.is_none());
        assert!(!resolved.ambiguous);
    }

    #[test]
    fn directory_flags_ambiguity_instead_of_picking_a_group() {
        let directory = ActorDirectory::new(
            vec![deputy("PA1", "Jean", "Dupont")],
            vec![
                group("PO_A", "A", "Groupe A"),
                group("PO_B", "B", "Groupe B"),
            ],
            vec![
                membership("PM1", "PA1", "PO_A", date(2024, 7, 19), None, "Membre"),
                membership("PM2", "PA1", "PO_B", date(2024, 7, 19), None, "Membre"),
            ],
        );

        let uid = ActorUid::new("PA1".into()).unwrap();
        let resolved = directory.resolve_at(&uid, date(2025, 1, 1)).unwrap();
        assert!(resolved.group.is_none());
        assert!(resolved.ambiguous);
    }

    #[test]
    fn dissolved_group_keeps_its_historical_label() {
        let dissolved = ParliamentaryGroup::new(
            GroupUid::new("PO845520".into()).unwrap(),
            17,
            "\u{00c0} Droite".into(),
            "AD".into(),
            Some("#3367A7".into()),
            Some(date(2024, 7, 18)),
            Some(date(2024, 9, 11)),
        )
        .unwrap();

        assert!(dissolved.is_dissolved());
        assert_eq!(dissolved.label(), "\u{00c0} Droite");
    }
}
