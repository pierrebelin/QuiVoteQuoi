import type { AssignedFamilyDto } from './themes'

export interface TallyDto {
  votes_for: number
  votes_against: number
  abstentions: number
  not_voting: number
  voluntary_not_voting: number
}

/** Parts rapportées aux seuls votants du groupe (pour + contre + abstention). */
export interface ShareDto {
  voters: number
  for_percent: number
  against_percent: number
  abstention_percent: number
  /** Position rassemblant le plus de votants. `null` à égalité. */
  leading: string | null
  leading_label: string | null
  leading_percent: number | null
  /** Positions à égalité en tête. Un seul élément hors égalité. */
  tied_labels: string[]
}

export interface GroupDto {
  uid: string
  abbrev: string
  label: string
  color: string | null
  /** Votes sur l'ensemble où le groupe apparaît. */
  final_vote_count: number
}

export interface StanceDto {
  group_uid: string
  abbrev: string
  label: string
  color: string | null
  member_count: number | null
  /** `for` / `against` / `abstention`, publié par la source, jamais recalculé. */
  majority: string | null
  majority_label: string | null
  tally: TallyDto
  /** `null` quand aucun membre du groupe ne s'est prononcé. */
  share: ShareDto | null
}

export interface FinalVoteDto {
  scrutin_uid: string
  number: string
  date: string
  ballot_type_label: string
  text_key: string
  text_label: string
  reading: string | null
  outcome_code: string
  outcome_label: string
  adopted: boolean
  dossier_uid: string | null
  dossier_label: string | null
  /** Version exacte seulement lorsqu'un acte officiel établit le rattachement. */
  official_text: OfficialTextVersionDto | null
  synthesis: TallyDto
  families: AssignedFamilyDto[]
  /**
   * Positions des groupes comparés (`groupes=`), ou de tous les groupes du
   * référentiel avec `tous_groupes=true` (exclusif avec `groupes`). Un groupe
   * sans ligne publiée sur ce vote en est absent : rien n'est comblé par un
   * zéro, et l'absence n'est ni une abstention ni une position.
   */
  stances: StanceDto[]
}

export interface OfficialTextVersionDto {
  document_uid: string
  document_title: string
  version_label: string
  document_published_on: string | null
  official_url: string
  mapping_source_url: string
  source_producer: string
  source_license: string
  source_metadata_fingerprint: string | null
  source_retrieved_at: string
}

export interface FinalVoteListResponse {
  items: FinalVoteDto[]
  total: number
  total_unfiltered: number
  total_with_family: number
  offset: number
  groups: GroupDto[]
  selected: GroupDto[]
  max_compared_groups: number
  scope_note: string
  share_note: string
  outcome_note: string
}
