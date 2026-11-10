import type { OutcomeDto } from '../types/dossiers'

/** Ce que le bandeau d'état lit d'un dossier, quelle que soit la liste qui le montre. */
export interface DossierStatusLike {
  outcome: OutcomeDto
  current_stage: { label: string; chamber: string } | null
  last_activity_label: string
  last_activity_date: string
}

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
 * Bandeau d'état : le sort quand la source en publie un, sinon l'étape qu'elle
 * atteste. Jamais « en cours » : sur la plupart des dossiers, ce serait une
 * affirmation que personne n'a écrite (README.md §6).
 */
export function dossierBanner(dossier: DossierStatusLike): { tone: string; text: string } {
  const { outcome, current_stage: stage } = dossier
  if (outcome.kind !== 'no_recorded_conclusion') {
    const tone =
      outcome.kind === 'promulgated'
        ? 'bg-yes-soft text-yes'
        : outcome.kind === 'rejected'
          ? 'bg-no-soft text-no'
          : 'bg-surface-soft text-ink-soft'
    const date = outcome.date ? ` le ${formatDate(outcome.date)}` : ''
    const law = outcome.law_code ? ` · loi n° ${outcome.law_code}` : ''
    return { tone, text: `${outcome.label}${date}${law}` }
  }
  if (stage) {
    return {
      tone: 'bg-info-soft text-info',
      text: `Étape : ${stage.label}${stage.chamber ? ` — ${stage.chamber}` : ''}`,
    }
  }
  return { tone: 'bg-surface-soft text-ink-soft', text: 'Sans conclusion à ce jour' }
}
