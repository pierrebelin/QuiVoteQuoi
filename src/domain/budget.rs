//! Budget de l'État : crédits de paiement (CP) votés en loi de finances initiale.
//!
//! Un exercice porte la loi qui l'a adopté, le fichier officiel dont il est tiré,
//! et ses missions, elles-mêmes faites de programmes. Le montant d'une mission
//! n'est jamais saisi : c'est la somme de ses programmes, comme le total du
//! budget général est la somme de ses missions. Une seule vérité par chiffre.
//!
//! Aucun chiffre n'est produit par un modèle (README.md §8) et aucun n'est
//! corrigé pour tomber juste : `BudgetExercise::verify` compare l'exercice à des
//! attendus tirés de la loi (article de crédits, états B et D) et refuse
//! l'import au moindre écart. Chaque mission est rattachée à un sujet du site
//! par `MISSION_FAMILIES` ; une mission sans sujet bloque l'import jusqu'à
//! décision, plutôt que d'être rangée au jugé (README.md §5).

use std::collections::HashSet;

use chrono::NaiveDate;

use super::theme::FamilyCode;

/// Première loi de finances votée sous la LOLF : avant, la nomenclature en
/// missions et programmes n'existe pas.
pub const MIN_YEAR: u16 = 2006;
/// Borne haute volontairement lâche : elle écarte une saisie absurde, pas une
/// année future plausible.
pub const MAX_YEAR: u16 = 2100;
/// Les numéros de programme ont trois chiffres (101, 841...). La borne laisse de
/// la marge sans accepter une cellule mal lue.
pub const MAX_PROGRAMME_NUMBER: u16 = 9_999;

/// Mission du budget général qui porte les remboursements et dégrèvements
/// d'impôts : une part importante du total, que la page isole.
pub const REFUNDS_AND_REBATES_MISSION: &str = "Remboursements et dégrèvements";

/// Rattachement des missions aux sujets du site, validé le 16/11/2026 pour
/// les exercices 2025 et 2026 (missions de 2025 disparues ou renommées en 2026
/// comprises).
///
/// Les noms sont ceux de la loi de finances ; la recherche passe par
/// `normalize_name` (apostrophes, espaces, casse). Une mission nouvelle d'un
/// exercice à l'autre n'est pas rattachée d'office : `verify` refuse l'import
/// tant que cette liste n'a pas été complétée.
pub const MISSION_FAMILIES: &[(&str, FamilyCode)] = &[
    (
        "Remboursements et dégrèvements",
        FamilyCode::EconomieFinancesPubliques,
    ),
    (
        "Engagements financiers de l'État",
        FamilyCode::EconomieFinancesPubliques,
    ),
    (
        "Gestion des finances publiques",
        FamilyCode::EconomieFinancesPubliques,
    ),
    (
        "Investir pour la France de 2030",
        FamilyCode::EconomieFinancesPubliques,
    ),
    ("Économie", FamilyCode::EconomieFinancesPubliques),
    (
        "Crédits non répartis",
        FamilyCode::EconomieFinancesPubliques,
    ),
    ("Plan de relance", FamilyCode::EconomieFinancesPubliques),
    ("Enseignement scolaire", FamilyCode::EducationCulture),
    (
        "Recherche et enseignement supérieur",
        FamilyCode::EducationCulture,
    ),
    ("Culture", FamilyCode::EducationCulture),
    (
        "Médias, livre et industries culturelles",
        FamilyCode::EducationCulture,
    ),
    (
        "Avances à l'audiovisuel public",
        FamilyCode::EducationCulture,
    ),
    ("Audiovisuel public", FamilyCode::EducationCulture),
    ("Défense", FamilyCode::InternationalDefense),
    (
        "Aide publique au développement",
        FamilyCode::InternationalDefense,
    ),
    (
        "Action extérieure de l'État",
        FamilyCode::InternationalDefense,
    ),
    (
        "Monde combattant, mémoire et liens avec la Nation",
        FamilyCode::InternationalDefense,
    ),
    (
        "Anciens combattants, mémoire et liens avec la Nation",
        FamilyCode::InternationalDefense,
    ),
    (
        "Solidarité, insertion et égalité des chances",
        FamilyCode::ProtectionSocialeRetraites,
    ),
    (
        "Régimes sociaux et de retraite",
        FamilyCode::ProtectionSocialeRetraites,
    ),
    ("Sécurités", FamilyCode::JusticeSecurite),
    ("Justice", FamilyCode::JusticeSecurite),
    (
        "Écologie, développement et mobilité durables",
        FamilyCode::EnvironnementEnergie,
    ),
    ("Cohésion des territoires", FamilyCode::Logement),
    (
        "Travail, emploi et administration des ministères sociaux",
        FamilyCode::TravailEmploi,
    ),
    (
        "Administration générale et territoriale de l'État",
        FamilyCode::InstitutionsViePolitique,
    ),
    ("Pouvoirs publics", FamilyCode::InstitutionsViePolitique),
    (
        "Direction de l'action du Gouvernement",
        FamilyCode::InstitutionsViePolitique,
    ),
    (
        "Conseil et contrôle de l'État",
        FamilyCode::InstitutionsViePolitique,
    ),
    (
        "Transformation et fonction publiques",
        FamilyCode::InstitutionsViePolitique,
    ),
    (
        "Agriculture, alimentation, forêt et affaires rurales",
        FamilyCode::AgricultureAlimentation,
    ),
    (
        "Relations avec les collectivités territoriales",
        FamilyCode::CollectivitesOutreMer,
    ),
    ("Outre-mer", FamilyCode::CollectivitesOutreMer),
    ("Immigration, asile et intégration", FamilyCode::Immigration),
    ("Santé", FamilyCode::Sante),
    (
        "Sport, jeunesse et vie associative",
        FamilyCode::SocieteLibertes,
    ),
];

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum BudgetError {
    #[error("amount must not be negative: {0}")]
    NegativeAmount(i64),
    #[error("budget year out of range: {0}")]
    InvalidYear(u16),
    #[error("invalid programme number: {0}")]
    InvalidProgrammeNumber(u16),
    #[error("mission name must not be empty")]
    EmptyMissionName,
    #[error("programme name must not be empty")]
    EmptyProgrammeName,
    #[error("{0} must not be empty")]
    EmptyField(&'static str),
    #[error("action code must not be empty")]
    EmptyActionCode,
    #[error("action name must not be empty")]
    EmptyActionName,
    #[error("unknown spending category: {0}")]
    UnknownCategory(u8),
    #[error("action {action} lists category {category} twice")]
    DuplicateCategory {
        action: ActionCode,
        category: SpendingCategory,
    },
    #[error("programme {programme} lists action {action} twice")]
    DuplicateAction {
        programme: ProgrammeNumber,
        action: ActionCode,
    },
    #[error("programme {programme} totals {expected} euros but its actions total {actual}")]
    ActionTotalMismatch {
        programme: ProgrammeNumber,
        expected: Euros,
        actual: Euros,
    },
    #[error("programme {programme} has {cp} euros but no action")]
    ProgrammeWithoutActions {
        programme: ProgrammeNumber,
        cp: Euros,
    },

    #[error("mission listed twice: {mission}")]
    DuplicateMission { mission: String },
    #[error("mission without any programme: {mission}")]
    EmptyMission { mission: String },
    #[error("programme number listed twice: {number}")]
    DuplicateProgramme { number: ProgrammeNumber },
    #[error("mission without a theme: {mission}")]
    MissionWithoutTheme { mission: String },
    #[error("off-budget account not expected: {mission}")]
    UnexpectedOffBudgetAccount { mission: String },
    #[error("general budget has {actual} missions, expected {expected}")]
    MissionCountMismatch { expected: usize, actual: usize },
    #[error("general budget has {actual} programmes, expected {expected}")]
    ProgrammeCountMismatch { expected: usize, actual: usize },
    #[error("general budget totals {actual} euros of payment credits, expected {expected}")]
    GeneralBudgetTotalMismatch { expected: Euros, actual: Euros },
    #[error("off-budget account missing: {mission}")]
    OffBudgetAccountMissing { mission: String },
    #[error("off-budget account {mission} has {actual} programmes, expected {expected}")]
    OffBudgetProgrammeCountMismatch {
        mission: String,
        expected: usize,
        actual: usize,
    },
    #[error("off-budget account {mission} totals {actual} euros, expected {expected}")]
    OffBudgetTotalMismatch {
        mission: String,
        expected: Euros,
        actual: Euros,
    },
}

/// Montant en euros entiers. Zéro est permis (un programme peut être doté de
/// zéro CP) ; un montant négatif ne l'est pas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Euros(i64);

impl Euros {
    pub const ZERO: Euros = Euros(0);

    pub fn new(amount: i64) -> Result<Self, BudgetError> {
        if amount < 0 {
            return Err(BudgetError::NegativeAmount(amount));
        }
        Ok(Self(amount))
    }

    pub fn get(&self) -> i64 {
        self.0
    }
}

impl std::fmt::Display for Euros {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// La somme sature au lieu de déborder : un budget n'approche pas 9,2 milliards
/// de milliards d'euros, et un total saturé ne passerait de toute façon pas
/// `verify`.
impl std::iter::Sum for Euros {
    fn sum<I: Iterator<Item = Euros>>(iter: I) -> Self {
        iter.fold(Euros::ZERO, |total, amount| {
            Euros(total.0.saturating_add(amount.0))
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BudgetYear(u16);

impl BudgetYear {
    pub fn new(year: u16) -> Result<Self, BudgetError> {
        if !(MIN_YEAR..=MAX_YEAR).contains(&year) {
            return Err(BudgetError::InvalidYear(year));
        }
        Ok(Self(year))
    }

    pub fn get(&self) -> u16 {
        self.0
    }
}

impl std::fmt::Display for BudgetYear {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProgrammeNumber(u16);

impl ProgrammeNumber {
    pub fn new(number: u16) -> Result<Self, BudgetError> {
        if number == 0 || number > MAX_PROGRAMME_NUMBER {
            return Err(BudgetError::InvalidProgrammeNumber(number));
        }
        Ok(Self(number))
    }

    pub fn get(&self) -> u16 {
        self.0
    }
}

impl std::fmt::Display for ProgrammeNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Nature du compte qui porte une mission.
///
/// Le total publié est celui du budget général. Les concours financiers (comptes
/// de concours financiers, dont les avances à l'audiovisuel public) ont leur
/// propre équilibre : ils s'affichent à part et ne s'ajoutent jamais au total.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccountKind {
    GeneralBudget,
    FinancialAssistance,
}

impl AccountKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GeneralBudget => "budget_general",
            Self::FinancialAssistance => "concours_financiers",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        [Self::GeneralBudget, Self::FinancialAssistance]
            .into_iter()
            .find(|kind| kind.as_str() == raw)
    }
}

/// Procédure d'adoption de la loi de finances : voir l'article de crédits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdoptionProcedure {
    Vote,
    Article49_3,
}

impl AdoptionProcedure {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Vote => "vote",
            Self::Article49_3 => "article_49_3",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        [Self::Vote, Self::Article49_3]
            .into_iter()
            .find(|procedure| procedure.as_str() == raw)
    }
}

/// Stade budgétaire. Seule la loi de finances initiale est publiée : ni projet
/// de loi, ni loi de règlement, ni décret d'avance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BudgetStage {
    Lfi,
}

impl BudgetStage {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Lfi => "lfi",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        [Self::Lfi].into_iter().find(|stage| stage.as_str() == raw)
    }
}

/// Loi qui a voté les crédits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawReference {
    pub number: String,
    pub date: NaiveDate,
    /// Article de la loi qui ouvre les crédits (état B).
    pub credits_article: u16,
    pub adoption_procedure: AdoptionProcedure,
    pub legifrance_url: String,
}

/// Fichier officiel dont l'exercice est tiré, avec son empreinte : le même
/// fichier redonne le même exercice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceReference {
    pub title: String,
    pub url: String,
    pub published_on: NaiveDate,
    pub downloaded_on: NaiveDate,
    pub sha256: String,
}

/// Nature d'une dépense : le titre de la LOLF (art. 5, I), premier chiffre de
/// la catégorie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Nature {
    PublicAuthorities,
    Personnel,
    Operating,
    DebtCharges,
    Investment,
    Intervention,
    FinancialOperations,
}

impl Nature {
    pub const ALL: [Nature; 7] = [
        Self::PublicAuthorities,
        Self::Personnel,
        Self::Operating,
        Self::DebtCharges,
        Self::Investment,
        Self::Intervention,
        Self::FinancialOperations,
    ];

    /// Numéro du titre, de 1 à 7.
    pub fn title(&self) -> u8 {
        match self {
            Self::PublicAuthorities => 1,
            Self::Personnel => 2,
            Self::Operating => 3,
            Self::DebtCharges => 4,
            Self::Investment => 5,
            Self::Intervention => 6,
            Self::FinancialOperations => 7,
        }
    }

    pub fn from_title(title: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|nature| nature.title() == title)
    }

    /// Libellé du titre dans la LOLF, art. 5, I.
    pub fn official_label(&self) -> &'static str {
        match self {
            Self::PublicAuthorities => "Dotations des pouvoirs publics",
            Self::Personnel => "Dépenses de personnel",
            Self::Operating => "Dépenses de fonctionnement",
            Self::DebtCharges => "Charges de la dette de l'État",
            Self::Investment => "Dépenses d'investissement",
            Self::Intervention => "Dépenses d'intervention",
            Self::FinancialOperations => "Dépenses d'opérations financières",
        }
    }
}

/// Catégories de dépense de la nomenclature par nature, avec leur libellé :
/// LOLF art. 5, II, plus la catégorie 10 (titre 1, qui n'a qu'une catégorie)
/// et la catégorie 53 de la nomenclature d'exécution. Une catégorie absente de
/// cette liste fait échouer l'import plutôt que d'être rangée au jugé.
const CATEGORIES: &[(u8, &str)] = &[
    (10, "Dotations des pouvoirs publics"),
    (21, "Rémunérations d'activité"),
    (22, "Cotisations et contributions sociales"),
    (23, "Prestations sociales et allocations diverses"),
    (
        31,
        "Dépenses de fonctionnement autres que celles de personnel",
    ),
    (32, "Subventions pour charges de service public"),
    (41, "Intérêts de la dette financière négociable"),
    (42, "Intérêts de la dette financière non négociable"),
    (43, "Charges financières diverses"),
    (51, "Dépenses pour immobilisations corporelles de l'État"),
    (52, "Dépenses pour immobilisations incorporelles de l'État"),
    (53, "Subventions pour charges d'investissement"),
    (61, "Transferts aux ménages"),
    (62, "Transferts aux entreprises"),
    (63, "Transferts aux collectivités territoriales"),
    (64, "Transferts aux autres collectivités"),
    (65, "Appels en garantie"),
    (71, "Prêts et avances"),
    (72, "Dotations en fonds propres"),
    (73, "Dépenses de participations financières"),
];

/// Catégorie de dépense (deux chiffres). Son premier chiffre est le titre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpendingCategory(u8);

impl SpendingCategory {
    pub fn new(code: u8) -> Result<Self, BudgetError> {
        if !CATEGORIES.iter().any(|(known, _)| *known == code) {
            return Err(BudgetError::UnknownCategory(code));
        }
        Ok(Self(code))
    }

    pub fn code(&self) -> u8 {
        self.0
    }

    pub fn nature(&self) -> Nature {
        Nature::from_title(self.0 / 10).expect("every listed category starts with a title 1 to 7")
    }

    pub fn official_label(&self) -> &'static str {
        CATEGORIES
            .iter()
            .find(|(code, _)| *code == self.0)
            .map(|(_, label)| *label)
            .expect("a category is only built from the listed codes")
    }
}

impl std::fmt::Display for SpendingCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}", self.0)
    }
}

/// Code d'une action dans son programme, tel que le fichier l'écrit (« 01 »).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionCode(String);

impl ActionCode {
    pub fn new(code: impl Into<String>) -> Result<Self, BudgetError> {
        let code = code.into().trim().to_string();
        if code.is_empty() {
            return Err(BudgetError::EmptyActionCode);
        }
        Ok(Self(code))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ActionCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// CP d'une action pour une catégorie de dépense.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CategoryCredit {
    pub category: SpendingCategory,
    pub cp: Euros,
}

/// Action d'un programme et ses crédits par catégorie de dépense. Une action
/// votée à 0 € de CP (des AE seulement) est gardée : elle est dans la loi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgrammeAction {
    code: ActionCode,
    name: String,
    credits: Vec<CategoryCredit>,
}

impl ProgrammeAction {
    pub fn new(
        code: ActionCode,
        name: impl Into<String>,
        credits: Vec<CategoryCredit>,
    ) -> Result<Self, BudgetError> {
        let name = name.into().trim().to_string();
        if name.is_empty() {
            return Err(BudgetError::EmptyActionName);
        }
        let mut seen = HashSet::new();
        if let Some(twice) = credits.iter().find(|credit| !seen.insert(credit.category)) {
            return Err(BudgetError::DuplicateCategory {
                action: code,
                category: twice.category,
            });
        }
        Ok(Self {
            code,
            name,
            credits,
        })
    }

    pub fn code(&self) -> &ActionCode {
        &self.code
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn credits(&self) -> &[CategoryCredit] {
        &self.credits
    }

    pub fn total(&self) -> Euros {
        self.credits.iter().map(|credit| credit.cp).sum()
    }
}

/// Répartition par nature : chaque titre présent, ses CP et ses catégories,
/// par titre puis par catégorie croissants.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NatureBreakdown {
    natures: Vec<NatureShare>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NatureShare {
    pub nature: Nature,
    pub cp: Euros,
    pub categories: Vec<CategoryCredit>,
}

impl NatureBreakdown {
    /// Somme des crédits par catégorie, regroupés par titre.
    pub fn of<'a>(credits: impl IntoIterator<Item = &'a CategoryCredit>) -> Self {
        let mut by_category: std::collections::BTreeMap<SpendingCategory, Euros> =
            std::collections::BTreeMap::new();
        for credit in credits {
            let total = by_category.entry(credit.category).or_insert(Euros::ZERO);
            *total = [*total, credit.cp].into_iter().sum();
        }
        let mut natures: Vec<NatureShare> = Vec::new();
        for (category, cp) in by_category {
            let credit = CategoryCredit { category, cp };
            match natures.last_mut() {
                Some(share) if share.nature == category.nature() => {
                    share.cp = [share.cp, cp].into_iter().sum();
                    share.categories.push(credit);
                }
                _ => natures.push(NatureShare {
                    nature: category.nature(),
                    cp,
                    categories: vec![credit],
                }),
            }
        }
        Self { natures }
    }

    pub fn natures(&self) -> &[NatureShare] {
        &self.natures
    }

    pub fn total(&self) -> Euros {
        self.natures.iter().map(|share| share.cp).sum()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Programme {
    number: ProgrammeNumber,
    name: String,
    cp: Euros,
    actions: Vec<ProgrammeAction>,
}

impl Programme {
    /// Programme sans action : celui que la nomenclature seule connaît (0 €),
    /// ou dont le détail n'est pas lu.
    pub fn new(
        number: ProgrammeNumber,
        name: impl Into<String>,
        cp: Euros,
    ) -> Result<Self, BudgetError> {
        Self::with_actions(number, name, cp, Vec::new())
    }

    /// Programme et ses actions. Leur somme n'est contrôlée que par
    /// `BudgetExercise::verify`, qui nomme le programme en écart.
    pub fn with_actions(
        number: ProgrammeNumber,
        name: impl Into<String>,
        cp: Euros,
        actions: Vec<ProgrammeAction>,
    ) -> Result<Self, BudgetError> {
        let name = name.into().trim().to_string();
        if name.is_empty() {
            return Err(BudgetError::EmptyProgrammeName);
        }
        Ok(Self {
            number,
            name,
            cp,
            actions,
        })
    }

    pub fn number(&self) -> ProgrammeNumber {
        self.number
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn cp(&self) -> Euros {
        self.cp
    }

    pub fn actions(&self) -> &[ProgrammeAction] {
        &self.actions
    }

    pub fn actions_total(&self) -> Euros {
        self.actions.iter().map(ProgrammeAction::total).sum()
    }

    fn credits(&self) -> impl Iterator<Item = &CategoryCredit> {
        self.actions.iter().flat_map(ProgrammeAction::credits)
    }

    pub fn nature(&self) -> NatureBreakdown {
        NatureBreakdown::of(self.credits())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mission {
    name: String,
    account_kind: AccountKind,
    programmes: Vec<Programme>,
}

impl Mission {
    /// Une mission sans programme se construit : c'est `BudgetExercise::verify`
    /// qui la refuse, avec son nom dans l'erreur.
    pub fn new(
        name: impl Into<String>,
        account_kind: AccountKind,
        programmes: Vec<Programme>,
    ) -> Result<Self, BudgetError> {
        let name = name.into().trim().to_string();
        if name.is_empty() {
            return Err(BudgetError::EmptyMissionName);
        }
        Ok(Self {
            name,
            account_kind,
            programmes,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn account_kind(&self) -> AccountKind {
        self.account_kind
    }

    pub fn programmes(&self) -> &[Programme] {
        &self.programmes
    }

    /// Somme des CP de ses programmes.
    pub fn total(&self) -> Euros {
        self.programmes.iter().map(Programme::cp).sum()
    }

    /// Sujet du site auquel la mission est rattachée, s'il en a un.
    pub fn theme(&self) -> Option<FamilyCode> {
        family_of(&self.name)
    }

    /// Ce que paient ses crédits : la nature de toutes ses actions.
    pub fn nature(&self) -> NatureBreakdown {
        NatureBreakdown::of(self.programmes.iter().flat_map(Programme::credits))
    }

    fn is_refunds_and_rebates(&self) -> bool {
        normalize_name(&self.name) == normalize_name(REFUNDS_AND_REBATES_MISSION)
    }
}

/// Compte hors budget général que l'exercice doit porter, avec ses attendus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OffBudgetAccount {
    pub mission: String,
    pub kind: AccountKind,
    pub programmes: usize,
    pub cp: Euros,
}

/// Attendus d'un exercice, tirés de la loi (article de crédits, états B et D),
/// jamais du fichier importé ni d'une maquette.
///
/// `missions` et `programmes` comptent le budget général seul. Les comptes hors
/// budget retenus sont exactement ceux de `off_budget` : toute autre ligne hors
/// budget général est écartée en amont, par l'import, explicitement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expectations {
    pub general_budget_cp: Euros,
    pub missions: usize,
    pub programmes: usize,
    pub off_budget: Vec<OffBudgetAccount>,
}

impl Expectations {
    /// Compte attendu sous ce nom (comparaison normalisée) et cette nature.
    pub fn off_budget_account(
        &self,
        mission: &str,
        kind: AccountKind,
    ) -> Option<&OffBudgetAccount> {
        let wanted = normalize_name(mission);
        self.off_budget
            .iter()
            .find(|account| account.kind == kind && normalize_name(&account.mission) == wanted)
    }
}

/// Données d'un exercice, avant validation des textes.
#[derive(Debug, Clone)]
pub struct NewBudgetExercise {
    pub year: BudgetYear,
    pub stage: BudgetStage,
    pub law: LawReference,
    pub source: SourceReference,
    pub missions: Vec<Mission>,
}

/// Exercice budgétaire : la loi, sa source et ses missions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetExercise {
    year: BudgetYear,
    stage: BudgetStage,
    law: LawReference,
    source: SourceReference,
    missions: Vec<Mission>,
}

impl BudgetExercise {
    /// Refuse un texte d'identification vide. Les chiffres ne sont contrôlés que
    /// par `verify`, qui a besoin des attendus.
    pub fn new(input: NewBudgetExercise) -> Result<Self, BudgetError> {
        let texts = [
            ("law number", &input.law.number),
            ("legifrance url", &input.law.legifrance_url),
            ("source title", &input.source.title),
            ("source url", &input.source.url),
            ("source sha256", &input.source.sha256),
        ];
        for (field, value) in texts {
            if value.trim().is_empty() {
                return Err(BudgetError::EmptyField(field));
            }
        }
        Ok(Self {
            year: input.year,
            stage: input.stage,
            law: input.law,
            source: input.source,
            missions: input.missions,
        })
    }

    pub fn year(&self) -> BudgetYear {
        self.year
    }

    pub fn stage(&self) -> BudgetStage {
        self.stage
    }

    pub fn law(&self) -> &LawReference {
        &self.law
    }

    pub fn source(&self) -> &SourceReference {
        &self.source
    }

    pub fn missions(&self) -> &[Mission] {
        &self.missions
    }

    pub fn general_budget_missions(&self) -> impl Iterator<Item = &Mission> {
        self.missions
            .iter()
            .filter(|mission| mission.account_kind == AccountKind::GeneralBudget)
    }

    pub fn off_budget_missions(&self) -> impl Iterator<Item = &Mission> {
        self.missions
            .iter()
            .filter(|mission| mission.account_kind != AccountKind::GeneralBudget)
    }

    /// Somme des CP des missions du budget général.
    pub fn general_budget_total(&self) -> Euros {
        self.general_budget_missions().map(Mission::total).sum()
    }

    /// CP de la mission des remboursements et dégrèvements, comprise dans le
    /// total du budget général. `None` quand l'exercice ne la porte pas.
    pub fn refunds_and_rebates_total(&self) -> Option<Euros> {
        self.general_budget_missions()
            .find(|mission| mission.is_refunds_and_rebates())
            .map(Mission::total)
    }

    /// Nature des dépenses de tout le budget général.
    pub fn general_budget_nature(&self) -> NatureBreakdown {
        NatureBreakdown::of(
            self.general_budget_missions()
                .flat_map(Mission::programmes)
                .flat_map(Programme::credits),
        )
    }

    /// Nature des dépenses du budget général, mission des remboursements et
    /// dégrèvements mise à part : ce sont des impôts rendus, pas une dépense
    /// de politique publique. Avec `refunds_and_rebates_total`, la somme
    /// redonne le total du budget général.
    pub fn nature_without_refunds(&self) -> NatureBreakdown {
        NatureBreakdown::of(
            self.general_budget_missions()
                .filter(|mission| !mission.is_refunds_and_rebates())
                .flat_map(Mission::programmes)
                .flat_map(Programme::credits),
        )
    }

    /// Compare l'exercice aux attendus. Le premier écart arrête la vérification,
    /// et rien ne s'écrit tant qu'elle n'a pas réussi.
    ///
    /// Une mission sans sujet est signalée avant les comptes : c'est le cas
    /// d'une mission nouvelle, et son nom dit quoi décider, là où un total
    /// faux ne dit rien.
    pub fn verify(&self, expectations: &Expectations) -> Result<(), BudgetError> {
        self.check_unique_missions()?;
        self.check_no_empty_mission()?;
        self.check_unique_programmes()?;
        self.check_actions()?;
        self.check_every_mission_has_a_theme()?;
        self.check_no_unexpected_off_budget_account(expectations)?;
        self.check_general_budget(expectations)?;
        self.check_off_budget_accounts(expectations)
    }

    fn check_unique_missions(&self) -> Result<(), BudgetError> {
        let mut seen = HashSet::new();
        for mission in &self.missions {
            if !seen.insert(normalize_name(mission.name())) {
                return Err(BudgetError::DuplicateMission {
                    mission: mission.name().to_string(),
                });
            }
        }
        Ok(())
    }

    fn check_no_empty_mission(&self) -> Result<(), BudgetError> {
        match self.missions.iter().find(|m| m.programmes().is_empty()) {
            Some(mission) => Err(BudgetError::EmptyMission {
                mission: mission.name().to_string(),
            }),
            None => Ok(()),
        }
    }

    fn check_unique_programmes(&self) -> Result<(), BudgetError> {
        let mut seen = HashSet::new();
        for programme in self.missions.iter().flat_map(Mission::programmes) {
            if !seen.insert(programme.number()) {
                return Err(BudgetError::DuplicateProgramme {
                    number: programme.number(),
                });
            }
        }
        Ok(())
    }

    /// Les actions d'un programme redonnent ses CP, au centime près. Un
    /// programme doté a des actions ; un programme à 0 € n'en a que si le
    /// fichier en vote à 0 € (autorisations d'engagement seules).
    fn check_actions(&self) -> Result<(), BudgetError> {
        for programme in self.missions.iter().flat_map(Mission::programmes) {
            let mut seen = HashSet::new();
            if let Some(twice) = programme
                .actions()
                .iter()
                .find(|action| !seen.insert(action.code()))
            {
                return Err(BudgetError::DuplicateAction {
                    programme: programme.number(),
                    action: twice.code().clone(),
                });
            }
            if programme.actions().is_empty() {
                if programme.cp() != Euros::ZERO {
                    return Err(BudgetError::ProgrammeWithoutActions {
                        programme: programme.number(),
                        cp: programme.cp(),
                    });
                }
                continue;
            }
            if programme.actions_total() != programme.cp() {
                return Err(BudgetError::ActionTotalMismatch {
                    programme: programme.number(),
                    expected: programme.cp(),
                    actual: programme.actions_total(),
                });
            }
        }
        Ok(())
    }

    fn check_every_mission_has_a_theme(&self) -> Result<(), BudgetError> {
        match self.missions.iter().find(|m| m.theme().is_none()) {
            Some(mission) => Err(BudgetError::MissionWithoutTheme {
                mission: mission.name().to_string(),
            }),
            None => Ok(()),
        }
    }

    fn check_no_unexpected_off_budget_account(
        &self,
        expectations: &Expectations,
    ) -> Result<(), BudgetError> {
        let unexpected = self.off_budget_missions().find(|mission| {
            expectations
                .off_budget_account(mission.name(), mission.account_kind())
                .is_none()
        });
        match unexpected {
            Some(mission) => Err(BudgetError::UnexpectedOffBudgetAccount {
                mission: mission.name().to_string(),
            }),
            None => Ok(()),
        }
    }

    fn check_general_budget(&self, expectations: &Expectations) -> Result<(), BudgetError> {
        let missions = self.general_budget_missions().count();
        if missions != expectations.missions {
            return Err(BudgetError::MissionCountMismatch {
                expected: expectations.missions,
                actual: missions,
            });
        }
        let programmes: usize = self
            .general_budget_missions()
            .map(|mission| mission.programmes().len())
            .sum();
        if programmes != expectations.programmes {
            return Err(BudgetError::ProgrammeCountMismatch {
                expected: expectations.programmes,
                actual: programmes,
            });
        }
        let total = self.general_budget_total();
        if total != expectations.general_budget_cp {
            return Err(BudgetError::GeneralBudgetTotalMismatch {
                expected: expectations.general_budget_cp,
                actual: total,
            });
        }
        Ok(())
    }

    fn check_off_budget_accounts(&self, expectations: &Expectations) -> Result<(), BudgetError> {
        for expected in &expectations.off_budget {
            let wanted = normalize_name(&expected.mission);
            let mission = self
                .off_budget_missions()
                .find(|m| m.account_kind() == expected.kind && normalize_name(m.name()) == wanted)
                .ok_or_else(|| BudgetError::OffBudgetAccountMissing {
                    mission: expected.mission.clone(),
                })?;
            let programmes = mission.programmes().len();
            if programmes != expected.programmes {
                return Err(BudgetError::OffBudgetProgrammeCountMismatch {
                    mission: expected.mission.clone(),
                    expected: expected.programmes,
                    actual: programmes,
                });
            }
            if mission.total() != expected.cp {
                return Err(BudgetError::OffBudgetTotalMismatch {
                    mission: expected.mission.clone(),
                    expected: expected.cp,
                    actual: mission.total(),
                });
            }
        }
        Ok(())
    }
}

/// Forme de comparaison d'un nom de mission : apostrophes typographiques
/// ramenées à `'`, blancs multiples réduits à un espace, casse ignorée. Les
/// accents sont conservés : « Économie » et « Economie » restent deux noms.
pub fn normalize_name(name: &str) -> String {
    name.replace(['\u{2019}', '\u{2018}', '\u{02BC}'], "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Sujet du site d'une mission, d'après `MISSION_FAMILIES`.
pub fn family_of(mission: &str) -> Option<FamilyCode> {
    let wanted = normalize_name(mission);
    MISSION_FAMILIES
        .iter()
        .find(|(name, _)| normalize_name(name) == wanted)
        .map(|(_, family)| *family)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn euros(amount: i64) -> Euros {
        Euros::new(amount).unwrap()
    }

    fn credit(category: u8, cp: i64) -> CategoryCredit {
        CategoryCredit {
            category: SpendingCategory::new(category).unwrap(),
            cp: euros(cp),
        }
    }

    fn action(code: &str, credits: Vec<CategoryCredit>) -> ProgrammeAction {
        ProgrammeAction::new(
            ActionCode::new(code).unwrap(),
            format!("Action {code}"),
            credits,
        )
        .unwrap()
    }

    fn programme_with(number: u16, cp: i64, actions: Vec<ProgrammeAction>) -> Programme {
        Programme::with_actions(
            ProgrammeNumber::new(number).unwrap(),
            format!("Programme {number}"),
            euros(cp),
            actions,
        )
        .unwrap()
    }

    /// Programme doté d'une action unique en fonctionnement (catégorie 31),
    /// sans action quand il est à 0 €.
    fn programme(number: u16, name: &str, cp: i64) -> Programme {
        let actions = if cp == 0 {
            Vec::new()
        } else {
            vec![action("01", vec![credit(31, cp)])]
        };
        Programme::with_actions(
            ProgrammeNumber::new(number).unwrap(),
            name,
            euros(cp),
            actions,
        )
        .unwrap()
    }

    fn mission(name: &str, kind: AccountKind, programmes: Vec<Programme>) -> Mission {
        Mission::new(name, kind, programmes).unwrap()
    }

    fn law() -> LawReference {
        LawReference {
            number: "2026-103".to_string(),
            date: NaiveDate::from_ymd_opt(2026, 2, 19).unwrap(),
            credits_article: 148,
            adoption_procedure: AdoptionProcedure::Article49_3,
            legifrance_url: "https://www.legifrance.example/loi".to_string(),
        }
    }

    fn source() -> SourceReference {
        SourceReference {
            title: "Crédits AE et CP votés".to_string(),
            url: "https://www.budget.example/fichier".to_string(),
            published_on: NaiveDate::from_ymd_opt(2026, 3, 3).unwrap(),
            downloaded_on: NaiveDate::from_ymd_opt(2026, 11, 16).unwrap(),
            sha256: "ab".repeat(32),
        }
    }

    fn exercise_of(missions: Vec<Mission>) -> BudgetExercise {
        BudgetExercise::new(NewBudgetExercise {
            year: BudgetYear::new(2026).unwrap(),
            stage: BudgetStage::Lfi,
            law: law(),
            source: source(),
            missions,
        })
        .unwrap()
    }

    /// Deux missions du budget général (600 + 400 + 250 CP, 3 programmes) et un
    /// compte hors budget (2 programmes, 70 CP).
    fn sample_missions() -> Vec<Mission> {
        vec![
            mission(
                "Défense",
                AccountKind::GeneralBudget,
                vec![
                    programme(144, "Environnement et prospective", 600),
                    programme(178, "Préparation", 400),
                ],
            ),
            mission(
                "Culture",
                AccountKind::GeneralBudget,
                vec![programme(131, "Création", 250)],
            ),
            mission(
                "Avances à l'audiovisuel public",
                AccountKind::FinancialAssistance,
                vec![
                    programme(841, "France Télévisions", 50),
                    programme(842, "ARTE France", 20),
                ],
            ),
        ]
    }

    fn expectations() -> Expectations {
        Expectations {
            general_budget_cp: euros(1_250),
            missions: 2,
            programmes: 3,
            off_budget: vec![OffBudgetAccount {
                mission: "Avances à l'audiovisuel public".to_string(),
                kind: AccountKind::FinancialAssistance,
                programmes: 2,
                cp: euros(70),
            }],
        }
    }

    #[test]
    fn a_consistent_exercise_passes_the_verification() {
        let exercise = exercise_of(sample_missions());

        assert_eq!(exercise.verify(&expectations()), Ok(()));
    }

    #[test]
    fn the_total_of_a_mission_is_the_sum_of_its_programmes() {
        let defense = &sample_missions()[0];

        assert_eq!(defense.total(), euros(1_000));
    }

    #[test]
    fn the_general_budget_total_leaves_the_off_budget_accounts_out() {
        let exercise = exercise_of(sample_missions());

        assert_eq!(exercise.general_budget_total(), euros(1_250));
    }

    #[test]
    fn the_refunds_and_rebates_total_is_the_one_of_its_general_budget_mission() {
        let mut missions = sample_missions();
        missions.push(mission(
            "Remboursements et dégrèvements",
            AccountKind::GeneralBudget,
            vec![programme(
                200,
                "Remboursements et dégrèvements d'impôts d'État",
                300,
            )],
        ));

        let exercise = exercise_of(missions);

        assert_eq!(exercise.refunds_and_rebates_total(), Some(euros(300)));
    }

    #[test]
    fn an_exercise_without_the_refunds_mission_has_no_refunds_total() {
        let exercise = exercise_of(sample_missions());

        assert_eq!(exercise.refunds_and_rebates_total(), None);
    }

    #[test]
    fn a_negative_amount_is_refused() {
        assert_eq!(Euros::new(-1), Err(BudgetError::NegativeAmount(-1)));
    }

    #[test]
    fn zero_euros_is_allowed() {
        assert_eq!(Euros::new(0).unwrap().get(), 0);
    }

    #[test]
    fn a_year_outside_the_bounds_is_refused() {
        assert_eq!(BudgetYear::new(1999), Err(BudgetError::InvalidYear(1999)));
        assert_eq!(BudgetYear::new(2101), Err(BudgetError::InvalidYear(2101)));
        assert_eq!(BudgetYear::new(2026).unwrap().get(), 2026);
    }

    #[test]
    fn a_programme_number_of_zero_or_beyond_four_digits_is_refused() {
        assert_eq!(
            ProgrammeNumber::new(0),
            Err(BudgetError::InvalidProgrammeNumber(0))
        );
        assert_eq!(
            ProgrammeNumber::new(10_000),
            Err(BudgetError::InvalidProgrammeNumber(10_000))
        );
        assert_eq!(ProgrammeNumber::new(841).unwrap().get(), 841);
    }

    #[test]
    fn a_blank_mission_or_programme_name_is_refused() {
        assert_eq!(
            Mission::new("   ", AccountKind::GeneralBudget, vec![]),
            Err(BudgetError::EmptyMissionName)
        );
        assert_eq!(
            Programme::new(ProgrammeNumber::new(144).unwrap(), " ", Euros::ZERO),
            Err(BudgetError::EmptyProgrammeName)
        );
    }

    #[test]
    fn a_blank_identification_text_is_refused() {
        let mut law = law();
        law.number = "  ".to_string();

        let result = BudgetExercise::new(NewBudgetExercise {
            year: BudgetYear::new(2026).unwrap(),
            stage: BudgetStage::Lfi,
            law,
            source: source(),
            missions: vec![],
        });

        assert_eq!(result, Err(BudgetError::EmptyField("law number")));
    }

    #[test]
    fn a_wrong_general_budget_total_names_both_amounts() {
        let mut expectations = expectations();
        expectations.general_budget_cp = euros(1_251);

        assert_eq!(
            exercise_of(sample_missions()).verify(&expectations),
            Err(BudgetError::GeneralBudgetTotalMismatch {
                expected: euros(1_251),
                actual: euros(1_250),
            })
        );
    }

    #[test]
    fn a_wrong_mission_count_is_refused() {
        let mut expectations = expectations();
        expectations.missions = 3;

        assert_eq!(
            exercise_of(sample_missions()).verify(&expectations),
            Err(BudgetError::MissionCountMismatch {
                expected: 3,
                actual: 2
            })
        );
    }

    #[test]
    fn a_wrong_programme_count_is_refused() {
        let mut expectations = expectations();
        expectations.programmes = 4;

        assert_eq!(
            exercise_of(sample_missions()).verify(&expectations),
            Err(BudgetError::ProgrammeCountMismatch {
                expected: 4,
                actual: 3
            })
        );
    }

    #[test]
    fn a_missing_off_budget_account_is_refused() {
        let mut missions = sample_missions();
        missions.pop();

        assert_eq!(
            exercise_of(missions).verify(&expectations()),
            Err(BudgetError::OffBudgetAccountMissing {
                mission: "Avances à l'audiovisuel public".to_string()
            })
        );
    }

    #[test]
    fn an_off_budget_account_with_a_wrong_programme_count_is_refused() {
        let mut expectations = expectations();
        expectations.off_budget[0].programmes = 7;

        assert_eq!(
            exercise_of(sample_missions()).verify(&expectations),
            Err(BudgetError::OffBudgetProgrammeCountMismatch {
                mission: "Avances à l'audiovisuel public".to_string(),
                expected: 7,
                actual: 2,
            })
        );
    }

    #[test]
    fn an_off_budget_account_with_a_wrong_total_is_refused() {
        let mut expectations = expectations();
        expectations.off_budget[0].cp = euros(71);

        assert_eq!(
            exercise_of(sample_missions()).verify(&expectations),
            Err(BudgetError::OffBudgetTotalMismatch {
                mission: "Avances à l'audiovisuel public".to_string(),
                expected: euros(71),
                actual: euros(70),
            })
        );
    }

    #[test]
    fn an_off_budget_account_nobody_expects_is_refused() {
        let mut expectations = expectations();
        expectations.off_budget.clear();

        assert_eq!(
            exercise_of(sample_missions()).verify(&expectations),
            Err(BudgetError::UnexpectedOffBudgetAccount {
                mission: "Avances à l'audiovisuel public".to_string()
            })
        );
    }

    #[test]
    fn a_mission_without_a_theme_is_refused_before_the_totals_are_compared() {
        let mut missions = sample_missions();
        missions.push(mission(
            "Mission nouvelle",
            AccountKind::GeneralBudget,
            vec![programme(999, "Programme nouveau", 10)],
        ));

        // Le total est aussi faux ici : le nom de la mission est ce qui dit
        // quoi décider.
        assert_eq!(
            exercise_of(missions).verify(&expectations()),
            Err(BudgetError::MissionWithoutTheme {
                mission: "Mission nouvelle".to_string()
            })
        );
    }

    #[test]
    fn a_programme_number_listed_twice_is_refused() {
        let mut missions = sample_missions();
        missions[1] = mission(
            "Culture",
            AccountKind::GeneralBudget,
            vec![programme(144, "Création", 250)],
        );

        assert_eq!(
            exercise_of(missions).verify(&expectations()),
            Err(BudgetError::DuplicateProgramme {
                number: ProgrammeNumber::new(144).unwrap()
            })
        );
    }

    #[test]
    fn a_mission_without_any_programme_is_refused() {
        let mut missions = sample_missions();
        missions.push(mission("Justice", AccountKind::GeneralBudget, vec![]));

        assert_eq!(
            exercise_of(missions).verify(&expectations()),
            Err(BudgetError::EmptyMission {
                mission: "Justice".to_string()
            })
        );
    }

    #[test]
    fn a_mission_listed_twice_is_refused() {
        let mut missions = sample_missions();
        missions.push(mission(
            "DÉFENSE",
            AccountKind::GeneralBudget,
            vec![programme(146, "Équipement", 5)],
        ));

        assert_eq!(
            exercise_of(missions).verify(&expectations()),
            Err(BudgetError::DuplicateMission {
                mission: "DÉFENSE".to_string()
            })
        );
    }

    #[test]
    fn the_missions_of_2025_and_2026_all_have_a_theme() {
        let by_theme: [(FamilyCode, &[&str]); 14] = [
            (
                FamilyCode::EconomieFinancesPubliques,
                &[
                    "Remboursements et dégrèvements",
                    "Engagements financiers de l'État",
                    "Gestion des finances publiques",
                    "Investir pour la France de 2030",
                    "Économie",
                    "Crédits non répartis",
                    "Plan de relance",
                ],
            ),
            (
                FamilyCode::EducationCulture,
                &[
                    "Enseignement scolaire",
                    "Recherche et enseignement supérieur",
                    "Culture",
                    "Médias, livre et industries culturelles",
                    "Avances à l'audiovisuel public",
                    "Audiovisuel public",
                ],
            ),
            (
                FamilyCode::InternationalDefense,
                &[
                    "Défense",
                    "Aide publique au développement",
                    "Action extérieure de l'État",
                    "Monde combattant, mémoire et liens avec la Nation",
                    "Anciens combattants, mémoire et liens avec la Nation",
                ],
            ),
            (
                FamilyCode::ProtectionSocialeRetraites,
                &[
                    "Solidarité, insertion et égalité des chances",
                    "Régimes sociaux et de retraite",
                ],
            ),
            (FamilyCode::JusticeSecurite, &["Sécurités", "Justice"]),
            (
                FamilyCode::EnvironnementEnergie,
                &["Écologie, développement et mobilité durables"],
            ),
            (FamilyCode::Logement, &["Cohésion des territoires"]),
            (
                FamilyCode::TravailEmploi,
                &["Travail, emploi et administration des ministères sociaux"],
            ),
            (
                FamilyCode::InstitutionsViePolitique,
                &[
                    "Administration générale et territoriale de l'État",
                    "Pouvoirs publics",
                    "Direction de l'action du Gouvernement",
                    "Conseil et contrôle de l'État",
                    "Transformation et fonction publiques",
                ],
            ),
            (
                FamilyCode::AgricultureAlimentation,
                &["Agriculture, alimentation, forêt et affaires rurales"],
            ),
            (
                FamilyCode::CollectivitesOutreMer,
                &[
                    "Relations avec les collectivités territoriales",
                    "Outre-mer",
                ],
            ),
            (
                FamilyCode::Immigration,
                &["Immigration, asile et intégration"],
            ),
            (FamilyCode::Sante, &["Santé"]),
            (
                FamilyCode::SocieteLibertes,
                &["Sport, jeunesse et vie associative"],
            ),
        ];

        let mut count = 0;
        for (family, names) in by_theme {
            for name in names {
                assert_eq!(family_of(name), Some(family), "mission {name}");
                count += 1;
            }
        }

        assert_eq!(count, 36);
        assert_eq!(MISSION_FAMILIES.len(), 36);
    }

    #[test]
    fn the_theme_of_a_mission_ignores_apostrophe_shape_spacing_and_case() {
        let expected = Some(FamilyCode::InternationalDefense);

        assert_eq!(family_of("Action extérieure de l\u{2019}État"), expected);
        assert_eq!(family_of("  action   extérieure de l'état "), expected);
        assert_eq!(family_of("ACTION EXTÉRIEURE DE L\u{2019}ÉTAT"), expected);
    }

    #[test]
    fn a_mission_the_list_does_not_know_has_no_theme() {
        assert_eq!(family_of("Mission nouvelle"), None);
    }

    #[test]
    fn two_listed_missions_never_share_a_normalized_name() {
        let names: HashSet<String> = MISSION_FAMILIES
            .iter()
            .map(|(name, _)| normalize_name(name))
            .collect();

        assert_eq!(names.len(), MISSION_FAMILIES.len());
    }

    #[test]
    fn an_off_budget_account_is_found_whatever_the_apostrophe() {
        let found = expectations()
            .off_budget_account(
                "Avances à l\u{2019}audiovisuel public",
                AccountKind::FinancialAssistance,
            )
            .map(|account| account.programmes);

        assert_eq!(found, Some(2));
    }

    #[test]
    fn closed_enums_read_back_what_they_write() {
        for kind in [AccountKind::GeneralBudget, AccountKind::FinancialAssistance] {
            assert_eq!(AccountKind::parse(kind.as_str()), Some(kind));
        }
        for procedure in [AdoptionProcedure::Vote, AdoptionProcedure::Article49_3] {
            assert_eq!(
                AdoptionProcedure::parse(procedure.as_str()),
                Some(procedure)
            );
        }
        assert_eq!(
            BudgetStage::parse(BudgetStage::Lfi.as_str()),
            Some(BudgetStage::Lfi)
        );
        assert_eq!(BudgetStage::parse("plf"), None);
    }
    #[test]
    fn the_title_of_a_category_is_its_first_digit() {
        assert_eq!(
            SpendingCategory::new(21).unwrap().nature(),
            Nature::Personnel
        );
        assert_eq!(
            SpendingCategory::new(64).unwrap().nature(),
            Nature::Intervention
        );
        assert_eq!(
            SpendingCategory::new(10).unwrap().nature(),
            Nature::PublicAuthorities
        );
    }

    #[test]
    fn a_category_outside_the_nomenclature_is_refused() {
        assert_eq!(
            SpendingCategory::new(66),
            Err(BudgetError::UnknownCategory(66))
        );
        assert_eq!(
            SpendingCategory::new(0),
            Err(BudgetError::UnknownCategory(0))
        );
    }

    #[test]
    fn every_title_round_trips_through_its_number() {
        for nature in Nature::ALL {
            assert_eq!(Nature::from_title(nature.title()), Some(nature));
        }
        assert_eq!(Nature::from_title(8), None);
    }

    #[test]
    fn a_blank_action_code_is_refused() {
        assert_eq!(ActionCode::new("  "), Err(BudgetError::EmptyActionCode));
    }

    #[test]
    fn an_action_listing_a_category_twice_is_refused() {
        let error = ProgrammeAction::new(
            ActionCode::new("01").unwrap(),
            "Aide juridictionnelle",
            vec![credit(61, 1), credit(61, 2)],
        )
        .unwrap_err();

        assert!(matches!(error, BudgetError::DuplicateCategory { .. }));
    }

    #[test]
    fn the_nature_sums_categories_and_groups_them_by_title() {
        let programme = programme_with(
            101,
            100,
            vec![
                action("01", vec![credit(61, 40), credit(31, 5)]),
                action("02", vec![credit(61, 10), credit(64, 20), credit(21, 25)]),
            ],
        );

        let nature = programme.nature();

        let titles: Vec<(u8, i64)> = nature
            .natures()
            .iter()
            .map(|share| (share.nature.title(), share.cp.get()))
            .collect();
        assert_eq!(titles, vec![(2, 25), (3, 5), (6, 70)]);
        let interventions: Vec<(u8, i64)> = nature.natures()[2]
            .categories
            .iter()
            .map(|credit| (credit.category.code(), credit.cp.get()))
            .collect();
        assert_eq!(interventions, vec![(61, 50), (64, 20)]);
        assert_eq!(nature.total(), euros(100));
    }

    #[test]
    fn actions_that_do_not_add_up_to_their_programme_are_refused() {
        let mut missions = sample_missions();
        missions[1] = mission(
            "Culture",
            AccountKind::GeneralBudget,
            vec![programme_with(
                131,
                250,
                vec![action("01", vec![credit(31, 249)])],
            )],
        );

        let error = exercise_of(missions).verify(&expectations()).unwrap_err();

        assert_eq!(
            error,
            BudgetError::ActionTotalMismatch {
                programme: ProgrammeNumber::new(131).unwrap(),
                expected: euros(250),
                actual: euros(249),
            }
        );
    }

    #[test]
    fn a_funded_programme_without_actions_is_refused() {
        let mut missions = sample_missions();
        missions[1] = mission(
            "Culture",
            AccountKind::GeneralBudget,
            vec![programme_with(131, 250, Vec::new())],
        );

        let error = exercise_of(missions).verify(&expectations()).unwrap_err();

        assert!(matches!(error, BudgetError::ProgrammeWithoutActions { .. }));
    }

    #[test]
    fn a_programme_at_zero_may_carry_actions_voted_at_zero() {
        let mut missions = sample_missions();
        missions[1] = mission(
            "Culture",
            AccountKind::GeneralBudget,
            vec![
                programme(131, "Création", 250),
                programme_with(336, 0, vec![action("01", vec![credit(31, 0)])]),
            ],
        );
        let mut expected = expectations();
        expected.programmes = 4;

        assert_eq!(exercise_of(missions).verify(&expected), Ok(()));
    }

    #[test]
    fn an_action_listed_twice_in_a_programme_is_refused() {
        let mut missions = sample_missions();
        missions[1] = mission(
            "Culture",
            AccountKind::GeneralBudget,
            vec![programme_with(
                131,
                250,
                vec![
                    action("01", vec![credit(31, 100)]),
                    action("01", vec![credit(32, 150)]),
                ],
            )],
        );

        let error = exercise_of(missions).verify(&expectations()).unwrap_err();

        assert!(matches!(error, BudgetError::DuplicateAction { .. }));
    }

    #[test]
    fn the_exercise_nature_sets_the_refunds_mission_apart() {
        let mut missions = sample_missions();
        missions.push(mission(
            REFUNDS_AND_REBATES_MISSION,
            AccountKind::GeneralBudget,
            vec![programme_with(
                200,
                300,
                vec![action("11", vec![credit(62, 300)])],
            )],
        ));
        let exercise = exercise_of(missions);

        let nature = exercise.nature_without_refunds();

        assert_eq!(nature.total(), euros(1_250));
        assert!(nature
            .natures()
            .iter()
            .all(|share| share.nature != Nature::Intervention));
        assert_eq!(
            exercise
                .refunds_and_rebates_total()
                .map(|total| total.get()),
            Some(300)
        );
    }
}
