import { capitalized } from '../../lib/entryFlow'
import type { WeekDayState } from '../../types/week'

// Libellés des jours de la semaine, communs aux pastilles et aux barres.

/** « Lun. 9 » (`short`) ou « Lundi 9 » (`long`). */
export function weekday(iso: string, weekday: 'short' | 'long') {
  const label = capitalized(new Date(iso + 'T00:00:00').toLocaleDateString('fr-FR', { weekday, day: 'numeric' }))
  // « Lun 9 » → « Lun. 9 » : l'abréviation garde son point.
  return weekday === 'short' ? label.replace(/^(\p{L}+)\.? /u, '$1. ') : label
}

/** Jour sans vote : un signe sur téléphone, sa légende à partir de 768 px (RM-06). */
export const EMPTY: Record<Exclude<WeekDayState, 'voted'>, { mark: string; caption: string }> = {
  no_public_vote: { mark: '—', caption: 'aucun vote public' },
  not_yet_published: { mark: '…', caption: 'pas encore publié' },
  upcoming: { mark: '…', caption: 'à venir' },
}
