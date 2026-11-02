export interface TallyDto {
  votes_for: number
  votes_against: number
  abstentions: number
  not_voting: number
  /** Publié par groupe, absent de la synthèse officielle qui affiche 0. */
  voluntary_not_voting: number
}

/** Nombre de positions portées par un décompte, non-votants compris. */
export function tallySize(tally: TallyDto) {
  return (
    tally.votes_for +
    tally.votes_against +
    tally.abstentions +
    tally.not_voting +
    tally.voluntary_not_voting
  )
}

export interface ScrutinSummaryDto {
  uid: string
  number: string
  date: string
  /** Objet officiel tel que publié par l'Assemblée. */
  subject: string
  /** Libellé court calculé côté serveur ; l'objet tel quel quand aucune forme ne le reconnaît. */
  short_label: string
  ballot_type: string
  outcome_code: string
  outcome_label: string
  tally: TallyDto
  dossier_uid: string | null
  dossier_label: string | null
  has_reconstructed_tallies: boolean
  official_url: string
}

export interface ScrutinListResponse {
  total: number
  count: number
  offset: number
  scrutins: ScrutinSummaryDto[]
  coverage_note: string
}

export interface DossierScrutinsResponse {
  total: number
  count: number
  offset: number
  scrutins: ScrutinSummaryDto[]
  coverage_note: string
}

export interface VoteDto {
  actor_uid: string
  /** Absent quand l'acteur ne figure pas au référentiel : l'identifiant reste brut. */
  full_name: string | null
  official_url: string | null
  position: 'for' | 'against' | 'abstention' | 'not_voting'
  /** Code publié par la source, affiché tel quel faute de libellé officiel. */
  cause_code: string | null
  by_delegation: boolean
  seat: number | null
}

export interface GroupBreakdownDto {
  group_uid: string | null
  abbrev: string | null
  /** Libellé officiel du groupe, jamais traduit en parti. */
  label: string | null
  color: string | null
  member_count: number | null
  majority_position: string | null
  tally: TallyDto
  origin: 'published' | 'reconstructed'
  /** Mention de méthode servie quand la répartition est reconstituée. */
  method_note: string | null
  votes: VoteDto[]
}

export interface CorrectionDto {
  actor_uid: string
  full_name: string | null
  claimed_position: 'for' | 'against' | 'abstention' | 'not_voting'
  malfunction: boolean
}

export interface SynthesisDto {
  voters: number
  expressed: number
  required: number
  announcement: string
  tally: TallyDto
}

export interface ScrutinDetailDto {
  uid: string
  number: string
  legislature: number
  date: string
  session_ref: string | null
  sitting_ref: string | null
  place: string | null
  ballot_type_code: string
  ballot_type_label: string
  majority_label: string | null
  outcome_code: string
  outcome_label: string
  requester: string | null
  subject: string
  synthesis: SynthesisDto
  groups: GroupBreakdownDto[]
  corrections: CorrectionDto[]
  dossier_uid: string | null
  dossier_label: string | null
  official_url: string
  unknown_actors: number
  coverage_note: string
}

export const positionLabels: Record<VoteDto['position'], string> = {
  for: 'Pour',
  against: 'Contre',
  abstention: 'Abstention',
  not_voting: 'Non-votant',
}

/** Fond, texte et anneau d'une pastille de position. Voir `Pill` dans `ui.tsx`. */
export const positionClasses: Record<VoteDto['position'], string> = {
  for: 'bg-yes-soft text-yes ring-yes/15',
  against: 'bg-no-soft text-no ring-no/15',
  abstention: 'bg-abstain-soft text-abstain ring-abstain/15',
  not_voting: 'bg-surface-soft text-ink-soft ring-line',
}

export function formatDate(iso: string) {
  return new Date(iso + 'T00:00:00').toLocaleDateString('fr-FR', {
    day: 'numeric',
    month: 'long',
    year: 'numeric',
  })
}

/** Forme courte, pour les listes où la date est une métadonnée et non un fait. */
export function formatDateShort(iso: string) {
  return new Date(iso + 'T00:00:00').toLocaleDateString('fr-FR', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
  })
}
