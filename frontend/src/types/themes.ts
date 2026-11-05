export interface FamilyDto {
  code: string
  label: string
  scope: string
}

export interface FamiliesResponse {
  families: FamilyDto[]
  max_families_per_text: number
  method_note: string
}

export interface AssignedFamilyDto {
  code: string
  label: string
  opened_on: string
  motive: string | null
}

export interface TextSummaryDto {
  key: string
  label: string
  scrutin_count: number
  first_vote: string | null
  last_vote: string | null
  dossier_uid: string | null
  dossier_label: string | null
  families: AssignedFamilyDto[]
  /** `no_family` : le modèle n'a rien retenu. `failed` : il n'a pas répondu. */
  last_attempt_outcome: string | null
}

export interface TextListResponse {
  items: TextSummaryDto[]
  total: number
  offset: number
  method_note: string
}

export interface TextScrutinDto {
  uid: string
  number: string
  date: string
  subject: string
  outcome_label: string
  votes_for: number
  votes_against: number
  abstentions: number
}

export interface AssignmentHistoryDto {
  code: string
  label: string
  opened_on: string
  closed_on: string | null
  author: string
  motive: string | null
}

export interface ProposedFamilyDto {
  code: string
  label: string
  justification: string
}

export interface ProposalDto {
  model: string
  prompt_version: string
  produced_on: string
  families: ProposedFamilyDto[]
}

export interface TextDetailResponse extends TextSummaryDto {
  scrutins: TextScrutinDto[]
  history: AssignmentHistoryDto[]
  proposal: ProposalDto | null
  method_note: string
}

export interface FamilyCoverageDto {
  code: string
  label: string
  text_count: number
  scrutin_count: number
}

export interface ThemeRuleDto {
  id: string
  /** Fragment cherché dans le libellé normalisé du texte. */
  marker: string
  /** Le fragment doit former un mot entier dans le libellé. */
  whole_word: boolean
  families: FamilyDto[]
  /** Énoncé publié, conservé comme justification du rattachement. */
  statement: string
}

/** Liens établis et absences de lien du rapprochement scrutins–amendements. */
export interface AmendmentLinkingDto {
  linked: number
  /** Parmi les liens, ceux où l'article cité a départagé plusieurs candidats. */
  linked_by_cited_article: number
  /** Une entrée par cause d'absence de lien, avec le libellé du domaine. */
  absences: LinkAbsenceDto[]
}

export interface LinkAbsenceDto {
  code: 'no_number' | 'no_dossier' | 'unresolved' | 'ambiguous' | 'author_mismatch'
  label: string
  count: number
}

export interface MethodResponse {
  families: FamilyCoverageDto[]
  max_families_per_text: number
  /** Table des règles publiée telle qu'elle s'applique (RM-13). */
  rules: ThemeRuleDto[]
  texts_total: number
  texts_assigned: number
  texts_without_family: number
  texts_attempt_failed: number
  texts_never_attempted: number
  scrutins_total: number
  scrutins_with_text: number
  scrutins_assigned: number
  scrutins_without_text: number
  dossiers_total: number
  dossiers_linked_to_text: number
  dossiers_assigned: number
  /** Dossiers qu'aucun scrutin ne relie à un texte. */
  dossiers_without_text: number
  amendment_linking: AmendmentLinkingDto
  /** Ce que recouvre « non résolu », dit sous le compteur. */
  amendment_linking_note: string
  extraction_rule: string
  model_scope: string
  rule_scope: string
  method_note: string
}
