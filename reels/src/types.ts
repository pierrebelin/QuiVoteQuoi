import type { ScrutinDetailDto } from '../../frontend/src/types/scrutins'

/**
 * Miroir de `ReelResponse` (src/api/reel_dto.rs), réponse de `GET /api/reels/{date}`.
 * Les vidéos n'affichent que ces champs et le détail `/api/scrutins/{uid}` (RM-10).
 */
export type ReelStatus = 'ready' | 'skipped' | 'incomplete'
export type ReelKind = 'veille' | 'semaine' | 'lundi'
export type ReelReason = 'no_vote' | 'missing_numbers' | 'not_synchronised'

export type ReelPeriodDto = {
  from: string
  to: string
  label: string
}

export type OtherOutcomeDto = {
  label: string
  count: number
}

export type OutcomeCountDto = {
  votes: number
  adopted: number
  rejected: number
  others: OtherOutcomeDto[]
}

export type ReelDayDto = {
  date: string
  votes: number
}

export type ReelFinalVoteDto = {
  scrutin_uid: string
  number: string
  date: string
  text_label: string
  outcome_code: string
  outcome_label: string
}

export type ReelTallyDto = {
  votes_for: number
  votes_against: number
  abstentions: number
}

export type MostFollowedDto = {
  uid: string
  number: string
  date: string
  subject: string
  outcome_label: string
  tally: ReelTallyDto
}

export type ReelProgrammeDto = {
  candidate_id: string
  name: string
  added: number
  removed: number
  themes: string[]
}

export type ReelSiteNewsDto = {
  id: string
  title: string
  text: string
  path: string
}

export type ReelThemeDto = {
  code: string
  label: string
  text_count: number
  scrutin_count: number
}

export type ReelResponse = {
  status: ReelStatus
  kind: ReelKind
  last_success_at: string | null
  reason: ReelReason | null
  empty_week_note: string | null
  reconstructed_note: string | null
  period: ReelPeriodDto
  /** 18 h à Paris le jour de préparation, RFC 3339. */
  due_at: string
  totals: OutcomeCountDto | null
  days: ReelDayDto[]
  final_votes: ReelFinalVoteDto[]
  most_followed: MostFollowedDto | null
  programmes: ReelProgrammeDto[]
  site_news: ReelSiteNewsDto | null
  theme: ReelThemeDto | null
  /** Légende ; `[lien]` y marque la place du lien (LINK_TOKEN). */
  caption: string | null
  link_path: string | null
  tags: string[]
}

/** Propriétés d'une composition, écrites par `scripts/prepare.mjs`. */
export type ReelProps = {
  reel: ReelResponse
  /** Détail publié de chaque scrutin montré, par uid. */
  details: Record<string, ScrutinDetailDto>
  /** Libellés publiés des thèmes (`/api/themes`), par code. */
  themeLabels: Record<string, string>
  /** Adresse publique du site, sans protocole (carte finale). */
  site: string
  /** Capture de la nouveauté, dans `public/` ; `null` hors nouveauté. */
  capture: string | null
  /** Piste `public/piste.mp3` présente. */
  audio: boolean
}

export type { ScrutinDetailDto }
