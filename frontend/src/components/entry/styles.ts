import type { GroupPosition } from '../../lib/entryFlow'

/*
 * Habillage des positions de vote.
 *
 * Seules les couleurs de position (pour, contre, abstention) portent le sens du
 * vote ; la couleur d'un groupe n'apparaît jamais sur un élément qui dit une
 * position (README.md §6). Les teintes de barre sont celles du site, moins
 * saturées que le texte : à pleine saturation, vert contre rouge se lit comme
 * un feu tricolore, donc comme un jugement.
 */

/** Pastille pleine : la position seule, par la couleur et par le glyphe. */
export const SWATCH_CLASS: Record<GroupPosition, string> = {
  for: 'bg-yes-bar text-white',
  against: 'bg-no-bar text-white',
  abstention: 'bg-abstain-bar text-black/80',
  split: 'bg-info text-surface',
  nobody: 'border border-dashed border-ink-faint bg-surface-soft text-ink-soft',
  missing: 'border border-dashed border-ink-faint bg-surface-soft text-ink-soft',
}

/** Fond et texte du bandeau du groupe suivi. */
export const BANNER_CLASS: Record<GroupPosition, string> = {
  for: 'bg-yes-soft text-yes',
  against: 'bg-no-soft text-no',
  abstention: 'bg-abstain-soft text-abstain',
  split: 'bg-info-soft text-info',
  nobody: 'bg-surface-soft text-ink-soft',
  missing: 'bg-surface-soft text-ink-soft',
}

export const BADGE_CLASS: Record<GroupPosition, string> = {
  for: 'bg-yes-bar text-white',
  against: 'bg-no-bar text-white',
  abstention: 'bg-abstain-bar text-black/80',
  split: 'bg-info text-surface',
  nobody: 'bg-line-strong text-ink',
  missing: 'bg-line-strong text-ink',
}

export const BADGE_WORD: Record<GroupPosition, string> = {
  for: 'pour',
  against: 'contre',
  abstention: 'abstention',
  split: 'partagé',
  nobody: '—',
  missing: '—',
}

/** Contour de focus visible au clavier, commun à tous les contrôles du parcours. */
export const FOCUS_RING =
  'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent'

/**
 * Classes d'une grande tuile de choix (sujet, groupe, « tous les groupes »).
 * `layout` remplace la disposition par défaut (une ligne centrée) : deux
 * classes d'alignement concurrentes dans un même attribut ne se départagent pas.
 */
export function choiceTileClass(on: boolean, layout = 'items-center gap-3'): string {
  return `group flex w-full ${layout} rounded-2xl border-2 text-left text-ink transition-colors ${FOCUS_RING} ${
    on
      ? 'border-accent bg-accent-soft'
      : 'border-line bg-surface hover:border-line-strong'
  }`
}
