import type { TallyDto } from './scrutins'

export interface DossierGroupActionsResponse {
  dossier_uid: string
  period_start: string | null
  period_end: string | null
  groups: GroupActionsDto[]
  notes: string[]
}

export interface GroupActionsDto {
  uid: string
  abbrev: string
  label: string
  color: string | null
  start_date: string | null
  end_date: string | null
  state: 'no_data' | 'ready' | 'summary_pending' | 'summary_unavailable'
  summary: SummaryDto | null
  final_votes: FinalVoteDto[]
  amendment_count: number
  /** Amendements du groupe par sort, libellé de la source, du plus fréquent au plus rare. */
  amendment_fates?: FateCountDto[]
  amendments_url: string
}

export interface FateCountDto {
  /** Vide quand la source ne publie aucun sort. */
  label: string
  count: number
}

export interface SummaryDto {
  label: 'Synthèse automatique'
  status: 'ready'
  text: string
  model: string | null
  prompt_version: string | null
  generated_at: string | null
  sources: SummarySourceDto[]
}

export interface SummarySourceDto {
  id: string
  kind: string
  uid: string
  label: string
  official_url: string | null
}

export interface FinalVoteDto {
  scrutin_uid: string
  number: string
  date: string
  subject: string
  text_label: string
  reading: string | null
  outcome_code: string
  outcome_label: string
  majority_position: string | null
  member_count: number | null
  tally: TallyDto
  official_url: string
  /** Orateurs rattachés à ce scrutin sous ce groupe, dans l'ordre du compte rendu (RM-09). */
  explanations: ExplanationMentionDto[]
}

export interface ExplanationMentionDto {
  speaker_uid: string
  speaker_name: string | null
  scrutin_uid: string
  rank: number
}
