/**
 * Sort d'un dossier, tel que la source l'atteste.
 *
 * `no_recorded_conclusion` est le cas majoritaire — 2 788 des 3 035 dossiers de
 * la législature 17 — et n'est pas un synonyme d'abandon : l'Assemblée ne
 * publie aucun acte de clôture pour un texte déposé et jamais inscrit. Le
 * dernier acte du dossier reste la seule information disponible.
 */
export type OutcomeKind =
  | 'promulgated'
  | 'withdrawn'
  | 'merged_into'
  | 'rejected'
  | 'no_recorded_conclusion'

export interface OutcomeDto {
  kind: OutcomeKind
  /** Libellé prêt à afficher. Pour un rejet, c'est le mot de la source. */
  label: string
  date: string | null
  /** Faux pour un rejet : la navette peut reprendre. */
  is_final: boolean
  law_code: string | null
  law_jo_date: string | null
  legifrance_url: string | null
  merged_into_uid: string | null
}

export interface InitiatorGroupDto {
  uid: string
  abbrev: string
  label: string
  quality: string | null
}

export interface InitiatorDto {
  full_name: string
  actor_uid: string | null
  role: string | null
  group: InitiatorGroupDto | null
  /** Date à laquelle le groupe a été lu. Toujours affichée avec le groupe. */
  reference_date: string | null
  official_url: string | null
}
