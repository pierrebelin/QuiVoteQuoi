//! Amendements de l'Assemblee nationale.
//!
//! Voir `todo/SPEC-amendements.md`. Trois invariants portent tout le reste:
//! - RM-01: tout amendement publie entre, y compris sans expose sommaire, sans
//!   cosignataire et sans texte rattachable;
//! - RM-02: le groupe d'un signataire est celui de la date de depot, jamais son
//!   appartenance courante (README.md §3.2);
//! - RM-03: l'expose sommaire est reproduit mot pour mot ou pas du tout. Aucun
//!   resume, aucun extrait choisi, aucun modele (README.md §6).
//!
//! L'agregat ne porte volontairement **aucune methode d'agregat**: pas de taux
//! d'adoption, pas de decompte de signatures par groupe, pas de comparaison.
//! Un tel calcul se lirait comme une note, ce que README.md §6 interdit.

use chrono::NaiveDate;
use serde::Serialize;

use super::actor::{ActorDirectory, ActorUid, GroupUid};
use super::theme::{clean, lowercase};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AmendmentError {
    #[error("amendment uid must not be empty")]
    EmptyAmendmentUid,
    #[error("amendment number must not be empty")]
    EmptyAmendmentNumber,
    #[error("legislative text ref must not be empty")]
    EmptyTextRef,
    #[error("amendment target title must not be empty")]
    EmptyTargetTitle,
    #[error("an amendment must carry an author")]
    MissingAuthor,
    #[error("institutional author label must not be empty")]
    EmptyInstitutionalAuthor,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct AmendmentUid(String);

impl AmendmentUid {
    pub fn new(raw: String) -> Result<Self, AmendmentError> {
        if raw.trim().is_empty() {
            return Err(AmendmentError::EmptyAmendmentUid);
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AmendmentUid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Numero d'amendement, tel que publie.
///
/// Une chaine, jamais un entier: la source publie « 45 », mais aussi « 45 rect. »
/// ou « CF120 » en commission. `key` en donne la forme normalisee, seule utilisee
/// pour rapprocher un amendement de l'objet d'un scrutin.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct AmendmentNumber(String);

impl AmendmentNumber {
    pub fn new(raw: String) -> Result<Self, AmendmentError> {
        if raw.trim().is_empty() {
            return Err(AmendmentError::EmptyAmendmentNumber);
        }
        Ok(Self(raw.trim().to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Forme normalisee: minuscules, caracteres non alphanumeriques retires.
    /// « 45 rect. » et « 45rect » se rejoignent, « 45 » et « 450 » restent
    /// distincts.
    pub fn key(&self) -> String {
        lowercase(&self.0)
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect()
    }
}

impl AmendmentNumber {
    /// Numero de reference: partie de finances ou prefixe de commission, puis
    /// numero entier; la rectification est ignoree (RM-01).
    pub fn reference(&self) -> Option<ReferenceNumber> {
        let lower = lowercase(&self.0);
        let token = lower.split_whitespace().next()?;
        let (prefix, digits) = if let Some(rest) = token.strip_prefix("ii-") {
            (Some(ReferencePrefix::Second), rest)
        } else if let Some(rest) = token.strip_prefix("i-") {
            (Some(ReferencePrefix::First), rest)
        } else {
            let letters: String = token.chars().take_while(|c| c.is_alphabetic()).collect();
            let rest = &token[letters.len()..];
            if letters.is_empty() {
                (None, rest)
            } else {
                (Some(ReferencePrefix::Commission(letters)), rest)
            }
        };
        let digits = digits.trim_end_matches('.');
        if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        Some(ReferenceNumber::new(prefix, digits.parse().ok()?))
    }

    /// Vrai quand ce numero publie designe le meme amendement que `cited`.
    pub fn answers(&self, cited: &AmendmentNumber) -> bool {
        match (self.reference(), cited.reference()) {
            (Some(stored), Some(cited)) => stored.answers(&cited),
            _ => false,
        }
    }
}

/// Prefixe d'un numero de reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferencePrefix {
    First,
    Second,
    Commission(String),
}

/// Numero de reference d'un amendement, lu par une regle fixe (RM-01, RM-03).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceNumber {
    prefix: Option<ReferencePrefix>,
    number: u32,
}

impl ReferenceNumber {
    pub fn new(prefix: Option<ReferencePrefix>, number: u32) -> Self {
        Self { prefix, number }
    }

    /// Meme numero; la partie de finances citee est absente ou egale, tout
    /// autre prefixe doit etre identique (RM-01).
    pub fn answers(&self, cited: &ReferenceNumber) -> bool {
        if self.number != cited.number {
            return false;
        }
        match (&self.prefix, &cited.prefix) {
            (stored, None) => !matches!(stored, Some(ReferencePrefix::Commission(_))),
            (stored, cited) => stored == cited,
        }
    }
}

impl std::fmt::Display for AmendmentNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Identifiant du texte legislatif que l'amendement modifie, publie par la
/// source (« PRJLANR5L17B0324 »). C'est lui qui relie l'amendement a un dossier,
/// par jointure sur les documents du dossier: un identifiant des deux cotes,
/// aucun rapprochement approximatif (RM-05).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct LegislativeTextRef(String);

impl LegislativeTextRef {
    pub fn new(raw: String) -> Result<Self, AmendmentError> {
        if raw.trim().is_empty() {
            return Err(AmendmentError::EmptyTextRef);
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for LegislativeTextRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Sort
// ---------------------------------------------------------------------------

/// Sort d'un amendement, ramene a un code stable pour les filtres.
///
/// `Other` n'est pas une categorie fourre-tout: c'est le marqueur d'une valeur
/// publiee que ce referentiel ne connait pas. Elle garde son libelle, elle est
/// comptee au rafraichissement, et elle se voit. La ranger en silence dans une
/// categorie voisine serait une reecriture de la source (README.md §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FateCode {
    Adopted,
    Rejected,
    Withdrawn,
    Fallen,
    NotSupported,
    Inadmissible,
    NotDiscussed,
    Unspecified,
    Other,
}

impl FateCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Adopted => "adopted",
            Self::Rejected => "rejected",
            Self::Withdrawn => "withdrawn",
            Self::Fallen => "fallen",
            Self::NotSupported => "not_supported",
            Self::Inadmissible => "inadmissible",
            Self::NotDiscussed => "not_discussed",
            Self::Unspecified => "unspecified",
            Self::Other => "other",
        }
    }
}

/// Sort publie: un code pour les filtres, un libelle affiche tel quel (§6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AmendmentFate {
    code: FateCode,
    label: String,
}

impl AmendmentFate {
    /// Lit le sort publie par la source.
    ///
    /// La comparaison porte sur une forme normalisee (minuscules, apostrophes et
    /// espaces insecables ramenes), et sur le **debut** du libelle: la source
    /// decline « Irrecevable art. 40 » et « Irrecevable art. 45 » sur la meme
    /// notion. Le libelle affiche reste celui de la source, non tronque.
    ///
    /// La table ci-dessous vaut pour les valeurs releveees a ce jour; toute autre
    /// valeur prend le code `Other` et remonte au rafraichissement.
    pub fn from_source(raw: Option<&str>) -> Self {
        let Some(raw) = raw else {
            return Self {
                code: FateCode::Unspecified,
                label: String::new(),
            };
        };
        let label = clean(raw);
        if label.is_empty() {
            return Self {
                code: FateCode::Unspecified,
                label: String::new(),
            };
        }

        let normalized = lowercase(&label);
        let code = if normalized.starts_with("adopt") {
            FateCode::Adopted
        } else if normalized.starts_with("rejet") {
            FateCode::Rejected
        } else if normalized.starts_with("retir") {
            FateCode::Withdrawn
        } else if normalized.starts_with("tomb") {
            FateCode::Fallen
        } else if normalized.starts_with("non soutenu") {
            FateCode::NotSupported
        } else if normalized.starts_with("irrecevable") {
            FateCode::Inadmissible
        } else if normalized.starts_with("non discut") || normalized.starts_with("a discuter") {
            FateCode::NotDiscussed
        } else {
            FateCode::Other
        };

        Self { code, label }
    }

    pub fn code(&self) -> FateCode {
        self.code
    }

    /// Libelle publie, jamais reformule (README.md §6, SPEC-scrutins RM-09).
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Vrai quand la source a publie un sort que le referentiel ne connait pas.
    pub fn is_unknown(&self) -> bool {
        self.code == FateCode::Other
    }
}

// ---------------------------------------------------------------------------
// Apercu et bandeau de sort
// ---------------------------------------------------------------------------

/// Longueur de l'apercu de l'expose sommaire, en caracteres (RM-05).
pub const PREVIEW_LENGTH: usize = 300;

/// Apercu de l'expose sommaire: ses premiers caracteres, jamais reformules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum SummaryPreview {
    Unpublished,
    Text { text: String, truncated: bool },
}

impl SummaryPreview {
    pub fn of(summary: Option<&str>) -> Self {
        let Some(summary) = summary else {
            return Self::Unpublished;
        };
        if summary.chars().count() <= PREVIEW_LENGTH {
            return Self::Text {
                text: summary.to_string(),
                truncated: false,
            };
        }
        let mut text: String = summary.chars().take(PREVIEW_LENGTH).collect();
        text.push('\u{2026}');
        Self::Text {
            text,
            truncated: true,
        }
    }
}

/// Bandeau de sort affiche sur la carte d'un amendement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FateBanner(String);

impl FateBanner {
    pub fn of(fate: FateCode, fate_label: &str, state_label: Option<&str>) -> Self {
        if fate != FateCode::Unspecified {
            return Self(fate_label.to_string());
        }
        Self(match state_label.map(str::trim).filter(|s| !s.is_empty()) {
            Some(state) => format!("Sort non publi\u{00e9} (\u{00e9}tat : {state})"),
            None => "Sort non publi\u{00e9}".to_string(),
        })
    }

    pub fn label(&self) -> &str {
        &self.0
    }
}

// ---------------------------------------------------------------------------
// Signataires
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SignatoryRole {
    Author,
    Cosignatory,
}

impl SignatoryRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Author => "author",
            Self::Cosignatory => "cosignatory",
        }
    }
}

/// D'ou vient le groupe affiche a cote d'un signataire (README.md §3.2).
///
/// `Published`: la source nomme le groupe dans l'amendement — date par
/// construction, comme la ligne de groupe d'un scrutin (SPEC-scrutins RM-04).
/// `ResolvedAtDeposit`: reconstitue depuis l'appartenance datee, a la date de
/// depot. `Unknown`: rien n'est resoluble, et rien n'est devine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupOrigin {
    Published,
    ResolvedAtDeposit,
    Unknown,
}

impl GroupOrigin {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Published => "published",
            Self::ResolvedAtDeposit => "resolved_at_deposit",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Signatory {
    actor_uid: ActorUid,
    role: SignatoryRole,
    /// Rang publie. Sert a restituer l'ordre de la source, jamais a classer.
    rank: u16,
    group_uid: Option<GroupUid>,
    group_origin: GroupOrigin,
    /// Vrai quand deux groupes concurrents revendiquent l'acteur a cette date:
    /// aucun groupe n'est alors affiche (ACTEURS RM-04).
    group_ambiguous: bool,
}

impl Signatory {
    /// Signataire tel que lu dans la source, avant resolution du groupe date.
    pub fn new(
        actor_uid: ActorUid,
        role: SignatoryRole,
        rank: u16,
        published_group: Option<GroupUid>,
    ) -> Self {
        let group_origin = if published_group.is_some() {
            GroupOrigin::Published
        } else {
            GroupOrigin::Unknown
        };
        Self {
            actor_uid,
            role,
            rank,
            group_uid: published_group,
            group_origin,
            group_ambiguous: false,
        }
    }

    pub fn actor_uid(&self) -> &ActorUid {
        &self.actor_uid
    }

    pub fn role(&self) -> SignatoryRole {
        self.role
    }

    pub fn rank(&self) -> u16 {
        self.rank
    }

    pub fn group_uid(&self) -> Option<&GroupUid> {
        self.group_uid.as_ref()
    }

    pub fn group_origin(&self) -> GroupOrigin {
        self.group_origin
    }

    pub fn group_ambiguous(&self) -> bool {
        self.group_ambiguous
    }
}

/// Auteur d'un amendement.
///
/// Tous les amendements ne sont pas de deputes: le Gouvernement et les
/// commissions en deposent, et la source ne publie alors aucun `acteurRef`. Le
/// libelle est conserve tel quel plutot que rattache de force a une personne.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Author {
    Deputy(Signatory),
    Institutional { label: String },
}

impl Author {
    pub fn actor_uid(&self) -> Option<&ActorUid> {
        match self {
            Self::Deputy(signatory) => Some(&signatory.actor_uid),
            Self::Institutional { .. } => None,
        }
    }
}

/// Ce que l'amendement vise dans le texte, tel que publie.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AmendmentTarget {
    /// « ARTICLE 3 », « APRÈS L'ARTICLE 12 » — libelle de la source.
    title: String,
    kind: Option<String>,
}

impl AmendmentTarget {
    pub fn new(title: String, kind: Option<String>) -> Result<Self, AmendmentError> {
        let title = clean(&title);
        if title.is_empty() {
            return Err(AmendmentError::EmptyTargetTitle);
        }
        Ok(Self { title, kind })
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn kind(&self) -> Option<&str> {
        self.kind.as_deref()
    }

    /// Article que le titre nomme, quand il en nomme un.
    pub fn article(&self) -> Option<super::article::ArticleNumber> {
        super::article::ArticleNumber::in_text(&self.title)
    }
}

// ---------------------------------------------------------------------------
// Agregat
// ---------------------------------------------------------------------------

/// Ce qu'une resolution de groupes dates a produit, pour le journal
/// d'ingestion. Les lacunes y figurent: un signataire sans groupe doit se voir
/// (README.md §2).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GroupResolutionReport {
    /// Groupes que la source nommait deja.
    pub published: usize,
    /// Groupes reconstitues a la date de depot.
    pub resolved: usize,
    /// Signataires sans groupe: acteur absent du referentiel, ou aucune
    /// appartenance couvrant la date de depot.
    pub unresolved: usize,
    /// Deux groupes concurrents a la date de depot.
    pub ambiguous: usize,
    /// Amendement sans date de depot: aucun groupe date n'est calculable, et on
    /// ne se rabat pas sur l'appartenance courante.
    pub undated: usize,
}

impl GroupResolutionReport {
    pub fn merge(&mut self, other: Self) {
        self.published += other.published;
        self.resolved += other.resolved;
        self.unresolved += other.unresolved;
        self.ambiguous += other.ambiguous;
        self.undated += other.undated;
    }
}

/// Elements d'un amendement, tels que lus dans la source.
///
/// Une struct plutot que quinze parametres positionnels: le champ est nomme au
/// site d'appel, et un oubli se voit a la compilation.
#[derive(Debug, Clone)]
pub struct NewAmendment {
    pub uid: AmendmentUid,
    pub legislature: u16,
    pub number: AmendmentNumber,
    pub text_ref: Option<LegislativeTextRef>,
    /// Lieu d'examen publie (organe de commission, seance).
    pub examination_ref: Option<String>,
    pub target: AmendmentTarget,
    pub author: Option<Author>,
    pub cosignatories: Vec<Signatory>,
    /// Expose sommaire, verbatim. `None` quand la source n'en publie pas.
    pub summary: Option<String>,
    pub fate: AmendmentFate,
    /// Etat de traitement publie, distinct du sort (« En traitement »).
    pub state_label: Option<String>,
    pub deposited_on: Option<NaiveDate>,
    /// Amendement dont celui-ci est un sous-amendement.
    pub parent_uid: Option<AmendmentUid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Amendment {
    uid: AmendmentUid,
    legislature: u16,
    number: AmendmentNumber,
    text_ref: Option<LegislativeTextRef>,
    examination_ref: Option<String>,
    target: AmendmentTarget,
    author: Author,
    cosignatories: Vec<Signatory>,
    summary: Option<String>,
    fate: AmendmentFate,
    state_label: Option<String>,
    deposited_on: Option<NaiveDate>,
    parent_uid: Option<AmendmentUid>,
}

impl Amendment {
    /// Construit l'agregat et applique les invariants.
    ///
    /// Deux nettoyages, et un seul motif pour chacun: l'auteur est retire de la
    /// liste des cosignataires (la source l'y fait parfois figurer, et il
    /// apparaitrait deux fois a l'ecran), et un cosignataire repete est reduit a
    /// sa premiere occurrence. Aucun tri: l'ordre publie est conserve, un tri
    /// alphabetique ou par groupe serait un classement (README.md §6).
    pub fn new(parts: NewAmendment) -> Result<Self, AmendmentError> {
        let author = parts.author.ok_or(AmendmentError::MissingAuthor)?;
        if let Author::Institutional { label } = &author {
            if label.trim().is_empty() {
                return Err(AmendmentError::EmptyInstitutionalAuthor);
            }
        }

        let mut cosignatories: Vec<Signatory> = Vec::with_capacity(parts.cosignatories.len());
        for signatory in parts.cosignatories {
            if author.actor_uid() == Some(&signatory.actor_uid) {
                continue;
            }
            if cosignatories
                .iter()
                .any(|kept| kept.actor_uid == signatory.actor_uid)
            {
                continue;
            }
            cosignatories.push(Signatory {
                role: SignatoryRole::Cosignatory,
                ..signatory
            });
        }

        Ok(Self {
            uid: parts.uid,
            legislature: parts.legislature,
            number: parts.number,
            text_ref: parts.text_ref,
            examination_ref: parts.examination_ref,
            target: parts.target,
            author,
            cosignatories,
            summary: parts.summary.filter(|s| !s.trim().is_empty()),
            fate: parts.fate,
            state_label: parts.state_label,
            deposited_on: parts.deposited_on,
            parent_uid: parts.parent_uid,
        })
    }

    pub fn uid(&self) -> &AmendmentUid {
        &self.uid
    }

    pub fn legislature(&self) -> u16 {
        self.legislature
    }

    pub fn number(&self) -> &AmendmentNumber {
        &self.number
    }

    pub fn text_ref(&self) -> Option<&LegislativeTextRef> {
        self.text_ref.as_ref()
    }

    pub fn examination_ref(&self) -> Option<&str> {
        self.examination_ref.as_deref()
    }

    pub fn target(&self) -> &AmendmentTarget {
        &self.target
    }

    pub fn author(&self) -> &Author {
        &self.author
    }

    pub fn cosignatories(&self) -> &[Signatory] {
        &self.cosignatories
    }

    /// Expose sommaire, verbatim. Le site ne le resume ni ne l'abrege (RM-03).
    pub fn summary(&self) -> Option<&str> {
        self.summary.as_deref()
    }

    pub fn fate(&self) -> &AmendmentFate {
        &self.fate
    }

    pub fn state_label(&self) -> Option<&str> {
        self.state_label.as_deref()
    }

    pub fn deposited_on(&self) -> Option<NaiveDate> {
        self.deposited_on
    }

    pub fn parent_uid(&self) -> Option<&AmendmentUid> {
        self.parent_uid.as_ref()
    }

    /// Acteurs cites par l'amendement, auteur en tete.
    ///
    /// L'appelant charge le referentiel pour ces seuls acteurs, comme le fait
    /// deja `RefreshScrutins` pour les positions nominales.
    pub fn signatory_uids(&self) -> Vec<ActorUid> {
        let mut uids = Vec::with_capacity(self.cosignatories.len() + 1);
        if let Some(uid) = self.author.actor_uid() {
            uids.push(uid.clone());
        }
        uids.extend(self.cosignatories.iter().map(|s| s.actor_uid.clone()));
        uids
    }

    /// Pose sur chaque signataire le groupe qu'il avait **a la date de depot**
    /// (README.md §3.2).
    ///
    /// Sans date de depot, la resolution n'a pas lieu: se rabattre sur
    /// l'appartenance courante reecrirait l'histoire, ce que la charte interdit
    /// explicitement. Un groupe deja publie par la source est conserve: il est
    /// date par construction.
    pub fn resolve_signatory_groups(
        &mut self,
        directory: &ActorDirectory,
    ) -> GroupResolutionReport {
        let mut report = GroupResolutionReport::default();

        let Some(date) = self.deposited_on else {
            report.undated =
                self.cosignatories.len() + usize::from(matches!(self.author, Author::Deputy(_)));
            return report;
        };

        if let Author::Deputy(signatory) = &mut self.author {
            resolve_one(signatory, directory, date, &mut report);
        }
        for signatory in &mut self.cosignatories {
            resolve_one(signatory, directory, date, &mut report);
        }

        report
    }
}

fn resolve_one(
    signatory: &mut Signatory,
    directory: &ActorDirectory,
    date: NaiveDate,
    report: &mut GroupResolutionReport,
) {
    if signatory.group_origin == GroupOrigin::Published {
        report.published += 1;
        return;
    }

    match directory.resolve_at(&signatory.actor_uid, date) {
        Some(at_date) => {
            if at_date.ambiguous {
                signatory.group_uid = None;
                signatory.group_origin = GroupOrigin::Unknown;
                signatory.group_ambiguous = true;
                report.ambiguous += 1;
                return;
            }
            match at_date.group {
                Some(group) => {
                    signatory.group_uid = Some(group.uid().clone());
                    signatory.group_origin = GroupOrigin::ResolvedAtDeposit;
                    report.resolved += 1;
                }
                None => {
                    signatory.group_uid = None;
                    signatory.group_origin = GroupOrigin::Unknown;
                    report.unresolved += 1;
                }
            }
        }
        None => {
            signatory.group_uid = None;
            signatory.group_origin = GroupOrigin::Unknown;
            report.unresolved += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// Rapprochement avec l'objet d'un scrutin
// ---------------------------------------------------------------------------

/// Numeros d'amendement cites par l'objet d'un scrutin.
///
/// « l'amendement n° 123 de M. X à l'article 3 du projet de loi Y » → `["123"]`
/// « les amendements identiques n° 12, 45 et 78 de … »             → `["12","45","78"]`
/// « l'article 12 du projet de loi Y »                              → `[]`
///
/// Regle deterministe et rejouable, sans modele (README.md §5, §9). Elle ne
/// produit pas le rattachement: elle rend ce que l'objet **cite**. C'est
/// l'appelant qui decide, et qui abandonne des qu'un numero designe autre chose
/// qu'un amendement unique.
///
/// Le texte porteur s'obtient a part, par `DebatedText::from_subject`, qui jette
/// justement cette portion de l'objet.
///
/// Les mentions de rectification (« 45 rectifié », « 45 rect. ») sont ignorees:
/// un amendement rectifie garde son numero, la mention designe une version du
/// meme amendement.
pub fn amendment_numbers_in_subject(subject: &str) -> Vec<AmendmentNumber> {
    let cleaned = clean(subject);
    let lower = lowercase(&cleaned);

    let Some(marker) = lower.find("amendement") else {
        return Vec::new();
    };
    let tail = &lower[marker..];

    let Some(numbers_at) = number_marker_end(tail) else {
        return Vec::new();
    };

    let mut numbers = Vec::new();
    for token in tail[numbers_at..].split_whitespace() {
        let token = token.trim_matches(|c: char| !c.is_alphanumeric());
        if token.is_empty() {
            continue;
        }
        let digits = token
            .strip_prefix("ii-")
            .or_else(|| token.strip_prefix("i-"))
            .unwrap_or(token);
        if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(number) = AmendmentNumber::new(token.to_string()) {
                if !numbers.contains(&number) {
                    numbers.push(number);
                }
            }
            continue;
        }
        if is_number_run_filler(token) {
            continue;
        }
        break;
    }

    numbers
}

/// Fin du marqueur de numerotation (« n° », « no ») qui ouvre la suite de
/// numeros, ou `None` quand l'objet n'en porte pas.
fn number_marker_end(haystack: &str) -> Option<usize> {
    ["n°", "n °", "no "]
        .iter()
        .filter_map(|marker| haystack.find(marker).map(|at| at + marker.len()))
        .min()
}

/// Mots qui prolongent une suite de numeros sans y mettre fin.
fn is_number_run_filler(token: &str) -> bool {
    matches!(token, "et" | "n" | "no" | "rect" | "bis" | "ter")
        || token.starts_with("rectifi")
        // « (2e rect.) », « (3e rect.) »
        || (token.len() <= 3 && token.ends_with('e') && token[..token.len() - 1]
            .chars()
            .all(|c| c.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::actor::{
        Actor, ActorRole, GroupMembership, MembershipPeriod, MembershipQuality, ParliamentaryGroup,
    };

    fn uid(raw: &str) -> AmendmentUid {
        AmendmentUid::new(raw.to_string()).unwrap()
    }

    fn actor_uid(raw: &str) -> ActorUid {
        ActorUid::new(raw.to_string()).unwrap()
    }

    fn group_uid(raw: &str) -> GroupUid {
        GroupUid::new(raw.to_string()).unwrap()
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn deputy_author(actor: &str) -> Author {
        Author::Deputy(Signatory::new(
            actor_uid(actor),
            SignatoryRole::Author,
            0,
            None,
        ))
    }

    fn parts(author: Author, cosignatories: Vec<Signatory>) -> NewAmendment {
        NewAmendment {
            uid: uid("AMANR5L17PO838901BTC0633P0D1N000078"),
            legislature: 17,
            number: AmendmentNumber::new("78".to_string()).unwrap(),
            text_ref: Some(LegislativeTextRef::new("PRJLANR5L17B0324".to_string()).unwrap()),
            examination_ref: None,
            target: AmendmentTarget::new("ARTICLE 3".to_string(), None).unwrap(),
            author: Some(author),
            cosignatories,
            summary: Some("Cet amendement vise à …".to_string()),
            fate: AmendmentFate::from_source(Some("Adopté")),
            state_label: None,
            deposited_on: Some(date(2025, 10, 14)),
            parent_uid: None,
        }
    }

    fn cosignatory(actor: &str, rank: u16) -> Signatory {
        Signatory::new(actor_uid(actor), SignatoryRole::Cosignatory, rank, None)
    }

    #[test]
    fn an_empty_uid_or_number_is_refused() {
        assert_eq!(
            AmendmentUid::new(String::new()).unwrap_err(),
            AmendmentError::EmptyAmendmentUid
        );
        assert_eq!(
            AmendmentUid::new("   ".to_string()).unwrap_err(),
            AmendmentError::EmptyAmendmentUid
        );
        assert_eq!(
            AmendmentNumber::new(String::new()).unwrap_err(),
            AmendmentError::EmptyAmendmentNumber
        );
        assert_eq!(
            LegislativeTextRef::new("  ".to_string()).unwrap_err(),
            AmendmentError::EmptyTextRef
        );
    }

    #[test]
    fn a_signatory_with_a_published_group_is_dated_by_the_source() {
        let signatory = Signatory::new(
            actor_uid("PA1"),
            SignatoryRole::Author,
            0,
            Some(group_uid("PO1")),
        );

        assert_eq!(signatory.group_origin(), GroupOrigin::Published);
        assert_eq!(signatory.group_uid(), Some(&group_uid("PO1")));
        assert!(!signatory.group_ambiguous());
    }

    #[test]
    fn a_signatory_without_a_published_group_has_an_unknown_origin() {
        let signatory = Signatory::new(actor_uid("PA2"), SignatoryRole::Cosignatory, 4, None);

        assert_eq!(signatory.group_origin(), GroupOrigin::Unknown);
        assert_eq!(signatory.group_uid(), None);
        assert!(!signatory.group_ambiguous());
    }

    #[test]
    fn a_signatory_exposes_the_actor_role_and_rank_it_was_built_with() {
        let signatory = Signatory::new(actor_uid("PA2"), SignatoryRole::Cosignatory, 4, None);

        assert_eq!(signatory.actor_uid(), &actor_uid("PA2"));
        assert_eq!(signatory.role(), SignatoryRole::Cosignatory);
        assert_eq!(signatory.rank(), 4);
    }

    #[test]
    fn a_target_with_a_blank_title_is_refused() {
        assert_eq!(
            AmendmentTarget::new("   ".to_string(), None).unwrap_err(),
            AmendmentError::EmptyTargetTitle
        );
        assert_eq!(
            AmendmentTarget::new(String::new(), Some("ARTICLE".to_string())).unwrap_err(),
            AmendmentError::EmptyTargetTitle
        );
    }

    #[test]
    fn a_target_title_is_cleaned_of_superfluous_spaces() {
        let target = AmendmentTarget::new(
            "  APRÈS   L'ARTICLE\u{00A0}12 ".to_string(),
            Some("ARTICLE_ADD".to_string()),
        )
        .unwrap();

        assert_eq!(target.title(), "APRÈS L'ARTICLE 12");
        assert_eq!(target.kind(), Some("ARTICLE_ADD"));
    }

    #[test]
    fn an_amendment_without_author_is_refused() {
        let mut parts = parts(deputy_author("PA1"), Vec::new());
        parts.author = None;
        assert!(matches!(
            Amendment::new(parts),
            Err(AmendmentError::MissingAuthor)
        ));
    }

    #[test]
    fn an_institutional_author_needs_a_label() {
        let mut parts = parts(deputy_author("PA1"), Vec::new());
        parts.author = Some(Author::Institutional {
            label: "  ".to_string(),
        });
        assert!(matches!(
            Amendment::new(parts),
            Err(AmendmentError::EmptyInstitutionalAuthor)
        ));
    }

    #[test]
    fn the_author_never_appears_among_the_cosignatories() {
        let amendment = Amendment::new(parts(
            deputy_author("PA1"),
            vec![cosignatory("PA1", 1), cosignatory("PA2", 2)],
        ))
        .unwrap();

        assert_eq!(amendment.cosignatories().len(), 1);
        assert_eq!(amendment.cosignatories()[0].actor_uid, actor_uid("PA2"));
    }

    #[test]
    fn a_repeated_cosignatory_is_kept_once_in_published_order() {
        let amendment = Amendment::new(parts(
            deputy_author("PA1"),
            vec![
                cosignatory("PA5", 1),
                cosignatory("PA2", 2),
                cosignatory("PA5", 3),
            ],
        ))
        .unwrap();

        let order: Vec<&str> = amendment
            .cosignatories()
            .iter()
            .map(|s| s.actor_uid.as_str())
            .collect();
        assert_eq!(order, vec!["PA5", "PA2"]);
    }

    #[test]
    fn an_empty_summary_is_stored_as_absent() {
        let mut parts = parts(deputy_author("PA1"), Vec::new());
        parts.summary = Some("   ".to_string());
        assert_eq!(Amendment::new(parts).unwrap().summary(), None);
    }

    #[test]
    fn a_published_fate_keeps_its_label() {
        let fate = AmendmentFate::from_source(Some("Adopté"));
        assert_eq!(fate.code(), FateCode::Adopted);
        assert_eq!(fate.label(), "Adopté");
    }

    #[test]
    fn known_fates_map_to_their_code() {
        let cases = [
            ("Adopté", FateCode::Adopted),
            ("Rejeté", FateCode::Rejected),
            ("Retiré", FateCode::Withdrawn),
            ("Retiré avant séance", FateCode::Withdrawn),
            ("Tombé", FateCode::Fallen),
            ("Non soutenu", FateCode::NotSupported),
            ("Irrecevable", FateCode::Inadmissible),
            ("Irrecevable art. 40", FateCode::Inadmissible),
            ("Non discuté", FateCode::NotDiscussed),
        ];
        for (label, expected) in cases {
            assert_eq!(
                AmendmentFate::from_source(Some(label)).code(),
                expected,
                "{label}"
            );
        }
    }

    #[test]
    fn an_unknown_fate_is_marked_and_keeps_its_label_intact() {
        let fate = AmendmentFate::from_source(Some("Réservé jusqu'au vote"));
        assert_eq!(fate.code(), FateCode::Other);
        assert!(fate.is_unknown());
        assert_eq!(fate.label(), "Réservé jusqu'au vote");
    }

    #[test]
    fn an_absent_fate_is_unspecified_not_rejected() {
        assert_eq!(
            AmendmentFate::from_source(None).code(),
            FateCode::Unspecified
        );
        assert_eq!(
            AmendmentFate::from_source(Some("  ")).code(),
            FateCode::Unspecified
        );
    }

    // -----------------------------------------------------------------------
    // Groupes dates
    // -----------------------------------------------------------------------

    /// Un depute qui change de groupe en cours de legislature: c'est le cas que
    /// README.md §3.2 vise nommement.
    fn directory_with_a_switcher() -> ActorDirectory {
        let actor = Actor::new(
            actor_uid("PA1"),
            Some("M.".to_string()),
            "Jean".to_string(),
            "Dupont".to_string(),
            ActorRole::Deputy,
        )
        .unwrap();
        let old = ParliamentaryGroup::new(
            group_uid("PO100"),
            17,
            "Ancien groupe".to_string(),
            "AG".to_string(),
            None,
            None,
            None,
        )
        .unwrap();
        let new = ParliamentaryGroup::new(
            group_uid("PO200"),
            17,
            "Nouveau groupe".to_string(),
            "NG".to_string(),
            None,
            None,
            None,
        )
        .unwrap();

        let quality = MembershipQuality::new("Membre".to_string()).unwrap();
        let before = GroupMembership::new(
            "PM1".to_string(),
            actor_uid("PA1"),
            group_uid("PO100"),
            17,
            MembershipPeriod::new(date(2024, 7, 18), Some(date(2025, 6, 30))).unwrap(),
            quality.clone(),
        );
        let after = GroupMembership::new(
            "PM2".to_string(),
            actor_uid("PA1"),
            group_uid("PO200"),
            17,
            MembershipPeriod::new(date(2025, 7, 1), None).unwrap(),
            quality,
        );

        ActorDirectory::new(vec![actor], vec![old, new], vec![before, after])
    }

    #[test]
    fn the_group_is_the_one_held_on_the_deposit_date_not_the_current_one() {
        let directory = directory_with_a_switcher();
        let mut parts = parts(deputy_author("PA1"), Vec::new());
        parts.deposited_on = Some(date(2025, 3, 12));

        let mut amendment = Amendment::new(parts).unwrap();
        let report = amendment.resolve_signatory_groups(&directory);

        let Author::Deputy(author) = amendment.author() else {
            panic!("expected a deputy author");
        };
        assert_eq!(author.group_uid, Some(group_uid("PO100")));
        assert_eq!(author.group_origin, GroupOrigin::ResolvedAtDeposit);
        assert_eq!(report.resolved, 1);
    }

    #[test]
    fn a_group_published_by_the_source_is_kept_as_is() {
        let directory = directory_with_a_switcher();
        let author = Author::Deputy(Signatory::new(
            actor_uid("PA1"),
            SignatoryRole::Author,
            0,
            Some(group_uid("PO999")),
        ));
        let mut parts = parts(author, Vec::new());
        parts.deposited_on = Some(date(2025, 3, 12));

        let mut amendment = Amendment::new(parts).unwrap();
        let report = amendment.resolve_signatory_groups(&directory);

        let Author::Deputy(author) = amendment.author() else {
            panic!("expected a deputy author");
        };
        assert_eq!(author.group_uid, Some(group_uid("PO999")));
        assert_eq!(author.group_origin, GroupOrigin::Published);
        assert_eq!(report.published, 1);
        assert_eq!(report.resolved, 0);
    }

    #[test]
    fn an_actor_absent_from_the_registry_gets_no_group() {
        let directory = directory_with_a_switcher();
        let mut amendment = Amendment::new(parts(deputy_author("PA404"), Vec::new())).unwrap();
        let report = amendment.resolve_signatory_groups(&directory);

        let Author::Deputy(author) = amendment.author() else {
            panic!("expected a deputy author");
        };
        assert_eq!(author.group_uid, None);
        assert_eq!(author.group_origin, GroupOrigin::Unknown);
        assert!(!author.group_ambiguous);
        assert_eq!(report.unresolved, 1);
    }

    #[test]
    fn two_competing_groups_on_the_deposit_date_yield_no_group_at_all() {
        let actor = Actor::new(
            actor_uid("PA1"),
            None,
            "Jean".to_string(),
            "Dupont".to_string(),
            ActorRole::Deputy,
        )
        .unwrap();
        let first = ParliamentaryGroup::new(
            group_uid("PO100"),
            17,
            "Groupe A".to_string(),
            "A".to_string(),
            None,
            None,
            None,
        )
        .unwrap();
        let second = ParliamentaryGroup::new(
            group_uid("PO200"),
            17,
            "Groupe B".to_string(),
            "B".to_string(),
            None,
            None,
            None,
        )
        .unwrap();
        let quality = MembershipQuality::new("Membre".to_string()).unwrap();
        let overlapping = vec![
            GroupMembership::new(
                "PM1".to_string(),
                actor_uid("PA1"),
                group_uid("PO100"),
                17,
                MembershipPeriod::new(date(2024, 7, 18), None).unwrap(),
                quality.clone(),
            ),
            GroupMembership::new(
                "PM2".to_string(),
                actor_uid("PA1"),
                group_uid("PO200"),
                17,
                MembershipPeriod::new(date(2024, 7, 18), None).unwrap(),
                quality,
            ),
        ];
        let directory = ActorDirectory::new(vec![actor], vec![first, second], overlapping);

        let mut amendment = Amendment::new(parts(deputy_author("PA1"), Vec::new())).unwrap();
        let report = amendment.resolve_signatory_groups(&directory);

        let Author::Deputy(author) = amendment.author() else {
            panic!("expected a deputy author");
        };
        assert_eq!(author.group_uid, None);
        assert!(author.group_ambiguous);
        assert_eq!(report.ambiguous, 1);
    }

    #[test]
    fn without_a_deposit_date_no_group_is_resolved_at_all() {
        let directory = directory_with_a_switcher();
        let mut parts = parts(deputy_author("PA1"), vec![cosignatory("PA2", 1)]);
        parts.deposited_on = None;

        let mut amendment = Amendment::new(parts).unwrap();
        let report = amendment.resolve_signatory_groups(&directory);

        let Author::Deputy(author) = amendment.author() else {
            panic!("expected a deputy author");
        };
        assert_eq!(author.group_uid, None);
        assert_eq!(report.undated, 2);
        assert_eq!(report.resolved, 0);
    }

    #[test]
    fn the_registry_is_queried_for_the_cited_actors_only() {
        let amendment = Amendment::new(parts(
            deputy_author("PA1"),
            vec![cosignatory("PA2", 1), cosignatory("PA3", 2)],
        ))
        .unwrap();

        let uids: Vec<String> = amendment
            .signatory_uids()
            .iter()
            .map(|uid| uid.as_str().to_string())
            .collect();
        assert_eq!(uids, vec!["PA1", "PA2", "PA3"]);
    }

    #[test]
    fn an_institutional_author_cites_no_actor() {
        let mut parts = parts(deputy_author("PA1"), Vec::new());
        parts.author = Some(Author::Institutional {
            label: "Gouvernement".to_string(),
        });
        let amendment = Amendment::new(parts).unwrap();
        assert!(amendment.signatory_uids().is_empty());
    }

    // -----------------------------------------------------------------------
    // Numeros cites par l'objet d'un scrutin
    // -----------------------------------------------------------------------

    fn numbers(subject: &str) -> Vec<String> {
        amendment_numbers_in_subject(subject)
            .iter()
            .map(|n| n.as_str().to_string())
            .collect()
    }

    #[test]
    fn a_single_amendment_is_read_from_the_subject() {
        assert_eq!(
            numbers(
                "l'amendement n° 123 de M. Tanguy à l'article 3 du projet de loi \
                 de finances pour 2026 (première lecture)."
            ),
            vec!["123"]
        );
    }

    #[test]
    fn identical_amendments_yield_every_number_cited() {
        assert_eq!(
            numbers(
                "les amendements identiques n° 12, 45 et 78 de Mme Rousseau à \
                 l'article 7 du projet de loi de financement de la sécurité sociale."
            ),
            vec!["12", "45", "78"]
        );
    }

    #[test]
    fn a_rectified_amendment_keeps_its_number() {
        assert_eq!(
            numbers("l'amendement n° 45 rectifié de M. Dupont à l'article 2 du projet de loi."),
            vec!["45"]
        );
        assert_eq!(
            numbers("l'amendement n° 45 (2e rect.) de M. Dupont à l'article 2."),
            vec!["45"]
        );
    }

    #[test]
    fn the_article_number_that_follows_is_never_taken_for_an_amendment() {
        assert_eq!(
            numbers("l'amendement n° 9 de M. X à l'article 35 du projet de loi."),
            vec!["9"]
        );
    }

    #[test]
    fn a_subject_that_names_no_amendment_yields_nothing() {
        assert!(numbers(
            "l'article 12 du projet de loi de financement de la sécurité sociale pour 2026."
        )
        .is_empty());
        assert!(
            numbers("la motion de rejet préalable, déposée par Mme Panot, du projet de loi.")
                .is_empty()
        );
    }

    #[test]
    fn an_amendment_mentioned_without_a_number_yields_nothing() {
        assert!(numbers("l'ensemble des amendements de suppression de l'article 3.").is_empty());
    }

    #[test]
    fn the_normalised_key_ignores_case_and_punctuation() {
        assert_eq!(
            AmendmentNumber::new("45 rect.".to_string()).unwrap().key(),
            "45rect"
        );
        assert_eq!(
            AmendmentNumber::new("CF120".to_string()).unwrap().key(),
            "cf120"
        );
    }

    // -----------------------------------------------------------------------
    // Numero de reference
    // -----------------------------------------------------------------------

    fn reference_of(raw: &str) -> Option<ReferenceNumber> {
        AmendmentNumber::new(raw.to_string()).unwrap().reference()
    }

    fn number_of(raw: &str) -> AmendmentNumber {
        AmendmentNumber::new(raw.to_string()).unwrap()
    }

    fn references(subject: &str) -> Vec<Option<ReferenceNumber>> {
        amendment_numbers_in_subject(subject)
            .iter()
            .map(AmendmentNumber::reference)
            .collect()
    }

    #[test]
    fn should_drop_finance_part_from_reference_number_when_number_is_prefixed() {
        assert_eq!(
            reference_of("I-1376"),
            Some(ReferenceNumber::new(Some(ReferencePrefix::First), 1376))
        );
        assert_eq!(
            reference_of("II-243"),
            Some(ReferenceNumber::new(Some(ReferencePrefix::Second), 243))
        );
    }

    #[test]
    fn should_drop_rectification_from_reference_number_when_number_is_rectified() {
        let expected = Some(ReferenceNumber::new(None, 45));
        assert_eq!(reference_of("45 rect."), expected);
        assert_eq!(reference_of("45 rectifié"), expected);
    }

    #[test]
    fn should_keep_commission_prefix_in_reference_number_when_number_is_a_commission_one() {
        assert_eq!(
            reference_of("CS1376"),
            Some(ReferenceNumber::new(
                Some(ReferencePrefix::Commission("cs".to_string())),
                1376
            ))
        );
        assert!(!number_of("CS1376").answers(&number_of("1376")));
    }

    #[test]
    fn should_answer_cited_number_whatever_its_case_when_stored_number_is_upper_case() {
        assert!(number_of("II-243").answers(&number_of("ii-243")));
    }

    #[test]
    fn should_keep_cited_finance_part_when_subject_cites_a_prefixed_number() {
        assert_eq!(
            references("l'amendement n° II-243 de M. X après l'article 15 du projet de loi."),
            vec![Some(ReferenceNumber::new(Some(ReferencePrefix::Second), 243))]
        );
    }

    #[test]
    fn should_keep_reading_number_run_when_it_holds_finance_parts() {
        assert_eq!(
            references("les amendements identiques n° I-12 et II-45 de M. X à l'article 3."),
            vec![
                Some(ReferenceNumber::new(Some(ReferencePrefix::First), 12)),
                Some(ReferenceNumber::new(Some(ReferencePrefix::Second), 45)),
            ]
        );
    }

    #[test]
    fn should_read_target_article_when_target_title_names_one() {
        let target = AmendmentTarget::new("APRÈS L'ARTICLE 12".to_string(), None).unwrap();
        assert_eq!(
            target.article().map(|article| article.to_string()),
            Some("12".to_string())
        );
    }

    // -----------------------------------------------------------------------
    // Apercu et bandeau de sort
    // -----------------------------------------------------------------------

    fn shown(text: impl Into<String>, truncated: bool) -> SummaryPreview {
        SummaryPreview::Text {
            text: text.into(),
            truncated,
        }
    }

    #[test]
    fn should_show_whole_summary_without_ellipsis_when_it_has_300_characters_or_less() {
        let exactly = "a".repeat(300);
        assert_eq!(
            SummaryPreview::of(Some(&exactly)),
            shown(exactly.clone(), false)
        );
        assert_eq!(
            SummaryPreview::of(Some("Supprime l'article.")),
            shown("Supprime l'article.", false)
        );
    }

    #[test]
    fn should_cut_summary_at_300_characters_mid_word_and_mark_ellipsis_when_it_is_longer() {
        let long = format!("{}{}", "a".repeat(298), "bcdef");
        assert_eq!(
            SummaryPreview::of(Some(&long)),
            shown(format!("{}bc\u{2026}", "a".repeat(298)), true)
        );
    }

    #[test]
    fn should_count_characters_not_bytes_when_summary_has_accents() {
        let accented = format!("{}ZZ", "\u{00e9}".repeat(299));
        assert_eq!(
            SummaryPreview::of(Some(&accented)),
            shown(format!("{}Z\u{2026}", "\u{00e9}".repeat(299)), true)
        );
    }

    #[test]
    fn should_mark_summary_unpublished_when_amendment_has_none() {
        assert_eq!(SummaryPreview::of(None), SummaryPreview::Unpublished);
    }

    #[test]
    fn should_announce_unpublished_fate_with_published_state_when_source_gives_no_fate() {
        let banner = FateBanner::of(FateCode::Unspecified, "", Some("D\u{00e9}pos\u{00e9}"));
        assert_eq!(
            banner.label(),
            "Sort non publi\u{00e9} (\u{00e9}tat : D\u{00e9}pos\u{00e9})"
        );
    }
}
