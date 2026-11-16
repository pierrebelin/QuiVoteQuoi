/*
 * Budget de l'État voté en loi de finances initiale (`GET /api/budget` et
 * `GET /api/budget/{year}`). Montants en euros entiers, tels que la source
 * officielle les donne : arrondis et parts se calculent à l'affichage.
 */

export type BudgetAccountKind = 'budget_general' | 'concours_financiers'

export type AdoptionProcedure = 'vote' | 'article_49_3'

export interface BudgetProgrammeDto {
  number: number
  name: string
  cp_euros: number
}

export interface BudgetMissionDto {
  name: string
  account_kind: BudgetAccountKind
  theme: { code: string; label: string } | null
  cp_euros: number
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
    /** Triées par montant décroissant. */
    missions: BudgetMissionDto[]
  }
  /** Comptes spéciaux retenus, hors du total du budget général. */
  off_budget: BudgetMissionDto[]
  /** Exercices disponibles, du plus récent au plus ancien. */
  available_years: number[]
}
