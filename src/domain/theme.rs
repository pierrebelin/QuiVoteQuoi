//! Thematisation: rattachement des textes debattus aux familles de README.md §5.
//!
//! Voir `todo/SPEC-thematisation.md`. Trois invariants portent le reste:
//! - RM-02: le texte debattu s'extrait de l'objet du scrutin par regle, sans modele;
//! - RM-03: trois familles au plus par objet;
//! - RM-07: reviser ne supprime rien, l'ancien rattachement est clos.

use chrono::NaiveDate;
use serde::Serialize;

use super::dossier::DossierUid;

/// Nombre maximal de familles portees par un objet (RM-03).
pub const MAX_FAMILIES: usize = 3;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ThemeError {
    #[error("unknown theme family: {0}")]
    UnknownFamily(String),
    #[error("a proposal must carry at least one family")]
    EmptyProposal,
    #[error("justification must not be empty")]
    EmptyJustification,
    #[error("assignment author must not be empty")]
    EmptyAuthor,
    #[error("text label must not be empty")]
    EmptyTextLabel,
    #[error("closing date {closed_on} precedes opening date {opened_on}")]
    ClosedBeforeOpened {
        opened_on: NaiveDate,
        closed_on: NaiveDate,
    },
}

/// Referentiel ferme des familles (RM-08). Le modele n'en cree aucune.
///
/// Seize familles depuis le 2 novembre 2026, apres huit puis treize.
///
/// - Huit au depart, jusqu'au 9 aout 2026: elles concentraient justice,
///   securite, immigration, education et culture dans « societe / libertes »,
///   et n'accueillaient ni l'international ni la defense.
/// - Treize ensuite: un visiteur cherchant l'immigration n'ouvre plus un bac
///   contenant aussi l'ecole.
/// - Seize depuis la relecture des 341 textes en base. « Pouvoir d'achat /
///   fiscalite » melait le budget de l'Etat et la consommation des menages;
///   « sante / social » melait les soins et la protection sociale, retraites
///   comprises; « institutions / procedure » servait de fourre-tout, collectivites
///   incluses. D'ou l'economie et les finances publiques separees du pouvoir
///   d'achat, la protection sociale separee de la sante, et les collectivites et
///   l'outre-mer separes des institutions.
///
/// Le decoupage porte sur l'objet des textes, jamais sur leur orientation
/// (README.md §5, §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FamilyCode {
    PouvoirAchat,
    EconomieFinancesPubliques,
    Logement,
    TravailEmploi,
    ProtectionSocialeRetraites,
    Sante,
    EnvironnementEnergie,
    AgricultureAlimentation,
    Numerique,
    JusticeSecurite,
    Immigration,
    EducationCulture,
    SocieteLibertes,
    InternationalDefense,
    InstitutionsViePolitique,
    CollectivitesOutreMer,
}

impl FamilyCode {
    /// Ordre d'affichage publie. Sert aussi d'ordre de presentation au modele.
    pub const ALL: [FamilyCode; 16] = [
        Self::PouvoirAchat,
        Self::EconomieFinancesPubliques,
        Self::Logement,
        Self::TravailEmploi,
        Self::ProtectionSocialeRetraites,
        Self::Sante,
        Self::EnvironnementEnergie,
        Self::AgricultureAlimentation,
        Self::Numerique,
        Self::JusticeSecurite,
        Self::Immigration,
        Self::EducationCulture,
        Self::SocieteLibertes,
        Self::InternationalDefense,
        Self::InstitutionsViePolitique,
        Self::CollectivitesOutreMer,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PouvoirAchat => "pouvoir-achat",
            Self::EconomieFinancesPubliques => "economie-finances-publiques",
            Self::Logement => "logement",
            Self::TravailEmploi => "travail-emploi",
            Self::ProtectionSocialeRetraites => "protection-sociale-retraites",
            Self::Sante => "sante",
            Self::EnvironnementEnergie => "environnement-energie",
            Self::AgricultureAlimentation => "agriculture-alimentation",
            Self::Numerique => "numerique",
            Self::JusticeSecurite => "justice-securite",
            Self::Immigration => "immigration",
            Self::EducationCulture => "education-culture",
            Self::SocieteLibertes => "societe-libertes",
            Self::InternationalDefense => "international-defense",
            Self::InstitutionsViePolitique => "institutions-vie-politique",
            Self::CollectivitesOutreMer => "collectivites-outre-mer",
        }
    }

    /// Libelle public, repris mot pour mot de README.md §5.
    pub fn label(&self) -> &'static str {
        match self {
            Self::PouvoirAchat => "Pouvoir d'achat",
            Self::EconomieFinancesPubliques => "Économie / finances publiques",
            Self::Logement => "Logement / urbanisme / foncier",
            Self::TravailEmploi => "Travail / emploi",
            Self::ProtectionSocialeRetraites => "Protection sociale / retraites",
            Self::Sante => "Santé",
            Self::EnvironnementEnergie => "Environnement / énergie",
            Self::AgricultureAlimentation => "Agriculture / alimentation",
            Self::Numerique => "Numérique",
            Self::JusticeSecurite => "Justice / sécurité",
            Self::Immigration => "Immigration",
            Self::EducationCulture => "Éducation / culture",
            Self::SocieteLibertes => "Société / libertés",
            Self::InternationalDefense => "International / défense",
            Self::InstitutionsViePolitique => "Institutions / vie politique",
            Self::CollectivitesOutreMer => "Collectivités / outre-mer",
        }
    }

    /// Perimetre publie de la famille, affiche sur la page methode (CU-06).
    pub fn scope(&self) -> &'static str {
        match self {
            Self::PouvoirAchat => "Prix et tarifs, dont ceux de l'énergie, protection du consommateur, frais et services bancaires aux particuliers, impôts et taxes des ménages. Rattachement sur ce que fixe le dispositif, jamais sur l'effet annoncé.",
            Self::EconomieFinancesPubliques => "Budget et comptes de l'État, lois de finances, fiscalité des entreprises, droit des sociétés, commerce, concurrence, banque et marchés financiers, commande publique, industrie.",
            Self::Logement => "Loyers, accès à la propriété, construction, locations de courte durée, urbanisme, droit de la propriété immobilière.",
            Self::TravailEmploi => "Droit du travail, chômage, formation professionnelle, fonction publique, dialogue social, indépendants.",
            Self::ProtectionSocialeRetraites => "Retraites, prestations et minima sociaux, politique familiale, protection de l'enfance, handicap, dépendance, financement de la sécurité sociale.",
            Self::Sante => "Accès aux soins, hôpital, médicament, professions de santé, prévention et santé publique.",
            Self::EnvironnementEnergie => "Transition et production d'énergie, transports, eau, biodiversité, déchets.",
            Self::AgricultureAlimentation => "Revenu agricole, pêche, produits phytosanitaires, alimentation, foncier agricole.",
            Self::Numerique => "Données personnelles, intelligence artificielle, réseaux sociaux, fraude en ligne.",
            Self::JusticeSecurite => "Droit pénal, police, gendarmerie, prisons, terrorisme, sécurité civile, organisation des juridictions ; procédure judiciaire lorsqu'elle est l'objet du texte.",
            Self::Immigration => "Entrée et séjour des étrangers, asile, éloignement, nationalité. Rattachement sur l'objet du texte, jamais sur son orientation.",
            Self::EducationCulture => "École, université, recherche, sport, culture, médias, audiovisuel.",
            Self::SocieteLibertes => "Égalité, droits des personnes, fin de vie, bioéthique, laïcité, libertés publiques, mémoire et reconnaissance. Rattachement sur l'objet du texte, jamais sur son orientation.",
            Self::InternationalDefense => "Ratification de traités, armées, aide au développement, affaires européennes.",
            Self::InstitutionsViePolitique => "Constitution, Parlement, motions de censure, élections, statut des élus, organisation de l'État.",
            Self::CollectivitesOutreMer => "Décentralisation, organisation et finances des collectivités, statuts de l'outre-mer et de la Corse.",
        }
    }

    /// Familles ou le rattachement se fait sur l'objet seul, jamais sur
    /// l'orientation du texte (RM-11).
    pub fn is_sensitive(&self) -> bool {
        matches!(self, Self::SocieteLibertes | Self::Immigration)
    }

    pub fn parse(raw: &str) -> Result<Self, ThemeError> {
        Self::ALL
            .into_iter()
            .find(|f| f.as_str() == raw)
            .ok_or_else(|| ThemeError::UnknownFamily(raw.to_string()))
    }
}

// ---------------------------------------------------------------------------
// Texte debattu
// ---------------------------------------------------------------------------

/// Formules qui ouvrent le libelle d'un texte dans l'objet d'un scrutin.
///
/// Ordre sans importance: on retient la derniere occurrence trouvee (voir
/// `DebatedText::from_subject`).
const TEXT_MARKERS: [&str; 6] = [
    "projet de loi",
    "proposition de loi",
    "proposition de résolution",
    "projet de résolution",
    "motion de censure",
    "déclaration de politique générale",
];

/// Mentions de stade de navette, retirees de la cle: le texte reste le meme
/// d'une lecture a l'autre (RM-02).
const STAGE_SUFFIXES: [&str; 9] = [
    "première lecture",
    "deuxième lecture",
    "seconde lecture",
    "troisième lecture",
    "nouvelle lecture",
    "lecture définitive",
    "lecture unique",
    "examen prioritaire",
    "seconde délibération",
];

/// Cle normalisee d'un texte debattu.
///
/// Deux objets nommant le meme texte donnent la meme cle, quelle que soit la
/// casse, la forme de l'apostrophe, l'espacement ou la mention de lecture. La
/// normalisation n'est pas cosmetique: la seule apostrophe typographique
/// separait « droit a l'aide a mourir » en deux cles portant 80 et 838 scrutins
/// (SPEC-thematisation H6).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct TextKey(String);

impl TextKey {
    /// Cle relue depuis la base ou depuis une adresse. Passe par la meme
    /// normalisation que l'extraction: une cle mal casee reste retrouvable.
    pub fn from_raw(raw: &str) -> Self {
        Self(lowercase(&clean(raw)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for TextKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Texte nomme par l'objet d'un scrutin. Porteur du rattachement (RM-06).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebatedText {
    key: TextKey,
    label: String,
}

impl DebatedText {
    pub fn new(label: String) -> Result<Self, ThemeError> {
        let cleaned = clean(&label);
        if cleaned.is_empty() {
            return Err(ThemeError::EmptyTextLabel);
        }
        let key = TextKey(lowercase(&cleaned));
        Ok(Self {
            key,
            label: cleaned,
        })
    }

    /// Extrait le texte debattu de l'objet d'un scrutin (RM-02).
    ///
    /// On retient la **derniere** formule de texte de l'objet, pas la premiere:
    /// une motion de rejet porte sur un texte qu'elle nomme apres elle
    /// (« motion de rejet préalable, déposée par …, du projet de loi X »), et le
    /// porteur thematique est X. Les motions de censure ne nomment aucun texte:
    /// elles forment leur propre cle.
    ///
    /// Rend `None` quand l'objet ne nomme aucun texte — 7 scrutins sur 8 434
    /// (H1). Ils restent consultables, non rattaches (RM-01).
    pub fn from_subject(subject: &str) -> Option<Self> {
        let cleaned: Vec<char> = clean(subject).chars().collect();
        let lower: Vec<char> = cleaned
            .iter()
            .map(|c| c.to_lowercase().next().unwrap_or(*c))
            .collect();

        let start = TEXT_MARKERS
            .iter()
            .filter_map(|marker| last_index_of(&lower, marker))
            .max()?;

        let mut label: String = cleaned[start..].iter().collect();
        strip_stage_suffixes(&mut label);
        Self::new(label).ok()
    }

    pub fn key(&self) -> &TextKey {
        &self.key
    }

    /// Libelle publie, casse d'origine conservee.
    pub fn label(&self) -> &str {
        &self.label
    }
}

/// Retire les mentions de stade et la ponctuation terminale, autant de fois
/// qu'elles se suivent: « … (examen prioritaire) (première lecture). ».
fn strip_stage_suffixes(label: &mut String) {
    loop {
        let before = label.len();
        while label
            .chars()
            .next_back()
            .is_some_and(|c| c == '.' || c == ',' || c == ';' || c.is_whitespace())
        {
            label.pop();
        }
        if let Some(open) = label.rfind('(') {
            if label.ends_with(')') {
                let inner = lowercase(label[open + 1..label.len() - 1].trim());
                if STAGE_SUFFIXES.contains(&inner.as_str()) {
                    label.truncate(open);
                }
            }
        }
        if label.len() == before {
            return;
        }
    }
}

/// Apostrophes ramenees a une seule forme, espaces (insecables compris) reduits.
///
/// Partage avec `domain::amendment`: la source ponctue ses libelles de la meme
/// facon partout, la normalisation n'a pas de raison d'exister en deux copies.
pub(crate) fn clean(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut pending_space = false;
    for c in raw.chars() {
        let c = match c {
            '\u{2019}' | '\u{02BC}' | '\u{FF07}' => '\'',
            '\u{00A0}' | '\u{202F}' | '\u{2009}' => ' ',
            other => other,
        };
        if c.is_whitespace() {
            pending_space = !out.is_empty();
            continue;
        }
        if pending_space {
            out.push(' ');
            pending_space = false;
        }
        out.push(c);
    }
    out
}

pub(crate) fn lowercase(raw: &str) -> String {
    raw.chars()
        .map(|c| c.to_lowercase().next().unwrap_or(c))
        .collect()
}

fn last_index_of(haystack: &[char], needle: &str) -> Option<usize> {
    let needle: Vec<char> = needle.chars().collect();
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    (0..=haystack.len() - needle.len())
        .rev()
        .find(|&i| haystack[i..i + needle.len()] == needle[..])
}

// ---------------------------------------------------------------------------
// Objets rattachables
// ---------------------------------------------------------------------------

/// Objet portant un rattachement.
///
/// Un scrutin n'en fait pas partie: il herite des familles de son texte
/// (RM-06). Un dossier n'est classe directement que lorsqu'aucun scrutin ne le
/// relie a un texte.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SubjectRef {
    Text(TextKey),
    Dossier(DossierUid),
}

impl SubjectRef {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Text(_) => "text",
            Self::Dossier(_) => "dossier",
        }
    }

    pub fn identifier(&self) -> &str {
        match self {
            Self::Text(key) => key.as_str(),
            Self::Dossier(uid) => uid.as_str(),
        }
    }

    pub fn parse(kind: &str, identifier: String) -> Option<Self> {
        match kind {
            "text" => Some(Self::Text(TextKey::from_raw(&identifier))),
            "dossier" => DossierUid::new(identifier).ok().map(Self::Dossier),
            _ => None,
        }
    }
}

/// Rattachement date d'un objet a une famille.
///
/// `closed_on` renseigne = rattachement historique: il a valu jusqu'a cette
/// date et reste lisible (RM-07).
///
/// Le rattachement ne porte pas de categorie d'auteur — regle, modele, humain.
/// Un rattachement vaut par ce qu'il rattache, pas par qui l'a ouvert: dire au
/// lecteur qu'un texte est « logement selon un modele » plutot que « logement »
/// deplace son attention sans rien lui apprendre sur le vote. `author` reste
/// renseigne, en clair, pour que l'historique dise qui a ouvert la ligne
/// (RM-07, README.md §9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeAssignment {
    subject: SubjectRef,
    family: FamilyCode,
    opened_on: NaiveDate,
    closed_on: Option<NaiveDate>,
    author: String,
    motive: Option<String>,
}

impl ThemeAssignment {
    pub fn open(
        subject: SubjectRef,
        family: FamilyCode,
        opened_on: NaiveDate,
        author: String,
        motive: Option<String>,
    ) -> Result<Self, ThemeError> {
        if author.trim().is_empty() {
            return Err(ThemeError::EmptyAuthor);
        }
        Ok(Self {
            subject,
            family,
            opened_on,
            closed_on: None,
            author,
            motive,
        })
    }

    /// Clot le rattachement. Aucune suppression: l'etat passe reste
    /// reconstituable (RM-07).
    pub fn close(&mut self, closed_on: NaiveDate) -> Result<(), ThemeError> {
        if closed_on < self.opened_on {
            return Err(ThemeError::ClosedBeforeOpened {
                opened_on: self.opened_on,
                closed_on,
            });
        }
        self.closed_on = Some(closed_on);
        Ok(())
    }

    pub fn is_current(&self) -> bool {
        self.closed_on.is_none()
    }

    pub fn subject(&self) -> &SubjectRef {
        &self.subject
    }
    pub fn family(&self) -> FamilyCode {
        self.family
    }
    pub fn opened_on(&self) -> NaiveDate {
        self.opened_on
    }
    pub fn closed_on(&self) -> Option<NaiveDate> {
        self.closed_on
    }
    pub fn author(&self) -> &str {
        &self.author
    }
    pub fn motive(&self) -> Option<&str> {
        self.motive.as_deref()
    }
}

/// Une famille proposee par le modele, avec sa justification (RM-05).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedFamily {
    family: FamilyCode,
    justification: String,
}

impl ProposedFamily {
    pub fn new(family: FamilyCode, justification: String) -> Result<Self, ThemeError> {
        if justification.trim().is_empty() {
            return Err(ThemeError::EmptyJustification);
        }
        Ok(Self {
            family,
            justification,
        })
    }

    pub fn family(&self) -> FamilyCode {
        self.family
    }
    pub fn justification(&self) -> &str {
        &self.justification
    }
}

/// Proposition du modele pour un objet, conservee telle que rendue.
///
/// Le modele ne produit ni note, ni rang, ni decompte (RM-10): cette structure
/// ne porte aucun nombre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeProposal {
    subject: SubjectRef,
    families: Vec<ProposedFamily>,
    model: String,
    prompt_version: String,
    produced_on: NaiveDate,
}

impl ThemeProposal {
    /// Retient les familles rendues, dedupliquees, dans l'ordre du modele, puis
    /// tronque a `MAX_FAMILIES` (RM-03).
    pub fn new(
        subject: SubjectRef,
        families: Vec<ProposedFamily>,
        model: String,
        prompt_version: String,
        produced_on: NaiveDate,
    ) -> Result<Self, ThemeError> {
        let mut kept: Vec<ProposedFamily> = Vec::with_capacity(MAX_FAMILIES);
        for proposed in families {
            if kept.iter().any(|k| k.family == proposed.family) {
                continue;
            }
            kept.push(proposed);
            if kept.len() == MAX_FAMILIES {
                break;
            }
        }
        if kept.is_empty() {
            return Err(ThemeError::EmptyProposal);
        }
        Ok(Self {
            subject,
            families: kept,
            model,
            prompt_version,
            produced_on,
        })
    }

    /// Rattachements a ouvrir depuis la proposition.
    pub fn into_assignments(&self) -> Result<Vec<ThemeAssignment>, ThemeError> {
        self.families
            .iter()
            .map(|proposed| {
                ThemeAssignment::open(
                    self.subject.clone(),
                    proposed.family,
                    self.produced_on,
                    self.model.clone(),
                    Some(proposed.justification.clone()),
                )
            })
            .collect()
    }

    pub fn subject(&self) -> &SubjectRef {
        &self.subject
    }
    pub fn families(&self) -> &[ProposedFamily] {
        &self.families
    }
    pub fn model(&self) -> &str {
        &self.model
    }
    pub fn prompt_version(&self) -> &str {
        &self.prompt_version
    }
    pub fn produced_on(&self) -> NaiveDate {
        self.produced_on
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 8, 3).unwrap()
    }

    fn key_of(subject: &str) -> String {
        DebatedText::from_subject(subject)
            .expect("subject names a text")
            .key()
            .as_str()
            .to_string()
    }

    #[test]
    fn amendment_and_article_of_the_same_text_share_one_key() {
        let amendment = key_of(
            "l'amendement n° 234 de Mme Sandrine Rousseau après l'article 7 du projet de loi \
             de financement de la sécurité sociale pour 2026 (première lecture).",
        );
        let article = key_of(
            "l'article 12 du projet de loi de financement de la sécurité sociale pour 2026 \
             (nouvelle lecture).",
        );
        assert_eq!(
            amendment,
            "projet de loi de financement de la sécurité sociale pour 2026"
        );
        assert_eq!(amendment, article);
    }

    #[test]
    fn typographic_apostrophe_does_not_split_a_text() {
        // Mesure du 3 aout 2026: cette seule difference separait 80 scrutins
        // des 838 autres du meme texte (H6).
        let straight = key_of("l'ensemble de la proposition de loi relative au droit à l'aide à mourir (première lecture).");
        let curly = key_of("l'ensemble de la proposition de loi relative au droit à l’aide à mourir (première lecture).");
        assert_eq!(straight, curly);
    }

    #[test]
    fn stacked_stage_mentions_are_stripped() {
        assert_eq!(
            key_of(
                "l'amendement n° 228 de Mme Élisa Martin à l'article 35 (examen prioritaire) \
                 du projet de loi relatif à l'organisation des jeux Olympiques et Paralympiques \
                 de 2030 (première lecture)."
            ),
            "projet de loi relatif à l'organisation des jeux olympiques et paralympiques de 2030"
        );
    }

    #[test]
    fn a_motion_carries_the_text_it_targets_not_its_author() {
        assert_eq!(
            key_of(
                "la motion de rejet préalable, déposée par Mme Mathilde Panot, du projet de loi \
                 portant approbation des comptes de la sécurité sociale (première lecture)."
            ),
            "projet de loi portant approbation des comptes de la sécurité sociale"
        );
    }

    #[test]
    fn a_censure_motion_is_its_own_text() {
        assert_eq!(
            key_of(
                "la motion de censure déposée en application de l'article 49, alinéa 3, \
                 de la Constitution par Mme Mathilde Panot et 87 membres de l'Assemblée."
            ),
            "motion de censure déposée en application de l'article 49, alinéa 3, de la \
             constitution par mme mathilde panot et 87 membres de l'assemblée"
        );
    }

    #[test]
    fn a_subject_naming_no_text_carries_no_key() {
        assert!(DebatedText::from_subject("l'ensemble du texte mis aux voix.").is_none());
    }

    #[test]
    fn label_keeps_the_published_casing() {
        let text = DebatedText::from_subject(
            "l'article premier du projet de loi de Finances pour 2026 (première lecture).",
        )
        .unwrap();
        assert_eq!(text.label(), "projet de loi de Finances pour 2026");
        assert_eq!(text.key().as_str(), "projet de loi de finances pour 2026");
    }

    #[test]
    fn a_proposal_keeps_three_families_at_most() {
        let subject = SubjectRef::Text(TextKey("projet de loi de finances pour 2026".into()));
        let families = FamilyCode::ALL
            .into_iter()
            .map(|f| ProposedFamily::new(f, "justification".into()).unwrap())
            .collect();
        let proposal =
            ThemeProposal::new(subject, families, "modele".into(), "v1".into(), date()).unwrap();
        assert_eq!(proposal.families().len(), MAX_FAMILIES);
        assert_eq!(proposal.families()[0].family(), FamilyCode::PouvoirAchat);
    }

    #[test]
    fn a_proposal_drops_repeated_families_before_truncating() {
        let subject = SubjectRef::Text(TextKey("texte".into()));
        let families = vec![
            ProposedFamily::new(FamilyCode::Logement, "une".into()).unwrap(),
            ProposedFamily::new(FamilyCode::Logement, "deux".into()).unwrap(),
            ProposedFamily::new(FamilyCode::Sante, "trois".into()).unwrap(),
        ];
        let proposal =
            ThemeProposal::new(subject, families, "modele".into(), "v1".into(), date()).unwrap();
        assert_eq!(proposal.families().len(), 2);
    }

    #[test]
    fn a_family_without_justification_is_refused() {
        assert_eq!(
            ProposedFamily::new(FamilyCode::Numerique, "   ".into()).unwrap_err(),
            ThemeError::EmptyJustification
        );
    }

    #[test]
    fn an_empty_proposal_is_refused() {
        let subject = SubjectRef::Text(TextKey("texte".into()));
        assert_eq!(
            ThemeProposal::new(subject, vec![], "modele".into(), "v1".into(), date()).unwrap_err(),
            ThemeError::EmptyProposal
        );
    }

    #[test]
    fn a_proposal_opens_one_assignment_per_family() {
        let subject = SubjectRef::Text(TextKey("texte".into()));
        let families = vec![ProposedFamily::new(FamilyCode::Logement, "motif".into()).unwrap()];
        let proposal =
            ThemeProposal::new(subject, families, "modele".into(), "v1".into(), date()).unwrap();
        let assignments = proposal.into_assignments().unwrap();
        assert_eq!(assignments.len(), 1);
        assert!(assignments[0].is_current());
        assert_eq!(assignments[0].motive(), Some("motif"));
        // L'historique dit qui a ouvert la ligne, sans la ranger dans une
        // categorie d'auteur (RM-07).
        assert_eq!(assignments[0].author(), "modele");
    }

    #[test]
    fn closing_keeps_the_assignment_readable() {
        let subject = SubjectRef::Text(TextKey("texte".into()));
        let mut assignment =
            ThemeAssignment::open(subject, FamilyCode::Logement, date(), "modele".into(), None)
                .unwrap();
        let later = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        assignment.close(later).unwrap();
        assert!(!assignment.is_current());
        assert_eq!(assignment.closed_on(), Some(later));
        assert_eq!(assignment.family(), FamilyCode::Logement);
    }

    #[test]
    fn closing_before_opening_is_refused() {
        let subject = SubjectRef::Text(TextKey("texte".into()));
        let mut assignment =
            ThemeAssignment::open(subject, FamilyCode::Logement, date(), "modele".into(), None)
                .unwrap();
        let earlier = NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
        assert_eq!(
            assignment.close(earlier).unwrap_err(),
            ThemeError::ClosedBeforeOpened {
                opened_on: date(),
                closed_on: earlier,
            }
        );
        assert!(assignment.is_current());
    }

    #[test]
    fn an_assignment_without_an_author_is_refused() {
        assert_eq!(
            ThemeAssignment::open(
                SubjectRef::Text(TextKey("texte".into())),
                FamilyCode::Logement,
                date(),
                "  ".into(),
                None,
            )
            .unwrap_err(),
            ThemeError::EmptyAuthor
        );
    }

    #[test]
    fn the_family_referential_is_closed() {
        assert!(FamilyCode::parse("logement").is_ok());
        assert_eq!(
            FamilyCode::parse("securite").unwrap_err(),
            ThemeError::UnknownFamily("securite".into())
        );
    }

    #[test]
    fn the_codes_retired_on_the_move_to_sixteen_families_are_refused() {
        for retired in [
            "pouvoir-achat-fiscalite",
            "sante-social",
            "institutions-procedure",
        ] {
            assert_eq!(
                FamilyCode::parse(retired).unwrap_err(),
                ThemeError::UnknownFamily(retired.into())
            );
        }
    }

    #[test]
    fn the_referential_holds_sixteen_distinct_families() {
        let codes: std::collections::BTreeSet<&str> =
            FamilyCode::ALL.iter().map(|f| f.as_str()).collect();
        assert_eq!(FamilyCode::ALL.len(), 16);
        assert_eq!(codes.len(), 16);
    }

    #[test]
    fn every_family_round_trips_through_its_code() {
        for family in FamilyCode::ALL {
            assert_eq!(FamilyCode::parse(family.as_str()).unwrap(), family);
            assert!(!family.label().is_empty());
            assert!(!family.scope().is_empty());
        }
    }
}
