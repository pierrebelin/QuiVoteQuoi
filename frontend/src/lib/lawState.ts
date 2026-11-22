import type { LawStateKind } from '../types/dossiers'

/**
 * Où en est la loi (todo/navette/SPEC-navette.md) : même libellé et même
 * couleur sur toutes les pages. Le vert est réservé aux textes qui vont
 * s'appliquer (RM-07).
 */
export const LAW_STATE_LABELS: Record<LawStateKind, string> = {
  in_shuttle: 'En navette',
  definitively_adopted: 'Adopté définitivement',
  became_law: 'Devenu loi',
}

/** Fond et texte de chaque état (RM-07). */
export const LAW_STATE_TONES: Record<LawStateKind, string> = {
  in_shuttle: 'bg-surface-soft text-ink',
  definitively_adopted: 'bg-yes-soft text-yes',
  became_law: 'bg-yes text-surface',
}
