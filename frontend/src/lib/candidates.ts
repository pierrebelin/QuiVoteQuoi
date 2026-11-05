import type { CandidateDto } from '../types/candidates'

/** Minuscules sans accents, pour comparer une saisie à un nom. */
export function normalizeText(text: string): string {
  return text.normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase()
}

/** Articles qui font partie du nom de famille : « Le Pen », pas « Pen ». */
const NAME_ARTICLES = new Set(['le', 'la', 'les'])

/**
 * Nom de famille d'une candidature, déduit de son nom affiché : le dernier mot,
 * précédé de son article quand il en a un (`Marine Le Pen` donne `Le Pen`).
 */
export function familyName(candidate: CandidateDto): string {
  const words = candidate.display_name.trim().split(/\s+/)
  const last = words[words.length - 1] ?? candidate.display_name
  const before = words[words.length - 2]
  if (words.length > 2 && before && NAME_ARTICLES.has(normalizeText(before))) {
    return `${before} ${last}`
  }
  return last
}

const collator = new Intl.Collator('fr', { sensitivity: 'base' })

/** Ordre alphabétique des noms de famille, sans égard aux accents ni à la casse. */
export function sortByFamilyName(candidates: CandidateDto[]): CandidateDto[] {
  return [...candidates].sort(
    (a, b) =>
      collator.compare(familyName(a), familyName(b)) ||
      collator.compare(a.display_name, b.display_name),
  )
}

/** Premier parti ou soutien déclaré, tel que la source le nomme. */
export function organizationLabel(candidate: CandidateDto): string {
  return candidate.political_organizations[0]?.label ?? ''
}

/** Identifiants du paramètre `candidats` : sans vide ni doublon, ordre conservé. */
export function parseCandidateIds(raw: string | null): string[] {
  const ids = (raw ?? '').split(',').map((value) => value.trim()).filter(Boolean)
  return [...new Set(ids)]
}
