/** Un jour du planning des Reels (RM-15). */
export interface ReelMethodDayDto {
  weekday: string
  kind: string
  period_label: string
}

/** Une famille de la rotation des thèmes : code et libellé public (RM-16). */
export interface ThemeRotationEntryDto {
  code: string
  label: string
}

/** Méthode des Reels : planning, critères et rotation appliqués par l'API (RM-15, RM-16). */
export interface ReelsMethodResponse {
  days: ReelMethodDayDto[]
  theme_rotation: ThemeRotationEntryDto[]
  due_time: string
  most_followed_rule: string
  completeness_rule: string
  monday_priority: string
}
