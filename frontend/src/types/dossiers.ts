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

/**
 * Où en est la loi (todo/navette/SPEC-navette.md) : le seul type d'état,
 * partagé par toutes les pages. Un dossier sans état (rejeté sans adoption,
 * retiré, fusionné, sans décision) reçoit `null` (RM-04).
 */
export type LawStateKind = 'in_shuttle' | 'definitively_adopted' | 'became_law'

/** `cmp` : commission mixte paritaire. */
export type LawChamber = 'an' | 'sn' | 'cmp'

export type DecisionSense =
  | 'adopted'
  | 'adopted_unchanged'
  | 'modified'
  | 'rejected'
  | 'cmp_agreement'
  | 'cmp_disagreement'

export interface LawDecision {
  /** `null` quand la source ne nomme pas la chambre. */
  chamber: LawChamber | null
  sense: DecisionSense
  date: string
}

export interface LawState {
  kind: LawStateKind
  /** Dernière décision, décision qui donne le même texte, ou promulgation. */
  date: string
  last_decision: LawDecision | null
  /** Devenu loi seulement. */
  law_code: string | null
  jo_date: string | null
  legifrance_url: string | null
}

export type MilestoneStep = 'chamber' | 'agreement' | 'law'
export type MilestoneStatus = 'done' | 'current' | 'upcoming'
export type AgreementKind = 'same_text' | 'joint_committee' | 'final_reading'

/** Un des quatre jalons (RM-05) ; les sous-titres se composent à l'affichage. */
export interface Milestone {
  step: MilestoneStep
  /** Étape de chambre : `an` ou `sn`. */
  chamber: 'an' | 'sn' | null
  status: MilestoneStatus
  sense: DecisionSense | null
  date: string | null
  agreement: AgreementKind | null
  law_code: string | null
  jo_date: string | null
  /** Fiche de vote : l'étape tranchée par ce scrutin (« · ce vote »). */
  is_this_vote: boolean
}

/** « Ce texte est-il devenu loi ? » (RM-06) : une phrase sans le lien Légifrance. */
export interface LawAnswer {
  kind: 'not_yet' | 'soon' | 'yes'
  sentence: string
}

/** État, jalons et réponse d'un dossier lu avec ses actes. */
export interface LawProgress {
  law_state: LawState | null
  milestones: Milestone[]
  answer: LawAnswer | null
}

/** Compteurs des trois états sur le périmètre affiché (Q2). */
export type LawStateCounts = Record<LawStateKind, number>

/** État d'un dossier tel que les listes le reçoivent (scrutins, semaine, dossiers). */
export interface DossierStatusDto {
  outcome: OutcomeDto
  current_stage: { label: string; chamber: string } | null
  last_activity_label: string
  last_activity_date: string
  law_state: LawState | null
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

/** « Proposition de loi » ou « Projet de loi », lu sur la procédure ; « Texte » sinon. */
export function textKind(procedure: string): string {
  if (procedure.startsWith('Projet de loi')) return 'Projet de loi'
  if (procedure.startsWith('Proposition de loi')) return 'Proposition de loi'
  return 'Texte'
}
