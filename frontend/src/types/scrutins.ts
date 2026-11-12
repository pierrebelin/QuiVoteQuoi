import type { OutcomeDto } from './dossiers'
import type { AssignedFamilyDto } from './themes'

/**
 * L'API envoie aussi `voluntary_not_voting`, volontairement absent d'ici : au
 * niveau du groupe, la source y recopie les abstentions. L'additionner compterait
 * deux fois les mêmes personnes.
 */
export interface TallyDto {
  votes_for: number
  votes_against: number
  abstentions: number
  not_voting: number
}

/** Nombre de positions portées par un décompte, non-votants compris. */
export function tallySize(tally: TallyDto) {
  return tally.votes_for + tally.votes_against + tally.abstentions + tally.not_voting
}

export type LeadingPosition = 'for' | 'against' | 'abstention'

/**
 * Position qui a réuni le plus de députés d'un décompte, lue sur ses comptes bruts.
 * `null` quand personne ne s'est prononcé ou que deux positions sont à égalité :
 * aucune position n'est alors choisie au hasard.
 */
export function leadingPosition(tally: TallyDto): LeadingPosition | null {
  const { votes_for, votes_against, abstentions } = tally
  const counts: [LeadingPosition, number][] = [
    ['for', votes_for],
    ['against', votes_against],
    ['abstention', abstentions],
  ]
  const top = Math.max(votes_for, votes_against, abstentions)
  const leaders = counts.filter(([, n]) => n === top)
  return top > 0 && leaders.length === 1 ? leaders[0][0] : null
}

/** État d'un dossier tel que la liste des scrutins le reçoit. */
export interface DossierStatusDto {
  outcome: OutcomeDto
  current_stage: { label: string; chamber: string } | null
  last_activity_label: string
  last_activity_date: string
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
  /**
   * Où en est le dossier rattaché : mêmes sort et étape que la liste des
   * dossiers. Absent sans dossier, ou quand le dossier n'est pas en base.
   */
  dossier_status: DossierStatusDto | null
  has_reconstructed_tallies: boolean
  official_url: string
  /** Amendements rapprochés, dans l'ordre de citation ; vide si le scrutin n'est pas rapproché. */
  cited: CitedAmendmentDto[]
  /** Familles rattachées ; absent d'un serveur antérieur à l'ajout. */
  families?: AssignedFamilyDto[]
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

export interface CitedAmendmentDto {
  dossier_uid: string
  number: string
  target_title: string
  /** Exposé entier : servi par le détail seulement, null dans la liste. */
  summary: string | null
  /** Aperçu de l'exposé (RM-05), null si la source n'en publie pas. */
  summary_preview: string | null
  summary_preview_truncated: boolean
  author_name: string | null
  author_group_abbrev: string | null
  /** Couleur publiée du groupe au dépôt (RM-10), null si le groupe n'en publie pas. */
  author_group_color: string | null
  /** Ton du texte lisible sur la couleur du groupe. */
  author_text_tone: 'white' | 'black' | null
  cosignatory_count: number
}

export interface CitedNumberDto {
  rank: number
  cited_number: string | null
  amendment: CitedAmendmentDto | null
  method_labels: string[]
  cause_code: string | null
  cause_label: string | null
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
  cited: CitedNumberDto[]
  unnumbered_identical_amendments: boolean
  coverage_note: string
  /** Familles héritées du texte mis aux voix ; vide tant qu'il n'est rattaché à aucune. */
  families: AssignedFamilyDto[]
  /** Mention de lecture d'un vote sur l'ensemble (« lecture définitive »), sinon `null`. */
  reading: string | null
  /** Explications de vote ; `null` hors vote sur l'ensemble. */
  explanations: Explanations | null
}

export type ExplanationsState =
  | 'report_not_published'
  | 'not_attached'
  | 'no_explanation'
  | 'explained'

export type GroupBasis = 'scrutin' | 'membership' | 'undetermined'

export interface Explanation {
  rank: number
  speaker_uid: string
  speaker_name: string | null
  /** `null` : groupe non établi, jamais deviné. */
  group_uid: string | null
  group_basis: GroupBasis
  /** `null` : position non publiée. */
  speaker_position: VoteDto['position'] | null
  group_tally: TallyDto | null
  /** `null` : pas d'aperçu, le texte tient en entier. */
  preview: string | null
  preview_truncated: boolean
  text: string
  official_url: string
}

export interface GroupWithoutSpeaker {
  group_uid: string
  abbrev: string | null
  label: string | null
}

export interface Explanations {
  state: ExplanationsState
  source_url: string | null
  report_version: string | null
  items: Explanation[]
  groups_without_speaker: GroupWithoutSpeaker[]
}

export const explanationsStateLabels: Record<Exclude<ExplanationsState, 'explained'>, string> = {
  report_not_published: "Le compte rendu de cette séance n'est pas encore publié.",
  not_attached: "Ce vote n'est pas rattaché à un passage du compte rendu.",
  no_explanation: "Aucune explication de vote n'a été prononcée avant ce vote.",
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
