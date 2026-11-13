import type { DossierStatusDto, ScrutinSummaryDto } from './scrutins'
import type { AssignedFamilyDto } from './themes'

export type WeekDayState = 'voted' | 'no_public_vote' | 'not_yet_published' | 'upcoming'

export interface WeekOtherOutcome {
  label: string
  count: number
}

export interface WeekTotals {
  votes: number
  adopted: number
  rejected: number
  others: WeekOtherOutcome[]
}

export interface WeekDay {
  date: string
  votes: number
  state: WeekDayState
}

export interface WeekGroupTally {
  votes_for: number
  votes_against: number
  abstentions: number
  not_voting: number
  voluntary_not_voting: number
}

export interface WeekGroupStance {
  group_uid: string
  abbrev: string
  label: string
  color: string | null
  member_count: number | null
  majority: string | null
  majority_label: string | null
  tally: WeekGroupTally
}

export interface WeekFamily {
  family: string
  opened_on: string
}

export interface WeekFinalVote {
  scrutin_uid: string
  number: string
  date: string
  text_label: string
  outcome_code: string
  outcome_label: string
  families: WeekFamily[]
  groups: WeekGroupStance[]
}

export interface WeekGroup {
  uid: string
  abbrev: string
  label: string
  color: string | null
  final_vote_count: number
}

export interface WeekLatestFinal {
  scrutin_uid: string
  date: string
  number: string
  outcome_code: string
  outcome_label: string
}

/** Compteurs d'une carte ou d'un jour : issues et dernier vote sur l'ensemble. */
export interface WeekCardTally extends WeekTotals {
  latest_final: WeekLatestFinal | null
}

export interface WeekText {
  dossier_uid: string | null
  dossier_label: string | null
  dossier_status: DossierStatusDto | null
  families: AssignedFamilyDto[]
  totals: WeekCardTally
  days: { date: string; tally: WeekCardTally }[]
  scrutins: ScrutinSummaryDto[]
}

export interface WeekResponse {
  monday: string
  sunday: string
  is_current: boolean
  current_week_empty: boolean
  last_published: string | null
  totals: WeekTotals
  texts_count: number
  days: WeekDay[]
  final_votes: WeekFinalVote[]
  groups: WeekGroup[]
  texts: WeekText[]
  previous: { monday: string; votes: number } | null
  next: string | null
  coverage_note: string
}
