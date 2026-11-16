/*
 * Parts des anneaux du budget (BudgetDonut) : les plus grosses une à une, le
 * reste regroupé. Les couleurs `--chart-1` à `--chart-6` (index.css) suivent
 * le rang de la part, jamais une couleur de groupe politique.
 */

/** Nombre de parts nommées avant la part « autres ». */
export const NAMED_SLICES = 5

export interface Slice<T> {
  key: string
  label: string
  euros: number
  /** Rang de couleur, 0 à `NAMED_SLICES` (la part « autres » prend le dernier). */
  colorIndex: number
  /** Éléments regroupés dans la part : un seul pour une part nommée. */
  items: T[]
}

/**
 * Couleur d'un rang. Au-delà de la part « autres », deux gris réservés à la
 * répartition par nature : 6 = remboursements d'impôts, 7 = autres titres.
 */
export function sliceColor(colorIndex: number): string {
  if (colorIndex <= NAMED_SLICES) return `var(--chart-${colorIndex + 1})`
  return colorIndex === NAMED_SLICES + 1 ? 'var(--color-ink-faint)' : 'var(--color-line-strong)'
}

/**
 * Parts nommées puis part « autres » ; `items` déjà triés par montant
 * décroissant. La part « autres » garde le gris même pour un seul élément :
 * une sixième couleur ne se distinguerait plus.
 */
export function toSlices<T>(
  items: T[],
  describe: (item: T) => { key: string; label: string; euros: number },
  otherLabel: (count: number) => string,
): Slice<T>[] {
  const named = items.slice(0, NAMED_SLICES)
  const rest = items.slice(named.length)
  const slices: Slice<T>[] = named.map((item, index) => ({
    ...describe(item),
    colorIndex: index,
    items: [item],
  }))
  if (rest.length > 0) {
    slices.push({
      key: 'autres',
      label: otherLabel(rest.length),
      euros: rest.reduce((sum, item) => sum + describe(item).euros, 0),
      colorIndex: NAMED_SLICES,
      items: rest,
    })
  }
  return slices
}
