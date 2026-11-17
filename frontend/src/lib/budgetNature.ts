import type { BudgetNatureDto } from '../types/budget'
import type { Slice } from './budgetSlices'

/*
 * Nature de la dépense : titre de la nomenclature budgétaire (LOLF, art. 5).
 * Libellés simples validés le 16/11/2026 ; le libellé officiel reste dans
 * l'API. Les titres 1 (pouvoirs publics) et 7 (opérations financières), qui
 * pèsent moins de 0,5 % ensemble, sont regroupés en « Autres ».
 *
 * La couleur suit la nature, pas le rang : le personnel est bleu partout,
 * dans l'anneau de l'exercice comme dans la barre d'une mission.
 */

export type NatureKey =
  | 'personnel'
  | 'interventions'
  | 'fonctionnement'
  | 'dette'
  | 'investissement'
  | 'remboursements'
  | 'autres'

interface NatureInfo {
  label: string
  /** Sous-titre de la légende. */
  hint: string
  /** « des interventions », pour une phrase. */
  partitive: string
  colorIndex: number
}

export const NATURES: Record<NatureKey, NatureInfo> = {
  personnel: {
    label: 'Personnel',
    hint: 'salaires et cotisations des agents',
    partitive: 'du personnel',
    colorIndex: 0,
  },
  interventions: {
    label: 'Interventions',
    hint: 'aides et transferts versés à d’autres',
    partitive: 'des interventions',
    colorIndex: 1,
  },
  fonctionnement: {
    label: 'Fonctionnement',
    hint: 'achats, loyers, subventions aux opérateurs',
    partitive: 'du fonctionnement',
    colorIndex: 2,
  },
  dette: {
    label: 'Charge de la dette',
    hint: 'intérêts et frais de la dette',
    partitive: 'de la charge de la dette',
    colorIndex: 3,
  },
  investissement: {
    label: 'Investissement',
    hint: 'équipements, bâtiments, armement',
    partitive: 'de l’investissement',
    colorIndex: 4,
  },
  remboursements: {
    label: 'Remboursements d’impôts',
    hint: 'mission « Remboursements et dégrèvements »',
    partitive: 'des remboursements d’impôts',
    colorIndex: 6,
  },
  autres: {
    label: 'Autres',
    hint: 'pouvoirs publics, opérations financières',
    partitive: 'd’autres dépenses',
    colorIndex: 7,
  },
}

const KEY_OF_TITLE: Record<number, NatureKey> = {
  1: 'autres',
  2: 'personnel',
  3: 'fonctionnement',
  4: 'dette',
  5: 'investissement',
  6: 'interventions',
  7: 'autres',
}

/** Une part de la répartition par nature : ses titres d'origine, catégories comprises. */
export interface NaturePart {
  key: NatureKey
  euros: number
  natures: BudgetNatureDto[]
}

/**
 * Titres regroupés par nature, du plus gros au plus petit, « Autres » en
 * dernier ; parts à 0 € écartées. `refunds` ajoute la part des
 * remboursements d'impôts, avant « Autres ».
 */
export function natureParts(natures: BudgetNatureDto[], refunds: number | null = null): NaturePart[] {
  const parts = new Map<NatureKey, NaturePart>()
  for (const nature of natures) {
    const key = KEY_OF_TITLE[nature.title] ?? 'autres'
    const part = parts.get(key) ?? { key, euros: 0, natures: [] }
    part.euros += nature.cp_euros
    part.natures.push(nature)
    parts.set(key, part)
  }
  const main = [...parts.values()]
    .filter((part) => part.key !== 'autres' && part.euros > 0)
    .sort((a, b) => b.euros - a.euros)
  if (refunds !== null && refunds > 0) {
    main.push({ key: 'remboursements', euros: refunds, natures: [] })
  }
  const others = parts.get('autres')
  if (others && others.euros > 0) main.push(others)
  return main
}

export function natureSlices(parts: NaturePart[]): Slice<NaturePart>[] {
  return parts.map((part) => ({
    key: part.key,
    label: NATURES[part.key].label,
    euros: part.euros,
    colorIndex: NATURES[part.key].colorIndex,
    items: [part],
  }))
}
