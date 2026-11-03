export type FreshnessStatus = 'up_to_date' | 'late' | 'never_synchronised'

export interface SourceFreshness {
  source: string
  label: string
  status: FreshnessStatus
  status_label: string
  last_success_at: string | null
  last_attempt_at: string | null
  last_attempt_succeeded: boolean | null
  consecutive_failures: number
}

export interface FreshnessResponse {
  sources: SourceFreshness[]
  stale_sources: string[]
  last_scrutin_date: string | null
}
