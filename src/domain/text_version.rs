//! Versions d'un texte de loi (SPEC-en-bref RM-01, RM-14, RM-15).
//!
//! Une version est un texte déposé ou adopté en séance, avec la chambre qui le
//! publie. Le domaine ne lit aucun texte : il décide seulement ce qui compte
//! comme version et laquelle est la dernière. L'ordre de passage des dossiers
//! est celui de `text_brief::brief_pass_order`.

use chrono::NaiveDate;

use super::dossier::DossierUid;

/// Préfixe des textes publiés par l'Assemblée nationale (RM-14).
const OFFICIAL_TEXT_URL_PREFIX: &str = "https://www.assemblee-nationale.fr/dyn/docs/";

/// Types de document qui portent un texte de loi : proposition (`PION`) et
/// projet (`PRJL`). Les rapports, avis, résolutions et études n'en sont pas.
const BILL_TYPES: [&str; 2] = ["PION", "PRJL"];

/// Statut d'adoption publié pour un texte adopté en séance.
const ADOPTED_IN_SITTING: &str = "ADOPTSEANCE";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TextVersionError {
    #[error("document uid is empty")]
    EmptyDocumentUid,
}

/// Référence d'un document de l'Assemblée (`PIONANR5L17B1100`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DocumentUid(String);

impl DocumentUid {
    pub fn new(raw: String) -> Result<Self, TextVersionError> {
        if raw.trim().is_empty() {
            return Err(TextVersionError::EmptyDocumentUid);
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DocumentUid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Lien officiel du texte d'un document publié par l'Assemblée. Le suffixe
/// `.raw` est celui des pages dynamiques de l'Assemblée, quel que soit le type
/// du document.
pub fn official_document_url(document_uid: &str) -> String {
    format!("{OFFICIAL_TEXT_URL_PREFIX}{document_uid}.raw")
}

/// Chambre qui publie une version, lue dans sa référence (`ANR` / `SNR`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chamber {
    Assembly,
    Senate,
}

impl Chamber {
    pub fn as_str(&self) -> &'static str {
        match self {
            Chamber::Assembly => "an",
            Chamber::Senate => "sn",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "an" => Some(Chamber::Assembly),
            "sn" => Some(Chamber::Senate),
            _ => None,
        }
    }
}

/// Nature d'une version. L'ordre de déclaration compte : à date égale, le
/// texte adopté suit le texte déposé (RM-01).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VersionKind {
    Deposited,
    Adopted,
}

impl VersionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            VersionKind::Deposited => "deposited",
            VersionKind::Adopted => "adopted",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "deposited" => Some(VersionKind::Deposited),
            "adopted" => Some(VersionKind::Adopted),
            _ => None,
        }
    }
}

/// État de lecture du texte d'une version. `Missing` est définitif : un texte
/// introuvable ou sans aucun article reconnu n'est pas relu (CU-01, Erreurs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadingStatus {
    Unread,
    Read,
    Missing,
}

impl ReadingStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReadingStatus::Unread => "unread",
            ReadingStatus::Read => "read",
            ReadingStatus::Missing => "missing",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "unread" => Some(ReadingStatus::Unread),
            "read" => Some(ReadingStatus::Read),
            "missing" => Some(ReadingStatus::Missing),
            _ => None,
        }
    }
}

/// Document tel que la source le publie, avant de savoir s'il est une version.
#[derive(Debug, Clone)]
pub struct SourceDocument {
    pub document_uid: DocumentUid,
    pub dossier_uid: DossierUid,
    /// Statut d'adoption publié (`ADOPTSEANCE`, `ADOPTCOM`, `REJETSEANCE`...).
    pub adoption_status: Option<String>,
    /// Date du document : dépôt, ou publication d'un texte adopté qui n'en a pas.
    pub deposited_on: Option<NaiveDate>,
    pub title: String,
}

/// Champs d'une version relue (base de données).
#[derive(Debug, Clone)]
pub struct NewTextVersion {
    pub document_uid: DocumentUid,
    pub dossier_uid: DossierUid,
    pub kind: VersionKind,
    pub chamber: Chamber,
    pub deposited_on: NaiveDate,
    pub title: String,
    pub reading: ReadingStatus,
}

/// Famille d'une référence : ce qu'elle dit du texte.
#[derive(Debug, PartialEq, Eq)]
enum Series {
    /// `…B<n>` : texte déposé.
    Deposit,
    /// `…BTA<n>` : texte adopté en séance.
    SittingText,
    /// `…BTC<n>` (commission), `…TAP<n>` (adopté provisoire) et le reste.
    Other,
}

/// Ce que la référence d'un document dit de lui.
struct Reference<'a> {
    document_type: &'a str,
    chamber: Chamber,
    series: Series,
}

impl<'a> Reference<'a> {
    /// `<type 4 lettres><ANR|SNR>5<L17|S479><B|BTA|BTC|TAP><n>`.
    fn parse(uid: &'a str) -> Option<Self> {
        let document_type = uid.get(..4)?;
        let chamber = match uid.get(4..7)? {
            "ANR" => Chamber::Assembly,
            "SNR" => Chamber::Senate,
            _ => return None,
        };
        let rest = uid.get(7..)?.strip_prefix('5')?.strip_prefix(['L', 'S'])?;
        let rest = rest.trim_start_matches(|c: char| c.is_ascii_digit());
        let (series, number) = if let Some(number) = rest.strip_prefix("BTA") {
            (Series::SittingText, number)
        } else if let Some(number) = rest.strip_prefix("BTC") {
            (Series::Other, number)
        } else if let Some(number) = rest.strip_prefix("TAP") {
            (Series::Other, number)
        } else if let Some(number) = rest.strip_prefix('B') {
            (Series::Deposit, number)
        } else {
            return None;
        };
        if number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        Some(Self {
            document_type,
            chamber,
            series,
        })
    }
}

/// Version d'un texte de loi : un document déposé ou adopté en séance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextVersion {
    document_uid: DocumentUid,
    dossier_uid: DossierUid,
    kind: VersionKind,
    chamber: Chamber,
    deposited_on: NaiveDate,
    title: String,
    reading: ReadingStatus,
}

impl TextVersion {
    /// RM-01 : une version est une proposition ou un projet de loi déposé
    /// (`…B<n>`) ou adopté en séance (`…BTA<n>` et `ADOPTSEANCE`), daté. Le
    /// texte de commission, le texte adopté provisoire, le texte rejeté, les
    /// rapports et les avis n'en sont pas. La chambre est lue dans la
    /// référence ; la version naît non lue.
    pub fn from_document(document: SourceDocument) -> Option<Self> {
        let reference = Reference::parse(document.document_uid.as_str())?;
        if !BILL_TYPES.contains(&reference.document_type) {
            return None;
        }
        let kind = match reference.series {
            Series::Deposit => VersionKind::Deposited,
            Series::SittingText
                if document.adoption_status.as_deref() == Some(ADOPTED_IN_SITTING) =>
            {
                VersionKind::Adopted
            }
            Series::SittingText | Series::Other => return None,
        };
        Some(Self::new(NewTextVersion {
            chamber: reference.chamber,
            kind,
            deposited_on: document.deposited_on?,
            document_uid: document.document_uid,
            dossier_uid: document.dossier_uid,
            title: document.title,
            reading: ReadingStatus::Unread,
        }))
    }

    /// Version relue telle qu'elle a été enregistrée.
    pub fn new(fields: NewTextVersion) -> Self {
        Self {
            document_uid: fields.document_uid,
            dossier_uid: fields.dossier_uid,
            kind: fields.kind,
            chamber: fields.chamber,
            deposited_on: fields.deposited_on,
            title: fields.title,
            reading: fields.reading,
        }
    }

    /// Même version, avec un autre état de lecture.
    pub fn with_reading(&self, reading: ReadingStatus) -> Self {
        Self {
            reading,
            ..self.clone()
        }
    }

    pub fn document_uid(&self) -> &DocumentUid {
        &self.document_uid
    }

    pub fn dossier_uid(&self) -> &DossierUid {
        &self.dossier_uid
    }

    pub fn kind(&self) -> VersionKind {
        self.kind
    }

    pub fn chamber(&self) -> Chamber {
        self.chamber
    }

    pub fn deposited_on(&self) -> NaiveDate {
        self.deposited_on
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn reading(&self) -> ReadingStatus {
        self.reading
    }

    /// RM-14 : seule une version dont l'Assemblée publie le texte est
    /// disponible. Une version du Sénat, ou un texte de l'Assemblée introuvable
    /// ou sans article reconnu, ne l'est pas.
    pub fn is_available(&self) -> bool {
        self.chamber == Chamber::Assembly && self.reading != ReadingStatus::Missing
    }

    /// Nom de la version, tel que la fiche l'écrit : la nature et la chambre.
    pub fn label(&self) -> &'static str {
        match (self.kind, self.chamber) {
            (VersionKind::Deposited, Chamber::Assembly) => "Texte déposé à l'Assemblée nationale",
            (VersionKind::Adopted, Chamber::Assembly) => "Texte adopté par l'Assemblée nationale",
            (VersionKind::Deposited, Chamber::Senate) => "Texte déposé au Sénat",
            (VersionKind::Adopted, Chamber::Senate) => "Texte adopté par le Sénat",
        }
    }

    /// RM-14 : ce que l'on sait du texte de la version.
    pub fn text_status(&self) -> TextStatus {
        match (self.chamber, self.reading) {
            (Chamber::Senate, _) => TextStatus::SenateOnly,
            (Chamber::Assembly, ReadingStatus::Unread) => TextStatus::Unread,
            (Chamber::Assembly, ReadingStatus::Read) => TextStatus::Read,
            (Chamber::Assembly, ReadingStatus::Missing) => TextStatus::Missing,
        }
    }

    /// Lien officiel du texte, pour une version publiée par l'Assemblée.
    pub fn official_url(&self) -> Option<String> {
        (self.chamber == Chamber::Assembly)
            .then(|| official_document_url(self.document_uid.as_str()))
    }

    /// Rang de récence (RM-01) : la date, puis le texte adopté après le texte
    /// déposé, puis la référence.
    fn recency(&self) -> (NaiveDate, VersionKind, &str) {
        (self.deposited_on, self.kind, self.document_uid.as_str())
    }
}

/// RM-01, RM-14 : la dernière version dont le texte est disponible.
pub fn latest_available(versions: &[TextVersion]) -> Option<&TextVersion> {
    versions
        .iter()
        .filter(|version| version.is_available())
        .max_by(|a, b| a.recency().cmp(&b.recency()))
}

/// RM-14 : la plus récente des versions indisponibles plus récentes que celle
/// qui est montrée, pour la signaler.
pub fn newer_unavailable<'a>(
    versions: &'a [TextVersion],
    shown: &TextVersion,
) -> Option<&'a TextVersion> {
    versions
        .iter()
        .filter(|version| !version.is_available() && version.recency() > shown.recency())
        .max_by(|a, b| a.recency().cmp(&b.recency()))
}

/// État du texte d'une version, tel que la fiche le dit (RM-14).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextStatus {
    /// Publié par l'Assemblée, pas encore lu.
    Unread,
    /// Publié par l'Assemblée, articles lus.
    Read,
    /// Publié par l'Assemblée, introuvable ou sans article reconnu.
    Missing,
    /// Publié par le Sénat seulement.
    SenateOnly,
}

impl TextStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TextStatus::Unread => "unread",
            TextStatus::Read => "read",
            TextStatus::Missing => "missing",
            TextStatus::SenateOnly => "senate_only",
        }
    }
}

/// RM-01 : les versions de la plus récente à la plus ancienne.
pub fn newest_first(versions: &[TextVersion]) -> Vec<TextVersion> {
    let mut ordered = versions.to_vec();
    ordered.sort_by(|a, b| b.recency().cmp(&a.recency()));
    ordered
}

/// Dossier voté, candidat à la passe « En bref » (RM-15).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BriefCandidate {
    pub dossier_uid: DossierUid,
    pub last_vote_on: NaiveDate,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dossier() -> DossierUid {
        DossierUid::new("DLR5L17N51670".to_string()).unwrap()
    }

    fn day(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn document(uid: &str, adoption_status: Option<&str>) -> SourceDocument {
        SourceDocument {
            document_uid: DocumentUid::new(uid.to_string()).unwrap(),
            dossier_uid: dossier(),
            adoption_status: adoption_status.map(str::to_string),
            deposited_on: Some(day(2025, 5, 27)),
            title: "proposition de loi relative au droit à l'aide à mourir".to_string(),
        }
    }

    fn version(
        uid: &str,
        kind: VersionKind,
        chamber: Chamber,
        on: NaiveDate,
        reading: ReadingStatus,
    ) -> TextVersion {
        TextVersion::new(NewTextVersion {
            document_uid: DocumentUid::new(uid.to_string()).unwrap(),
            dossier_uid: dossier(),
            kind,
            chamber,
            deposited_on: on,
            title: "titre".to_string(),
            reading,
        })
    }

    fn assembly(uid: &str, kind: VersionKind, on: NaiveDate) -> TextVersion {
        version(uid, kind, Chamber::Assembly, on, ReadingStatus::Unread)
    }

    #[test]
    fn should_take_deposited_bill_as_version_when_reference_is_a_deposit() {
        let taken = TextVersion::from_document(document("PIONANR5L17B1100", None)).unwrap();

        assert_eq!(taken.kind(), VersionKind::Deposited);
        assert_eq!(taken.chamber(), Chamber::Assembly);
        assert_eq!(taken.document_uid().as_str(), "PIONANR5L17B1100");
        assert_eq!(taken.dossier_uid(), &dossier());
        assert_eq!(taken.deposited_on(), day(2025, 5, 27));
        assert_eq!(taken.reading(), ReadingStatus::Unread);
        assert!(taken.is_available());
    }

    #[test]
    fn should_take_deposited_government_bill_as_version_when_reference_is_a_deposit() {
        let taken = TextVersion::from_document(document("PRJLANR5L17B3210", None)).unwrap();

        assert_eq!(taken.kind(), VersionKind::Deposited);
        assert_eq!(taken.chamber(), Chamber::Assembly);
    }

    #[test]
    fn should_take_adopted_text_as_version_when_adopted_in_sitting() {
        let taken = TextVersion::from_document(document("PIONANR5L17BTA0122", Some("ADOPTSEANCE")))
            .unwrap();

        assert_eq!(taken.kind(), VersionKind::Adopted);
        assert_eq!(taken.chamber(), Chamber::Assembly);
        assert!(taken.is_available());
    }

    #[test]
    fn should_skip_document_when_it_is_a_committee_text() {
        let skipped = TextVersion::from_document(document("PIONANR5L17BTC1364", Some("ADOPTCOM")));

        assert_eq!(skipped, None);
    }

    #[test]
    fn should_skip_document_when_it_is_a_provisional_adopted_text() {
        let skipped =
            TextVersion::from_document(document("PIONANR5L17TAP0122", Some("ADOPTSEANCE")));

        assert_eq!(skipped, None);
    }

    #[test]
    fn should_skip_document_when_sitting_rejected_it() {
        let skipped =
            TextVersion::from_document(document("PIONSNR5S479BTA0043", Some("REJETSEANCE")));

        assert_eq!(skipped, None);
    }

    #[test]
    fn should_skip_document_when_it_is_a_report() {
        let skipped = TextVersion::from_document(document("RAPPANR5L17B1364", None));

        assert_eq!(skipped, None);
    }

    #[test]
    fn should_skip_document_when_it_is_a_resolution_or_an_opinion() {
        assert_eq!(
            TextVersion::from_document(document("PNREANR5L17B2134", None)),
            None
        );
        assert_eq!(
            TextVersion::from_document(document("AVISANR5L17B0524", None)),
            None
        );
    }

    #[test]
    fn should_skip_document_when_it_has_no_date() {
        let mut undated = document("PIONANR5L17B1100", None);
        undated.deposited_on = None;

        assert_eq!(TextVersion::from_document(undated), None);
    }

    #[test]
    fn should_skip_document_when_reference_is_not_a_known_shape() {
        for reference in [
            "PION",
            "PIONANR5L17B",
            "PIONANR5L17B11x0",
            "PIONXXX5L17B1100",
            "é",
        ] {
            assert_eq!(
                TextVersion::from_document(document(reference, None)),
                None,
                "{reference} must not be a version"
            );
        }
    }

    #[test]
    fn should_mark_senate_version_unavailable_when_reference_is_a_senate_one() {
        let taken = TextVersion::from_document(document("PIONSNR5S479B0440", None)).unwrap();

        assert_eq!(taken.chamber(), Chamber::Senate);
        assert_eq!(taken.kind(), VersionKind::Deposited);
        assert!(!taken.is_available());
        assert_eq!(taken.official_url(), None);
    }

    #[test]
    fn should_take_adopted_senate_text_as_version_when_adopted_in_sitting() {
        let taken =
            TextVersion::from_document(document("PIONSNR5S379BTA0027", Some("ADOPTSEANCE")))
                .unwrap();

        assert_eq!(taken.kind(), VersionKind::Adopted);
        assert_eq!(taken.chamber(), Chamber::Senate);
        assert!(!taken.is_available());
    }

    #[test]
    fn should_give_official_url_when_version_is_published_by_assembly() {
        let taken = TextVersion::from_document(document("PIONANR5L17B1560", None)).unwrap();

        assert_eq!(
            taken.official_url().as_deref(),
            Some("https://www.assemblee-nationale.fr/dyn/docs/PIONANR5L17B1560.raw")
        );
        assert_eq!(
            official_document_url("PIONANR5L17B1560"),
            "https://www.assemblee-nationale.fr/dyn/docs/PIONANR5L17B1560.raw"
        );
    }

    #[test]
    fn should_refuse_document_uid_when_blank() {
        assert_eq!(
            DocumentUid::new("  ".to_string()),
            Err(TextVersionError::EmptyDocumentUid)
        );
    }

    #[test]
    fn should_take_adopted_after_deposited_as_latest_when_dates_are_equal() {
        let on = day(2026, 1, 15);
        let versions = vec![
            assembly("PIONANR5L17BTA0200", VersionKind::Adopted, on),
            assembly("PIONANR5L17B0900", VersionKind::Deposited, on),
        ];

        let latest = latest_available(&versions).unwrap();

        assert_eq!(latest.document_uid().as_str(), "PIONANR5L17BTA0200");
    }

    #[test]
    fn should_take_most_recent_date_as_latest_even_when_it_is_a_deposit() {
        let versions = vec![
            assembly("PIONANR5L17BTA0200", VersionKind::Adopted, day(2026, 1, 15)),
            assembly("PIONANR5L17B0900", VersionKind::Deposited, day(2026, 3, 2)),
        ];

        let latest = latest_available(&versions).unwrap();

        assert_eq!(latest.document_uid().as_str(), "PIONANR5L17B0900");
    }

    #[test]
    fn should_take_greater_reference_as_latest_when_date_and_kind_are_equal() {
        let on = day(2026, 1, 15);
        let versions = vec![
            assembly("PIONANR5L17B0901", VersionKind::Deposited, on),
            assembly("PIONANR5L17B0900", VersionKind::Deposited, on),
        ];

        let latest = latest_available(&versions).unwrap();

        assert_eq!(latest.document_uid().as_str(), "PIONANR5L17B0901");
    }

    #[test]
    fn should_have_no_latest_when_no_version_is_available() {
        let versions = vec![version(
            "PIONSNR5S479B0440",
            VersionKind::Deposited,
            Chamber::Senate,
            day(2026, 1, 15),
            ReadingStatus::Unread,
        )];

        assert_eq!(latest_available(&versions), None);
        assert_eq!(latest_available(&[]), None);
    }

    #[test]
    fn should_signal_newer_senate_version_when_latest_available_is_older() {
        let versions = vec![
            assembly("PIONANR5L17B0900", VersionKind::Deposited, day(2026, 1, 15)),
            version(
                "PIONSNR5S479BTA0043",
                VersionKind::Adopted,
                Chamber::Senate,
                day(2026, 4, 1),
                ReadingStatus::Unread,
            ),
        ];
        let shown = latest_available(&versions).unwrap();

        let newer = newer_unavailable(&versions, shown).unwrap();

        assert_eq!(newer.document_uid().as_str(), "PIONSNR5S479BTA0043");
    }

    #[test]
    fn should_not_signal_senate_version_when_it_is_older_than_the_shown_one() {
        let versions = vec![
            version(
                "PIONSNR5S479B0440",
                VersionKind::Deposited,
                Chamber::Senate,
                day(2025, 11, 1),
                ReadingStatus::Unread,
            ),
            assembly("PIONANR5L17B0900", VersionKind::Deposited, day(2026, 1, 15)),
        ];
        let shown = latest_available(&versions).unwrap();

        assert_eq!(newer_unavailable(&versions, shown), None);
    }

    #[test]
    fn should_skip_missing_assembly_version_when_choosing_latest() {
        let versions = vec![
            assembly("PIONANR5L17B0900", VersionKind::Deposited, day(2026, 1, 15)),
            version(
                "PIONANR5L17BTA0200",
                VersionKind::Adopted,
                Chamber::Assembly,
                day(2026, 2, 1),
                ReadingStatus::Missing,
            ),
        ];

        let latest = latest_available(&versions).unwrap();

        assert_eq!(latest.document_uid().as_str(), "PIONANR5L17B0900");
        assert!(!versions[1].is_available());
    }

    #[test]
    fn should_label_version_with_kind_and_chamber_when_shown() {
        let label = |kind, chamber| {
            version(
                "PIONANR5L17B1",
                kind,
                chamber,
                day(2026, 1, 1),
                ReadingStatus::Read,
            )
            .label()
        };

        assert_eq!(
            label(VersionKind::Deposited, Chamber::Assembly),
            "Texte déposé à l'Assemblée nationale"
        );
        assert_eq!(
            label(VersionKind::Adopted, Chamber::Assembly),
            "Texte adopté par l'Assemblée nationale"
        );
        assert_eq!(
            label(VersionKind::Deposited, Chamber::Senate),
            "Texte déposé au Sénat"
        );
        assert_eq!(
            label(VersionKind::Adopted, Chamber::Senate),
            "Texte adopté par le Sénat"
        );
    }

    #[test]
    fn should_state_text_status_from_chamber_and_reading() {
        let status = |chamber, reading| {
            version(
                "PIONANR5L17B1",
                VersionKind::Deposited,
                chamber,
                day(2026, 1, 1),
                reading,
            )
            .text_status()
        };

        assert_eq!(
            status(Chamber::Assembly, ReadingStatus::Unread),
            TextStatus::Unread
        );
        assert_eq!(
            status(Chamber::Assembly, ReadingStatus::Read),
            TextStatus::Read
        );
        assert_eq!(
            status(Chamber::Assembly, ReadingStatus::Missing),
            TextStatus::Missing
        );
        assert_eq!(
            status(Chamber::Senate, ReadingStatus::Unread),
            TextStatus::SenateOnly
        );
        assert_eq!(
            status(Chamber::Senate, ReadingStatus::Read),
            TextStatus::SenateOnly
        );
    }

    #[test]
    fn should_name_text_statuses_when_written_for_the_api() {
        assert_eq!(TextStatus::Unread.as_str(), "unread");
        assert_eq!(TextStatus::Read.as_str(), "read");
        assert_eq!(TextStatus::Missing.as_str(), "missing");
        assert_eq!(TextStatus::SenateOnly.as_str(), "senate_only");
    }

    #[test]
    fn should_order_versions_from_newest_to_oldest_with_the_latest_rank() {
        let versions = vec![
            assembly("PIONANR5L17B0900", VersionKind::Deposited, day(2026, 1, 15)),
            assembly("PIONANR5L17BTA0200", VersionKind::Adopted, day(2026, 1, 15)),
            assembly("PIONANR5L17B0100", VersionKind::Deposited, day(2025, 6, 1)),
            version(
                "PIONSNR5S479BTA0043",
                VersionKind::Adopted,
                Chamber::Senate,
                day(2026, 4, 1),
                ReadingStatus::Unread,
            ),
        ];

        let ordered = newest_first(&versions);

        let uids: Vec<&str> = ordered.iter().map(|v| v.document_uid().as_str()).collect();
        assert_eq!(
            uids,
            [
                "PIONSNR5S479BTA0043",
                "PIONANR5L17BTA0200",
                "PIONANR5L17B0900",
                "PIONANR5L17B0100"
            ]
        );
    }

    #[test]
    fn should_round_trip_kind_and_chamber_when_parsed_back() {
        for kind in [VersionKind::Deposited, VersionKind::Adopted] {
            assert_eq!(VersionKind::parse(kind.as_str()), Some(kind));
        }
        for chamber in [Chamber::Assembly, Chamber::Senate] {
            assert_eq!(Chamber::parse(chamber.as_str()), Some(chamber));
        }
        assert_eq!(VersionKind::parse("committee"), None);
        assert_eq!(Chamber::parse("cc"), None);
    }

    #[test]
    fn should_round_trip_reading_status_when_parsed_back() {
        for reading in [
            ReadingStatus::Unread,
            ReadingStatus::Read,
            ReadingStatus::Missing,
        ] {
            assert_eq!(ReadingStatus::parse(reading.as_str()), Some(reading));
        }
        assert_eq!(ReadingStatus::parse("pending"), None);
    }

    #[test]
    fn should_keep_every_field_when_reading_status_changes() {
        let unread = assembly("PIONANR5L17B0900", VersionKind::Deposited, day(2026, 1, 15));

        let read = unread.with_reading(ReadingStatus::Read);

        assert_eq!(read.reading(), ReadingStatus::Read);
        assert_eq!(read.document_uid(), unread.document_uid());
        assert_eq!(read.deposited_on(), unread.deposited_on());
        assert_eq!(read.kind(), unread.kind());
    }
}
