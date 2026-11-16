/*
 * Budget de l'État voté en loi de finances initiale (`GET /api/budget` et
 * `GET /api/budget/{year}`). Montants en euros entiers, tels que la source
 * officielle les donne : arrondis et parts se calculent à l'affichage.
 */

export type BudgetAccountKind = 'budget_general' | 'concours_financiers'

export type AdoptionProcedure = 'vote' | 'article_49_3'

/** Catégorie de dépense (deux chiffres, le premier est le titre) et son libellé officiel. */
export interface BudgetCategoryDto {
  code: number
  label: string
  cp_euros: number
}

/**
 * Nature de la dépense : titre de la LOLF (art. 5), de 1 à 7, libellé
 * officiel et catégories. Seuls les titres présents sont listés, du 1 au 7.
 */
export interface BudgetNatureDto {
  title: number
  label: string
  cp_euros: number
  categories: BudgetCategoryDto[]
}

export interface BudgetActionDto {
  /** Code de l'action dans son programme, « 01 ». */
  code: string
  name: string
  cp_euros: number
}

export interface BudgetProgrammeDto {
  number: number
  name: string
  cp_euros: number
  nature: BudgetNatureDto[]
  /** Triées par montant décroissant ; vide pour un programme à 0 €. */
  actions: BudgetActionDto[]
}

export interface BudgetMissionDto {
  name: string
  account_kind: BudgetAccountKind
  theme: { code: string; label: string } | null
  cp_euros: number
  /** Ce que paient ses crédits. */
  nature: BudgetNatureDto[]
  programmes: BudgetProgrammeDto[]
}

export interface BudgetExerciseDto {
  year: number
  stage: 'lfi'
  law: {
    number: string
    /** Date de la loi, `AAAA-MM-JJ`. */
    date: string
    credits_article: number
    adoption_procedure: AdoptionProcedure
    legifrance_url: string
  }
  source: {
    title: string
    url: string
    published_on: string
  }
  general_budget: {
    cp_euros: number
    /** Mission « Remboursements et dégrèvements » ; `null` si l'exercice n'en a pas. */
    refunds_and_rebates_cp_euros: number | null
    /**
     * Nature des dépenses sans la mission des remboursements et dégrèvements :
     * sa somme et `refunds_and_rebates_cp_euros` redonnent `cp_euros`.
     */
    nature_without_refunds: BudgetNatureDto[]
    /** Triées par montant décroissant. */
    missions: BudgetMissionDto[]
  }
  /** Comptes spéciaux retenus, hors du total du budget général. */
  off_budget: BudgetMissionDto[]
  /** Exercices disponibles, du plus récent au plus ancien. */
  available_years: number[]
}
