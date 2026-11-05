export interface GroupSummaryDto {
  uid: string
  abbrev: string
  /** Sigles antérieurs quand le groupe a été renommé en cours de législature. */
  former_abbrevs: string[]
  /** Libellé officiel du groupe, jamais traduit en parti. */
  label: string
  color: string | null
  legislature: number
  created_on: string | null
  dissolved_on: string | null
  dissolved: boolean
  /** Date à laquelle `member_count` est compté. */
  reference_date: string
  member_count: number
  /** Scrutins où la source publie une ligne pour ce groupe. */
  scrutin_count: number
  first_scrutin_date: string | null
  last_scrutin_date: string | null
}

export interface GroupListResponse {
  groups: GroupSummaryDto[]
  total: number
  party_note: string
  hand_vote_note: string
}

export interface QualityCountDto {
  /** Libellé de la source, conservé tel quel. */
  quality: string
  members: number
}

export interface MemberCountRangeDto {
  min: number
  max: number
  stable: boolean
}

/** `expressed` réunit les pour et les contre : le sens du vote n'est pas cumulé. */
export interface ParticipationCountsDto {
  expressed: number
  abstentions: number
  not_voting: number
  published_positions: number
}

export interface ParticipationRatesDto {
  base: number
  expressed_per_mille: number
  abstention_per_mille: number
  absence_per_mille: number
}

export interface GroupDetailResponse extends GroupSummaryDto {
  total_member_count: number
  qualities: QualityCountDto[]
  published_member_range: MemberCountRangeDto | null
  line_count: number
  reconstructed_count: number
  silent_line_count: number
  counts: ParticipationCountsDto
  /** `null` quand la source ne publie aucune position pour ce groupe. */
  rates: ParticipationRatesDto | null
  party_note: string
  rate_note: string
  comparison_note: string
  hand_vote_note: string
  reconstructed_note: string
  silent_line_note: string
}

/**
 * Le serveur sert des pour mille pour qu'une abstention à 0,4 % ne s'affiche
 * pas « 0 % » — un chiffre arrondi à zéro se lit comme une donnée manquante.
 */
export function formatPerMille(perMille: number) {
  return `${(perMille / 10).toLocaleString('fr-FR', {
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  })} %`
}

export function formatCount(count: number) {
  return count.toLocaleString('fr-FR')
}
