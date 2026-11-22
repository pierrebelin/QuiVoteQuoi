import type { FinalVoteDto, GroupDto } from './finalVotes'
import type { DossierStatusDto, DecisionSense, LawChamber, LawState, LawStateKind } from './dossiers'
import type { ScrutinSummaryDto } from './scrutins'
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

/** Événement de la semaine d'un texte qui a avancé (RM-08). */
export interface WeekEvent {
  date: string
  /** Absent pour une promulgation. */
  chamber: LawChamber | null
  /** Code de lecture de la source (`AN1`, `SN2`, `CMP`…), `PROM` pour une promulgation. */
  reading: string
  sense: DecisionSense | null
  has_public_vote: boolean
  joint_committee_agreement_date: string | null
}

/** Texte qui a avancé, rangé selon son état au soir du dimanche (RM-08, Q4). */
export interface AdvancedText {
  dossier_uid: string
  title: string
  group: LawStateKind
  law_state: LawState
  event: WeekEvent
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
  /** Devenus loi, adoptés définitivement, en navette ; le plus récent d'abord. */
  advanced: AdvancedText[]
  advanced_count: number
}
