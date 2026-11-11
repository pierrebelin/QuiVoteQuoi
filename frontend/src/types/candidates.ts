export interface PoliticalOrganizationDto {
  label: string
  official_url: string | null
  source_url: string
  source_label: string
}

/** Une candidature n'apparaît qu'avec une déclaration publique primaire. */
export interface CandidateDto {
  id: string
  display_name: string
  declared_on: string
  declaration_source_url: string
  declaration_source_label: string
  official_site_url: string | null
  program_url: string | null
  /** Au moins un extrait de programme, tous thèmes confondus : sans, rien à comparer. */
  has_program_excerpts: boolean
  political_organizations: PoliticalOrganizationDto[]
}

/** Extrait attribué à un programme : jamais une synthèse ni une note du site. */
export interface CandidateProgramProposalDto {
  candidate_id: string
  theme_code: string
  excerpt: string
  source_url: string
  source_label: string
  source_published_on: string | null
}

/** Association explicitement sourcée ; parti et groupe restent distincts. */
export interface CandidateParliamentaryGroupDto {
  candidate_id: string
  group_uid: string
  abbrev: string
  label: string
  color: string | null
  linked_on: string
  source_url: string
  source_label: string
}

export interface CandidateComparisonResponse {
  candidates: CandidateDto[]
  selected: CandidateDto[]
  proposals: CandidateProgramProposalDto[]
  parliamentary_groups: CandidateParliamentaryGroupDto[]
  selected_theme: string | null
  max_compared_candidates: number
  declaration_note: string
  proposals_note: string
  groups_note: string
}
