import type { DossierStatusDto } from '../types/dossiers'

export function formatDate(iso: string) {
  return new Date(iso + 'T00:00:00').toLocaleDateString('fr-FR', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
  })
}

/** Date sans l'année quand elle tombe dans l'année en cours. */
export function formatShortDate(iso: string, today: string) {
  if (iso.slice(0, 4) !== today.slice(0, 4)) return formatDate(iso)
  return new Date(iso + 'T00:00:00').toLocaleDateString('fr-FR', {
    day: 'numeric',
    month: 'short',
  })
}

/**
 * Lecture désignée par le préfixe du code d'acte. Le libellé n'est qu'un nom
 * de lecture : il ne dit rien du sort, que seuls les actes publiés portent.
 */
const READING_LABELS: Record<string, string> = {
  AN1: 'Assemblée · 1re lecture',
  SN1: 'Sénat · 1re lecture',
  AN2: 'Assemblée · 2e lecture',
  SN2: 'Sénat · 2e lecture',
  AN3: 'Assemblée · 3e lecture',
  SN3: 'Sénat · 3e lecture',
  ANLUNI: 'Assemblée · lecture unique',
  CMP: 'Commission mixte paritaire',
  ANNLEC: 'Assemblée · nouvelle lecture',
  SNNLEC: 'Sénat · nouvelle lecture',
  ANLDEF: 'Assemblée · lecture définitive',
  CC: 'Conseil constitutionnel',
  PROM: 'Promulgation',
}

/** Préfixe de lecture d'un code d'acte : « AN1 » pour « AN1-DEBATS-DEC ». */
export function readingKey(code: string | null | undefined) {
  return (code ?? '').split('-')[0]
}

export function readingLabel(key: string): string | undefined {
  return READING_LABELS[key]
}

/**
 * Un rejet n'est pas une fin : la navette peut reprendre (le texte budgétaire
 * rejeté par l'Assemblée part au Sénat). Le bandeau le dit, sans le rouge
 * d'une issue close.
 */
export const REJECTION_NOT_FINAL = 'la navette peut reprendre'

/**
 * Bandeau d'état : le sort quand la source en publie un, sinon l'étape qu'elle
 * atteste. Jamais « en cours » : sur la plupart des dossiers, ce serait une
 * affirmation que personne n'a écrite (README.md §6).
 */
export function dossierBanner(dossier: DossierStatusDto): { tone: string; text: string } {
  const { outcome, current_stage: stage } = dossier
  if (outcome.kind !== 'no_recorded_conclusion') {
    // Refonte A : plus de bandeau teinté, le sort est un mot ; vert pour une
    // promulgation seulement, un rejet reste à l'encre (il n'est pas définitif).
    const tone = outcome.kind === 'promulgated' ? 'text-yes' : 'text-ink'
    const date = outcome.date ? ` le ${formatDate(outcome.date)}` : ''
    const law = outcome.law_code ? ` · loi n° ${outcome.law_code}` : ''
    const shuttle = outcome.kind === 'rejected' ? ` · ${REJECTION_NOT_FINAL}` : ''
    return { tone, text: `${outcome.label}${date}${law}${shuttle}` }
  }
  if (stage) {
    return {
      tone: 'text-ink',
      text: `Étape : ${stage.label}${stage.chamber ? ` — ${stage.chamber}` : ''}`,
    }
  }
  return { tone: 'text-ink-soft', text: 'Sans conclusion à ce jour' }
}
