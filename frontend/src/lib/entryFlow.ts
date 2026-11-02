import type { FinalVoteDto, StanceDto } from '../types/finalVotes'

/*
 * Logique pure du parcours d'accueil (sujets, groupes, votes).
 *
 * Rien ici ne touche au DOM ni à React : les règles qui décident ce que le
 * lecteur lit — position d'un groupe, titre affiché, ordre des groupes — se
 * relisent et se vérifient sans monter une page.
 */

/* ------------------------------------------------------------------- adresse */

/** Paramètres de l'adresse : le parcours est entièrement dans l'URL, donc partageable. */
export const PARAM_THEMES = 'sujets'
export const PARAM_GROUPS = 'suivis'
export const PARAM_STEP = 'etape'

/** Écrans du parcours. `etape` ne vaut que pour revenir en arrière. */
export type EntryStep = 'themes' | 'groups' | 'votes'

export function splitList(value: string | null): string[] {
  return (value ?? '')
    .split(',')
    .map((token) => token.trim())
    .filter(Boolean)
}

/**
 * Construit la requête à la main : `URLSearchParams` encode la virgule en
 * `%2C`, et une adresse à partager se lit mieux avec `sujets=a,b`.
 */
export function entrySearch(state: {
  themes: string[]
  followed: string[]
  step: EntryStep
}): string {
  const parts: string[] = []
  const list = (values: string[]) => values.map(encodeURIComponent).join(',')
  if (state.themes.length > 0) parts.push(`${PARAM_THEMES}=${list(state.themes)}`)
  if (state.followed.length > 0) parts.push(`${PARAM_GROUPS}=${list(state.followed)}`)
  if (state.step === 'themes' && state.themes.length > 0) parts.push(`${PARAM_STEP}=sujets`)
  if (state.step === 'groups') parts.push(`${PARAM_STEP}=groupes`)
  return parts.length > 0 ? `?${parts.join('&')}` : ''
}

export function toggled<T>(list: T[], item: T): T[] {
  return list.includes(item) ? list.filter((entry) => entry !== item) : [...list, item]
}

/* ------------------------------------------------------------------ groupes */

/*
 * Ordre des groupes dans l'hémicycle, de la gauche vers la droite.
 *
 * Non dérivable à l'exécution : `planHemicycle.ts` ne donne que la
 * coordonnée de chaque siège numéroté, et le groupe qui l'occupe n'est publié
 * qu'avec les positions nominales d'un scrutin (`VoteDto.seat`), absentes de la
 * liste des votes sur l'ensemble. L'ordre a donc été relevé une fois, en
 * projetant sur `PLAN_SIEGES` les sièges occupés par chaque groupe sur six
 * scrutins à forte participation : angle moyen autour du centre du plan, de
 * gauche à droite, confirmé par les numéros de siège (qui décroissent de la
 * droite vers la gauche). Les non-inscrits n'ont pas de bloc propre, leurs
 * sièges sont dispersés : ils ferment la liste. Un groupe absent d'ici est
 * ajouté en fin plutôt qu'écarté (README.md §2).
 */
export const HEMICYCLE_ORDER = [
  'LFI-NFP',
  'GDR',
  'ECOS',
  'SOC',
  'LIOT',
  'DEM',
  'EPR',
  'HOR',
  'DR',
  'UDDPLR',
  'RN',
  'NI',
] as const

export function inHemicycleOrder<T extends { abbrev: string }>(groups: T[]): T[] {
  const rank = (abbrev: string) => {
    const index = (HEMICYCLE_ORDER as readonly string[]).indexOf(abbrev)
    return index === -1 ? HEMICYCLE_ORDER.length : index
  }
  // Tri stable : deux groupes inconnus gardent l'ordre de l'API.
  return [...groups].sort((a, b) => rank(a.abbrev) - rank(b.abbrev))
}

/**
 * Noms courts, pour les cartes et les pastilles. Le libellé officiel complet
 * reste porté par l'infobulle : un nom raccourci n'est jamais la seule trace du
 * groupe (README.md §3.1).
 */
const GROUP_SHORT_NAMES: Record<string, string> = {
  'LFI-NFP': 'La France insoumise',
  GDR: 'Gauche démocrate et républicaine',
  ECOS: 'Écologiste et social',
  SOC: 'Socialistes',
  LIOT: 'LIOT',
  DEM: 'Les Démocrates',
  EPR: 'Ensemble pour la République',
  HOR: 'Horizons et indépendants',
  DR: 'Droite républicaine',
  UDDPLR: 'Union des droites',
  RN: 'Rassemblement national',
  NI: 'Non-inscrits',
}

export function groupShortName(group: { abbrev: string; label: string }): string {
  return GROUP_SHORT_NAMES[group.abbrev] ?? typographic(group.label)
}

/* ------------------------------------------------------------------ position */

/**
 * Position d'un groupe sur un vote.
 *
 * `majority` seul ne suffit pas : la source la publie aussi quand personne n'a
 * voté dans le groupe, et la lire telle quelle ferait apparaître un « pour »
 * que nul n'a exprimé. On compte donc d'abord les votants (pour + contre +
 * abstention, hors non-votants), puis l'égalité, avant de retenir la majorité.
 *
 * `missing` : le groupe n'a aucune ligne publiée sur ce vote. Ce n'est ni une
 * abstention ni une position.
 */
export type GroupPosition =
  | 'for'
  | 'against'
  | 'abstention'
  | 'split'
  | 'nobody'
  | 'missing'

export function voterCount(stance: StanceDto): number {
  return stance.tally.votes_for + stance.tally.votes_against + stance.tally.abstentions
}

export function groupPosition(stance: StanceDto | undefined): GroupPosition {
  if (!stance) return 'missing'
  if (voterCount(stance) === 0) return 'nobody'
  if ((stance.share?.tied_labels.length ?? 0) > 1) return 'split'
  const lead = stance.majority ?? stance.share?.leading ?? null
  if (lead === 'for' || lead === 'against' || lead === 'abstention') return lead
  // Votants présents mais aucune tête : c'est une égalité que la source n'a pas nommée.
  return 'split'
}

export function findStance(
  vote: FinalVoteDto,
  group: { uid: string; abbrev: string },
): StanceDto | undefined {
  return (
    vote.stances.find((stance) => stance.group_uid === group.uid) ??
    vote.stances.find((stance) => stance.abbrev === group.abbrev)
  )
}

const plural = (count: number, word: string) => `${count} ${word}${count > 1 ? 's' : ''}`

/** « 2 pour · 0 contre · 0 abstention » : chiffres bruts du groupe, sans adverbe. */
export function stanceCounts(stance: StanceDto): string {
  const { votes_for, votes_against, abstentions } = stance.tally
  return `${votes_for} pour · ${votes_against} contre · ${plural(abstentions, 'abstention')}`
}

export const MISSING_STANCE_NOTE = 'Aucune ligne publiée pour ce groupe sur ce vote'

const POSITION_WORDS: Record<GroupPosition, string> = {
  for: 'pour',
  against: 'contre',
  abstention: 'abstention',
  split: 'partagé',
  nobody: 'personne n’a voté',
  missing: MISSING_STANCE_NOTE.toLowerCase(),
}

/** Phrase lue et affichée en infobulle sur la tuile d'un groupe. */
export function stanceDescription(
  group: { label: string },
  stance: StanceDto | undefined,
): string {
  const position = groupPosition(stance)
  if (!stance || position === 'missing') return `${group.label} : ${MISSING_STANCE_NOTE}`
  const { votes_for, votes_against, abstentions, not_voting, voluntary_not_voting } = stance.tally
  const nonVoters = not_voting + voluntary_not_voting
  const counts = [
    `${votes_for} pour`,
    `${votes_against} contre`,
    plural(abstentions, 'abstention').replace(' ', ' '),
    ...(nonVoters > 0 ? [plural(nonVoters, 'non-votant').replace(' ', ' ')] : []),
  ].join(', ')
  return `${group.label} : ${POSITION_WORDS[position]} (${counts})`
}

/** Phrase du bandeau du groupe suivi. */
export function stanceSentence(stance: StanceDto | undefined): string {
  const position = groupPosition(stance)
  if (!stance || position === 'missing') {
    return `${MISSING_STANCE_NOTE}. Cette absence ne vaut ni abstention ni position.`
  }
  if (position === 'nobody') return 'Aucun député de ce groupe n’a voté.'
  const members = stance.member_count
  const of =
    members == null ? '' : `, sur ${members} député${members > 1 ? 's' : ''} du groupe`
  const lead = position === 'split' ? 'Le groupe s’est partagé : ' : ''
  return `${lead}${stanceCounts(stance)}${of}.`
}

/* ------------------------------------------------------------------- textes */

/** Apostrophes droites des libellés de l'API → apostrophes typographiques. */
export function typographic(text: string): string {
  return text.replaceAll("'", '’')
}

const LEADING_FORMS = [
  'proposition de loi visant à ',
  'projet de loi visant à ',
  'proposition de loi relative à ',
]

/**
 * Titre lisible d'un vote sur l'ensemble : l'intitulé du dossier quand il
 * existe, sinon le libellé officiel du texte sans sa formule d'ouverture,
 * première lettre en capitale. Déterministe, aucun modèle de langage : le
 * libellé officiel complet reste porté par l'infobulle du lien.
 */
export function voteTitle(vote: Pick<FinalVoteDto, 'dossier_label' | 'text_label'>): string {
  const dossier = vote.dossier_label?.trim()
  if (dossier) return typographic(dossier)
  let label = vote.text_label.trim()
  const lower = label.toLowerCase()
  const prefix = LEADING_FORMS.find((form) => lower.startsWith(form))
  if (prefix) label = label.slice(prefix.length)
  return typographic(label.charAt(0).toUpperCase() + label.slice(1))
}

export function capitalized(text: string): string {
  return text.charAt(0).toUpperCase() + text.slice(1)
}

export function themesSummary(labels: string[]): string {
  if (labels.length === 0) return 'aucun'
  if (labels.length === 1) return typographic(labels[0])
  return `${labels.length} sujets`
}

export function groupsSummary(followedNames: string[], total: number): string {
  if (followedNames.length === 0 || followedNames.length >= total) return 'Tous les groupes'
  if (followedNames.length === 1) return followedNames[0]
  return `${followedNames.length} groupes`
}
