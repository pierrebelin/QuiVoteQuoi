/** Origine du groupe affiché à côté d'un signataire (README.md §3.2). */
export type GroupOrigin = 'published' | 'resolved_at_deposit' | 'unknown'

export interface AmendmentDto {
  uid: string
  number: string
  target_title: string
  target_kind: string | null
  author_kind: 'deputy' | 'institutional'
  author_actor_uid: string | null
  author_name: string | null
  author_official_url: string | null
  /** Groupe du signataire **à la date de dépôt**, jamais son groupe actuel. */
  author_group_label: string | null
  author_group_abbrev: string | null
  /** Couleur publiée du groupe au dépôt (RM-10), null si le groupe n'en publie pas. */
  author_group_color: string | null
  /** Ton du texte lisible sur la couleur du groupe. */
  author_text_tone: 'white' | 'black' | null
  /** Aperçu de l'exposé (RM-05), null si la source n'en publie pas. */
  summary_preview: string | null
  summary_preview_truncated: boolean
  /** Sort tel qu'affiché sur la carte (RM-08). */
  fate_banner: string
  author_group_origin: GroupOrigin
  author_group_ambiguous: boolean
  fate_code: string
  fate_label: string
  state_label: string | null
  deposited_on: string | null
  /** Exposé sommaire entier. Le serveur ne le tronque jamais (RM-03). */
  summary: string | null
  cosignatory_count: number
}

export interface AmendmentCoverageDto {
  /** Amendements en base, tous dossiers confondus. */
  base_total: number
  total: number
  without_summary: number
  unknown_fates: number
}

export interface DossierAmendmentsResponse {
  total: number
  count: number
  offset: number
  limit: number
  amendments: AmendmentDto[]
  groups: AmendmentGroupDto[]
  coverage: AmendmentCoverageDto
  coverage_note: string
  pagination_note: string
  source_note: string
}

export interface AmendmentGroupDto {
  uid: string
  label: string
  abbrev: string
}
