import type { FinalVoteDto, GroupDto } from './finalVotes'
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
  /** Votes sur l'ensemble, au format de l'accueil (`/api/votes-finaux`). */
  final_votes: FinalVoteDto[]
  groups: GroupDto[]
  texts: WeekText[]
  previous: { monday: string; votes: number } | null
  next: string | null
  coverage_note: string
}
